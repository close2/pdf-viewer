# 2809 — The four envelope ciphers §7.6.5.3 deprecates are read, from their own specifications

Session 1999, a ledger round (§7.6.5.3). Status: implemented. Rows: §7.6.5, §7.6.5.3.
Carries out item 2 of ADR 2801's "What is left, in order"; the owner's A800 ordered the handler built.

## Context

§7.6.5.3 lists the ciphers "that shall be used to encrypt the enveloped data in the CMS object":
RC4 "with key lengths up to 256-bits", "DES, Triple DES, RC2 with key lengths up to 128 bits", each
"(deprecated in PDF 2.0)", and AES-CBC at 128, 192 and 256 bits. ADR 2802 decrypted the three AES
widths and refused the other four by name. A refusal by name is honest, but it leaves a document a
pre-2.0 producer enveloped under Triple DES or RC2 unopenable by the one reader who holds its key,
and "deprecated" is a word to writers: the clause still lists the four as what a CMS object may use.

## Decision

1. **`pdf_pki::enveloped` decrypts all seven.** Each is recognised by the object identifier its own
   specification assigns and read with that specification's parameters: `desCBC` and an
   eight-octet IV (RFC 8018 appendix B.2.1), `des-ede3-cbc` and an eight-octet IV (RFC 3370 section
   5.1), `rc2-cbc` with RFC 2268 section 6's `RC2-CBCParameter` — a bare IV for the default 32
   effective key bits, or a version and an IV, the version read back through section 6's
   `Table[]` below 256 and taken as the bit count from 256 to 1024 — and `rc4` with no parameters.
   An identifier that is none of the seven is still `EnvelopeError::Cipher`, named.
2. **Key lengths are the clause's bounds, enforced.** RC4 takes 1 to 32 octets, RC2 1 to 16, DES 8;
   Triple DES takes 24 octets (three keys) or 16 (two keys, K1 K2 K1), because RFC 3370 section 5.1
   says "[t]he same algorithm identifiers are used for Three-Key Triple-DES and Two-Key Triple-DES".
   A key of any other length is the one `EnvelopeError::Decryption`, like any key that does not open
   the envelope, so a wrong key and a malformed one stay indistinguishable (RFC 8017 section 7.2.2).
3. **The ciphers are RustCrypto's `des` 0.9 and `rc2` 0.9, and the workspace's `rc4`**, the family
   `aes` and `cbc` already come from, on the same `cipher` 0.5; ADR 0031's argument (a reviewed
   implementation of a published algorithm over a fresh one) holds unchanged, and both are pure Rust
   under MIT OR Apache-2.0. They are read-only here: nothing in the tree writes a CMS object.
4. **Each is pinned twice.** A known answer from the cipher's own specification — FIPS 81 Table C1
   for DES and, with one key three times or two keys as K1 K2 K1, for Triple DES; RFC 2268 section
   5's vectors with their effective key bits for RC2; RFC 6229 section 2's keystreams at 128 and 256
   bits for RC4 — and an envelope per cipher written by `openssl cms -encrypt` to `pdf-pki`'s own
   test certificate (`crates/pdf-pki/tests/envelopes/`), which must open to the plaintext the script
   put in, with the cipher OpenSSL states in the envelope asserted to be the one read.

## Consequences

`pdf-pki` gains two packages (`des`, `rc2`) and names a third (`rc4`) the workspace already had.
No document any corpus here holds uses a deprecated envelope cipher, so nothing that opened before
moves; what moves is the refusal, which no longer fires for the four.
