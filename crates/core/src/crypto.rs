//! Primitives: Argon2id + HKDF key derivation, XChaCha20-Poly1305 sealing, Padmé padding.
//! Spec: `docs/CRYPTO.md`.

use core::fmt;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use rand_core::CryptoRng;
use sha2::Sha256;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

use crate::{Error, Result};

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 24;
pub const TAG_LEN: usize = 16;

/// Argon2id cost parameters. Stored in the vault header so they can be raised later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KdfParams {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl KdfParams {
    /// 64 MiB, 3 passes: ~135 ms in WASM on a fast laptop (benchmarks in docs/CRYPTO.md).
    pub const DEFAULT: Self = Self { m_kib: 64 * 1024, t: 3, p: 1 };
    /// OWASP's Argon2id minimum. Nothing weaker is ever created or accepted.
    pub const MIN_M_KIB: u32 = 19 * 1024;
    /// A tampered header must not be able to make every unlock allocate unbounded memory before
    /// the authentication check fails.
    pub const MAX_M_KIB: u32 = 1024 * 1024;
    pub const MAX_T: u32 = 16;
    pub const MAX_P: u32 = 8;

    pub fn validate(self) -> Result<Self> {
        let ok = (Self::MIN_M_KIB..=Self::MAX_M_KIB).contains(&self.m_kib)
            && (1..=Self::MAX_T).contains(&self.t)
            && (1..=Self::MAX_P).contains(&self.p);
        if ok { Ok(self) } else { Err(Error::InvalidKdfParams) }
    }
}

/// A 256-bit symmetric key, wiped from memory on drop.
#[derive(Clone)]
pub struct Key(Zeroizing<[u8; KEY_LEN]>);

impl Key {
    pub fn random(rng: &mut impl CryptoRng) -> Self {
        let mut k = Zeroizing::new([0u8; KEY_LEN]);
        rng.fill_bytes(&mut *k);
        Self(k)
    }

    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Key(<redacted>)")
    }
}

/// Argon2id over the NFKC-normalized password, so the same password typed on different
/// keyboards/OSes always derives the same key.
pub(crate) fn argon2id(password: &str, salt: &[u8], params: KdfParams) -> Result<Key> {
    let params = params.validate()?;
    let argon = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(params.m_kib, params.t, params.p, Some(KEY_LEN)).map_err(|_| Error::InvalidKdfParams)?,
    );
    let normalized = Zeroizing::new(password.nfkc().collect::<String>());
    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    argon.hash_password_into(normalized.as_bytes(), salt, &mut *out).map_err(|_| Error::InvalidKdfParams)?;
    Ok(Key(out))
}

/// HKDF-SHA256 expand to one key.
pub(crate) fn hkdf(ikm: &[u8], salt: &[u8], info: &[u8]) -> Key {
    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    Hkdf::<Sha256>::new(Some(salt), ikm)
        .expand(info, &mut *out)
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    Key(out)
}

/// XChaCha20-Poly1305 with a fresh random nonce. Output: `nonce ‖ ciphertext ‖ tag`.
pub(crate) fn seal(key: &Key, aad: &[u8], plaintext: &[u8], rng: &mut impl CryptoRng) -> Vec<u8> {
    let mut nonce = [0u8; NONCE_LEN];
    rng.fill_bytes(&mut nonce);
    let ct = XChaCha20Poly1305::new(key.as_bytes().into())
        .encrypt(&XNonce::from(nonce), Payload { msg: plaintext, aad })
        .expect("in-memory encryption cannot fail");
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    out
}

/// Inverse of [`seal`]. Any modification of the nonce, ciphertext, tag or AAD fails.
pub(crate) fn open(key: &Key, aad: &[u8], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if sealed.len() < NONCE_LEN + TAG_LEN {
        return Err(Error::Corrupted);
    }
    let (nonce, ct) = sealed.split_at(NONCE_LEN);
    let nonce = XNonce::try_from(nonce).map_err(|_| Error::Corrupted)?;
    XChaCha20Poly1305::new(key.as_bytes().into())
        .decrypt(&nonce, Payload { msg: ct, aad })
        .map(Zeroizing::new)
        .map_err(|_| Error::Corrupted)
}

/// Smallest padded size; hides the size of small vaults completely.
const MIN_PADDED: usize = 4096;

