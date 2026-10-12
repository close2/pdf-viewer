# 1506 — An operator is classified once, and the lexer's string step is counted and not kept

Slot 5 of batch eighty, 2026-10-12, a measure round of the owner's performance target (the
partial-row word yields to it). ADR 1849; no ADR 1850, no question. No ledger row moved: §7.8.2,
§8.2 Table 50 and §7.3.4 are where they were, and no token, operand or command changes.

**Premise.** Held: ADR 1833's row 2 is 12.31 G on HEAD `849a5cfc` (44.68 G whole, 20.41 G at the
operand stage), 28 992 212 operators over 78 175 396 tokens.

**Hypothesis.** Did not hold as stated. Summed by source line at each token class's execution
count: the keyword's classification (a `Word` copy, `count_of` twice, the byte dispatch) is 2.24 G,
a fifth of row 2, not half; the largest part is ADR 1813's string test before every fontless
operand, 2.44 G; the rest, 7.6 G, is the closure's `Step` protocol, the per-operator bookkeeping and
the arms' operand reads.

**Built.** `content::reader::Operator`: Table 50 matched once, in the operand closure, while the
keyword is borrowed; `operand_count`, `is_colour`, the dispatch and ADR 1004's salvage test read it.
Only a keyword Table 50 does not list is copied. Three unit tests hold the spellings, the
non-operators (`Sh`, `true`) and the counts. A misplaced doc comment (`comment_open`'s, on
`LiteralStringEnd`) is back on its function.

**Built and not kept.** The string test moved into the lexer (`Lexer::stepping_over_strings`):
byte-identical, 42.59 → 40.36 G, and slower than the classification alone in all three quiet-enough
holds (quiet: 2.254 s against 2.138 s, minimum of nine). Habit 87; why is not found.

**Moved.** The witness's interpretation, quiet `--clock` hold, nine interleaved rounds, eight fast
cores, one thread: 2.435 → 2.138 s minimum (−12.2 %), 2.474 → 2.150 s median; instructions
44.73 → 42.59 G (−4.8 %); the list's digest `07eb7445…68e3` on both; the six launch documents'
`open_kinstructions` −0.03 % to +0.01 %. No display involved (not this contract's).

**Gates.** Tier 1: `rustfmt --check --edition 2024` on `run.rs`, `reader.rs`, `report.rs` exit 0;
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-model --all-targets` exit 0 on HEAD plus this patch (the
shared worktree stops on a sibling's `stroke_cut_census.rs`); `cargo nextest run -p pdf-model` 2 039
passed, 19 skipped, exit 0; `cargo test -p conformance` exit 101 on 1503's, 1505's and 1508's
records alone. Tier 2, from private exports of HEAD
and of HEAD plus this patch, each its own `CARGO_TARGET_DIR`: `raster_golden` exit 0, held 974, moved
0, queued 1 165 s, held 27 s; the six arms each exit 0, 0 pages moved by digest or mean error against
`/home/AI/arms-1502/`, queued 0 s, held 1 672 s; the quiet `--clock` A/B exit 0, queued 916 s, held 67
s (three more exit 0); `launch_path` (`--clock`, both trees, one hold) exit 0 each, 0 of 31
outside, queued 919 s. Stepped arm: `launch_path` exit 101, arms stopped after three (143). 13 000 s.
