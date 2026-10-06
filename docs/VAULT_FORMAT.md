# Vault format, v1

All integers are little-endian. `‖` is concatenation. Reference implementation: `crates/core`.
Independent re-implementation: `crates/core/tests/vectors/crosscheck.py`.

**Compatibility rule:** readers reject unknown format versions with "update Scytale". Writers bump the
version on any change an older reader would mishandle. Item JSON also rejects unknown fields, so an old
client can never silently drop data written by a newer one.

## Header — `header.scyh` (157 bytes)

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | magic `SCYH` |
| 4 | 1 | format version = `1` |
| 5 | 16 | vault id |
| 21 | 8 | header version (u64, +1 on every master-password change) |
| 29 | 4 | Argon2id memory, KiB (u32) |
| 33 | 4 | Argon2id passes (u32) |
| 37 | 4 | Argon2id lanes (u32) |
| 41 | 16 | salt |
| 57 | 28 | writer HLC: wall ms (u64) ‖ counter (u32) ‖ device id (16) |
| 85 | 24 | nonce |
| 109 | 48 | XChaCha20-Poly1305(KEK, vault key) = 32-byte ciphertext ‖ 16-byte tag |

- AAD = bytes `0..85`: every field except the wrapped key.
- KDF bounds on read: memory 19 MiB … 1 GiB, passes 1 … 16, lanes 1 … 8. Anything else is rejected
  before any work is done.

## Device snapshot — `devices/<device_id>.scyv`

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | magic `SCYV` |
| 4 | 1 | format version = `1` |
| 5 | 16 | vault id |
| 21 | 16 | device id |
| 37 | 8 | seq (u64, +1 on every write by this device) |
| 45 | 24 | nonce |
| 69 | n + 16 | XChaCha20-Poly1305(vault key, padded body) |

- AAD = bytes `0..45`.
- Padded body = `len (u32) ‖ body ‖ zeros`, total size = Padmé(max(4 + len, 4096)).
- Body = UTF-8 JSON:

```json
{
  "device_name": "Firefox on Linux",
  "vault": {
    "entries": {
      "<item id hex>": {
        "version": { "w": 1700000000001, "c": 0, "d": "<device id hex>" },
        "item": {
          "title": "Example",
          "notes": "",
          "favorite": false,
          "data": { "type": "login", "username": "…", "password": "…", "urls": ["…"], "totp": "", "exact_host": false }
        },
        "history": [ { "password": "…", "last_used": { "w": 1700000000001, "c": 0, "d": "…" } } ],
        "cleared": null
      }
    }
  }
}
```

- `item: null` is a deletion (tombstone). `cleared` is the HLC of the latest delete; history entries at or
  before it are gone.
- `data.type` is one of `login`, `note`, `card` (`holder`, `number`, `expiry`, `cvv`, `pin`), `identity`
  (`full_name`, `email`, `phone`, `company`, `address1`, `address2`, `city`, `region`, `postal_code`,
  `country`).

## Encrypted export — `*.scyx`

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | magic `SCYX` |
| 4 | 1 | format version = `1` |
| 5 | 12 | Argon2id memory KiB ‖ passes ‖ lanes (3 × u32) |
| 17 | 16 | salt |
| 33 | 24 | nonce |
| 57 | n + 16 | XChaCha20-Poly1305(key, padded JSON array of items) |

- `key = HKDF-SHA256(ikm = Argon2id(NFKC(export password), salt), salt, info = "scytale/v1/export")`.
  There is no Secret Key, so the file opens on any machine with the export password alone.
- AAD = bytes `0..33`.
