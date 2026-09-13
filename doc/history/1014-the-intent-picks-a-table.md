# 1014 — the intent picks a table

Date: 2026-09-13. Branch: `batch-1006-1011`, worktree `/home/AI/pdf-viewer-rounds`, shared with
five sibling rounds. ADR: [1032](../adr/1032-the-intent-selects-the-transform.md).

Three ledger rows: §11.7.5.3's `A2B0`/`A2B2` selection by intent, §8.6.5.8's `/RI`, and §8.9.5.1's
`/UseBlackPtComp` at the image `Do` — the last located in `crates/pdf-render`'s `image.rs` at six
paint call sites. **That defect was closed in the six-hundred-and-seventh session**: `pdf-render`
has no `image.rs`, the six literal `true`s in `pdf-model/src/image.rs` are long gone, and all three
routes to an intent were already read. The live half of all three rows was one thing: an intent
read, kept apart from the other names, carried the length of `crate::colour` and consumed by
nothing, because `crate::icc::Profile` evaluated `A2B1` whatever any intent said.

`icc.rs` gains `A2b` (which of a profile's three "to CIE" transforms an intent selects) and
`Rendering` (that beside §8.6.5.9's black point); `Profile` keeps its `A2B0` and `A2B2` tags as
bytes and parses one on first use, with its own black point, falling back to the colorimetric route
where the profile states no table of that intent's. In `colour.rs` `Rendering` replaces the
`black_point: bool` that already travelled everywhere, so `Conversion`, `Compositing::paint` and
every `ColourSpace` conversion carry both and neither can be supplied without the other.
`GraphicsState::rendering`, `rendering_under` (an image's own `/Intent`) and
`PatternInitial::rendering` (§11.6.7's definition state) replace the three `black_point` readers.
`tests/rendering_intent.rs` gains a hand-built profile whose `A2B0`, `A2B1` and `A2B2` answer a
mid-tone red, neutral and blue — ends shared, so compensation and the device range are identical
and a probe reads the table alone — and six tests: the three-colour calibration, `ri`,
`/ExtGState /RI`, an image's `/Intent` disagreeing with the page's `ri`, the one-table fallback,
and an absolute intent taking the colorimetric table uncompensated. All three rows stay `partial`,
each naming a smaller debt: §8.6.5.8's is one half of one name (`AbsoluteColorimetric`'s media
white point, which ISO 15076-1 derives through `wtpt` and nothing here reads); §11.7.5.3 keeps
black generation and the press's conversion *out* at a group `Do`.

**A silence that was not one.** Table 69 describes each intent as an appearance and no arithmetic;
§8.6.5.8's sentence about the ICC and §10.3.1's `shall` answer it between them and §10.4.2.1 ranks
that route — the third time here that "the specification defines nothing" was a claim nobody
checked. **Calibrated (trap 13)**: with `Profile::route` forced to the colorimetric branch, the
four selection tests fail and the two pinning what did *not* move still pass.

**Gates.** `cargo fmt -p pdf-model --check` 0 · `cargo nextest run -p pdf-model` 0, 1314 passed ·
`cargo test -p conformance` 0 · `cargo check --workspace --all-targets` 0 ·
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-model --all-targets` 101, on five lints in a sibling
round's live `structure.rs` and nothing else — the same run with that file's two lint classes
allowed (`-A clippy::doc_markdown -A clippy::float_cmp`) is 0. `cargo fmt -p pdf-model` reformatted
two files that round had open; package scope is the finest `cargo fmt` offers.
