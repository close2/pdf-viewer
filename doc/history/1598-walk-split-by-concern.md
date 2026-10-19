# 1598 — `Walk` split by concern

Session 1598, slot 7 of batch ninety-four, a docs refactoring round (Sonnet 5.5); 2026-10-19. ADR
2032 (the split); 2033 is unused. The owner's words: "Refactor god classes,..." and "Do not fix any
bugs." No bug was fixed; none was found in the code moved.

**Built.** `doc/todo/74` entry 3: `crates/pdf-transform/src/redact.rs` keeps the state, the entry
points and the page's planning and writing (6 052 to 3 837 lines); seven modules under `redact/`
hold the `impl Walk` by concern: `text`, `painting`, `frames`, `images`, `inline`, `graphics_state`,
`regions`. The `impl Walk` left in the root is `new`, `run`, `run_stream` and `operator`.

**The premise that did not hold.** The entry named `text`, `paths`, `images`, `forms`; `paths` and
`forms` already exist as constructions, so the walk's side is `painting` and `frames`, and the inline
splice, `gs` and the region's boxes are three more.

**The proof that nothing moved.** The multiset of non-blank lines: 24 removed-only (22 signatures
that gained `pub(super)`, one import, one module line), the added-only lines being module docs, `use`
blocks, seven `impl` wrappers and the 22 prefixes. `cargo test -p pdf-transform`: all passed.

**Ledger.** Twenty-five rows' `code` re-pointed (habit 96); two notes naming `redact.rs`'s
`type3_codes` now name `redact/text.rs`. No `doc/teaching/` diagram names `redact.rs`.

**Unfinished.** Entries 4 to 11 of `doc/todo/74`; the root's page-writing half (about 2 000 lines of
free functions) is not a `Walk` method and is the next cut if the census asks for it.

**Gates.** `cargo fmt --check -p pdf-transform`: 0. `RUSTFLAGS=-D warnings cargo clippy -p
pdf-transform --all-targets`: 0. `cargo test -p pdf-transform`: every target passed. `cargo test --profile
gates -p pdf-transform --test foreign_corpus -- --ignored`: 0, 1 passed. `cargo test -p conformance
--no-fail-fast`: every target passed but `bounded` (`tools/wait.sh:[24]` declares no `--lock` kind, a
sibling's file) and `records` (this record's own gates, fixed by this paragraph). `--bin quotations` and
`--bin pointers`: 0, none naming `redact`. Six arms: not walked, because neither `render-raster`'s
corpus binary nor `pdf-sandbox` has `pdf-transform` in its dependency graph (`cargo tree -i` finds
no package), so those binaries cannot differ. Duration: about 11 400 s, nearly all of it the lock's queue.
