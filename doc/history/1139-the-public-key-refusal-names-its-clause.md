# 1139 — The public-key refusal names its own clause, and §7.6.5 is deferred behind the crate graph

2026-09-16. Files: `crates/pdf-syntax/src/crypt.rs`, `crates/pdf-syntax/tests/encryption.rs`,
`doc/conformance/ledger.toml` (§7.6.5 row), `doc/questions/Q66-*`, `doc/adr/1134-*`, this record.
Contract: §7.6.5 (public-key security handlers) and §7.6.4.1's residue. No pixel touched; five
siblings mid-flight, all changes inside the crypt/security-handler code and §7.6.5's row.

## Census first (trap 8)

`grep -la` over `corpus-cache/` (89 286 pdfs; submodules not checked out here): **5** name
`/Filter /Adobe.PubSec`, all `/Recipients` + `adbe.pkcs7.s5` — matching the §7.6.5 row's session-892
count of five over 90 535. Its finding stands: none of the five carries a recipient certificate that
could belong to this reader (four are one Apache bug report's attachments; the fifth is device-bound,
`zune-tuner://…`), so §7.6.5.1's `shall` — find a match — has none, and a build's gain is zero docs.

## What the unwrap needs, and why it was not built

Over `crypt.rs`'s AES-CBC + SHA-256, §7.6.5 needs a CMS `EnvelopedData`/`RecipientInfo` reader,
X.509 recipient matching, and RSA **decryption** (RSADP, `c^d mod n`). None is a new primitive —
`pdf_signature::bigint::modpow` over `crypto-bigint` is the arithmetic, and `der`/`cms`/`x509` read
certificates and `SignedData` — but **all live in `pdf-signature`, which depends on `pdf-syntax`**,
so `crypt.rs` cannot reach them without a cycle, and `pdf-syntax` carries none of them. No
`EnvelopedData` parser or RSA-decryption path exists anywhere (`cms.rs` is `SignedData` only). So
it is a `doc/stack.md` dependency choice — a public-key stack duplicated into `pdf-syntax` vs. a
shared crypto crate below both — the owner's call: `Q66`. `ADR 1134` records the deferral and the
host key shape (ADR 1076's `Supply`: a host input, once, a `Secret`, default none).

## What was built

`crypt::Encryption::new` refuses the three `/SubFilter` values §7.6.5.1 lists via
`reject_non_standard_handler`, naming the handler as a public-key one **by §7.6.5** instead of the
generic "not the standard handler (§7.6.4)" that misattributed the clause. Calibrated test with a
control (trap 13): a public-key dict is refused naming §7.6.5; a subfilter §7.6.5.1 does not list
falls to the §7.6.4 branch. Row `7.6.5` stays **`reported`** — the refusal is the whole answer and
correct today, since no reachable recipient exists.

## Gates

`test -p pdf-syntax --test encryption` (lock): exit 0, 27 passed. `fmt -p pdf-syntax --check` and
`clippy -p pdf-syntax --all-targets`: exit 0. `batch.sh check` clean but for siblings' `redact.rs`.
