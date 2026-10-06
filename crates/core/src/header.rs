//! Vault header (`header.scyh`): KDF parameters, salt and the wrapped vault key.
//! Byte layout: `docs/VAULT_FORMAT.md`.

use rand_core::CryptoRng;
use zeroize::Zeroizing;

use crate::crypto::{self, KdfParams, Key};
use crate::{Error, Hlc, Result, SecretKey, VaultId};

const MAGIC: &[u8; 4] = b"SCYH";
const FORMAT: u8 = 1;
const AAD_LEN: usize = 4 + 1 + 16 + 8 + 12 + 16 + Hlc::ENCODED_LEN; // 85
const WRAPPED_LEN: usize = crypto::NONCE_LEN + crypto::KEY_LEN + crypto::TAG_LEN; // 72
const LEN: usize = AAD_LEN + WRAPPED_LEN;
const KEK_INFO: &[u8] = b"scytale/v1/kek";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub vault_id: VaultId,
    /// Incremented on every master-password change.
    pub version: u64,
    pub kdf: KdfParams,
    pub salt: [u8; 16],
    /// Who wrote this header and when; breaks ties between concurrent password changes.
    pub written: Hlc,
    wrapped_key: [u8; WRAPPED_LEN],
}

impl Header {
    /// Creates a new vault. Returns the header and the vault key it wraps.
    /// Randomness is consumed in this order: vault id, salt, vault key, nonce.
    pub fn create(
        password: &str,
        secret_key: &SecretKey,
        kdf: KdfParams,
        written: Hlc,
        rng: &mut impl CryptoRng,
    ) -> Result<(Header, Key)> {
        let vault_id = VaultId::random(rng);
        let mut salt = [0u8; 16];
        rng.fill_bytes(&mut salt);
        let vault_key = Key::random(rng);
        let header = Self::wrap(vault_id, 1, kdf, salt, written, password, secret_key, &vault_key, rng)?;
        Ok((header, vault_key))
    }

    /// Unwraps the vault key. Fails with [`Error::WrongCredentials`] for a wrong password or
    /// Secret Key, without saying which.
    pub fn unlock(&self, password: &str, secret_key: &SecretKey) -> Result<Key> {
        let kek = derive_kek(password, secret_key, &self.salt, &self.vault_id, self.kdf)?;
        let raw = crypto::open(&kek, &self.aad(), &self.wrapped_key).map_err(|_| Error::WrongCredentials)?;
        let bytes: [u8; crypto::KEY_LEN] = raw.as_slice().try_into().map_err(|_| Error::Corrupted)?;
        Ok(Key::from_bytes(bytes))
    }

    /// New master password and/or KDF cost: re-wraps the same vault key under a fresh salt, so
    /// nothing else has to be re-encrypted.
    pub fn change_password(
        &self,
        vault_key: &Key,
        new_password: &str,
        secret_key: &SecretKey,
        kdf: KdfParams,
        written: Hlc,
        rng: &mut impl CryptoRng,
    ) -> Result<Header> {
        let mut salt = [0u8; 16];
        rng.fill_bytes(&mut salt);
        Self::wrap(self.vault_id, self.version + 1, kdf, salt, written, new_password, secret_key, vault_key, rng)
    }

