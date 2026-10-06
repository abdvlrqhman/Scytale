# Cryptography

Status: **v1, implemented** in `crates/core`. Byte-level layouts: [VAULT_FORMAT.md](VAULT_FORMAT.md).

## Principles

- No custom primitives. RustCrypto crates only: `argon2`, `chacha20poly1305`, `hkdf`, `sha2`, `hmac`
  (TOTP), `zeroize`.
- The core never reads the OS random source itself: every function takes a `CryptoRng`. Apps pass the
  OS generator; tests pass fixed streams, which is what makes the known-answer vectors possible.
- Every ciphertext is authenticated (AEAD), and its context is bound through associated data.
- Every derivation step has known-answer test vectors in `crates/core/tests/vectors/`.

## Secrets

| Secret | Size | Where it lives |
|---|---|---|
| Master password | user-chosen | The user's head. Never stored, never sent anywhere. |
| Secret Key | 128 bits, random | Each of the user's devices + the printed Emergency Kit. Never uploaded. |
| Vault key | 256 bits, random | Memory while unlocked; at rest only wrapped (encrypted) by the KEK. |

**Why two secrets.** The encrypted vault sits in the user's own cloud storage, so the realistic attacker
is someone who obtains that file and guesses passwords offline. The Secret Key adds 128 random bits to
every guess, so offline guessing is infeasible even with a weak master password. To break the vault, an
attacker needs the file, the Secret Key **and** the master password.

## Key derivation

```
pw_key    = Argon2id(NFKC(master_password), salt, m, t, p)            32 bytes
kek       = HKDF-SHA256(ikm  = pw_key ‖ secret_key,
                        salt = vault_id,
                        info = "scytale/v1/kek")                      32 bytes
wrapped_vk = XChaCha20-Poly1305(key = kek, nonce = random 24 bytes,
                                plaintext = vault_key,
                                aad = every header field except wrapped_vk)
```

- **Argon2id defaults:** `m = 64 MiB, t = 3, p = 1`, stored in the vault header so they can change
  later via a re-wrap (no re-encryption). Measured unlock time in WASM under V8 (Ryzen 9 7940HS,
  2026-10-06): 19 MiB/t2 = 25 ms, 64 MiB/t3 = 135 ms, 128 MiB/t3 = 285 ms. A mid-range laptop is
  roughly 2–3× slower, so 64 MiB/t3 unlocks in well under the 2 s budget. Memory is capped at 64 MiB
  because iOS autofill extensions (v2) are killed above roughly that; the Secret Key, not Argon2, is
  what makes stolen vault files unbreakable. `p = 1` because WASM runs single-threaded.
- `salt` is 16 random bytes. `vault_id` is 16 random bytes, fixed for the vault's lifetime.
- The password is NFKC-normalized first, so `ä` typed as one code point or as `a` + combining
  diaeresis derives the same key on every OS and keyboard.
- KDF parameters read from a header are bounds-checked (19 MiB – 1 GiB, 1 – 16 passes, 1 – 8 lanes)
  before any work, so a tampered header cannot force a huge allocation.
- A wrong password or Secret Key shows up as an AEAD failure when unwrapping the vault key. The error
  does not say which of the two was wrong.

## Encrypting a device snapshot

```
snapshot = XChaCha20-Poly1305(key = vault_key, nonce = random 24 bytes,
                              plaintext = Padmé(serialized items),
                              aad = magic ‖ format_version ‖ vault_id ‖ device_id ‖ seq)
```

- **Random 24-byte nonces.** XChaCha's 192-bit nonce makes random nonces collision-safe at any realistic
  volume, so no nonce counter has to be kept in sync across devices.
- **Padmé padding** hides the exact vault size. It leaks at most O(log log n) bits with ≤ 12 % overhead.
- **AAD** ties each file to its vault, device and sequence number, so files cannot be swapped between
  vaults or devices, or replayed under a different `seq`.

## Operations

| Operation | Effect |
|---|---|
| Create vault | Generate `vault_id`, `salt`, Secret Key and vault key → write header → show the Emergency Kit. |
| Unlock | Derive the KEK → unwrap the vault key → decrypt the local snapshot. |
| Change master password | New `salt` → new KEK → re-wrap the **same** vault key → `header_version + 1`. Instant, with no re-encryption of items. |
| Lock | Zeroize the vault key and decrypted items (desktop); drop all references and clear session storage (extension). |

## Secret Key format

128 random bits as 26 Crockford base32 characters (the last 2 bits are zero padding), plus 2 check
characters = the top 10 bits of `SHA-256("scytale/v1/secret-key-check" ‖ key)`, behind a version
prefix, grouped for reading aloud: `S1-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXX`.

Parsing ignores case, spaces and dashes, and maps the look-alikes `O→0` and `I/L→1`. The check
characters catch ~99.9 % of typos before an unlock attempt.

## Emergency Kit

A printable page, generated locally, holding the Secret Key, the vault's storage location and a blank
line for the master password. **Losing both the Secret Key and every signed-in device makes the vault
unrecoverable.** Onboarding states this plainly and does not let the user continue until the kit has
been saved.

## Test vectors

- Argon2id: RFC 9106 §5.3
- HKDF-SHA256: RFC 5869 Appendix A
- XChaCha20-Poly1305: draft-irtf-cfrg-xchacha §A.3
- The full chain (password + Secret Key → header → snapshot) and the Secret Key text:
  `crates/core/tests/vectors/chain.json`, re-derived in CI by `crosscheck.py` using libsodium (PyNaCl),
  argon2-cffi and pyca/cryptography, which share no code with ours.
