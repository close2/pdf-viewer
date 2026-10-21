# 1669 — `viewer-core`'s and `render-raster`'s rustdoc held at zero, and the history gates read C

Session 1669, slot 7 of batch one hundred and three, a docs round under the owner's switch to code
improvement; 2026-10-21. No ADR (2174 and 2175 unused): ADR 1938 gives every answer and its status
line names the two crates. No Q564. No ledger row moves; no clause is read.

**Premise.** Held in kind, larger in count: the doc build failed with 136 errors at 136 locations
(112 bracketed letters, 16 private links, 8 for a hand), not the handful the records named. The
hypothesis on C did not hold: `quorra.h` holds no ordinal at HEAD; `open_a_page.c` held two (606,
1450), and `doc/running-the-viewer.md` eighteen, not one.

**Done.** `tools/rustdoc-brackets.py --apply` made 128 edits in 25 files. By hand: `Self::present`
(gone) is `Self::render` or `Self::render_page_only` and the paragraph says what the call answers;
`Self::record` on a free function is `UncapturedErrors::record`; `pdf_render::thin_segment` by a
reference definition `fn@`; `viewer_accessibility::role` (a crate depending on this one) a code span;
`crate::Interpretation::text` is `pdf_model::Interpretation::text`; the quoted `[`Control::CheckBox`]`
kept verbatim by a reference definition; a test-only definition dropped, its use a code span. The
paragraph about `interpret` moved from `first_read` to `interpret`. `tools/state.sh docs` holds nine
crates and is a `large` walk: the walk log's lines for it peak at 4.4 to 5.8 GiB with seven crates,
and the nine reached 6.73 GiB on a rebuild and were killed by the small lane's 6 GiB.

**The gates.** `round_numbers` lexes `.c`/`.h` in C's terms (no nesting, no raw string, a block
comment's ` * ` decoration skipped so a wrapped number follows its word) and reads the document as
prose; `spelled_ordinals` reads both line by line; each asserts it read C. The two C sites and the
eighteen lines are rewritten as what is, citing the ADR, with the three sentences that pointed back
at them ("until that session", "that round") and `open_a_page.c:195`'s "before this round". `doc/PLAN.md`'s sentence on the two gates says so.

**Gates.** Multiset of non-comment lines, my 131 hunks applied to HEAD's 27 files: 0 differ (a
planted `index: u32` named). `tools/state.sh --round 1669 docs`: 0 over nine crates, last (2.65 GiB); a planted `[t]he`
in each crate: 101 naming both. `round_numbers`, `spelled_ordinals`: 0; a planted C header and line
in the document: 101, each naming both files (the ordinal, `Session 411` and a decorated wrap).
`cargo test -p conformance --no-fail-fast`: 454 passed, 0 failed. `--bin quotations` 0, `--bin
pointers` 0. `cargo fmt --check -p viewer-core -p render-raster -p conformance` 0. In the export
(HEAD + my hunks): `-D warnings` clippy of the three `--all-targets` 0; `nextest -p viewer-core -p
render-raster` 511 passed; `viewer-ffi`'s C program test: 2 passed. About 4 100 s, 1 759 s of it the last docs walk
queued behind another round's arms on the first lane — the price of `large`.
