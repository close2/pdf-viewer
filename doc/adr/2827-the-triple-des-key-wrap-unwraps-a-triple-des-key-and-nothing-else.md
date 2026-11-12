# 2827 — The Triple-DES key wrap unwraps a Triple-DES key, and nothing else

Session 2004, a ledger round (§7.6.5.3). Status: implemented. Rows: §7.6.5, §7.6.5.3.
Takes the third of the four effort-only refusals ADR 2821 left on §7.6.5.3, and records why the
other two stay.

## Context

RFC 5753 section 7.1.5 names four key wraps for an elliptic-curve recipient: "Triple-DES, AES-128,
AES-192, and AES-256". ADR 2821 ran the three AES wraps (RFC 3394) and refused RFC 3370 section
4.3.1's `id-alg-CMS3DESwrap` by name. That identifier is RFC 3217 section 3's algorithm: a SHA-1
checksum and two CBC passes under the Triple-DES key-encryption key, with a reversal between them.
`openssl cms -encrypt -wrap des3-wrap` writes it, and no document this tree holds uses it.

## Decision

1. **`pdf_pki::key_wrap::unwrap_triple_des` is RFC 3217 section 3.2, step by step**, over the `des`
   package ADR 2809 took and the `sha1` the crate pins: no package is added. It is pinned by the
   section 3.4 example, both ways — the RESULT unwraps to the CEK, and the tests' own section 3.1
   wrap reproduces the RESULT — and by planted failures of step 7's checksum, step 8's parity (a
   key of even parity wrapped with a checksum that matches it), step 1's length and the KEK's.
2. **The key agreement derives a 24-octet key for it**, and states the wrap in RFC 5753 section
   7.2's `keyInfo` with NULL parameters, as that section says ("3DES wrap has NULL parameters while
   the AES wraps have absent parameters"); a recipient whose `KeyWrapAlgorithm` states the wrap
   without RFC 3370's "MUST be NULL" parameters is `Malformed`. Two envelopes `openssl cms
   -encrypt -des3 -wrap des3-wrap` wrote to the P-256 test certificate, under SHA-1 and SHA-256
   KDFs, open to the plaintext put in.
3. **A wrapped key that is not 40 octets is refused by name, not tried.** RFC 3217 section 3
   "specifies the algorithm for wrapping one Triple-DES key with another Triple-DES key", and step 1
   of section 3.2 is "[i]f the wrapped key is not 40 octets, then error". RFC 3370 section 4.3 lets
   an implementation "support mixed key-encryption and content-encryption algorithms", but no
   section defines the algorithm for wrapping a key of another length under this identifier. OpenSSL
   writes one — `-aes128 -wrap des3-wrap` wraps the 16-octet AES key into 32 octets — and opens it
   again; that agreement is OpenSSL's with itself, not a reading of a clause, so the reader names it
   (`EnvelopeError::KeyWrap`, the identifier and the two lengths) and the fixture holds the refusal.
   A 24-octet AES-192 key under the same wrap passes step 1 and fails step 8's parity unless its
   writer set odd parity, as step 1 of section 3.1 tells it to.

## What stays refused, and why

- **1-Pass ECMQV** (`mqvSinglePass-*`, RFC 5753 section 3.2). Effort only, and not small: section
  3.2.3 runs SP 800-56A's ECMQV over the sender's static key (from `originator`), its ephemeral key
  (from `ukm`'s `MQVuserKeyingMaterial`) and the recipient's static pair used twice, which needs
  scalar and point arithmetic the reviewed curve packages expose but `pdf_pki::ec` does not yet
  call. No tool here that states its algorithm writes such an envelope — OpenSSL does not implement
  ECMQV in CMS — so its fixture is this tree's own generator's, written from SEC 1 section 3.4; it
  is the lane's next ledger round, after the walks this round owed.
- **brainpoolP512r1.** The brief's condition is a reviewed curve package in the lock or a minor
  version of one. Neither exists: RustCrypto publishes `bp256` and `bp384` and no `bp512`
  (crates.io, 2026-11-11), and `bp512-nestler` is PolyForm Noncommercial, which `doc/todo/65`
  section 5 already judged unusable under Apache-2.0. `pdf-signature`'s own brainpoolP512r1 (A170,
  ADR 1385) is a verification module, variable-time over public values, and private to the crate
  above this one; Diffie–Hellman with a reader's private scalar is not what it was built or
  reviewed for, so it is not borrowed. The key stays refused by the curve's name when it is handed
  over, and `doc/todo/65` section 5's re-check is the trigger.
