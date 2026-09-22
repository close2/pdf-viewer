# 1191 — a policy is read, a curve is re-measured, and two residues were nobody's

Contract: the §12.8 family's blocked rows re-measured and the buildable ones built, plus §9.8.3.3
and §12.11.3 disposed of honestly. ADRs 1219 and 1220.

## What moved

- **§12.8.3.4.4 stays `partial` and gains the reader.** `pdf_signature::policy` reads ETSI EN 319
  122-1 clause 5.2.9's attribute whole — which policy, its digest with the all-zero *not known* kept
  apart by name, and the three qualifiers clause 5.2.9.2 defines — and clause 5.2.10's store beside
  it, with `SignaturePolicy::binding` making the comparison that clause's own note describes.
  `Signature::signature_policy` is the entry; a corpus test asks all ten signatures and finds none,
  held at zero; the census carries the column for the crawl. ADR 1219.
- **§9.8.3.3 `partial` → `implemented`.** The clause addresses one act to a processor — the class
  descriptor's overriding, "for that class only" — and it is performed. Every other sentence binds
  the dictionary a producer writes; none asks a reader to reject or report. ADR 1220.
- **§12.11.3 and §12.11 `partial` → `implemented`.** The residue is a sentence with no modal verb in
  it, and its one outward-looking half carries the clause's own condition, "if alternatives are
  available". ADR 1220.
- **§12.8.3.1's measurement re-taken 2026-09-22.** Nothing released: `ed448-goldilocks` is still
  0.14.0-pre.15 (2026-06-24), no `bp512` exists, and RustCrypto's brainpoolP512r1 is pull request
  1914, open and last touched 2026-09-18. The row already named `ed448` 0.5.0 and `bp512-nestler`
  0.2.1 as refused for reasons that are not a version number; both hold.

## What was not done, and why

- **The policy reaches `pdf-signature` and not the program.** `viewer-core/src/notes.rs` belongs to
  another round this batch, so nothing says the policy, the binding or the user notice out loud yet.
  That is the "reached the crate and never reached the program" shape and it is owed.
- Tier 2's pixel lines were not run: the diff is a new module, one test and two doc comments, no
  drawing path changed — and `pdf-render`, `pdf-model` and the rasterisers are mid-edit by siblings,
  so a walk now would measure their work rather than mine. The merge runs them all.

## Gates

`rustfmt --check` on my eight files (0); `clippy -p pdf-signature --all-targets` with
`RUSTFLAGS="-D warnings"` (0); `nextest -p pdf-signature` 194/194; `cargo test -p pdf-font --lib
glyph_class` 8/8; `cargo test -p conformance` — two failures, both a sibling's
(`pdf-model/src/view.rs`, `pdf-syntax/examples/linearised_census.rs`,
`pdf-model/tests/shadings.rs`), none in my rows or files.
