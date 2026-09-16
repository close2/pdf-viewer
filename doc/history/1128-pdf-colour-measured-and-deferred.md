# 1128 — `pdf-colour` measured and deferred; the Annex C refusal says it is a choice

2026-09-16. Files: this record; `doc/adr/1131-pdf-colour-is-not-a-mechanical-extraction.md`;
`doc/reviews/984-direction-and-boundaries.md` (dated addendum appended, body untouched); one doc
paragraph in `crates/pdf-transform/src/optimize.rs`. Every other `git status` path is a sibling's
(this batch shares one worktree). No ledger edit; no test moved.

## The contract was Finding 1's residue — the `pdf-colour` extraction — and it is not mechanical

Review 984 named it one of "two mechanical extractions". Measured against the tree (trap 8, before
proposing any move), it is not. Three blockers, with the coupling map in ADR 1131:

- **Whole stack → a crate cycle.** `shading.rs:24`/`mesh.rs:50` `use crate::content::Transfer`;
  `Transfer` is built on `function::Function`. Move `colour icc function shading mesh` out and
  `pdf-colour` depends on `pdf-model` (Transfer) while `pdf-model` depends on `pdf-colour`
  (function) — cargo refuses. The fix relocates `Transfer` out of the 16,982-line interpreter
  Finding 1 keeps separate.
- **Subset `{colour,icc,function}` → a `#[cfg(test)]` fixture across the boundary.**
  `soft_mask.rs:719` (a test) uses `crate::icc::fixtures::…`, `#[cfg(test)] pub(crate)` in
  `icc.rs:2092`; not compiled for a dependent. With no `[features]` allowed, the outs are shipping
  test code publicly or duplicating a fixture (the `CONSISTENT`-×3 shape F7 flags).
- **A private-item doc link** (`image.rs:4957 [crate::colour::cie_to_srgb]`) needs `pub`.

Source-level the five modules are clean (only `pdf-render`, `pdf-syntax`, `rayon`; `pdf-render` has
no internal dep), so the subset is the right *first* move once the fixture question is answered —
on a clean worktree, not this batch's shared one where `ledger.toml` and `image.rs` are already
dirty. Deferred, per the contract's cycle branch. `cargo tree`: no crate added, so no new cycle.

## The Finding-7 inversion taken instead: the Annex C wording

Most of F7 is already fixed since 984 (clippy.toml threshold, the two `allow`→`expect`, the crate
map's `pdf-archive` row and `render-raster` name, the lexer's Acrobat/pdf.js reason). Still live:
`optimize.rs`'s refusal cited "§C.4's recovery" without saying Annex C is *informative*. Added one
paragraph at the canonical decision (`refuse_a_document_only_recovery_reads`) quoting the annex
title, "Advice on maximising portability", and framing the refusal as this program's choice, not a
rule the standard imposes — F7's "should say so". The seven shorthand sites cite this one.

Gates: tier 1 build/lint/nextest, `cargo tree`, `cargo test -p conformance` — lines in the report.
