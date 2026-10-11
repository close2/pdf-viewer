# 1833 — The read is lexing first, and a content stream's two tokens have a body of their own

Session 1498, a measure round of the project owner's performance target of 2026-10-10 ("improve
performance", loading and display time first), on `/tmp/demo_heavy_engineering_A1.pdf`. Status: **accepted** and **built**. It
prices and cuts ADR 1817's candidate C, the interpretation every window spends before any pixel.
Context: ADRs 0341, 0370, 0424, 1011, 1521, 1531, 1811, 1813, 1817. Code: `pdf_syntax::Lexer::next_token`
and `next_token_at`, `Lexer::fixed_format_token`. Instrument: `crates/pdf-model/examples/callgrind_interpret.rs` (`--stage=`, `--digest`).
Clauses: §7.2.3, §7.3.3, §7.8.2; no ledger row moves.

## 1. Where the read goes, stage by stage

The page is one `FlateDecode` stream of 278 579 415 bytes: 78 175 395 tokens, 49 183 183 of them
operands, about 29 million operators, 4 408 405 display-list commands. Counted with callgrind at one
thread on HEAD `75531da7`'s release build, the whole run is 45.62 G instructions, of which the
interpretation is 43.41 G and dropping the list at the instrument's end 2.20 G (no window pays that
before a pixel). The stages are **prefixes, each a counted run of its own** (`--stage=read`, `lex`,
`operands`), because a profile alone cannot divide them: most of the token loop is inlined into
`Interpreter::run_reader`. Each row is the difference between its prefix and the one before it:

| | stage | instructions | share of the interpretation |
|---|---|---|---|
| 1 | lexing: the window's token loop, `next_content_token`, `Lexer::next_token` | 15.33 G | 35.3 % |
| 2 | the operators' loop body around the operators, a residual (below) | 12.31 G | 28.4 % |
| 3 | the display-list push, `Interpreter::end_path` for 4 400 294 `S` | 4.59 G | 10.6 % |
| 4 | §7.8.2's operand stack: each operand made an `Object`, held and cleared | 3.52 G | 8.1 % |
| 5 | §8.6.8's colour, 1 288 313 `CS` and 1 289 500 `SC` (`take_colour_space`, `Compositing::paint`, `resource_entry`, `initial_colour`) | 2.86 G | 6.6 % |
| | inflating and pumping the window (`--stage=read`, with the open) | 2.50 G | 5.8 % |
| | §8.4.2's `q`/`Q`: `GraphicsState::clone`, its drop and the stack's moves | 2.31 G | 5.3 % |

Row 2 is what is left of the operators' 22.07 G once rows 3, 5 and the `q`/`Q` row are taken out:
the keyword matched against Table 50 (`count_of`, the arm `match`), the operand reads, the step
closure's work beyond row 4's floor. It is a residual and is named as one: about 420 instructions an
operator, and its own division is the next profile's. Resources and fonts are not a row: the page
selects one resource, `/Cs1`, and states no text (2 222 `show_text` calls cost 11.6 M).

**The brief's hypothesis did not hold.** It placed the cost in the display-list push and the
operand stack, and a reserve sized from the stream's length was to cut a fifth: rows 3 and 4 are
18.7 % together, and the list's command vector grows 22 times for 0.24 M instructions — there is no
fifth to reserve away. **The lexer dominates**, at 196 instructions a token for 3.56 bytes a token.

## 2. What the lexer's instructions were

By line (a `bench`-profile build of HEAD) and by instruction (`--dump-instr=yes` on the `lex`
stage): `Lexer::next_token`'s own work is tight — a whitespace skip by bit mask, a digit loop of
thirteen instructions a digit — and much of the rest is **plumbing paid per token**: the entry and
exit lines of a 4 095-byte function whose one `match` holds every kind of token, 1.88 G; and
`next_content_token`, called and not inlined, moving the 32-byte token from one out-pointer to
another and testing the salvage, another 1.88 G.

## 3. The decision, and the half of it the clock refused

**The two tokens a content stream is made of are read in a small `next_token`, and every other in
`next_token_at`.** A byte that may begin a number goes first to `fixed_format_token` — the one
statement of §7.3.3's fixed format read off the cursor and standing only where the byte it stopped at
ends the token, which `read_number` also asks first — and a regular byte that cannot begin a number is
§7.2.3's run, the keyword: exactly the arm the full `match` would have taken for that byte, so the
token is the same by construction. Everything else, and a number the fixed format declines, takes the
full `match` unchanged. These two kinds are 76.9 million of the 78.2 million tokens, and the body they
take is 730 bytes. No grammar is stated twice and no token changes.

**The second half was built, counted, and not kept.** Making the content reader's salvage test a cold
call (`salvaged`) let `next_content_token` inline into `run_reader`, and it cut a further 1.09 G — and
the clock read it as 0.41 s *slower*. Four arms, interleaved in one `--clock` hold, pinned to the eight
fast cores, one thread, three rounds, `display_list_census --stages`' interpretation time:

| arm | instructions, `lex` stage | interpretation, minimum of three |
|---|---|---|
| HEAD | 17.83 G | 2.180 s (2.180–2.197) |
| the lexer split alone — **kept** | 16.89 G | **2.153 s** (2.153–2.170) |
| the reader's wrapper inlined alone | not counted | 2.586 s (2.586–2.616) |
| both | 15.80 G | 2.566 s (2.566–2.582) |

Fewer instructions and a slower page is what a count cannot see, the rule `doc/todo/02` section 7
states for pixels holding for time; why the inlined wrapper costs 0.4 s inside a
76-kilobyte `run_reader` was not found here and is not guessed at. Also measured and dropped:
`#[inline]` on the wrapper alone (no change, 17.83 G), and the three reads that allocate put out of
line (+0.15 G).

## 4. What it moved

| | HEAD | the change |
|---|---|---|
| lexing alone (`lex` less `read`) | 15.33 G | 14.39 G (−6.1 %) |
| the interpretation (`interpret_into`) | 43.41 G | 42.48 G (−2.1 %) |
| the witness's interpretation, wall, pinned, minimum of three | 2.180 s | 2.153 s (−1.2 %) |
| the display list and reports, SHA-256 of their `Debug` rendering | `07eb7445…68e3` | the same, 4 408 405 commands, 776 097 064 bytes charged |
| the six launch documents' open and first read, process instructions | | −0.015 % to +0.22 % |

## 5. Is interpreting on the toolkit thread the next thing to move?

**Not for this target.** A zoom never interprets: `Viewer::arrange` interprets only the pages of an
arrangement not interpreted yet, and the triggers that discard an interpretation (`Open::stale`) are
a view state's — an audience, a clock, separations, a committed field, an annotation under the
pointer — never a magnification; §12.5.3's magnification-dependent annotations are re-placed by
`replace` without reading the content stream (ADR 0777). The first read is already off the toolkit's
thread (ADRs 1531, 1811). So on the witness, ADR 1811's shape for a page turn would save **0 s per
zoom step**. What it would save is responsiveness on a *turn* to an uninterpreted heavy page, whose
read holds the toolkit's thread today for its whole duration — about 2.2 s for a page like this one on
the fast cores — and that time moves off the thread without getting any shorter. The read's next
floor is row 2, classified once: the keyword matched against Table 50 once, into a value `count_of`,
the colour intercept and the dispatch all read, rather than three matches over its bytes — and judged
on the clock as well as the count, which is this ADR's second lesson.
