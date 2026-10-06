//! Vault contents and the merge that makes sync conflict-free. Spec: `docs/SYNC.md`.
//!
//! Each item id maps to an [`Entry`]: a last-writer-wins register for the item (ordered by HLC) plus
//! a grow-only password history. [`Vault::merge`] is a join: commutative, associative and idempotent,
//! so devices converge no matter in which order they see each other's snapshots.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{DeviceId, Hlc, ItemId};

/// Passwords kept per item, newest first.
pub const HISTORY_LIMIT: usize = 32;

/// Wiped on drop. Inner types only derive `Zeroize` (wiped through this drop): a `Drop` impl on them
/// would forbid `..Default::default()` and container-level serde defaults.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub title: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub favorite: bool,
    pub data: ItemData,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Zeroize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemData {
    Login(Login),
    Note,
    Card(Card),
    Identity(Identity),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Zeroize)]
#[serde(default, deny_unknown_fields)]
pub struct Login {
    pub username: String,
    pub password: String,
    pub urls: Vec<String>,
    /// `otpauth://` URI or bare base32 secret.
    pub totp: String,
    /// Only fill on the exact host, not on other subdomains of the same site.
    pub exact_host: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Zeroize)]
#[serde(default, deny_unknown_fields)]
pub struct Card {
    pub holder: String,
    pub number: String,
    /// `MM/YY`
    pub expiry: String,
    pub cvv: String,
    pub pin: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Zeroize)]
#[serde(default, deny_unknown_fields)]
pub struct Identity {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub company: String,
    pub address1: String,
    pub address2: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
}

impl Item {
    pub fn password(&self) -> Option<&str> {
        match &self.data {
            ItemData::Login(l) if !l.password.is_empty() => Some(&l.password),
            _ => None,
        }
    }
}

/// One password and the newest version that used it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
#[serde(deny_unknown_fields)]
pub struct PasswordUse {
    pub password: String,
    #[zeroize(skip)]
    pub last_used: Hlc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    #[zeroize(skip)]
    pub version: Hlc,
    /// `None` = deleted (tombstone).
    pub item: Option<Item>,
    /// Every password this item has had, including the current one. Grow-only, except that a
    /// delete clears everything up to its timestamp.
    pub history: Vec<PasswordUse>,
    #[zeroize(skip)]
    pub cleared: Option<Hlc>,
}

impl Entry {
    fn join(&self, other: &Entry) -> Entry {
        let winner = if (other.version, &other.item) > (self.version, &self.item) { other } else { self };
        let cleared = self.cleared.max(other.cleared);
        Entry {
            version: winner.version,
            item: winner.item.clone(),
            history: normalize_history(self.history.iter().chain(&other.history), cleared),
            cleared,
        }
    }
}

