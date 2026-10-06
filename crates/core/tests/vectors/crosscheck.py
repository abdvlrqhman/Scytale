"""Independent re-derivation of chain.json with libsodium (PyNaCl), argon2-cffi and pyca/cryptography.

Shares no code with the Rust implementation; it only follows docs/CRYPTO.md and docs/VAULT_FORMAT.md.
Run:  pip install -r requirements.txt && python crosscheck.py
"""

import hashlib
import json
import pathlib
import struct
import unicodedata

from argon2.low_level import Type, hash_secret_raw
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.hkdf import HKDF
from nacl.bindings import (
    crypto_aead_xchacha20poly1305_ietf_decrypt as xchacha_open,
    crypto_aead_xchacha20poly1305_ietf_encrypt as xchacha_seal,
)

v = json.loads(pathlib.Path(__file__).with_name("chain.json").read_text(encoding="utf-8"))
h = bytes.fromhex
kdf, written = v["kdf"], v["written"]

# --- Secret Key display: Crockford base32 of 128 bits + 2 zero bits, then 10 check bits ---------
ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
sk = h(v["secret_key"])
n = int.from_bytes(sk, "big") << 2
chars = "".join(ALPHABET[(n >> (5 * i)) & 31] for i in reversed(range(26)))
check = int.from_bytes(hashlib.sha256(b"scytale/v1/secret-key-check" + sk).digest()[:2], "big") >> 6
chars += ALPHABET[check >> 5] + ALPHABET[check & 31]
display = "S1-" + "-".join(chars[i : i + 5] for i in range(0, len(chars), 5))
assert display == v["secret_key_display"], (display, v["secret_key_display"])

# --- Header: Argon2id -> HKDF -> XChaCha20-Poly1305 wrap of the vault key ----------------------
password = unicodedata.normalize("NFKC", v["password"]).encode()
pw_key = hash_secret_raw(
    password, h(v["salt"]), time_cost=kdf["t"], memory_cost=kdf["m_kib"],
    parallelism=kdf["p"], hash_len=32, type=Type.ID, version=19,
)
kek = HKDF(algorithm=hashes.SHA256(), length=32, salt=h(v["vault_id"]), info=b"scytale/v1/kek").derive(pw_key + sk)

aad = (
    b"SCYH" + bytes([1]) + h(v["vault_id"]) + struct.pack("<Q", 1)
    + struct.pack("<III", kdf["m_kib"], kdf["t"], kdf["p"]) + h(v["salt"])
    + struct.pack("<QI", written["wall_ms"], written["counter"]) + h(written["device"])
)
nonce = h(v["header_nonce"])
header = aad + nonce + xchacha_seal(h(v["vault_key"]), aad, nonce, kek)
assert header == h(v["header"]), "header mismatch"

# --- Snapshot: decrypt, unpad (u32 length + JSON + zeros, Padmé-sized), parse -------------------
def padme(length: int) -> int:
    if length < 2:
        return length
    e = length.bit_length() - 1
    s = e.bit_length()
    mask = (1 << (e - s)) - 1
    return (length + mask) & ~mask

snap = h(v["snapshot"])
assert snap[:4] == b"SCYV" and snap[4] == 1
assert snap[5:21] == h(v["vault_id"]) and snap[21:37] == h(written["device"])
plain = xchacha_open(snap[69:], snap[:45], snap[45:69], h(v["vault_key"]))
length = struct.unpack("<I", plain[:4])[0]
assert len(plain) == padme(max(4 + length, 4096)), "padding size"
assert plain[4 + length :] == bytes(len(plain) - 4 - length), "padding must be zeros"
body = json.loads(plain[4 : 4 + length])
assert body["device_name"] == v["snapshot_device_name"]
(entry,) = body["vault"]["entries"].values()
assert entry["item"]["data"]["password"] == v["snapshot_item_password"]

print("crosscheck OK: secret key, header and snapshot match an independent implementation")
