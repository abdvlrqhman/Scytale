/// Every failure the core can report. Messages are safe to show to users: they never contain secrets.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The master password or the Secret Key is wrong. Deliberately does not say which.
    #[error("wrong master password or Secret Key")]
    WrongCredentials,
    #[error("data is corrupted or was tampered with")]
    Corrupted,
    #[error("unsupported format version {0}; update Scytale")]
    UnsupportedVersion(u8),
    #[error("this file belongs to a different vault")]
    WrongVault,
    #[error("device {device} sent an older file (seq {got} < {seen}); possible rollback")]
    Rollback { device: String, got: u64, seen: u64 },
    #[error("invalid Secret Key: {0}")]
    InvalidSecretKey(&'static str),
    #[error("unsafe key-derivation parameters")]
    InvalidKdfParams,
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = core::result::Result<T, Error>;
