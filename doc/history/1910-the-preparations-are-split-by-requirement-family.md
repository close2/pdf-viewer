# 1910 — the preparations are split by requirement family

Session 1910, batch 131, Slot 10 (docs), in two sessions. ADR 2654; ADR 2655 and Q786 unused.

**What.** `crates/pdf-transform/src/archive/prepare.rs` keeps `Prepared`, its one entry
`Prepared::of`, `Owed`, `Spare` and what two families share; six modules under `archive/prepare/`
(font rewrites, colour and graphics, files and structure, annotations and attachments, metadata,
appended pages) hold each family's preparations, sentences and helpers. A script wrote the split,
every moved line the base's but for ADR 2654's seven rewrites (module paths, visibility words, an
`include_bytes!` path, doc-link targets); the new prose is the seven module documents. Regenerated
from the worktree's file, which carries this batch's archive hunks, it landed in one window; three
doc comments and the PDF/A teaching walk that named a moved item by its old path name its module.

**Premise.** "Six modules, no function over 100 lines introduced" holds: no function was written,
and no prepared item belongs to the content-stream fences, so the sixth module is `appended_pages`.
The packet header's cut was first placed with files and structure; its requirement is a
`metadata/` row, so it moved there.

**Proof.** A line-for-line comparison of the landed split: 7 318 non-blank lines of the base,
7 272 matched with multiplicity once the rewrites are undone, the other 14 rustfmt's wraps (their
tokens the generated lines' but for three commas and two braces); 287 string literals and 1 952
comment lines all present. Its first run found a comment above an import dropped; the generator
now carries it, and the rerun on the base the census used and on the landed base passed.

**Gates.** In the round's export of HEAD: `cargo check -p pdf-transform --tests` exit 0;
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform --all-targets --no-deps` exit 0;
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p pdf-transform` exit 0; `cargo test -p
pdf-transform --test archive --test profiles` exit 0, 524 passed and 6 ignored, 13 passed, and
`--lib archive::prepare` 1 passed, the same as the base's file in the same export; the census
without the CJK pack (exit 0, 779 lines) and with it (exit 0, 771 lines) identical line by line to
HEAD's but for the build lines HEAD's printed, the test's elapsed seconds and the hold's footer. The
plant: `NOT_HELD_SITE` in `prepare/appended_pages.rs` changed by one word through the generator,
`--test archive` exit 101 with 1 failed of 524; regenerated from the base, `cmp` equal. In the
worktree after landing: `cargo check -p pdf-transform --tests` exit 0; `--bin cited` and
`--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 0, 69 binaries, 470
passed (a first run's three reds were the old paths and two sibling records being written); the
history sweep finds 0 lines in the seven files.

**Found, not fixed.** `prepare/metadata.rs` (2 143 lines) and `prepare.rs` (2 227) remain the
largest files in `archive/`; a further split of either is its own round.
