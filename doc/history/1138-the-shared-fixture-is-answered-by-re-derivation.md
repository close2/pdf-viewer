# 1138 — the shared fixture is answered by re-derivation

Round 1138, architecture slot. ADR 1131 §4 step 2 (the shared `#[cfg(test)]` fixture) done; step 3
(create `crates/pdf-colour`) deferred with a new blocker measured.

## Step 2 — the fixture

`icc::fixtures` is `#[cfg(test)] pub(crate)`; a dependent crate cannot see it, so once `icc` moves
into `pdf-colour`, `soft_mask.rs`'s test (staying in `pdf-model`) could not reach across for
`two_way_cmyk_profile`. Of ADR 1131 §4's three options, (a) can't apply — the test tests
`entry_with_output_intent`, a `soft_mask` function that stays in `pdf-model`; (b) is barred — the
fixture has no non-test caller (`grep` for `icc::fixtures` finds only tests), so `pub` would ship
test code. So (c): the test now holds its own byte-identical two-way CMYK profile in a private
`mod cmyk`, the answer `tests/transparency_groups.rs` already gave as an external crate at the same
boundary. `icc::fixtures`' only remaining callers, `icc.rs` and `colour.rs`, both move with it: no
test item crosses the boundary, no `[features]` gate, no public test code.

## Step 3 — deferred, with evidence

- **A new cycle.** `transfer.rs:222` (a `#[cfg(test)]` test, created by step 1 in round 1134) calls
  `crate::Pages::new`; `Pages` is `page.rs`'s, above colour. Moving `transfer` makes `pdf-colour`'s
  dev-deps need `pdf-model` while `pdf-model` needs `pdf-colour` — a cycle `cargo metadata` reports.
  Absent from ADR 1131 §4 because `transfer.rs` post-dates it.
- **Upward doc links.** `colour.rs` `[crate::image]`, `shading.rs`/`mesh.rs`
  `[crate::Unsupported::LimitReached]` resolve to `pdf-model` items and break below it.
- **Sibling collision.** Step 3's `cie_to_srgb`→`pub` needs `image.rs`'s intra-doc link re-pointed;
  `image.rs` is a sibling's file this batch (1137, rendering). High-collision refactor round.

The six modules hold zero `use crate::content` and zero non-test code reference to a pdf-model item
outside the six (`grep`, all six): code-clean but for the test above. A clean worktree moves the
six, fixes the three links and widens `cie_to_srgb` in one step.

## Gates

`cargo test -p pdf-model --lib soft_mask::a_luminosity…` 1 passed. `clippy -p pdf-model --lib
--tests` exit 0, my code 0 warnings. `raster_golden` held 974, moved 0. `cargo test -p conformance`
green bar `records`, which fails on siblings 1139/1137's over-budget records, not mine.
`rustfmt --check soft_mask.rs` clean.
