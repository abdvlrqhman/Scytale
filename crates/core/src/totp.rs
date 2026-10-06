//! TOTP codes (RFC 6238) from `otpauth://` URIs or bare base32 secrets.

use hmac::{EagerHash, Hmac, KeyInit, Mac};
use url::Url;
use zeroize::Zeroizing;

use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Totp {
    secret: Zeroizing<Vec<u8>>,
    pub algorithm: Algorithm,
    pub digits: u32,
    pub period: u64,
}

impl Totp {
    pub fn new(secret: Vec<u8>, algorithm: Algorithm, digits: u32, period: u64) -> Result<Self> {
        if secret.is_empty() || !(6..=8).contains(&digits) || !(1..=300).contains(&period) {
            return Err(Error::Invalid("invalid TOTP settings".into()));
        }
        Ok(Self { secret: Zeroizing::new(secret), algorithm, digits, period })
    }

    /// Accepts `otpauth://totp/...?secret=...` or a bare base32 secret (spaces and case ignored).
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        if !input.to_ascii_lowercase().starts_with("otpauth://") {
            return Self::new(base32_decode(input)?, Algorithm::Sha1, 6, 30);
        }
        let url = Url::parse(input).map_err(|_| Error::Invalid("invalid otpauth URI".into()))?;
        if url.host_str() != Some("totp") {
            return Err(Error::Invalid("only TOTP codes are supported".into()));
        }
        let (mut secret, mut algorithm, mut digits, mut period) = (None, Algorithm::Sha1, 6, 30);
        for (k, v) in url.query_pairs() {
            match k.to_ascii_lowercase().as_str() {
                "secret" => secret = Some(base32_decode(&v)?),
                "algorithm" => {
                    algorithm = match v.to_ascii_uppercase().as_str() {
                        "SHA1" => Algorithm::Sha1,
                        "SHA256" => Algorithm::Sha256,
                        "SHA512" => Algorithm::Sha512,
                        _ => return Err(Error::Invalid("unsupported TOTP algorithm".into())),
                    }
                }
                "digits" => digits = v.parse().map_err(|_| Error::Invalid("invalid TOTP digits".into()))?,
                "period" => period = v.parse().map_err(|_| Error::Invalid("invalid TOTP period".into()))?,
                _ => {}
            }
        }
        Self::new(secret.ok_or_else(|| Error::Invalid("otpauth URI has no secret".into()))?, algorithm, digits, period)
    }

    /// The code valid at `unix_secs`, zero-padded.
    pub fn code(&self, unix_secs: u64) -> String {
        let counter = (unix_secs / self.period).to_be_bytes();
        let digest = match self.algorithm {
            Algorithm::Sha1 => hmac::<sha1::Sha1>(&self.secret, &counter),
            Algorithm::Sha256 => hmac::<sha2::Sha256>(&self.secret, &counter),
            Algorithm::Sha512 => hmac::<sha2::Sha512>(&self.secret, &counter),
        };
        let offset = (digest[digest.len() - 1] & 0x0f) as usize;
        let bin = u32::from_be_bytes(digest[offset..offset + 4].try_into().expect("4 bytes")) & 0x7fff_ffff;
        format!("{:0width$}", bin % 10u32.pow(self.digits), width = self.digits as usize)
    }

    pub fn seconds_remaining(&self, unix_secs: u64) -> u64 {
        self.period - unix_secs % self.period
    }
}

fn hmac<D: EagerHash>(key: &[u8], msg: &[u8]) -> Vec<u8>
where
    Hmac<D>: KeyInit + Mac,
{
    let mut mac = <Hmac<D> as KeyInit>::new_from_slice(key).expect("HMAC takes any key length");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

/// RFC 4648 base32 decode; ignores case, spaces, dashes and `=` padding.
fn base32_decode(input: &str) -> Result<Vec<u8>> {
    let (mut buf, mut bits, mut out) = (0u64, 0u32, Vec::new());
    for c in input.chars().filter(|c| !matches!(c, ' ' | '-' | '=')) {
        let v = match c.to_ascii_uppercase() {
            c @ 'A'..='Z' => c as u64 - 'A' as u64,
            c @ '2'..='7' => c as u64 - '2' as u64 + 26,
            _ => return Err(Error::Invalid("TOTP secret is not valid base32".into())),
        };
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    if out.is_empty() {
        return Err(Error::Invalid("TOTP secret is empty".into()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 Appendix B.
    #[test]
    fn rfc6238_vectors() {
        let sha1 = Totp::new(b"12345678901234567890".to_vec(), Algorithm::Sha1, 8, 30).unwrap();
        let sha256 = Totp::new(b"12345678901234567890123456789012".to_vec(), Algorithm::Sha256, 8, 30).unwrap();
        let sha512 =
            Totp::new(b"1234567890".repeat(6).into_iter().chain(*b"1234").collect(), Algorithm::Sha512, 8, 30).unwrap();
        let table = [
            (59, "94287082", "46119246", "90693936"),
            (1111111109, "07081804", "68084774", "25091201"),
            (1111111111, "14050471", "67062674", "99943326"),
            (1234567890, "89005924", "91819424", "93441116"),
            (2000000000, "69279037", "90698825", "38618901"),
            (20000000000, "65353130", "77737706", "47863826"),
        ];
        for (t, a, b, c) in table {
            assert_eq!(sha1.code(t), a, "SHA1 @ {t}");
            assert_eq!(sha256.code(t), b, "SHA256 @ {t}");
            assert_eq!(sha512.code(t), c, "SHA512 @ {t}");
        }
    }

    #[test]
    fn parses_uris_and_bare_secrets() {
        // "12345678901234567890" in base32.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let bare = Totp::parse(&secret.to_lowercase()).unwrap();
        assert_eq!(bare.code(59), "287082");
        let uri =
            Totp::parse(&format!("otpauth://totp/Ex:me?secret={secret}&issuer=Ex&digits=8&algorithm=SHA1")).unwrap();
        assert_eq!(uri.code(59), "94287082");
        assert_eq!(uri.seconds_remaining(59), 1);
        assert!(Totp::parse("otpauth://hotp/x?secret=GEZD").is_err());
        assert!(Totp::parse("not base32!").is_err());
        assert!(Totp::parse("otpauth://totp/x?secret=GEZD&digits=12").is_err());
    }
}
