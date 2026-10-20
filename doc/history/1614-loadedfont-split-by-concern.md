# 1614 — `LoadedFont` split by concern

Session 1614, slot 8 of batch ninety-six, a docs refactoring round (Sonnet 5.5); 2026-10-20. ADR
2064 (the split); 2065 is unused. The owner's words: "Refactor god classes,..." and "Do not fix any
bugs." No bug was fixed; none was found in the code moved.

**Built.** `doc/todo/74` entry 6: `crates/pdf-font/src/loading.rs` keeps `LoadedFont`, `standard`,
`load`, `metrics_only`, the shared types and free functions and the tests (4 319 to 2 390 lines);
eight modules under `loading/` hold the `impl LoadedFont` by concern: `opening`, `findings`,
`codes`, `program`, `advances`, `glyphs`, `drawing`, `substitution`.

**The premise that did not hold.** The entry's five concerns (reading, encoding, glyph lookup,
metrics, substitution) became eight: the encoding has no methods of its own in the block, and the
opening, the findings, the program's own statements and the outlines each hold a dozen.

**The proof that nothing moved.** The multiset of non-blank lines, whitespace stripped: removed-only
are twelve private signatures (which gained `pub(super)`) and the old `use` blocks; added-only are
module docs, `use` blocks, eight `impl LoadedFont {` wrappers, eight `mod` lines and one doc-comment
line (`Engine::Interpreter` written with its `skrifa::outline::` path, since the root no longer
imports `Engine` and the names sweep read the bare prefix as unresolved).

**Ledger.** Fifteen rows' `code` now also name the module citing their clause (habit 96). The
`test` entries name tests that stay in `loading.rs`. No `doc/teaching/` page names `loading.rs`.
`doc/crate-map.md`'s `pdf-font` row says what `loading/` holds.

**Found, not fixed.** No bug in the code moved.

**Unfinished.** Entries 4 and 7 to 11 of `doc/todo/74`; the root's four test modules.

**Gates.** `cargo fmt --check` and `RUSTFLAGS=-D warnings cargo clippy --all-targets` in
`crates/pdf-font`: 0. `cargo test` there: 0, 283 unit tests. `cargo test -p conformance`: 0 on the
second run (the first stopped at `names` on the `Engine::Interpreter` prefix, repaired as above).
Six arms against HEAD's: 968 pages at 1x and 965 at 4x on each of cpu, gpu and compute, 0 missing and 0 differing by digest and mean error.
Duration: about 3 900 s, of which the walk took 1 922 s under the lock.
