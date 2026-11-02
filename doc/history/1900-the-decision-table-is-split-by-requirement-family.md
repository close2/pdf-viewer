# 1900 — the decision table is split by requirement family

Session 1900, batch 130, Slot 10 (docs), in two sessions. ADR 2634; ADR 2635 and Q776 unused.

**What.** `crates/pdf-transform/src/archive/decision.rs` keeps the shared types, the dispatch and the
four tables' documentation; six modules under `archive/decision/` (fonts, colour and graphics,
content-stream fences, files and structure, annotations and attachments, metadata —
`doc/pdfa-blocking-issues.md` sections 3 to 6) hold each family's rows of `REMEDIES`, `CONDITIONAL`,
`SHAPES` and `REFUSED_BY_NAME` and the sentences and helpers only those rows read. Each table is
assembled at compile time from its families' slices by two `const fn`s. The split was written by a
script from the base's file, so every moved line is the base's but for the module path one level
deeper, a visibility word on an item's head line, and one import divided between two modules; the new
prose is the six module documents, the family slices' heads and the two helpers.

**Premise.** "Six modules, the dispatch table unchanged" holds for the dispatch. The tables' row order
became the families' order; no consumer reads it (each requirement has one row a table, and every
reader finds, tests or maps by requirement), and the census shows none of it.

**Proof.** A line-for-line comparison: 4 345 non-blank lines of the base, 4 330 matched once module
paths and visibility words are normalised, the other 15 the divided import and rustfmt's reflow of
lines the longer paths pushed past the width, proven by a token comparison (11 498 tokens of the base,
none missing); 395 string literals, every one byte-identical with its multiplicity.

**Gates.** In the round's export: `cargo check -p pdf-transform --tests` exit 0; `RUSTFLAGS="-D
warnings" cargo clippy -p pdf-transform --all-targets --no-deps` exit 0; `RUSTDOCFLAGS="-D warnings"
cargo doc --no-deps -p pdf-transform` exit 0; `cargo test -p pdf-transform --test archive --test
profiles` exit 0, 515 passed and 6 ignored, 13 passed, the same as the base's file in the same export;
the census without the CJK pack (exit 0, 823 lines) and with it (exit 0, 815 lines) identical line by
line to HEAD's, the harness's one-minute notice printed at another line aside. The plant: one word of
`SYNTAX_INSIDE_CONTENT` changed in `decision/content_stream_fences.rs`, the census changed on its four
lines and nowhere else; reverted, `cmp` equal. In the worktree, after the three-way merge: `cargo
check -p pdf-transform --tests` exit 0; `--bin cited` exit 0 (its three lines naming `decision/` are
pairs whose rows did not name `decision.rs` at the base either); `--bin quotations` exit 0;
`cargo test -p conformance --no-fail-fast` exit 0, 69 binaries, 470 passed, 0 failed;
`tools/comment-history.py` finds no line in the seven files.

**Found, not fixed.** `crates/pdf-transform/tests/archive.rs:1697`'s message asks for "a row in
decision.rs"; the rows now sit in `archive/decision/`, one module a family.
