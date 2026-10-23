# 1727 — The text interpreter splits by concern

Session 1727, slot 8 of batch 111, the standing code-quality slot; ADR 2288; no question.
No bug fixed; no model change; ledger statuses unchanged.

**Choice.** `crates/pdf-model/src/content/text.rs` (2 306 lines, `Interpreter`'s 1 426-line text impl) is the
module root (392 lines: `TextObject`, `Coverage`, the `WordGaps` re-export and the tests) over eight children in
`content/text/`: `show`, `readback`, `word_gaps`, `glyph_painting`, `placement`, `paint`, `end_object`, `type3`.
Methods called from `content.rs`, `run.rs` or `divide.rs` are `pub(in crate::content)` (what `pub(super)` meant in
`text.rs`); methods and fields only a sibling reads are `pub(super)`, set where the compiler refused.

**Proof.** The multiset of non-comment, non-`use` lines of HEAD's `text.rs` against the union of the nine files, the
visibility prefixes stripped, differs only by two signatures rustfmt rewrapped, `super::ledger` written
`crate::content::ledger`, and the repeated `impl` pairs. Against HEAD's six arms, every page's digest in all six arms
is compared (result under Gates).

**Ledger.** Eight code lists re-pointed: 8.7.3 to `glyph_painting`; 9.3.8 to the root, `end_object`, `glyph_painting`;
9.6.4 to `type3`; 9.10.2 to `readback`, `show`; 11.7.4 to `glyph_painting`, `paint`; 11.7.4.3 adds `show`;
11.7.4.4 to `end_object`, `glyph_painting`, `paint`; 14.9.4 to the root, `readback`, `word_gaps`; the 9.x note naming
`content/text.rs` names `show.rs`. The teaching diagrams of `how-text-is-selected-and-copied.md` name `show.rs` and
`readback.rs`; `how-a-page-lands-on-the-screen.md` names the directory; `doc/crate-map.md` lists the modules.

**Gates.** `cargo test -p pdf-model --lib` 661 passed (32 s); clippy `-D warnings --all-targets`, `cargo fmt --check -p pdf-model` and `cargo doc -p pdf-model --no-deps --document-private-items` with warnings denied exit 0. Against HEAD's six arms on an export of this round's nine files: cpu, gpu and compute at 1x and 4x each exit 0, 968 pages at 1x and 965 at 4x, and zero pages differ in frame digest or mean error (2 754 s under the lock, load average above 30). `raster_golden` on the export passes (58 s). `cargo test -p conformance --no-fail-fast` leaves one failure that is no file of this round's (`the_ledger_agrees_with_the_standard_and_with_the_tree`: §10.7.4 names ADR 2281, which has no file yet); the evidence, ADR-status, names and teaching gates this split broke (a `content::text::glyph_quad` path in `selection_geometry.rs` and a short path in a diagram) are re-pointed and pass.

**Found, not fixed.** The intra-doc links to `Interpreter::…` are now long lines (a later pass may rewrap them);
`show_text` is still one 390-line function under `#[expect(clippy::too_many_lines)]`, whose reason (nine parameters
for the loop's state) still holds; several `pub(super)` fields on `GlyphPainting` and `WordGaps` are read in one
sibling only and could be reached through a method.
