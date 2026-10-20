# 1622 — the transparency and pattern interpreters split by concern

Session 1622, slot 8 of batch ninety-seven, a docs refactoring round (Sonnet 5.5); 2026-10-20. ADRs
2080 and 2081. The owner's words: "Refactor god classes,..." and "Do not fix any bugs." None was fixed.

**Built.** `doc/todo/74` entry 7. `crates/pdf-model/src/content/transparency.rs` (6 900 to 2 453
lines, 1 963 of them tests) keeps `TransparencyGroup`, `GroupBody`, `GroupRun`, the group entry points
and the tests; twelve modules under `transparency/` hold the rest: `transfer`, `commands`,
`blending_space`, `into_parent`, `press`, `group_runs`, `soft_mask`, `readings`, `shape`, `knockout`,
`implicit_knockout`, `departures`. `pattern.rs` (2 458 to 516 lines) keeps the paint types and `pattern`,
`tile`, `fill_paint`, `stroke_paint`, `mark_transfer`; eight modules beside `reach.rs` hold the rest:
`initial`, `colouring`, `shading`, `tiling`, `lattice`, `cell`, `repeat`, `wrapping`.

**The premise that did not hold.** The hypothesis named blend modes as a concern; they appear only as
`blend_at_the_do` (in `implicit_knockout`) and one soft-mask note. The large concerns were the
alpha-source readings, the shape statements and the blending-space departure. The pattern's "tiling,
shading and the colour of each" held, with the tiling cut four ways.

**The proof that nothing moved.** The multiset of tokens of each old file against its root and modules,
visibility words removed: removed-only is empty for `pattern.rs` and one `[` and one `]` for
`transparency.rs` (a doc link written as a code span). Added-only are module docs, `use` blocks, `mod`
lines, `impl Interpreter` wrappers and `crate::content::` for `super::` in moved bodies. The line
multiset also shows signatures `rustfmt` re-wrapped for `pub(in crate::content)` and the test imports.
Six arms of an export of the tree against HEAD's: 0 missing, 0 differing by digest or mean error.

**Ledger.** Twenty-six rows' `code` name the module that cites their clause (habit 96), read by
`§<clause>` in each file; three cite it in neither root and keep it. The teaching walk's two table rows,
`doc/crate-map.md`, `doc/todo/74` and `doc/todo/23` follow; stale paths in seven comments and twenty-two
doc links in the moved code are re-qualified (habit 126).

**Found, not fixed.** No bug. Quotes with a bracketed letter ("appl[ies]" in `press.rs`) read as rustdoc
links; about 800 such errors exist in `pdf-model` under `-D warnings`, which ADR 1938 does not hold.

**Unfinished.** Entries 4 and 8 to 11 of `doc/todo/74`; the root's 1 963 test lines.

**Gates.** `cargo fmt --check -p pdf-model` and `RUSTFLAGS=-D warnings cargo clippy -p pdf-model
--all-targets`: 0. `cargo test -p pdf-model --no-fail-fast`: 0, 2 120 passed, 21 ignored (the first run stopped at five `image_masks` tests on a stale `pdf-sandbox-worker`, rebuilt).
`cargo test -p conformance`: 0, 443 passed. Six arms, 968 pages at 1x and 965 at 4x on each of cpu, gpu
and compute (5 799 lines): 0 missing, 0 differing; one lock hold of 1 803 s, peak 3.84 GiB.
Duration: about 3 300 s.
