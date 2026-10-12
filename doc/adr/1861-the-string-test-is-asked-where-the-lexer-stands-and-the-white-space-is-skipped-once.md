# 1861 — The string test is asked where the lexer stands, and the white space is skipped once

Session 1512, a measure round of the project owner's performance target of 2026-10-10 ("improve
performance"), on `/tmp/demo_heavy_engineering_A1.pdf`. Status: **accepted** and **built**. It cuts
ADR 1849's part 1, the next floor that ADR named. Context: ADRs 1521, 1813, 1833, 1849. Code:
`ContentReader::with_operand`, `Window::with_operand`, `string_end` and `content_token_at_cursor`
in `content/reader.rs`; `pdf_syntax::Lexer::token_at_cursor`. Instruments:
`crates/pdf-model/examples/callgrind_interpret.rs` (`--digest`), `display_list_census --stages`.
Clauses: §7.3.4.2, §7.3.4.3, §9.3.1 Table 103 (ADR 1521's reading, unchanged); no ledger row moves.

## 1. What the test was, and the premise

HEAD `4c0c04f9`, `bench` profile, callgrind at one thread: the witness is 42.64 G instructions. Of
its 78 175 396 tokens **two are strings**; every token is an operand read with no font in force, so
ADR 1813's `may_open_a_string` ran before each: a match on the reader's shape, the buffer sliced at
the cursor, the white space scanned to its first other byte and that byte tested — 0.78 G on the
reader's own lines and the scan's `is_whitespace` inlined beside them — and then the lexer scanned the
same white space again. The brief's premise (2.44 G, a test cheap per call and called 78 M times)
held; its hypothesis, a test hoisted into the lexer's fast path, is the shape below without the
lexer carrying it.

## 2. The decision

**Under `Strings::StepOver` the reader builds the lexer it would have built anyway, has it skip the
white space and comments, and asks `string_end` of the byte it stopped at; a string the buffer ends
is stepped over there, and anything else is read from where the lexer stands** —
`Lexer::token_at_cursor`, which is `next_token` less its skip, so the white space is scanned
once. The two share one body, `token_here`, always inlined so that `next_token` compiles as it did
(below). `with_token` is
`with_operand` under `Strings::Read`, so there is one reading of the window's fast arm and not two.
The window keeps ADR 1813's condition: a string is stepped over only where the buffer holds its
end and the fast arm's `SLACK` holds, so no comment and no string is cut by a refill; a string the
buffer does not end is lexed as any other token. `Ahead`, `ahead`, `Window::ahead`,
`may_open_a_string` and `step_over_a_string` are gone.

## 3. Three shapes, counted

| | shape | instructions | |
|---|---|---|---|
| | HEAD | 42.64 G | |
| A | the test in the fast arm, the token and its end carried out of a `match` as a pair | 41.86 G | −1.8 % |
| A2 | the same, each branch returning its own `read` | 41.80 G | −2.0 % |
| **B** | **A2, and the token read from where the skip left the cursor** | **40.79 G** | **−4.3 %** |

A carried the pair through memory (`Option<Token>` 0.57 → 1.04 G on its lines); A2's remaining
cost was the white space scanned twice, which B removes. The reader's closure is not inlined into
`run_reader` in any of them, so a second `read` call site costs a call and not a copy of the body.
Every arm's display list and reports hash to `07eb7445…68e3`, 4 408 405 commands, 776 097 064 bytes
charged.

**B as first built moved the object parser, and the built shape does not.** With the shared body an
ordinary `#[inline]` function, `next_token` became a skip and a call, and `Parser::parse_object_from`
took the skip into itself: `launch_path` read the six launch documents' `open_kinstructions` at
−0.19 % to +0.67 % against HEAD's run in the same hold, `opt_demo.pdf` 1 373.7 k outside its band's
1 372. With `token_here` `#[inline(always)]` the witness counts the same (40.79 G, 32 k fewer) and
each launch document's open and first page, counted by `callgrind_interpret` at one thread, is
below HEAD's: −0.84 %, −0.33 %, −0.63 %, −3.05 %, −0.15 % and −0.43 % (`PDF20_AN001-BPC`,
`Well-Tagged-PDF-WTPDF-1.0`, ISO 32000-2, `bug1815476`, `xfa_filled_imm1344e`, `opt_demo`).

## 4. The clock

`--clock` holds at `--nice 0`, arms interleaved ABBA, pinned to the eight fast cores (derived from
`cpuinfo_max_freq`), one thread, `display_list_census --stages`' interpretation, nine rounds:

| hold (load average) | HEAD, min / median | A2, min / median | B, min / median | B / HEAD by round |
|---|---|---|---|---|
| 1 (16–38) | 2.339 / 2.483 s | 2.290 / 2.675 s | 2.216 / 2.418 s | not read |
| 2 (5–9) | 2.182 / 2.309 s | 2.112 / 2.265 s | 2.090 / 2.268 s | median 0.944, 8 of 9 below |
| 3 (5–6) | **2.020 / 2.123 s** | | **1.932 / 1.994 s** | median 0.954, 9 of 9 below |

A hold at load 2–4, as ADR 1849's quietest, was not had in this round's budget: the lock's queue held
every wait for an hour and more. Hold 3 is read; B is faster in every round of it.

ADR 1849's lexer flag cut 2.2 G and lost on the clock; this keeps the cut and wins it. What
differs is that the lexer's grammar is untouched — no field on `Lexer`, no test in `next_token_at`,
`next_token` compiled as before — and the only new branch in the operand loop is on a
value it already held. Why the flag lost is still not found, and this ADR does not claim to have
found it.

## 5. What it moved

| | HEAD | the change |
|---|---|---|
| the witness's interpretation, hold 3, minimum / median of nine | 2.020 / 2.123 s | 1.932 / 1.994 s (−4.4 % / −6.1 %) |
| instructions, whole run (callgrind, one thread) | 42.64 G | 40.79 G (−4.3 %) |
| the display list and reports, SHA-256 of their `Debug` rendering | `07eb7445…68e3` | the same |
| the six launch documents' open and first page (callgrind, one thread) | | −0.15 % to −3.05 % |

A drawing that states text pays one test of a value per token: under `Strings::Read` the arm reads
as it did. The clock above was taken on B as first built, whose operand loop counts as the built
shape's to 32 k instructions of 40.79 G; `launch_path` was not re-run on the built shape.
