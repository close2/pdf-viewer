# 1670 — `Walk`'s survey of a document split by concern

Session 1670, slot 8 of batch one hundred and three, a docs refactoring round (Sonnet 5.5); 2026-10-21.
ADR 2176 (2177 unused). The owner's words: "Refactor god classes,..." and "Do not fix any bugs." None
was fixed. `doc/todo/74` entry 12 (new): `impl Walk` in `crates/pdf-archive/src/survey.rs`.

**Built.** `survey.rs` (4 300 to 2 862 lines) keeps `Survey`, the observation types, the walk's state
and the tests; eleven modules under `survey/` hold `impl Walk` by concern: `pages` (190 lines),
`operators` (245), `fonts` (140), `xobjects` (226), `graphics_state` (71), `colour_spaces` (109),
`painting` (98), `patterns` (87), `records` (186), `literals` (157) and `marked_content` (98).

**The premise that held.** The brief named `Walk` in `pdf-archive` as a candidate; slot 1 owns the
crate but edits `table/` and `pdf-transform`, not `survey.rs`. The entry numbering moved: "Files with
no large block" and "Functions" became 13 and 14.

**The proof that nothing moved.** Line multiset of the old file (whitespace removed) against the root
and the eleven modules: removed-only 6 lines (two import lists, four signatures re-wrapped), added-only
143 non-blank (module docs, `use`/`mod`, eleven `impl Walk<'_> {` wrappers with braces, the wrapped
signatures). `pdf-archive` is outside the arms' graph: the six arms are not owed.

**Ledger and docs.** Eight rows that named `survey.rs` also name the module holding the row's item;
`doc/crate-map.md` and `doc/todo/74` name the new files; no teaching walk names `survey.rs`. Four doc
links re-qualified. Unfinished: the root keeps 1 500 lines of tests and about 1 000 of `Survey`.

**Found, not fixed.** No bug in the code moved. `cargo doc -p pdf-archive --document-private-items`
reports links to private items in `clarification.rs`, `finding.rs`, `reach.rs`, `requirement.rs`,
`editions.rs` and `survey.rs` (`keyword`, `SHOWN_BUDGET`, `MAX_IDENTIFIERS`); `pdf-archive` is not in
`tools/state.sh docs`'s held list. A `const fn` is skipped by a naive item scan (it names `fn`).

**Gates.** `cargo fmt --check -p pdf-archive`: 0. `RUSTFLAGS=-D warnings cargo clippy -p pdf-archive
--all-targets`: 0. `cargo test -p pdf-archive --lib`: 226 passed. `cargo test -p conformance
--no-fail-fast`: every binary ok, no failure. `--bin quotations` and `--bin pointers`: exit 0.
Duration: about 1 500 s.