/// Dedupe by password (keeping the newest use), drop anything cleared by a delete, keep the newest
/// [`HISTORY_LIMIT`]. Canonical order: newest first, ties by password.
fn normalize_history<'a>(uses: impl Iterator<Item = &'a PasswordUse>, cleared: Option<Hlc>) -> Vec<PasswordUse> {
    let mut newest: BTreeMap<&str, Hlc> = BTreeMap::new();
    for u in uses {
        let e = newest.entry(&u.password).or_insert(u.last_used);
        *e = (*e).max(u.last_used);
    }
    let mut out: Vec<PasswordUse> = newest
        .into_iter()
        .filter(|(_, at)| cleared.is_none_or(|c| *at > c))
        .map(|(pw, at)| PasswordUse { password: pw.to_owned(), last_used: at })
        .collect();
    out.sort_by(|a, b| b.last_used.cmp(&a.last_used).then_with(|| a.password.cmp(&b.password)));
    out.truncate(HISTORY_LIMIT);
    out
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vault {
    entries: BTreeMap<ItemId, Entry>,
}

impl Vault {
    pub fn new() -> Self {
        Self::default()
    }

    /// Live items, in id order.
    pub fn items(&self) -> impl Iterator<Item = (&ItemId, &Item)> {
        self.entries.iter().filter_map(|(id, e)| e.item.as_ref().map(|i| (id, i)))
    }

    pub fn get(&self, id: &ItemId) -> Option<&Item> {
        self.entries.get(id)?.item.as_ref()
    }

    pub fn entry(&self, id: &ItemId) -> Option<&Entry> {
        self.entries.get(id)
    }

    /// Previous passwords of an item, newest first, excluding the current one.
    pub fn password_history(&self, id: &ItemId) -> Vec<&PasswordUse> {
        let Some(e) = self.entries.get(id) else {
            return Vec::new();
        };
        let current = e.item.as_ref().and_then(Item::password);
        e.history.iter().filter(|u| Some(u.password.as_str()) != current).collect()
    }

    /// Newest timestamp in the vault; feed it to [`Hlc::tick`].
    pub fn clock(&self) -> Hlc {
        self.entries.values().map(|e| e.version).max().unwrap_or_default()
    }

    /// Creates or replaces an item. Returns the new version.
    pub fn upsert(&mut self, id: ItemId, item: Item, now_ms: u64, device: DeviceId) -> Hlc {
        let version = Hlc::tick(self.clock(), now_ms, device);
        let pw = item.password().map(str::to_owned);
        let entry =
            self.entries.entry(id).or_insert_with(|| Entry { version, item: None, history: Vec::new(), cleared: None });
        entry.version = version;
        entry.item = Some(item);
        if let Some(password) = pw {
            let added = PasswordUse { password, last_used: version };
            entry.history = normalize_history(entry.history.iter().chain([&added]), entry.cleared);
        }
        version
    }

    /// Deletes an item and its password history. Returns false if it did not exist.
    pub fn delete(&mut self, id: &ItemId, now_ms: u64, device: DeviceId) -> bool {
        if self.get(id).is_none() {
            return false;
        }
        let version = Hlc::tick(self.clock(), now_ms, device);
        self.entries.insert(*id, Entry { version, item: None, history: Vec::new(), cleared: Some(version) });
        true
    }

    /// Join with another device's vault.
    pub fn merge(&mut self, other: &Vault) {
        for (id, theirs) in &other.entries {
            let joined = match self.entries.get(id) {
                Some(ours) => ours.join(theirs),
                None => theirs.join(theirs),
            };
            self.entries.insert(*id, joined);
        }
    }

    /// Drops tombstones older than `max_age_ms`.
    // ponytail: age-based purge; a device offline longer than max_age can resurrect a deleted item.
    // Upgrade to "purge once every known device has seen the delete" if that is ever reported.
    pub fn compact(&mut self, now_ms: u64, max_age_ms: u64) {
        self.entries.retain(|_, e| e.item.is_some() || now_ms.saturating_sub(e.version.wall_ms) < max_age_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const A: DeviceId = DeviceId([0xa; 16]);
    const B: DeviceId = DeviceId([0xb; 16]);

    fn login(title: &str, password: &str) -> Item {
        Item {
            title: title.into(),
            notes: String::new(),
            favorite: false,
            data: ItemData::Login(Login { password: password.into(), ..Default::default() }),
        }
    }

    fn id(n: u8) -> ItemId {
        ItemId([n; 16])
    }

    #[test]
    fn concurrent_edits_converge_and_keep_both_passwords() {
        let mut base = Vault::new();
        base.upsert(id(1), login("mail", "old"), 1_000, A);

        let (mut a, mut b) = (base.clone(), base.clone());
        a.upsert(id(1), login("mail", "from-a"), 2_000, A);
        b.upsert(id(1), login("mail", "from-b"), 2_001, B);

        let mut ab = a.clone();
        ab.merge(&b);
        let mut ba = b.clone();
        ba.merge(&a);
        assert_eq!(ab, ba);
        assert_eq!(ab.get(&id(1)).unwrap().password(), Some("from-b"));
        let hist: Vec<_> = ab.password_history(&id(1)).iter().map(|u| u.password.as_str()).collect();
        assert_eq!(hist, ["from-a", "old"]);
    }

    #[test]
    fn delete_beats_older_edit_and_clears_history() {
        let mut a = Vault::new();
        a.upsert(id(1), login("x", "p1"), 1_000, A);
        let mut b = a.clone();
        b.upsert(id(1), login("x", "p2"), 2_000, B); // older than the delete below
        a.delete(&id(1), 3_000, A);

        a.merge(&b);
        assert!(a.get(&id(1)).is_none());
        assert!(a.entry(&id(1)).unwrap().history.is_empty());

        // An edit made after the delete resurrects the item, without the old history.
        a.upsert(id(1), login("x", "p3"), 4_000, A);
        assert_eq!(a.password_history(&id(1)).len(), 0);
        assert_eq!(a.get(&id(1)).unwrap().password(), Some("p3"));
    }

    #[test]
    fn compact_drops_only_old_tombstones() {
        let mut v = Vault::new();
        v.upsert(id(1), login("keep", "p"), 0, A);
        v.upsert(id(2), login("gone", "p"), 0, A);
        v.upsert(id(3), login("recent", "p"), 0, A);
        v.delete(&id(2), 10, A);
        v.delete(&id(3), 1_000, A);
        v.compact(1_005, 100);
        assert!(v.entry(&id(1)).is_some());
        assert!(v.entry(&id(2)).is_none());
        assert!(v.entry(&id(3)).is_some());
    }

    #[test]
    fn json_round_trip() {
        let mut v = Vault::new();
        v.upsert(id(1), login("a", "b"), 5, A);
        v.upsert(id(2), Item { title: "n".into(), notes: "hi".into(), favorite: true, data: ItemData::Note }, 6, A);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(serde_json::from_str::<Vault>(&json).unwrap(), v);
    }

    #[test]
    fn unknown_fields_are_rejected_not_dropped() {
        let json = r#"{"title":"t","data":{"type":"note"},"from_the_future":1}"#;
        assert!(serde_json::from_str::<Item>(json).is_err());
    }

    // --- Merge laws -------------------------------------------------------------------------

    #[derive(Clone, Debug)]
    enum Op {
        Upsert { item: u8, pw: u8, title: u8 },
        Delete { item: u8 },
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            (0..4u8, 0..6u8, 0..3u8).prop_map(|(item, pw, title)| Op::Upsert { item, pw, title }),
            (0..4u8).prop_map(|item| Op::Delete { item }),
        ]
    }

    /// A vault built by one device applying ops; devices share a time base so versions interleave.
    fn vault(device: u8) -> impl Strategy<Value = Vault> {
        proptest::collection::vec((op(), 0..50u64), 0..12).prop_map(move |ops| {
            let dev = DeviceId([device; 16]);
            let mut v = Vault::new();
            for (op, t) in ops {
                match op {
                    Op::Upsert { item, pw, title } => {
                        v.upsert(id(item), login(&format!("t{title}"), &format!("pw{pw}")), t, dev);
                    }
                    Op::Delete { item } => {
                        v.delete(&id(item), t, dev);
                    }
                }
            }
            v
        })
    }

    fn join(a: &Vault, b: &Vault) -> Vault {
        let mut out = a.clone();
        out.merge(b);
        out
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2_000))]

        #[test]
        fn merge_is_commutative(a in vault(1), b in vault(2)) {
            prop_assert_eq!(join(&a, &b), join(&b, &a));
        }

        #[test]
        fn merge_is_associative(a in vault(1), b in vault(2), c in vault(3)) {
            prop_assert_eq!(join(&join(&a, &b), &c), join(&a, &join(&b, &c)));
        }

        #[test]
        fn merge_is_idempotent(a in vault(1), b in vault(2)) {
            let ab = join(&a, &b);
            prop_assert_eq!(join(&ab, &ab), ab.clone());
            prop_assert_eq!(join(&ab, &b), ab);
        }

        #[test]
        fn no_live_password_is_lost(a in vault(1), b in vault(2)) {
            // Every password currently live on either side survives a merge (as current or history).
            let m = join(&a, &b);
            for side in [&a, &b] {
                for (id, item) in side.items() {
                    let Some(pw) = item.password() else { continue };
                    let e = m.entry(id).unwrap();
                    if e.cleared.is_some_and(|c| c >= side.entry(id).unwrap().version) { continue }
                    let kept = e.history.iter().any(|u| u.password == pw);
                    prop_assert!(kept || e.history.len() == HISTORY_LIMIT, "lost {pw}");
                }
            }
        }
    }
}
