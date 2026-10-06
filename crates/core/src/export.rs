//! Exports: a portable encrypted file (all item types) and a plain CSV (logins and notes, readable by
//! browsers and most password managers).

use rand_core::CryptoRng;
use zeroize::Zeroizing;

use crate::crypto::{self, KdfParams};
use crate::{Error, Item, ItemData, Result, Vault};

const MAGIC: &[u8; 4] = b"SCYX";
const FORMAT: u8 = 1;
const AAD_LEN: usize = 4 + 1 + 12 + 16; // 33
const EXPORT_INFO: &[u8] = b"scytale/v1/export";

/// Encrypts every live item under a key derived from `password` alone (no Secret Key), so the
/// file can be opened on any machine.
pub fn encrypted(vault: &Vault, password: &str, kdf: KdfParams, rng: &mut impl CryptoRng) -> Result<Vec<u8>> {
    let items: Vec<&Item> = vault.items().map(|(_, i)| i).collect();
    let json = Zeroizing::new(serde_json::to_vec(&items).map_err(|e| Error::Invalid(e.to_string()))?);
    let mut salt = [0u8; 16];
    rng.fill_bytes(&mut salt);
    let mut out = Vec::with_capacity(AAD_LEN);
    out.extend_from_slice(MAGIC);
    out.push(FORMAT);
    for v in [kdf.m_kib, kdf.t, kdf.p] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&salt);
    let key = crypto::hkdf(crypto::argon2id(password, &salt, kdf)?.as_bytes(), &salt, EXPORT_INFO);
    let sealed = crypto::seal(&key, &out, &crypto::pad(&json)?, rng);
    out.extend_from_slice(&sealed);
    Ok(out)
}

pub fn open_encrypted(bytes: &[u8], password: &str) -> Result<Vec<Item>> {
    if bytes.len() < 5 || &bytes[..4] != MAGIC {
        return Err(Error::Corrupted);
    }
    if bytes[4] != FORMAT {
        return Err(Error::UnsupportedVersion(bytes[4]));
    }
    if bytes.len() < AAD_LEN {
        return Err(Error::Corrupted);
    }
    let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
    let kdf = KdfParams { m_kib: u32_at(5), t: u32_at(9), p: u32_at(13) };
    let salt = &bytes[17..33];
    let key = crypto::hkdf(crypto::argon2id(password, salt, kdf)?.as_bytes(), salt, EXPORT_INFO);
    let padded = crypto::open(&key, &bytes[..AAD_LEN], &bytes[AAD_LEN..]).map_err(|_| Error::WrongCredentials)?;
    serde_json::from_slice(crypto::unpad(&padded)?).map_err(|_| Error::Corrupted)
}

/// Chrome-compatible CSV (`name,url,username,password,note,totp`). Cards and identities cannot be
/// represented and are counted in the second return value so the UI can warn.
pub fn csv(vault: &Vault) -> Result<(Zeroizing<String>, usize)> {
    let mut w = csv::Writer::from_writer(Vec::new());
    let mut omitted = 0;
    let err = |e: csv::Error| Error::Invalid(e.to_string());
    w.write_record(["name", "url", "username", "password", "note", "totp"]).map_err(err)?;
    for (_, item) in vault.items() {
        match &item.data {
            ItemData::Login(l) => {
                let mut note = item.notes.clone();
                if l.urls.len() > 1 {
                    note.push_str(&format!(
                        "{}Other URLs: {}",
                        if note.is_empty() { "" } else { "\n" },
                        l.urls[1..].join(" ")
                    ));
                }
                let url = l.urls.first().map(String::as_str).unwrap_or("");
                w.write_record([item.title.as_str(), url, &l.username, &l.password, &note, &l.totp]).map_err(err)?;
            }
            ItemData::Note => w.write_record([item.title.as_str(), "", "", "", &item.notes, ""]).map_err(err)?,
            ItemData::Card(_) | ItemData::Identity(_) => omitted += 1,
        }
    }
    let bytes = w.into_inner().map_err(|e| Error::Invalid(e.to_string()))?;
    Ok((Zeroizing::new(String::from_utf8(bytes).expect("CSV of UTF-8 strings")), omitted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Card, DeviceId, ItemId, Login};
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    const FAST: KdfParams = KdfParams { m_kib: KdfParams::MIN_M_KIB, t: 1, p: 1 };

    fn vault() -> Vault {
        let mut v = Vault::new();
        let d = DeviceId([1; 16]);
        let login = Login {
            username: "u".into(),
            password: "p,\"q".into(),
            urls: vec!["https://a.com".into(), "https://b.com".into()],
            ..Default::default()
        };
        v.upsert(
            ItemId([1; 16]),
            Item { title: "A".into(), notes: String::new(), favorite: false, data: ItemData::Login(login) },
            1,
            d,
        );
        v.upsert(
            ItemId([2; 16]),
            Item { title: "Card".into(), notes: String::new(), favorite: false, data: ItemData::Card(Card::default()) },
            2,
            d,
        );
        v
    }

    #[test]
    fn encrypted_round_trip() {
        let v = vault();
        let rng = &mut ChaCha20Rng::seed_from_u64(1);
        let bytes = encrypted(&v, "export pw", FAST, rng).unwrap();
        let items = open_encrypted(&bytes, "export pw").unwrap();
        assert_eq!(items, v.items().map(|(_, i)| i.clone()).collect::<Vec<_>>());
        assert_eq!(open_encrypted(&bytes, "wrong"), Err(Error::WrongCredentials));
        let mut bad = bytes.clone();
        bad[20] ^= 1; // salt is authenticated
        assert!(open_encrypted(&bad, "export pw").is_err());
    }

    #[test]
    fn csv_round_trips_through_importer() {
        let (text, omitted) = csv(&vault()).unwrap();
        assert_eq!(omitted, 1);
        let back = crate::import::csv(&text).unwrap();
        let ItemData::Login(l) = &back.items[0].data else { panic!() };
        assert_eq!((l.password.as_str(), l.urls[0].as_str()), ("p,\"q", "https://a.com"));
        assert_eq!(back.items[0].notes, "Other URLs: https://b.com");
    }
}
