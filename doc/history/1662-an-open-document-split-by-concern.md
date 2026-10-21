# 1662 — `Open`'s document walk split by concern

Session 1662, slot 8 of batch one hundred and two, a docs refactoring round (Sonnet 5.5); 2026-10-21.
ADR 2160 (2161 unused). The owner's words: "Refactor god classes,..." and "Do not fix any bugs." None
was fixed. `doc/todo/74` entry 11 (new): `impl Open` in `crates/viewer-core/src/open.rs`.

**Built.** `open.rs` (2 545 to 1 197 lines) keeps `Open`, the shared types, the constructors,
`first_read`, `interpret`, `Preparation` and `Prepared`, and the tests; seven modules under `open/`
hold `impl Open` by concern: `deferred` (92 lines), `interpretation` (165), `pages` (98), `edits`
(328), `magnification` (180, with `raster_extent`, `content_box`, `fitted`), `fragments` (411) and
`destinations` (165). `raster_extent` is re-exported under the path `layout.rs` uses.

**The premise that did not hold.** The brief's candidates were `Walk` in `pdf-archive`, `Interpreter`
in `content/text.rs` and `Open`; `crates/pdf-archive/` is slot 1's, so `Open` was taken. The entry
numbering moved: "Files with no large block" and "Functions" became 12 and 13.

**The proof that nothing moved.** Line multiset of the old file (indent removed) against the root and
the seven modules: removed-only 5 lines (the old import list, two `use` lines, and the signatures of
`content_box` and `fitted`, now `pub(super)`), added-only 81 non-blank (module docs, `use`/`mod`, six
`impl Open {` wrappers with their braces, the widened signatures). `recount` was moved from
`interpretation` to `pages` after the multiset was taken; the multiset was unchanged by it.
`viewer-core` is outside the arms' graph (`cargo tree -p render-raster -e normal,dev,build` and
`-p pdf-sandbox` print no `viewer-core`): the six arms are not owed.

**Ledger and docs.** Eighteen rows that named `open.rs` also name the module holding the row's item
(two stay on the root: §8.10.4.2 and §12.6.3 are fields); two teaching walks name `open/edits.rs` and
`open/interpretation.rs`; `doc/crate-map.md` and `doc/todo/74` name the new files. Four doc links
re-qualified (habit 126). Unfinished: `fragments.rs` is 411 lines, `apply_parameter` in it the next cut.

**Found, not fixed.** No bug in the code moved. `first_read` carries two doc paragraphs, the second
(`The interpretation of a page, with its reports already worded`) describes `interpret`, which has
none. Two `[t]`/`[a]` standard quotations read as links in rustdoc (`open/pages.rs:24`,
`open/edits.rs`, `open.rs`), older than this split. `cargo test -p conformance --no-fail-fast`:
`no_comment_names_a_round_by_number` fails on siblings' scratch examples
(`pdf-transform/examples/r1655_no_descriptor.rs`, `render-raster/examples/medium_price.rs`), not mine.

**Gates.** `cargo fmt --check -p viewer-core`: 0. `RUSTFLAGS=-D warnings cargo clippy -p viewer-core
--all-targets`: 0. `cargo test -p viewer-core`: all binaries ok (84, 4, 2, 3, 1, 5, 6, 21, 141, 7
passed). `cargo test -p conformance --no-fail-fast`: one failure (above, siblings'); `teaching`
green after the fix. `--bin quotations` and `--bin pointers`: exit 0. Duration: about 3 600 s.
