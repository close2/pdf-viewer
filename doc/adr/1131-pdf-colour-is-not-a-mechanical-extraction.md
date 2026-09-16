# 1131 — `pdf-colour` is not the mechanical extraction the review assumed

Status: accepted. Round 1128.
Context: `doc/reviews/984-direction-and-boundaries.md` Finding 1 and its "First step", which name
two "mechanical extractions, in this order, each one round: `pdf-signature` … then `pdf-colour`
(`colour icc function shading mesh`)". The first was done (`crates/pdf-signature` exists). This ADR
records that the second, measured against the tree at round 1128, is **not** mechanical: it is
blocked by an interpreter-type cycle and a shared test fixture, and is deferred. No code under
`crates/` changes from this ADR. `CLAUDE.md` principle 4 (no cycles) and its "No `[features]`
table" are the two constraints the blockers run into.

## 1. What was measured

The five modules are self-contained at the source level: their only non-`std` `use`s are
`pdf-render`, `pdf-syntax` and `rayon` (`grep -nE "^use " colour.rs icc.rs function.rs shading.rs
mesh.rs | grep -v "crate\|super\|std"`), and `pdf-render` depends on no internal crate, so a
`pdf-colour` sitting on those three is acyclic *in isolation*. The coupling that breaks it is
narrow and specific.

## 2. The three blockers

- **A cycle through the content interpreter (the whole stack).** `shading.rs:24` and `mesh.rs:50`
  do `use crate::content::Transfer`; `Transfer` (`content/ext_gstate.rs:55`) is `[Option<Arc<Function>>; 3]`
  — built on `function::Function`. Extracting `colour icc function shading mesh` whole makes
  `pdf-colour` depend on `pdf-model` (for `Transfer`) while `pdf-model` depends on `pdf-colour`
  (for `function`/`colour`): a crate cycle cargo refuses. Resolving it means relocating the
  `Transfer`/`Stated` cluster out of `content/ext_gstate.rs` — surgery on the 16,982-line
  interpreter Finding 1 lists as a *separate* concern.
- **A `#[cfg(test)]` fixture shared across the boundary (even the clean subset).**
  `soft_mask.rs:719` (a `#[test]`) calls `crate::icc::fixtures::two_way_cmyk_profile()`;
  `icc::fixtures` is `#[cfg(test)] pub(crate) mod` (`icc.rs:2092`). A `#[cfg(test)]` item is not
  compiled when its crate is a dependency, so moving `icc` breaks `soft_mask`'s test. With no
  `[features]` table permitted, the only fixes are shipping test code in the public API or
  duplicating a fixture — the latter is exactly the `CONSISTENT`-×3 shape Finding 7 flags.
- **A private-item doc link.** `image.rs:4957` intra-doc-links `[crate::colour::cie_to_srgb]`, a
  *private* fn; the boundary would need it widened to `pub`. Minor, but it is another item the
  "mechanical" framing did not foresee, and `image.rs` is a file a sibling is editing this batch.

## 3. Decision

Defer. The subset `{colour, icc, function}` (12,364 lines, no cycle) is genuinely extractable and
is the recommended first move, but it still needs the fixture question answered first, and this
round shares a mid-flight worktree in which `doc/conformance/ledger.toml` (the 110-row rewrite's
target) and `image.rs` are already modified. A crate move here would collide.

## 4. What unblocks it, in order

1. Relocate the §10.5 `Transfer` type (with `apply`, `read`, `Stated`) into the colour code; it is
   a colour primitive, and `TransferState`/`Component`/halftone composition stay in the interpreter
   depending on it. 2. Answer the shared-fixture question (a test-support home that is neither
   `#[cfg(test)]`-cross-crate nor a `[features]` gate). 3. Widen `cie_to_srgb` to `pub`. 4. Extract
   with a `pub use pdf_colour::{…}` re-export in `pdf-model` so the ~15 consumer crates and the
   inbound modules are unchanged, on a clean worktree. The ledger rewrite is `sed` over 110 rows
   (`grep -cE "pdf-model/src/(colour|icc|function|shading|mesh)\.rs" doc/conformance/ledger.toml`).

**Step 1 done (round 1134).** `Transfer`, `Stated`, `read` and `apply` now live in
`crates/pdf-model/src/transfer.rs`, a colour-stack sibling of `function`; `TransferState`,
`Component` and `compose` stay in `content/ext_gstate.rs`, depending on `crate::transfer` through
two `pub(crate)` helpers (`from_channels`, `channel`). `shading`/`mesh` import
`crate::transfer::Transfer`; the colour stack now holds zero `use crate::content`. `content::Transfer`
stays a re-export, so consumers are unchanged. Steps 2–4 remain, and step 4's ledger rewrite
subsumes adding `transfer.rs` to the §10.5 row's `code` list. See `doc/history/1134`.

**Step 2 done (round 1138).** The shared fixture is answered by re-derivation, not a shared home.
`icc::fixtures` stays `#[cfg(test)] pub(crate)`; its only cross-crate caller was `soft_mask.rs`'s
test, which now holds its own byte-identical two-way CMYK profile (a private `mod cmyk`) — the
answer `tests/transparency_groups.rs` already gave as an external crate at the same boundary.
Option (b) — export the fixture as a non-test helper — was barred: it has no non-test caller, so
`pub` would ship test code. After the move `icc::fixtures` is reached only from `icc.rs` and
`colour.rs`, both of which move with it, so no `#[cfg(test)]` item crosses the boundary and no
`[features]` gate is needed. See `doc/history/1138`.

**Step 3 done (round 1142).** `crates/pdf-colour` holds the six modules (`colour`, `icc`,
`function`, `shading`, `mesh`, `transfer`) with their tests, over `pdf-syntax`, `pdf-render`,
`rayon`, `md-5` and `thiserror` and nothing from `pdf-model`; `cargo metadata` reports no cycle.
`pdf-model` re-exports each module — `pub use pdf_colour::{colour, function, icc, mesh, shading}`
and `pub(crate) use pdf_colour::transfer` — so the consumer crates and the inbound modules
(`content`, `image`, `soft_mask`) are unchanged. The four snags 1138 recorded were the only ones:
the `transfer` test now walks the page tree through `pdf_syntax::Document` instead of `Pages`; the
three upward doc links became prose; `cie_to_srgb` widened to `pub`. The items `content` and
`image` reach across the boundary — `Transfer::{read, from_channels, channel}`, `Stated`,
`ColourSpace::{default_decode, component_range}` — widened from `pub(crate)` to `pub`, the one
visible cost of the split. The ledger rows' `code`/`test` sites moved to `crates/pdf-colour/` (111
sites); `raster_golden` moved 0. See `doc/history/1142`.
