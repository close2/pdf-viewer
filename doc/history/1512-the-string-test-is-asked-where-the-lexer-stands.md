# 1512 — The string test is asked where the lexer stands, and the white space is skipped once

Slot 4 of batch eighty-one, 2026-10-12, a measure round of the owner's performance target (the
partial-row word yields to it). ADR 1861; no ADR 1862, no question. No ledger row moved: §7.3.4.2,
§7.3.4.3 and §9.3.1 are read as ADR 1521 read them, and no token, operand or command changes.

**Premise.** Held: on HEAD `4c0c04f9` the test before every fontless operand is 78 175 396 calls,
the white space scanned and then scanned again by the lexer; the witness's page holds two strings.

**Hypothesis.** Held in substance, not in place: the test is cheap where the lexer already stands,
but it is asked in the reader's fast arm after the lexer's own skip, not inside the lexer, and the
lexer reads the token from where the skip left it (`Lexer::token_at_cursor`).

**Built.** `ContentReader::with_operand` and `Window::with_operand`: under `StepOver` the lexer
skips, `string_end` asks the byte, and the token is read at the cursor; `with_token` is the `Read`
arm of the same body. `may_open_a_string`, `step_over_a_string` and `Ahead` went. `next_token` and
`token_at_cursor` share `token_here`, `#[inline(always)]`: as an ordinary inline body it moved the
object parser's inlining and `launch_path`'s `open_kinstructions` by −0.19 % to +0.67 %. Counted and
not kept: the token carried out as a pair (41.86 G) and the white space scanned twice (41.80 G).

**Moved.** The witness's interpretation, `--clock` hold at load 5–6, nine interleaved rounds, eight
fast cores, one thread: 2.020 → 1.932 s minimum (−4.4 %), 2.123 → 1.994 s median (−6.1 %), all nine
rounds below HEAD's; instructions 42.64 → 40.79 G (−4.3 %); the list's digest `07eb7445…68e3` on
both; the six launch documents' open and first page −0.15 % to −3.05 % (callgrind). No hold at load
2–4 was had in the budget. No display involved (not this contract's).

**Gates.** Tier 1: `rustfmt --check --edition 2024` on `reader.rs`, `lexer.rs` exit 0; `RUSTFLAGS="-D
warnings" cargo clippy -p pdf-syntax -p pdf-model --all-targets` exit 0; `cargo nextest run -p
pdf-syntax -p pdf-model` 2 337 passed, 21 skipped, exit 0 (planted: `<<` read as a string fails the
stepped test, a window string ended at the buffer fails `content_window`); `cargo test -p conformance`
431 passed, exit 0. Tier 2, private exports with their own `CARGO_TARGET_DIR`,
on the first-built shape: `raster_golden` exit 0, held 974, moved 0, queued 8 229 s, held 18 s; arms
`cpu-1x`, `gpu-1x`, `compute-1x`, `cpu-4x` exit 0, 0 pages moved by digest or mean error against
`/home/AI/arms-1509/`, `gpu-4x` stopped by this round (143) and `compute-4x` not walked, queued
8 231 s, held 649 s; the read A/B (`--clock`) exit 0 four times, queued 0–1 244 s; `launch_path`
(`--clock`, both trees, one hold) exit 101 on HEAD and on the change alike, load 6–7, queued 6 294 s,
not re-run on the built shape. 14 400 s.