    /// True if `self` should replace `other` (higher version; ties broken by writer timestamp).
    pub fn supersedes(&self, other: &Header) -> bool {
        (self.version, self.written) > (other.version, other.written)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = self.aad();
        out.extend_from_slice(&self.wrapped_key);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Header> {
        if bytes.len() < 5 || &bytes[..4] != MAGIC {
            return Err(Error::Corrupted);
        }
        if bytes[4] != FORMAT {
            return Err(Error::UnsupportedVersion(bytes[4]));
        }
        if bytes.len() != LEN {
            return Err(Error::Corrupted);
        }
        let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
        Ok(Header {
            vault_id: VaultId(bytes[5..21].try_into().expect("16 bytes")),
            version: u64::from_le_bytes(bytes[21..29].try_into().expect("8 bytes")),
            kdf: KdfParams { m_kib: u32_at(29), t: u32_at(33), p: u32_at(37) }.validate()?,
            salt: bytes[41..57].try_into().expect("16 bytes"),
            written: Hlc::decode(bytes[57..85].try_into().expect("28 bytes")),
            wrapped_key: bytes[AAD_LEN..].try_into().expect("72 bytes"),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn wrap(
        vault_id: VaultId,
        version: u64,
        kdf: KdfParams,
        salt: [u8; 16],
        written: Hlc,
        password: &str,
        secret_key: &SecretKey,
        vault_key: &Key,
        rng: &mut impl CryptoRng,
    ) -> Result<Header> {
        let mut header =
            Header { vault_id, version, kdf: kdf.validate()?, salt, written, wrapped_key: [0; WRAPPED_LEN] };
        let kek = derive_kek(password, secret_key, &salt, &vault_id, kdf)?;
        let sealed = crypto::seal(&kek, &header.aad(), vault_key.as_bytes(), rng);
        header.wrapped_key = sealed.try_into().expect("nonce + key + tag");
        Ok(header)
    }

    /// Every header field except the wrapped key itself.
    fn aad(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(LEN);
        out.extend_from_slice(MAGIC);
        out.push(FORMAT);
        out.extend_from_slice(&self.vault_id.0);
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.kdf.m_kib.to_le_bytes());
        out.extend_from_slice(&self.kdf.t.to_le_bytes());
        out.extend_from_slice(&self.kdf.p.to_le_bytes());
        out.extend_from_slice(&self.salt);
        self.written.encode(&mut out);
        debug_assert_eq!(out.len(), AAD_LEN);
        out
    }
}

/// `kek = HKDF-SHA256(ikm = Argon2id(password, salt) ‖ secret_key, salt = vault_id, info = "scytale/v1/kek")`
fn derive_kek(
    password: &str,
    secret_key: &SecretKey,
    salt: &[u8; 16],
    vault_id: &VaultId,
    kdf: KdfParams,
) -> Result<Key> {
    let pw_key = crypto::argon2id(password, salt, kdf)?;
    let mut ikm = Zeroizing::new([0u8; 48]);
    ikm[..32].copy_from_slice(pw_key.as_bytes());
    ikm[32..].copy_from_slice(secret_key.as_bytes());
    Ok(crypto::hkdf(&*ikm, &vault_id.0, KEK_INFO))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::DeviceId;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    pub(crate) const FAST: KdfParams = KdfParams { m_kib: KdfParams::MIN_M_KIB, t: 1, p: 1 };

    fn hlc(ms: u64) -> Hlc {
        Hlc { wall_ms: ms, counter: 0, device: DeviceId([1; 16]) }
    }

    #[test]
    fn create_unlock_change_password() {
        let rng = &mut ChaCha20Rng::seed_from_u64(1);
        let sk = SecretKey::generate(rng);
        let (h, vk) = Header::create("hunter2", &sk, FAST, hlc(1), rng).unwrap();

        let decoded = Header::decode(&h.encode()).unwrap();
        assert_eq!(decoded, h);
        assert_eq!(decoded.unlock("hunter2", &sk).unwrap().as_bytes(), vk.as_bytes());

        assert_eq!(h.unlock("hunter3", &sk).unwrap_err(), Error::WrongCredentials);
        let other_sk = SecretKey::generate(rng);
        assert_eq!(h.unlock("hunter2", &other_sk).unwrap_err(), Error::WrongCredentials);

        let h2 = h.change_password(&vk, "correct horse", &sk, FAST, hlc(2), rng).unwrap();
        assert_eq!(h2.version, 2);
        assert!(h2.supersedes(&h) && !h.supersedes(&h2));
        assert_eq!(h2.unlock("correct horse", &sk).unwrap().as_bytes(), vk.as_bytes());
        assert_eq!(h2.unlock("hunter2", &sk).unwrap_err(), Error::WrongCredentials);
    }

    #[test]
    fn every_header_byte_is_authenticated() {
        let rng = &mut ChaCha20Rng::seed_from_u64(2);
        let sk = SecretKey::generate(rng);
        let (h, _) = Header::create("pw", &sk, FAST, hlc(1), rng).unwrap();
        let bytes = h.encode();
        // Skip magic/format (rejected by decode itself) and the KDF params (bounds-checked; a
        // changed-but-valid param simply derives a different KEK — covered by the loop anyway).
        for i in 5..bytes.len() {
            let mut bad = bytes.clone();
            bad[i] ^= 0x01;
            if let Ok(hdr) = Header::decode(&bad) {
                assert!(hdr.unlock("pw", &sk).is_err(), "byte {i} not authenticated");
            }
        }
    }

    #[test]
    fn decode_rejects_bad_input() {
        assert_eq!(Header::decode(b"nope"), Err(Error::Corrupted));
        let mut v = b"SCYH".to_vec();
        v.push(9);
        assert_eq!(Header::decode(&v), Err(Error::UnsupportedVersion(9)));
        v[4] = 1;
        assert_eq!(Header::decode(&v), Err(Error::Corrupted));
    }
}
