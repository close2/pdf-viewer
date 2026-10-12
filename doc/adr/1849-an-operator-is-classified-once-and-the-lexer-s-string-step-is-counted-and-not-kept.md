# 1849 — An operator is classified once, and the lexer's string step is counted and not kept

Session 1506, a measure round of the project owner's performance target of 2026-10-10 ("improve
performance"), on `/tmp/demo_heavy_engineering_A1.pdf`. Status: **accepted** and **built**. It
divides and cuts ADR 1833's row 2, the operators' loop body around the operators. Context: ADRs
0302, 0306, 0341, 1004, 1521, 1813, 1833. Code: `content::reader::Operator` (`of`, `name`) and
`Operator::operand_count`, `Operator::is_colour` and the dispatch in `content/run.rs`.
Instruments: `crates/pdf-model/examples/callgrind_interpret.rs` (`--stage=operands`, `--digest`),
`display_list_census --stages`. Clauses: §7.8.2, §8.2 Table 50, §7.3.4.2, §7.3.4.3, §9.3.1 Table
103; no ledger row moves.

## 1. What row 2 held

HEAD `849a5cfc`, a `bench`-profile build, callgrind with `--dump-instr=yes`, one thread. The page
is 78 175 396 tokens, 49 183 183 of them operands and 28 992 212 operators — `l`, `Q`, `q`, `S`,
`cm` and `m` 4.40 M each, `SC` and `CS` 1.29 M each, everything else 0.02 M. The whole run is
44.68 G instructions and the operand stage 20.41 G, so row 2 is still ADR 1833's 12.31 G. An
instruction's cost is its execution count, so a part is a sum of source lines at the count its
token class runs at — 78.2 M a token, 29.0 M an operator — whatever was inlined into what:

| | part | instructions | per |
|---|---|---|---|
| 1 | the string test asked before every operand with no font in force (`may_open_a_string`): the held buffer sliced, the white space at the cursor scanned and its first byte tested, then scanned again by the lexer | 2.44 G | 31 a token |
| 2 | the keyword's classification: the copy into a `Word` in the closure (0.87 G), `count_of` twice (0.55 G), the `Word` read back (0.43 G), the dispatch's byte match (0.38 G) | 2.24 G | 77 an operator |
| | the rest: the closure's `Step` returned through memory and its jump table, the operator count and budget test, the ledger and uncoloured tests, the arms' operand reads | 7.6 G | |

**The brief's hypothesis did not hold as stated.** Classifying once is worth 2.24 G, a fifth of
row 2 and not half; the largest single part was ADR 1813's string test, which every operand of a
drawing that states no text pays.

## 2. The decision

**A keyword is matched against Table 50 once, where it is lexed.** `Operator::of` is the one match
over an operator's bytes; the operand loop's closure calls it while the keyword is still borrowed
from the lexer's buffer, so a known operator leaves the closure as one byte and only a keyword
Table 50 does not list is copied, for the report that quotes it. `operand_count`, `is_colour` and
the dispatch read the value, and ADR 1004's salvage test (`names_an_operator`) asks the same
match, so the table is stated once where it was stated twice. Table 90's `ID` and `EI`, met
outside an inline image, are reported in the same words as before.

## 3. The half the clock refused

The string test was moved into the lexer — a `stepping_over_strings` flag on `pdf_syntax::Lexer`,
read in `next_token_at`'s `(` and non-`<<` `<` arms, with `LiteralStringEnd` and
`hexadecimal_string_end` moved beside it so that it and the window's `drop_token` state a string's
end once; `may_open_a_string`, `step_over_a_string`, `Ahead`, `ahead` and `Window::ahead` went. It
was byte-identical (the witness's digest, `raster_golden` held 974 moved 0, three of the six corpus
arms 0 pages moved, and a lexer test of both readings' positions that fails with the escape
planted out), it cut the page from 42.59 G to 40.36 G instructions — and **it was slower than the
classification alone in every hold quiet enough to read**. Arms interleaved, ABBA, in `--clock` holds, pinned to
the eight fast cores, one thread, `display_list_census --stages`' interpretation:

| hold (load average) | HEAD, min / median | classified, min / median | classified and stepped |
|---|---|---|---|
| 9 rounds (2–4) | 2.435 / 2.474 s | **2.138 / 2.150 s** | 2.254 / 2.293 s |
| 9 rounds (10–19) | 3.368 / 3.742 s | **3.124 / 3.334 s** | 3.316 / 3.585 s |
| 5 rounds (29) | 4.122 / 4.217 s | **3.600 / 3.663 s** | 3.823 / 3.917 s |

A fourth hold, at load 15–25, ranged from 2.6 to 9.3 s within an arm and is not read. So it is
not kept (habit 87). Its `launch_path` run also put three documents' `peak_anon_mib`
0.25–0.87 MiB under their bands' low edges, which HEAD's and the classified tree's runs did not.
Why 2.2 G fewer instructions read as a slower page was not found here, and callgrind's branch
simulation does not say: conditional and indirect mispredictions are 262.6 M and 94.3 M on HEAD,
210.8 M and 149.1 M classified, 190.7 M and 132.1 M stepped — fewest in its model for the arm the
machine runs slowest.

## 4. What it moved

| | HEAD | classified |
|---|---|---|
| the witness's interpretation, quiet hold, minimum / median of nine | 2.435 / 2.474 s | 2.138 / 2.150 s (−12.2 % / −13.1 %) |
| instructions, whole run (callgrind, one thread, the A/B's binaries) | 44.73 G | 42.59 G (−4.8 %) |
| the display list and reports, SHA-256 of their `Debug` rendering | `07eb7445…68e3` | the same, 4 408 405 commands, 776 097 064 bytes charged |
| the six launch documents' `open_kinstructions` | | −0.03 % to +0.01 %, all 31 figures in band |

A cut of 4.8 % of the instructions is 12 % of the clock: what the byte match cost was more than
its instructions, which is the other face of habit 87 and why a count ranks and the clock decides.

**The next floor is part 1, still in the tree.** ADR 1813 priced the string test at 0.70 G; on
this HEAD it is 2.44 G of the witness's run, and a cut of it is judged on the clock first — the
lexer flag above is the shape that lost, not a reason to stop looking.