/// Padmé (Nikitin et al., PURBs 2019): rounds `len` up so at most O(log log len) bits of the
/// length leak, with ≤ 12 % overhead.
pub(crate) fn padme(len: usize) -> usize {
    if len < 2 {
        return len;
    }
    let e = usize::BITS - 1 - len.leading_zeros(); // floor(log2 len)
    let s = u32::BITS - 1 - e.leading_zeros() + 1; // floor(log2 e) + 1
    let last_bits = e - s;
    let mask = (1usize << last_bits) - 1;
    (len + mask) & !mask
}

/// Frames `data` as `u32 length ‖ data ‖ zeros` padded to `padme(max(4 + len, MIN_PADDED))`.
pub(crate) fn pad(data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    let len = u32::try_from(data.len()).map_err(|_| Error::Invalid("vault too large".into()))?;
    let total = padme((4 + data.len()).max(MIN_PADDED));
    let mut out = Zeroizing::new(Vec::with_capacity(total));
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(data);
    out.resize(total, 0);
    Ok(out)
}

pub(crate) fn unpad(buf: &[u8]) -> Result<&[u8]> {
    let len = buf.get(..4).ok_or(Error::Corrupted)?;
    let len = u32::from_le_bytes(len.try_into().expect("4 bytes")) as usize;
    buf.get(4..4 + len).ok_or(Error::Corrupted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    fn rng() -> ChaCha20Rng {
        ChaCha20Rng::seed_from_u64(7)
    }

    #[test]
    fn seal_open_round_trip_and_tamper_detection() {
        let key = Key::random(&mut rng());
        let sealed = seal(&key, b"aad", b"secret", &mut rng());
        assert_eq!(&**open(&key, b"aad", &sealed).unwrap(), b"secret");

        assert_eq!(open(&key, b"other aad", &sealed), Err(Error::Corrupted));
        assert_eq!(open(&Key::from_bytes([9; 32]), b"aad", &sealed), Err(Error::Corrupted));
        for i in 0..sealed.len() {
            let mut bad = sealed.clone();
            bad[i] ^= 1;
            assert_eq!(open(&key, b"aad", &bad), Err(Error::Corrupted), "flipped byte {i}");
        }
        assert_eq!(open(&key, b"aad", &sealed[..10]), Err(Error::Corrupted));
    }

    #[test]
    fn padme_matches_paper_and_bounds_overhead() {
        // Hand-computed from the paper's formula.
        assert_eq!(padme(0), 0);
        assert_eq!(padme(1), 1);
        assert_eq!(padme(9), 10);
        assert_eq!(padme(1000), 1024);
        assert_eq!(padme(4097), 4352);
        for len in 2..200_000usize {
            let p = padme(len);
            assert!(p >= len && p - len <= len / 8 + 1, "len {len} -> {p}");
        }
    }

    #[test]
    fn pad_unpad_round_trip_and_hides_small_sizes() {
        let a = pad(b"").unwrap();
        let b = pad(&[7; 3000]).unwrap();
        assert_eq!(a.len(), b.len());
        assert_eq!(unpad(&b).unwrap(), &[7; 3000][..]);
        assert_eq!(unpad(&[1, 0]), Err(Error::Corrupted));
        assert_eq!(unpad(&[200, 0, 0, 0, 1]), Err(Error::Corrupted));
    }

    #[test]
    fn nfkc_makes_equivalent_passwords_derive_the_same_key() {
        let p = KdfParams { m_kib: KdfParams::MIN_M_KIB, t: 1, p: 1 };
        let composed = argon2id("p\u{e4}ss", &[1; 16], p).unwrap(); // ä as one code point
        let decomposed = argon2id("pa\u{308}ss", &[1; 16], p).unwrap(); // a + combining diaeresis
        assert_eq!(composed.as_bytes(), decomposed.as_bytes());
    }

    #[test]
    fn kdf_params_bounds() {
        assert!(KdfParams::DEFAULT.validate().is_ok());
        assert!(KdfParams { m_kib: 1024, t: 3, p: 1 }.validate().is_err());
        assert!(KdfParams { m_kib: u32::MAX, t: 3, p: 1 }.validate().is_err());
        assert!(KdfParams { m_kib: 65536, t: 0, p: 1 }.validate().is_err());
        assert!(KdfParams { m_kib: 65536, t: 3, p: 0 }.validate().is_err());
    }
}
