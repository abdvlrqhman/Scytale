# Threat model

No software is "100 % safe". This page states what Scytale protects against and what it does not.

## Assets

The contents of the vault: passwords, TOTP secrets, notes, cards, identities. Also the master password
and the Secret Key.

## Protected

| Attacker | Why they fail |
|---|---|
| **The Scytale maintainers** | There is no Scytale server. Our OAuth client IDs give us no access to anyone's storage, and no token or ciphertext ever reaches us. |
| **The cloud storage provider**, or anyone who steals the vault files | They hold only ciphertext. Decrypting it needs the master password **and** the 128-bit Secret Key, which is never uploaded. Padding hides the exact item count. |
| **Someone who tampers with or replays sync files** | Every file is authenticated (AEAD) and bound to its vault, device and sequence number. Existing devices reject older sequence numbers. |
| **Phishing sites** | Autofill matches the page's registrable domain (Public Suffix List). It never fills cross-origin iframes, and never fills over `http` a login saved for `https`. |
| **Hidden-UI clickjacking** | v1 injects no UI into web pages. Filling only happens from the extension popup, a keyboard shortcut or the context menu. |
| **A stolen, powered-off laptop** | The vault on disk is encrypted. The Secret Key alone does not decrypt it; the master password is also required. |

## Not protected (out of scope)

- Malware, keyloggers or screen recorders on the user's device.
- A compromised browser or operating system.
- Other browser extensions with broad host permissions reading what the user types or sees.
- The user typing their master password into a phishing page.
- Physical access to an **unlocked** device. Auto-lock reduces the exposure window.
- Losing the Secret Key **and** every signed-in device: the vault cannot be recovered. That is the price
  of no one else being able to recover it.

## Known limitations

- **Secret Key at rest in the extension.** It is stored in `chrome.storage.local`, which is on disk in
  plaintext and protected only by the OS user account (the same model 1Password uses). The desktop app
  stores it in the OS credential store instead.
- **Key zeroization in the extension.** JavaScript and WASM cannot guarantee that key material is wiped
  from memory. The desktop app (Rust) zeroizes keys on lock.
- **First sync on a new device** trusts whatever is currently in storage. The rollback guard only protects
  devices that have already synced.
- **Metadata** visible to the storage provider: number of devices, file sizes (padded) and modification
  times.

## Reporting

See [SECURITY.md](../SECURITY.md).
