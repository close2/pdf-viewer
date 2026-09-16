# 1134 — The public-key reader key is a host input, deferred behind the crate graph

Round 1139. Status: **accepted** (a deferral and the shape a build will take).

## Context

§7.6.5's public-key security handlers wrap the file encryption key in a per-recipient CMS
`EnvelopedData` (RFC 5652); a reader holding a private key whose certificate matches a recipient
unwraps the content-encryption key, decrypts the envelope to a 20-byte seed and a 4-byte
permission word, and derives the file key by SHA-256 over the seed, the `/Recipients` bytes and an
optional `0xFF` (§7.6.5.1). From there decryption is §7.6.4's, which this tree runs.

Round 1139's contract was to build the recipient-match-and-unwrap if the CMS unwrap was buildable
with the project's existing crypto crates. It is not — for a structural reason, not a missing
primitive. The census confirms the stakes are small: five documents in ~89 000 name a public-key
handler, all `adbe.pkcs7.s5`, and none carries a recipient certificate that could belong to this
reader (four are one Apache bug report's attachments; the fifth is encrypted to a device).

## Decision 1 — defer, because the blocker is the dependency graph

The three parts §7.6.5 adds over `crypt.rs`'s AES-CBC and SHA-256 — a CMS `EnvelopedData` reader,
X.509 recipient matching, and RSA **decryption** (RSADP, `c^d mod n`) — are not new primitives:
`pdf_signature::bigint::modpow` over `crypto-bigint` is the arithmetic, and `pdf_signature`'s
`der`/`cms`/`x509` already read certificates and `SignedData`. But all of them live in
`pdf-signature`, which **depends on** `pdf-syntax`, where `crypt.rs` is; the call cannot be made
without a cycle, and `pdf-syntax` carries no `der`/`pkcs1`/`x509`/`crypto-bigint`. No
`EnvelopedData` parser and no RSA-decryption path exist anywhere (`cms.rs` is `SignedData` only).

So building §7.6.5 means a structural choice `doc/stack.md` governs — a public-key stack duplicated
into `pdf-syntax`, or a shared crypto crate below both — and `doc/stack.md`'s rule is that this
parsing stays in one place. That is the owner's call: `doc/questions/Q66`. Adding it silently in a
crypt round is exactly the unargued dependency change the stack file forbids.

## Decision 2 — the reader's private key is a host input, on ADR 1076's Supply shape

When it is built, the private key is **not** read from the file or the environment by
`pdf-syntax`. It is supplied by the host exactly as ADR 1076's trust anchors are: a
`Supply`-shaped lending the host owns, answered in one place in `viewer_host::policy` beside the
other host decisions, carried inward on a `viewer_core::Command`, and defaulting to *none* — a
document that names no reachable recipient is refused, as all five corpus documents would be. The
private key is a `viewer_core::Secret`, like the standard handler's password (§7.6.4.1's row), so
it cannot reach a trace through a derived `Debug`; and it crosses into the confined worker inside
the open command, because §7.6.5 decryption happens where the document's bytes are. This keeps
principle 3's direction: untrusted envelope bytes are parsed under `#![forbid(unsafe_code)]`, and
the one trusted input — the key — enters once, from a place a host controls, never from the file.

## What the tree does now

`crypt::Encryption::new` reads `/SubFilter` and refuses the three values §7.6.5.1 lists
(`adbe.pkcs7.s3`/`s4`/`s5`) as a public-key security handler **by that clause**, quoting the
handler and subfilter, rather than misattributing the generic "not the standard handler" message
to §7.6.4. A calibrated test pins it with a control: a subfilter §7.6.5.1 does not list falls to
the §7.6.4 branch. The §7.6.5 ledger row stays `reported` — nothing of the decryption is built,
and the refusal is the whole answer, correct today because no reachable recipient exists.
