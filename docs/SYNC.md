# Sync

Status: **implemented.** WebDAV (both apps), a synced folder (desktop) and Dropbox (enabled by an app key at build time). Google Drive and OneDrive are not built yet.

Scytale has no server. Devices sync through storage the user already owns: WebDAV (Nextcloud and
others), Dropbox, OneDrive, Google Drive, or a synced folder (Syncthing, iCloud Drive, a cloud provider's
desktop client). The storage only ever holds ciphertext.

## Layout

```
<app folder>/scytale/<vault_id>/
  header.scyh                KDF params, salt, wrapped vault key, header version
  devices/<device_id>.scyv    one device's full encrypted snapshot
```

**Each device writes only its own file.** No device ever overwrites another device's file, so writes
can never conflict. That makes sync correct on Google Drive (which has no conditional writes) and on plain
synced folders (which have no concurrency control at all).

## A sync round

1. List `devices/`. Download every file whose version or ETag changed since the last round.
2. Decrypt each one. Reject any file whose `seq` is lower than the highest already seen from that device
   (rollback guard).
3. Merge all snapshots into local state (pure function in `core`).
4. If local state changed, increment our `seq`, encrypt, and upload our own file.

**Triggers:** on unlock, 2 s after an edit, every 5 minutes while unlocked, on browser start.

## Merge rules

Implemented in `crates/core/src/vault.rs`. Each item id maps to an entry with two parts:

- **The item:** a last-writer-wins register. The highest Hybrid Logical Clock timestamp (wall clock +
  counter + `device_id`) wins, so ties are impossible between devices.
- **Its password history:** every password the item has had, each with the newest timestamp that used
  it, capped at the newest 32. Merging takes the union. This is how a losing concurrent edit's password
  survives: **no password is ever silently lost.**

Deletes are tombstones (`item: null`) that also record `cleared`. History at or before that timestamp
is dropped, so deleting an item really deletes its old passwords. An edit made after the delete brings
the item back.

The merge is a join: commutative, associative and idempotent. Property tests check all three, and that
no live password is lost, over 2,000 random multi-device histories per law.

`ponytail:` tombstones are purged after 90 days. If an old offline device ever resurrects an item,
upgrade to "purge once every known device has seen it".

**Schema changes:** a device that finds a snapshot with a newer format version stops syncing and asks
to be updated, instead of rewriting data it does not understand.

## Header changes

The header changes only when the master password changes, so writes to it are rare.

- Where the provider supports preconditions, header writes use them as a fast-fail check: Dropbox
  `mode: update` + `rev` + `strict_conflict: true`, WebDAV `If-Match`, OneDrive upload-session
  `if-match`.
- Google Drive has none, so the highest `header_version` wins. Ties break by HLC, then `device_id`.
- The vault key itself never changes, so a device holding an older header keeps syncing. It only needs
  the new master password the next time it unlocks.

## Providers

Ordered by OAuth friction. All OAuth uses PKCE public clients: client IDs are committed (they are public
by design), secrets never are. Sign-in always happens in the system browser or `launchWebAuthFlow`,
never in an embedded webview.

| Provider | Auth | Notes |
|---|---|---|
| WebDAV | Username + app password | Uses `If-Match`. Ignore weak (`W/`) ETags, which some gzip reverse proxies produce. |
| Dropbox | PKCE + offline refresh token | App-folder scope. Long-lived refresh tokens. |
| OneDrive | PKCE, `Files.ReadWrite.AppFolder` | In the extension, refresh tokens for the SPA redirect type expire after **24 h**: try silent re-auth first, then prompt. The desktop app gets 90 days. |
| Google Drive | Desktop: PKCE + loopback. Extension: needs a spike (see below) | `drive.appdata` / `drive.file` are non-sensitive scopes, so no security assessment is required. Needs brand verification + "In production" status. |
| Folder | None (desktop only) | Any folder another tool syncs. |

**Open spikes** (each must pass before its adapter is written):

1. Google Web client + PKCE with no secret, from a `chromiumapp.org` redirect.
2. Google through Firefox's `127.0.0.1/mozoauth2/…` loopback redirect.
3. Is `appDataFolder` shared between the extension's and the desktop app's client IDs in the same Cloud
   project? If not, use `drive.file` with a visible `Scytale/` folder.
4. Does an MV3 service worker send an `Origin` header to Microsoft Entra?

## Later

An optional Cloudflare Worker + R2 template that each user deploys to **their own** free Cloudflare
account. It is just another `RemoteStore` adapter; the maintainer still hosts nothing.
