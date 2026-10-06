//! Scytale core: crypto, vault format and sync merge.
//!
//! No network, filesystem or clock access: callers pass the current time in, and randomness is
//! injectable so every path is deterministic under test. See `docs/ARCHITECTURE.md`.

mod crypto;
mod error;
mod header;
mod ids;
#[cfg(test)]
mod kat;
mod secret_key;
mod vault;

pub mod export;
pub mod generator;
pub mod import;
pub mod snapshot;
pub mod totp;
pub mod url_match;

pub use crypto::{KdfParams, Key};
pub use error::{Error, Result};
pub use header::Header;
pub use ids::{DeviceId, Hlc, ItemId, VaultId};
pub use secret_key::SecretKey;
pub use vault::{Card, Entry, HISTORY_LIMIT, Identity, Item, ItemData, Login, PasswordUse, Vault};
