//! 128-bit random identifiers and the Hybrid Logical Clock that orders every edit.

use core::fmt;

use rand_core::CryptoRng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Error, Result};

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(pub [u8; 16]);

        impl $name {
            pub fn random(rng: &mut impl CryptoRng) -> Self {
                let mut b = [0u8; 16];
                rng.fill_bytes(&mut b);
                Self(b)
            }

            pub fn to_hex(&self) -> String {
                to_hex(&self.0)
            }

            pub fn from_hex(s: &str) -> Result<Self> {
                from_hex(s).map(Self)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.to_hex())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.to_hex())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
                s.serialize_str(&self.to_hex())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                Self::from_hex(&s).map_err(serde::de::Error::custom)
            }
        }
    };
}

id_type!(
    /// Identifies a vault for its whole lifetime.
    VaultId
);
id_type!(
    /// Identifies one installation (one browser profile, one desktop app).
    DeviceId
);
id_type!(
    /// Identifies one item across all devices.
    ItemId
);

fn to_hex(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(s: &str) -> Result<[u8; 16]> {
    let bad = || Error::Invalid(format!("not a 32-character hex id: {s:?}"));
    if s.len() != 32 {
        return Err(bad());
    }
    let mut out = [0u8; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(s.get(i * 2..i * 2 + 2).ok_or_else(bad)?, 16).map_err(|_| bad())?;
    }
    Ok(out)
}

/// Hybrid Logical Clock timestamp. Ordered by wall time, then counter, then device, so any two
/// timestamps compare deterministically on every device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Hlc {
    #[serde(rename = "w")]
    pub wall_ms: u64,
    #[serde(rename = "c")]
    pub counter: u32,
    #[serde(rename = "d")]
    pub device: DeviceId,
}

impl Hlc {
    /// The next timestamp for `device`, strictly greater than `last` (the newest timestamp this
    /// device has seen). `now_ms` is the device's wall clock.
    // ponytail: a device whose clock runs far ahead drags every later edit's wall time with it;
    // ordering stays correct, only displayed times skew. Cap remote drift if that ever matters.
    pub fn tick(last: Hlc, now_ms: u64, device: DeviceId) -> Hlc {
        if now_ms > last.wall_ms {
            Hlc { wall_ms: now_ms, counter: 0, device }
        } else {
            Hlc { wall_ms: last.wall_ms, counter: last.counter.saturating_add(1), device }
        }
    }

    pub(crate) const ENCODED_LEN: usize = 8 + 4 + 16;

    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.wall_ms.to_le_bytes());
        out.extend_from_slice(&self.counter.to_le_bytes());
        out.extend_from_slice(&self.device.0);
    }

    pub(crate) fn decode(b: &[u8; Self::ENCODED_LEN]) -> Hlc {
        Hlc {
            wall_ms: u64::from_le_bytes(b[0..8].try_into().expect("8 bytes")),
            counter: u32::from_le_bytes(b[8..12].try_into().expect("4 bytes")),
            device: DeviceId(b[12..28].try_into().expect("16 bytes")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip_and_rejects_garbage() {
        let id = ItemId([0xab; 16]);
        assert_eq!(ItemId::from_hex(&id.to_hex()).unwrap(), id);
        assert!(ItemId::from_hex("zz").is_err());
        assert!(ItemId::from_hex(&"g".repeat(32)).is_err());
        assert!(ItemId::from_hex("é".repeat(16).as_str()).is_err());
    }

    #[test]
    fn tick_is_strictly_monotonic() {
        let d = DeviceId([1; 16]);
        let a = Hlc::tick(Hlc::default(), 100, d);
        let b = Hlc::tick(a, 100, d); // same millisecond
        let c = Hlc::tick(b, 50, d); // clock went backwards
        let e = Hlc::tick(c, 200, d);
        assert!(a < b && b < c && c < e);
    }
}
