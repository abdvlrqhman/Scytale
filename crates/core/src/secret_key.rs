//! The 128-bit Secret Key: generated once per vault, kept on devices and on the Emergency Kit,
//! never uploaded. Format: `S1-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXX` (Crockford base32 + 2 check chars).

use core::fmt;

use rand_core::CryptoRng;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{Error, Result};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const PREFIX: &str = "S1";
const KEY_CHARS: usize = 26; // 128 bits → 26 × 5 bits (last 2 bits zero)
const CHECK_CHARS: usize = 2; // 10 bits: catches ~99.9 % of typos

#[derive(Clone)]
pub struct SecretKey(Zeroizing<[u8; 16]>);

impl SecretKey {
    pub fn generate(rng: &mut impl CryptoRng) -> Self {
        let mut b = Zeroizing::new([0u8; 16]);
        rng.fill_bytes(&mut *b);
        Self(b)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Human-readable form for the Emergency Kit and for typing on a new device.
    pub fn to_display(&self) -> Zeroizing<String> {
        let mut chars = Zeroizing::new(encode(&self.0));
        chars.push_str(&check_chars(&self.0));
        let mut out = Zeroizing::new(String::from(PREFIX));
        for group in chars.as_bytes().chunks(5) {
            out.push('-');
            out.push_str(core::str::from_utf8(group).expect("ASCII"));
        }
        out
    }

    /// Accepts any case, any dashes/spaces, and the Crockford look-alikes O→0, I/L→1.
    pub fn parse(input: &str) -> Result<Self> {
        let cleaned: Zeroizing<String> = Zeroizing::new(
            input.chars().filter(|c| !c.is_whitespace() && *c != '-').collect::<String>().to_ascii_uppercase(),
        );
        // The version digit gets the same look-alike forgiveness as the rest ("SL-…", "SI-…").
        let body = cleaned.strip_prefix('S').map(normalize).transpose()?;
        let body = body.as_deref().and_then(|b| b.strip_prefix(&PREFIX[1..]));
        let body = body.ok_or(Error::InvalidSecretKey("it must start with S1"))?;
        if body.len() != KEY_CHARS + CHECK_CHARS {
            return Err(Error::InvalidSecretKey("wrong length"));
        }
        let (key_part, check_part) = body.split_at(KEY_CHARS);
        let key = decode(key_part)?;
        if check_chars(&key) != check_part {
            return Err(Error::InvalidSecretKey("there is a typo"));
        }
        Ok(Self(key))
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey(<redacted>)")
    }
}

fn encode(bytes: &[u8; 16]) -> String {
    let n = u128::from_be_bytes(*bytes);
    // 130 bits of output: the key's 128 bits followed by two zero bits.
    (0..KEY_CHARS)
        .map(|i| {
            let shift = 128 + 2 - 5 * (i + 1);
            let v = if shift >= 2 { (n >> (shift - 2)) & 31 } else { (n << (2 - shift)) & 31 };
            ALPHABET[v as usize] as char
        })
        .collect()
}

fn decode(chars: &str) -> Result<Zeroizing<[u8; 16]>> {
    let mut acc: u128 = 0;
    let mut tail = 0u8;
    for (i, c) in normalize(chars)?.bytes().enumerate() {
        let v = ALPHABET.iter().position(|&a| a == c).expect("normalized") as u128;
        if i < KEY_CHARS - 1 {
            acc = (acc << 5) | v;
        } else {
            // Last char: 3 key bits + 2 padding bits that must be zero.
            acc = (acc << 3) | (v >> 2);
            tail = (v & 0b11) as u8;
        }
    }
    if tail != 0 {
        return Err(Error::InvalidSecretKey("there is a typo"));
    }
    Ok(Zeroizing::new(acc.to_be_bytes()))
}

fn normalize(chars: &str) -> Result<String> {
    chars
        .chars()
        .map(|c| match c {
            'O' => Ok('0'),
            'I' | 'L' => Ok('1'),
            c if c.is_ascii() && ALPHABET.contains(&(c as u8)) => Ok(c),
            _ => Err(Error::InvalidSecretKey("it contains a character that is not allowed")),
        })
        .collect()
}

fn check_chars(key: &[u8; 16]) -> String {
    let digest = Sha256::new().chain_update(b"scytale/v1/secret-key-check").chain_update(key).finalize();
    let bits = u16::from_be_bytes([digest[0], digest[1]]) >> 6; // top 10 bits
    [ALPHABET[(bits >> 5) as usize] as char, ALPHABET[(bits & 31) as usize] as char].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    #[test]
    fn display_parse_round_trip() {
        let mut rng = ChaCha20Rng::seed_from_u64(1);
        for _ in 0..500 {
            let k = SecretKey::generate(&mut rng);
            let shown = k.to_display();
            assert_eq!(shown.len(), 2 + 6 + KEY_CHARS + CHECK_CHARS);
            assert_eq!(SecretKey::parse(&shown).unwrap().as_bytes(), k.as_bytes());
        }
    }

    #[test]
    fn known_encoding() {
        let k = SecretKey::from_bytes([0xff; 16]);
        assert!(k.to_display().starts_with("S1-ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-W"));
        let zero = SecretKey::from_bytes([0; 16]);
        assert!(zero.to_display().starts_with("S1-00000-00000-00000-00000-00000-0"));
    }

    #[test]
    fn parse_is_forgiving_about_formatting() {
        let k = SecretKey::generate(&mut ChaCha20Rng::seed_from_u64(2));
        let shown = k.to_display().to_lowercase().replace('-', " ").replace('0', "o").replace('1', "l");
        assert_eq!(SecretKey::parse(&shown).unwrap().as_bytes(), k.as_bytes());
    }

    #[test]
    fn parse_catches_typos() {
        let k = SecretKey::generate(&mut ChaCha20Rng::seed_from_u64(3));
        let shown = k.to_display().to_string();
        let mut caught = 0;
        let mut total = 0;
        for (i, c) in shown.char_indices().skip(3).filter(|(_, c)| *c != '-') {
            for &r in ALPHABET.iter().filter(|&&r| r as char != c) {
                let mut typo = shown.clone();
                typo.replace_range(i..i + 1, &(r as char).to_string());
                total += 1;
                if SecretKey::parse(&typo).is_err() {
                    caught += 1;
                }
            }
        }
        assert!(caught * 1000 >= total * 995, "caught {caught}/{total}");
        assert!(SecretKey::parse("S2-00000").is_err());
        assert!(SecretKey::parse("S1-0000").is_err());
        assert!(SecretKey::parse(&shown.replace(&shown[4..5], "U")).is_err());
    }
}
