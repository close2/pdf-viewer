# 1605 — `Tree` split by concern

Session 1605, slot 7 of batch ninety-five, a docs refactoring round (Sonnet 5.5); 2026-10-19. ADR
2046 (the split); 2047 is unused. The owner's words: "Refactor god classes,..." and "Do not fix any
bugs." No bug was fixed; none was found in the code moved.

**Built.** `doc/todo/74` entry 5: `crates/pdf-model/src/structure.rs` keeps `Tree`, the `/K` entry
points, `logical_range` and the shared types and helpers (8 430 to 3 632 lines); five modules under
`structure/` hold the `impl Tree` by concern: `roles`, `attributes`, `tables`, `owners`, `logical`;
the tests are `structure/tests.rs` (2 908 lines).

**The premise that did not hold.** None; the entry's five concerns held, with roles and namespaces
one module because both end in the same role-map walk.

**The proof that nothing moved.** The multiset of non-blank lines, whitespace stripped: one
removed-only line (`fn descend(`, which gained `pub(super)`); the added-only lines are module docs,
`use` blocks, five `impl Tree {` wrappers, five `mod` lines and the root's module paragraph.

**Ledger.** Forty-two rows' `code` re-pointed to the module citing their clause, 91 `test` entries
now name `structure/tests.rs` (habit 96). No `doc/teaching/` diagram names `structure.rs`.

**Coordination.** `logical_range` stays in the root because slot 5 edits it with `-U0` hunks; moving
it into `logical` is one cut after that merge. Line numbers in `structure.rs` shifted by about 1 900.

**Unfinished.** Entries 4 and 6 to 11 of `doc/todo/74`; the root's types and helpers (about 3 000
lines) are a different split.

**Found, not fixed.** No bug in the code moved. A sibling's `tools/__pycache__/` broke the workspace
glob (every cargo command failed); I deleted that generated directory.

**Gates.** `cargo fmt --check -p pdf-model`: 0. `RUSTFLAGS=-D warnings cargo clippy -p pdf-model
--all-targets`: 0. `cargo test -p pdf-model --lib`: 0, 657 passed. `cargo test -p conformance
--no-fail-fast`: every target passed (an earlier plain run stopped at `bounded` while slot 6 edited
`tools/bounded.sh`). `--bin pointers`: 0, 9 absent, none naming `structure`. `--bin quotations`: 0.
`tools/state.sh docs`: 0. `t3-accessibility`: not run. Six arms (an export of HEAD plus the `structure` paths against HEAD's arms, by page): cpu, gpu and compute at 1x and 4x each exit 0, 968 pages at 1x and 965 at 4x, none missing, 0 frame digests or mean errors differing.
Duration: about 3 000 s.
