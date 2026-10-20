# 1638 — `Rewriter` split by concern

Session 1638, slot 8 of batch ninety-nine, a docs refactoring round (Sonnet 5.5); 2026-10-20. ADR 2112
(2113 unused). The owner's words: "Refactor god classes,..." and "Do not fix any bugs." None was fixed.
`doc/todo/74` entry 4, `Rewriter` in `crates/pdf-transform/src/archive/rewrite.rs`.

**Built.** `rewrite.rs` (5 470 to 3 209 lines, about 1 100 of them `Rewrite`'s descriptions) keeps
`Rewrite`, the state, the object walk and the routing (`rewrite`, `rewrite_array`, `rewrite_dictionary`,
`owed_rewrites`, `wants*`); eight modules under `rewrite/` hold `impl Rewriter` by concern: `values`,
`appearance_states`, `configurations`, `page_structure`, `attachments`, `annotations`, `catalogue` and
`streams`. The entry's seven families became eight: the restated values and the descriptor and
`/CIDToGIDMap` rewrites went to `values`, and `preserve` joined the appended pages.

**The premise that did not hold.** None; the entry's method list matched the file. The census had the
block at 2 612 lines and 65 methods; the split left no impl block above 1 000 lines in this file.

**The proof that nothing moved.** Line multiset of the old file against the root and eight modules:
removed-only 65 lines (signatures that gained `pub(super)`, `super::` paths re-qualified to
`super::super::`, import lists), added-only 136 (module docs, `use`/`mod` lines, eight `impl` wrappers
and the same lines). 40 methods are `pub(super)`, each widened from E0624; none narrowed (habit 137).
`pdf-transform` is outside the arm binaries' graph (`cargo tree -i`: the virtual file systems and the
hosts), so the six arms are not owed.

**Ledger.** Twenty-three rows named `rewrite.rs`; each now names the files that cite `§<clause>`
(habit 96, 19 rows changed). `doc/crate-map.md` and `doc/todo/74` follow; no teaching diagram names it.

**Found, not fixed.** No bug. Siblings' mid-edits broke shared gates while this ran: four JPX
transcode tests failed once during a lock change and passed on re-run; `conformance` fails on slot 3's
`§7.4.7` test name and `state_sections` on slot 4's removed `asked.rs`, neither mine.

**Unfinished.** Entries 9 to 11; `Rewrite`'s 1 100 lines of descriptions.

**Gates.** `cargo fmt --check -p pdf-transform` and `RUSTFLAGS=-D warnings cargo clippy -p
pdf-transform --all-targets`: 0. `cargo test -p pdf-transform`: 0, 244 passed in `archive` and 96 in
the lib. `cargo test -p conformance --no-fail-fast`: not green, only on the two sibling causes above;
every other test binary passed. Duration: about 3 000 s.
