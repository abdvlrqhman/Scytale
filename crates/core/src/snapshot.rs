//! Device snapshot (`devices/<device_id>.scyv`): one device's full vault, encrypted under the vault
//! key and bound to its vault, device and sequence number. Byte layout: `docs/VAULT_FORMAT.md`.

use std::collections::BTreeMap;

use rand_core::CryptoRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::crypto::{self, Key};
use crate::{DeviceId, Error, Result, Vault, VaultId};

const MAGIC: &[u8; 4] = b"SCYV";
const FORMAT: u8 = 1;
const AAD_LEN: usize = 4 + 1 + 16 + 16 + 8; // 45

/// What a device publishes about itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Shown in the devices list, e.g. "Firefox on Linux".
    pub device_name: String,
    pub vault: Vault,
}

/// Unauthenticated routing info, readable without the key. Trust it only after [`open`] succeeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotMeta {
    pub vault_id: VaultId,
    pub device: DeviceId,
    pub seq: u64,
}

pub fn seal(key: &Key, meta: SnapshotMeta, snapshot: &Snapshot, rng: &mut impl CryptoRng) -> Result<Vec<u8>> {
    let json = Zeroizing::new(serde_json::to_vec(snapshot).map_err(|e| Error::Invalid(e.to_string()))?);
    let aad = encode_meta(meta);
    let sealed = crypto::seal(key, &aad, &crypto::pad(&json)?, rng);
    let mut out = aad;
    out.extend_from_slice(&sealed);
    Ok(out)
}

pub fn peek(bytes: &[u8]) -> Result<SnapshotMeta> {
    if bytes.len() < 5 || &bytes[..4] != MAGIC {
        return Err(Error::Corrupted);
    }
    if bytes[4] != FORMAT {
        return Err(Error::UnsupportedVersion(bytes[4]));
    }
    if bytes.len() < AAD_LEN {
        return Err(Error::Corrupted);
    }
    Ok(SnapshotMeta {
        vault_id: VaultId(bytes[5..21].try_into().expect("16 bytes")),
        device: DeviceId(bytes[21..37].try_into().expect("16 bytes")),
        seq: u64::from_le_bytes(bytes[37..45].try_into().expect("8 bytes")),
    })
}

/// Decrypts a snapshot that must belong to `vault_id`, then runs the rollback guard. The guard only
/// ever sees authenticated metadata, so a forged file (e.g. `seq = u64::MAX`) cannot poison it.
pub fn open(key: &Key, vault_id: VaultId, bytes: &[u8], guard: &mut SeqGuard) -> Result<(SnapshotMeta, Snapshot)> {
    let meta = peek(bytes)?;
    if meta.vault_id != vault_id {
        return Err(Error::WrongVault);
    }
    let padded = crypto::open(key, &bytes[..AAD_LEN], &bytes[AAD_LEN..])?;
    let snapshot = serde_json::from_slice(crypto::unpad(&padded)?).map_err(|_| Error::Corrupted)?;
    guard.check_and_record(meta)?;
    Ok((meta, snapshot))
}

fn encode_meta(meta: SnapshotMeta) -> Vec<u8> {
    let mut out = Vec::with_capacity(AAD_LEN);
    out.extend_from_slice(MAGIC);
    out.push(FORMAT);
    out.extend_from_slice(&meta.vault_id.0);
    out.extend_from_slice(&meta.device.0);
    out.extend_from_slice(&meta.seq.to_le_bytes());
    out
}

/// Rollback guard: remembers the highest sequence number seen from each device and rejects older
/// files. Re-reading the same file is fine. Updated only by [`open`]; persist it with the local vault.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeqGuard(BTreeMap<DeviceId, u64>);

impl SeqGuard {
    fn check_and_record(&mut self, meta: SnapshotMeta) -> Result<()> {
        let seen = self.0.entry(meta.device).or_insert(meta.seq);
        if meta.seq < *seen {
            return Err(Error::Rollback { device: meta.device.to_hex(), got: meta.seq, seen: *seen });
        }
        *seen = meta.seq;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Item, ItemData, ItemId};
    use proptest::prelude::*;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    fn sample() -> (Key, SnapshotMeta, Snapshot) {
        let mut vault = Vault::new();
        let note = Item { title: "t".into(), notes: "n".into(), favorite: false, data: ItemData::Note };
        vault.upsert(ItemId([1; 16]), note, 10, DeviceId([2; 16]));
        let meta = SnapshotMeta { vault_id: VaultId([3; 16]), device: DeviceId([2; 16]), seq: 7 };
        (Key::from_bytes([4; 32]), meta, Snapshot { device_name: "Test".into(), vault })
    }

    #[test]
    fn round_trip() {
        let (key, meta, snap) = sample();
        let bytes = seal(&key, meta, &snap, &mut ChaCha20Rng::seed_from_u64(1)).unwrap();
        assert_eq!(peek(&bytes).unwrap(), meta);
        assert_eq!(open(&key, meta.vault_id, &bytes, &mut SeqGuard::default()).unwrap(), (meta, snap));
    }

    #[test]
    fn rejects_swapped_metadata_and_wrong_vault() {
        let (key, meta, snap) = sample();
        let bytes = seal(&key, meta, &snap, &mut ChaCha20Rng::seed_from_u64(1)).unwrap();

        let g = &mut SeqGuard::default();
        assert_eq!(open(&key, VaultId([9; 16]), &bytes, g), Err(Error::WrongVault));
        // Re-label as another device or a higher seq: authentication fails.
        for range in [21..37, 37..45] {
            let mut bad = bytes.clone();
            bad[range.start] ^= 1;
            assert_eq!(open(&key, meta.vault_id, &bad, g), Err(Error::Corrupted));
        }
        assert_eq!(open(&Key::from_bytes([5; 32]), meta.vault_id, &bytes, g), Err(Error::Corrupted));
        assert_eq!(g, &SeqGuard::default(), "failed opens must not touch the guard");
    }

    #[test]
    fn rollback_rejected_and_forged_seq_cannot_poison_guard() {
        let (key, meta, snap) = sample();
        let rng = &mut ChaCha20Rng::seed_from_u64(1);
        let file = |seq| seal(&key, SnapshotMeta { seq, ..meta }, &snap, &mut ChaCha20Rng::seed_from_u64(seq)).unwrap();
        let g = &mut SeqGuard::default();
        open(&key, meta.vault_id, &file(5), g).unwrap();
        open(&key, meta.vault_id, &file(5), g).unwrap(); // re-reading is fine
        open(&key, meta.vault_id, &file(6), g).unwrap();
        assert!(matches!(open(&key, meta.vault_id, &file(4), g), Err(Error::Rollback { got: 4, seen: 6, .. })));

        // Attacker without the key relabels a genuine file as seq u64::MAX.
        let mut forged = seal(&key, meta, &snap, rng).unwrap();
        forged[37..45].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(open(&key, meta.vault_id, &forged, g), Err(Error::Corrupted));
        open(&key, meta.vault_id, &file(7), g).unwrap(); // genuine files still accepted
    }

    proptest! {
        #[test]
        fn open_never_panics_on_garbage(bytes in proptest::collection::vec(any::<u8>(), 0..200)) {
            let _ = open(&Key::from_bytes([0; 32]), VaultId([0; 16]), &bytes, &mut SeqGuard::default());
            let _ = crate::Header::decode(&bytes);
        }
    }
}
