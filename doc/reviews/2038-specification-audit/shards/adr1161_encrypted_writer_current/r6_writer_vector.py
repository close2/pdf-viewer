#!/usr/bin/env python3
"""Independent R6 Algorithms 8–10 vector for serialize_encrypted::Counter(0).

Uses PyCryptodome, not the Rust implementation. Inputs: user/owner UTF-8 passwords,
file key 00..1f, salts 20..3f, filler 40..43, P=-4, EncryptMetadata=true.
"""
from hashlib import sha256, sha384, sha512
from Crypto.Cipher import AES


def alg2b(password: bytes, salt: bytes, extra: bytes) -> bytes:
    k = sha256(password + salt + extra).digest()
    round_no = 0
    while True:
        k1 = (password + k + extra) * 64
        e = AES.new(k[:16], AES.MODE_CBC, k[16:32]).encrypt(k1)
        selector = int.from_bytes(e[:16], "big") % 3
        k = (sha256, sha384, sha512)[selector](e).digest()
        round_no += 1
        if round_no >= 64 and e[-1] <= round_no - 32:
            return k[:32]


def enc(password: bytes, salt: bytes, extra: bytes) -> bytes:
    return alg2b(password, salt, extra)


def main() -> None:
    user_pw = b"reader"
    owner_pw = b"keeper"
    file_key = bytes(range(32))
    salts = bytes(range(32, 64))
    filler = bytes(range(64, 68))
    uvs, uks, ovs, oks = salts[:8], salts[8:16], salts[16:24], salts[24:]
    u = alg2b(user_pw, uvs, b"") + uvs + uks
    ue = AES.new(alg2b(user_pw, uks, b""), AES.MODE_CBC, bytes(16)).encrypt(file_key)
    o = alg2b(owner_pw, ovs, u) + ovs + oks
    oe = AES.new(alg2b(owner_pw, oks, u), AES.MODE_CBC, bytes(16)).encrypt(file_key)
    perms_plain = ((0xFFFFFFFF00000000 | (0xFFFFFFFC & 0xFFFFFFFF)).to_bytes(8, "little")
                   + b"Tadb" + filler)
    perms = AES.new(file_key, AES.MODE_ECB).encrypt(perms_plain)
    for name, value in [("U", u), ("UE", ue), ("O", o), ("OE", oe), ("Perms", perms),
                        ("perms_plain", perms_plain)]:
        print(f"{name}={value.hex()}")

if __name__ == "__main__":
    main()
