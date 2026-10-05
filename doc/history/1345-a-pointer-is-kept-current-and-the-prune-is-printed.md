# 1345 — A pointer is kept current, a comment reads no corpus, and the prune is printed

Instruments slot of batch fifty-three. ADRs 1525, 1526. No ledger row moved; no question written.

**Pointers (ADR 1525).** Habits and traps swept for `doc/todo/02` sections, `tools/` sub-commands,
"fifth round" and `CARGO_TARGET_DIR`: every sub-command named exists. Two sentences were false. The
gates-worker habit now names section 2's tier-2 first line and `batch.sh gates`' `build-sandbox`, and its
"shares one `CARGO_TARGET_DIR`" is past tense. Two `doc/todo/02` section 2 sentences now name section 5's install.
Three trap pointers were read and are still true.

**Classifier.** Not fixed before this round; line 1668 is the attribute walk above a function. Now
`read_functions` drops `//` lines before both the corpus-root match and the helper-call match.
`a_corpus_root_in_a_comment_reads_no_corpus` fails without the fix (watched: `CorpusWitness`).
Holdings unchanged: 0 rows held only by walks.

**Found: `turn_path` was in no section 2 block.** It was tracked in 47ec7f10, and `state_sections` failed
on HEAD. Added the section 2 line and `tools/state.sh turn` (in `all`), plus two edits to
`crates/render-raster/tests/turn_path.rs`: a `// no sandbox worker:` line (no page of its ten carries
a sandboxed filter) and `ceiling` → `load_limit`, which `ratchets.rs` took for a hand-written bound.

**Ed448.** Five crates judged in `doc/todo/65` (2026-10-05), none a candidate. `frost-ed448` 3.0.0
is the first with an audit on record (NCC 2023, of 0.6.0, `ed448-goldilocks` 0.9.0's curve operations
in scope), but its decoder refuses torsion points that RFC 8032 section 5.2.7 admits. Q192 gains a
dated section; the recommendation stands. `oxicrypto-sig` does name Ed448, which corrects
`doc/todo/65`. A twenty-row search lists nine more names, recorded as unjudged.

**Prune (ADR 1526).** `open` (before the worktree) and `close` (after) print `debug`'s size and
`rm -rf` when over `BATCH_DEBUG_RULE_KIB` (100 GiB); neither runs it. Tested both ways on a planted
2 MiB tree. The `du` takes 0.22 s on today's 171 GiB.

**Documents.** HANDOVER's latency row names `turn_path` and re-banding; PLAN gets the turn-path gate,
the prune print and the classifier; crate-map gets 1501, 1503, 1505, 1507, 1513; state-of-play gets
1503, 1505, 1507, 1513; `todo/README` 36. Navigation before/after: 14 absent, 28 undefined, 22
overtaken, history 0. Records 1333–1339 are 34–40 lines, each with `**Gates.**`. `batches`: 47ec7f10
is 36053 s over 10 figures, 7 rounds, no remainder.

**Gates.** `rustfmt --check` on my three Rust files exit 0; `bash -n` state.sh, batch.sh exit 0;
clippy `-D warnings -p conformance --all-targets` exit 0; `cargo test -p conformance` exit 0, every
target green (`batch` 9, `read_only` 2, `state_sections` 3, `ratchets`, `the_frontier_map` 1).
