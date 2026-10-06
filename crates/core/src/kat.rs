//! Known-answer tests.
//!
//! 1. Published vectors (RFC 9106, RFC 5869, draft-irtf-cfrg-xchacha-03) pin the primitives.
//! 2. `tests/vectors/chain.json` pins the full derivation chain and both file formats. It is
//!    re-derived independently by `tests/vectors/crosscheck.py` (libsodium, argon2-cffi, pyca).
//!    Regenerate deliberately with `SCYTALE_WRITE_VECTORS=1 cargo test -p scytale-core kat`.

use core::convert::Infallible;

use argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};
use hkdf::Hkdf;
use rand_core::{TryCryptoRng, TryRng};
use serde_json::{Value, json};
use sha2::Sha256;

use crate::crypto::{self, KdfParams, Key};
use crate::snapshot::{self, Snapshot, SnapshotMeta};
use crate::{DeviceId, Header, Hlc, Item, ItemData, ItemId, Login, SecretKey, Vault};

/// Hands out predetermined bytes, so "random" values in a KAT are fixed and documented.
struct Replay(Vec<u8>);

impl TryRng for Replay {
    type Error = Infallible;
    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        let mut b = [0; 4];
        self.try_fill_bytes(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }
    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        let mut b = [0; 8];
        self.try_fill_bytes(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }
    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        assert!(self.0.len() >= dst.len(), "replay stream exhausted");
        dst.copy_from_slice(&self.0[..dst.len()]);
        self.0.drain(..dst.len());
        Ok(())
    }
}
impl TryCryptoRng for Replay {}

fn seq(start: u8, len: u8) -> Vec<u8> {
    (start..start + len).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    hex::decode(s.replace(' ', "")).unwrap()
}

#[test]
fn rfc9106_argon2id() {
    let params = ParamsBuilder::new()
        .m_cost(32)
        .t_cost(3)
        .p_cost(4)
        .data(AssociatedData::new(&[4; 12]).unwrap())
        .output_len(32)
        .build()
        .unwrap();
    let argon = Argon2::new_with_secret(&[3; 8], Algorithm::Argon2id, Version::V0x13, params).unwrap();
    let mut out = [0u8; 32];
    argon.hash_password_into(&[1; 32], &[2; 16], &mut out).unwrap();
    assert_eq!(out.to_vec(), unhex("0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659"));
}

#[test]
fn rfc5869_hkdf_case_1() {
    let mut okm = [0u8; 42];
    Hkdf::<Sha256>::new(Some(&unhex("000102030405060708090a0b0c")), &[0x0b; 22])
        .expand(&unhex("f0f1f2f3f4f5f6f7f8f9"), &mut okm)
        .unwrap();
    assert_eq!(
        okm.to_vec(),
        unhex("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865")
    );
}

#[test]
fn xchacha20poly1305_draft_a3_through_our_seal() {
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let key = Key::from_bytes(seq(0x80, 32).try_into().unwrap());
    let aad = unhex("50515253c0c1c2c3c4c5c6c7");
    let sealed = crypto::seal(&key, &aad, plaintext, &mut Replay(seq(0x40, 24)));
    let expected_ct = unhex(
        "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb
         731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452
         2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9
         21f9664c97637da9768812f615c68b13b52e
         c0875924c1c7987947deafd8780acf49"
            .replace(['\n', ' '], "")
            .as_str(),
    );
    assert_eq!(&sealed[..24], &seq(0x40, 24)[..]);
    assert_eq!(&sealed[24..], &expected_ct[..]);
    assert_eq!(&**crypto::open(&key, &aad, &sealed).unwrap(), &plaintext[..]);
}

/// Inputs of the full-chain vector. Changing any of them changes every expected output.
fn chain() -> Value {
    let password = "correct horse battery staple \u{1F40E} pa\u{308}ss"; // decomposed ä: exercises NFKC
    let secret_key = SecretKey::from_bytes(seq(0x00, 16).try_into().unwrap());
    let kdf = KdfParams { m_kib: KdfParams::MIN_M_KIB, t: 2, p: 1 };
    let device = DeviceId([0xdd; 16]);
    let written = Hlc { wall_ms: 1_700_000_000_000, counter: 7, device };

    // create() consumes: vault id, salt, vault key, nonce.
    let stream = [seq(0x20, 16), seq(0x30, 16), seq(0x40, 32), seq(0x60, 24)].concat();
    let (header, vault_key) = Header::create(password, &secret_key, kdf, written, &mut Replay(stream)).unwrap();

    let mut vault = Vault::new();
    let login = Login {
        username: "ada@example.com".into(),
        password: "p@ss \u{e9}".into(),
        urls: vec!["https://example.com".into()],
        ..Default::default()
    };
    let item = Item { title: "Example".into(), notes: String::new(), favorite: false, data: ItemData::Login(login) };
    vault.upsert(ItemId([0x11; 16]), item, 1_700_000_000_001, device);
    let meta = SnapshotMeta { vault_id: header.vault_id, device, seq: 1 };
    let snap = Snapshot { device_name: "KAT".into(), vault };
    let snapshot_bytes = snapshot::seal(&vault_key, meta, &snap, &mut Replay(seq(0x80, 24))).unwrap();

    json!({
        "password": password,
        "secret_key": hex::encode(secret_key.as_bytes()),
        "secret_key_display": secret_key.to_display().as_str(),
        "kdf": { "m_kib": kdf.m_kib, "t": kdf.t, "p": kdf.p },
        "written": { "wall_ms": written.wall_ms, "counter": written.counter, "device": device.to_hex() },
        "vault_id": header.vault_id.to_hex(),
        "salt": hex::encode(header.salt),
        "vault_key": hex::encode(vault_key.as_bytes()),
        "header_nonce": hex::encode(seq(0x60, 24)),
        "header": hex::encode(header.encode()),
        "snapshot": hex::encode(&snapshot_bytes),
        "snapshot_device_name": "KAT",
        "snapshot_item_password": "p@ss \u{e9}",
    })
}

#[test]
fn full_chain_matches_committed_vectors() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/chain.json");
    let actual = chain();
    if std::env::var_os("SCYTALE_WRITE_VECTORS").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&actual).unwrap() + "\n").unwrap();
    }
    let expected: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(actual, expected, "format or crypto changed; if deliberate, regenerate and re-run crosscheck.py");

    // And the vector decrypts through the public API.
    let header = Header::decode(&unhex(expected["header"].as_str().unwrap())).unwrap();
    let sk = SecretKey::parse(expected["secret_key_display"].as_str().unwrap()).unwrap();
    let vk = header.unlock(expected["password"].as_str().unwrap(), &sk).unwrap();
    let bytes = unhex(expected["snapshot"].as_str().unwrap());
    let (_, snap) = snapshot::open(&vk, header.vault_id, &bytes, &mut snapshot::SeqGuard::default()).unwrap();
    assert_eq!(snap.vault.items().next().unwrap().1.password(), Some("p@ss \u{e9}"));
}
