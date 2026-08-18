# ADR 0424 — The second walk over every number a page states

Status: accepted, 2026-08-18. Session 589. Fuses §7.3.3's fixed-format parse with the §7.2.3 run
scan in `pdf_syntax::lexer`, and replaces the nine-byte window in `parser::find_endstream` with a
first-byte search. Amends §7.2.3's, §7.3.3's and §7.3.8.2's ledger rows, and §12.5.6.11's on the
spec-driven side. Adds two lexer tests and one annotation test.

## What was measured, and by whom

ADR 0423 reported, from a `zoom_frame` run under callgrind, that `Lexer::next_token` is 40.2% of
interpreting page one of the owner's `tmp/Entwurf.pdf` and that §7.4's inflation is 23.1%. This
round re-took the measurement with its own binaries, through the instrument that stops at the
display list, and both numbers reproduce:

```sh
cargo build --profile bench -p pdf-model --example callgrind_interpret
RAYON_NUM_THREADS=1 valgrind --tool=callgrind --callgrind-out-file=cg \
  target/…/examples/callgrind_interpret tmp/Entwurf.pdf 1
```

| | Ir | share |
|---|---:|---:|
| **whole interpretation** | 11 470 896 205 | |
| `Lexer::next_token`, all of it inlined | 4 603 960 642 | **40.13%** |
| `zlib_rs::inflate::inflate_fast_help_avx2` | 2 455 926 157 | 21.41% |
| `Interpreter::run_reader` and its closure | 2 716 197 128 | 23.68% |
| `content::run::token_to_object` | 494 180 187 | 4.31% |
| `Parser::parse_stream_data` | 447 109 864 | 3.90% |
| `drop_glue::<Object>` | 268 303 235 | 2.34% |

## What the 40% is made of

The `bench` profile carries line tables, so the lexer's 4.60 G divides. **The page states
20 831 607 tokens and 17 654 000-odd of them are numbers**, which is what a CAD-style drawing is:
`fixed_format_number` reaches its end 17 651 377 times and returns a `Real` 17 509 740 of them, over
104 508 676 digits — 5.92 digits a number. That is 221 instructions a token, and it went:

| inside `next_token` | Ir |
|---|---:|
| §7.3.3's `fixed_format_number` | ≈1 380 M |
| §7.2.3's `read_regular_run` | ≈418 M in this file, plus its share of the inlined `core` |
| the function's own prologue, dispatch and epilogue | ≈604 M |
| `skip_whitespace` | ≈66 M |
| inlined `core` — `slice::index`, `option`, `uint_macros`, `slice::iter` | 1 750 M |

**Two of those rows are the same bytes twice.** `read_number` found the run of regular characters
first and then handed it to `fixed_format_number`, and every well-formed number was therefore walked
once to find its end and once to read its value. ADR 0370 had already taken this from three passes to
two by asking the grammar before the digit scan; the third walk is the run scan itself.

## The fix, and the attribution that had to be done twice

`fixed_format_number` now reads from the cursor and returns **how many bytes it consumed**, stopping
at the first byte outside its grammar rather than refusing the whole run. `read_number` accepts the
answer only where §7.2.3 agrees that the byte which stopped the scan also ends the token; where it
does not — `12pt`, `1.2.3`, `5f`, `1e5` — the answer is discarded and the older
`read_regular_run`-then-salvage path owns the run unchanged. The equivalence is exact in both
directions: a run the old function accepted is one this one consumes to a non-regular byte, and a run
it refused is one whose stopping byte is regular.

**And the first version of that was a 1.1% regression.** Written as

```rust
for &byte in body { … _ => break, } read += 1; }
```

it measured **11 598 899 759** — worse than the baseline — with 454 M of the growth in code callgrind
could attribute to no line of the file. The suspect was removed rather than reasoned about: capping
the slice at seventeen bytes (sign, fifteen digits, period) made it *worse still* (11 634 202 441), so
the slice's length was not the cause. Written with an index instead —

```rust
while let Some(&byte) = body.get(read) { … _ => break, } read += 1; }
```

— the same function, the same arithmetic and the same answers measure **10 848 712 891**. **750 M
between two spellings of one loop, 6.5% of interpreting the page.** A slice iterator that must also
say where it stopped is two cursors the compiler has to keep in step, and it does not; ADR 0370 found
the same thing from the other side, where replacing an index *with* an iterator measured +2.64%. The
lesson is in the code beside the loop, because a future round will be tempted by the tidier spelling.

The second change in the same function is smaller and was measured on its own: recording the
**index of the period** rather than incrementing a fractional-digit counter inside the digit arm is
worth **69.8 M** (10 918 485 365 against 10 848 712 891), because the counter was a load, a test and a
store on each of 104.5 million digits.

## The inflation half: this tree asks zlib-rs for exactly one inflation, and pays for it twice around

`doc/todo/41`'s decoded-stream cache and `doc/todo/14`'s streaming window were read first, as the
round was told to. Neither is the answer here, and the reason is a count rather than an opinion:

- **The stream is inflated once.** All of the zlib cost is inside `Window::refill`, which callgrind
  records as **2 409 calls** costing 2 645 819 575 inclusive — one pass over the content stream's
  147 972 263 bytes at about 60 KiB a call. There is no second inflation to memoise, no buffer that
  doubles past its bound (ADR 0354 closed that), and no per-part setup: `/Contents` is one part.
- **16.6 instructions per output byte is zlib-rs doing real work.** 49.7 MB of file becomes 148 MB
  of content, a ratio of 2.96, which is a stream of mostly literals — and a literal is a Huffman
  decode apiece.

What *is* ours is the 613 M standing beside it, and it is on the launch path rather than on an error
path. Table 5 makes `/Length` "shall be an indirect reference" for a producer that does not know the
length until the data is written, a parser cannot follow one (ADR 0366), and this file's content
stream is exactly that case. So:

- **`find_endstream` scanned about 89 MB comparing nine bytes at every offset** — `windows(9)
  .position(..)`, five instructions a byte, **446 M**. Looking for the `e` and comparing the other
  eight only where one is found is the same answer for **253 M**, because compressed stream data
  holds about one `e` in 256. Taken.
- **The encoded bytes are copied twice** — `parse_stream_data` copies the scan's answer into an
  `Arc<[u8]>` and `Document::with_stated_length` then copies the file's answer into a second one,
  99 MB of `memcpy` for a 49.7 MB stream. **Not taken**: removing it means `Stream::data` stopping
  being an owned `Arc<[u8]>`, which is `doc/todo/10` §3's residue and a larger design than this
  round. It is priced here so the next round does not have to re-derive it.

**`memchr` was considered for the scan and declined on principle 3.** It would take the search from
2.5 instructions a byte to about a tenth of one; it is also hand-written SIMD `unsafe` that would be
fed a hostile file's bytes directly, in the crate `CLAUDE.md` most wants `#![forbid(unsafe_code)]` on.
A 2.3% launch-path saving is not what that trade is worth, and the decision is recorded rather than
left to be re-opened by whoever next reads the profile.

## What it is worth

**Instructions**, callgrind, A/B in one sitting, `RAYON_NUM_THREADS=1`, one patch applied and
reversed with `git apply`:

| | before | after | |
|---|---:|---:|---|
| `tmp/Entwurf.pdf` page 1 | 11 470 896 346 | **10 654 605 393** | **−7.12%** |
| ISO 32000-2 page 101, ×50 | 1 217 722 945 | **1 211 873 618** | −0.48% |

The second row is the honest other end of the range: a dense text page is not made of numbers, so it
gets the `find_endstream` change and almost none of the lexer one, and it does not regress.

**Latency, which is the number `CLAUDE.md` principle 2 ranks first.** Two release binaries from one
tree, `Xvfb` at 900×1100 on `llvmpipe`, `--trace=launch`, four launches an arm, alternating:

| `tmp/Entwurf.pdf` | before | after |
|---|---|---|
| interpretation of page one, 58 009 commands | 689.7 / 690.3 / 702.8 / 726.6 ms | **637.2 / 646.8 / 651.6 / 660.4 ms** |
| whole launch, process start to first present | 1435.0 / 1442.4 / 1461.2 / 1475.3 ms | **1409.7 / 1425.9 / 1426.5 / 1427.1 ms** |

Neither pair of ranges overlaps. The interpretation step is **−6.8% on the median**, which is the
callgrind figure arriving in a clock; the whole launch is −1.7%, which is the same milliseconds
against a larger denominator.

## Proof that nothing draws differently

**Byte-identical display lists**, which is the strong form and this tree has the instrument for it:
`examples/display_list_digest` over `doc/pdf.js/test/pdfs`, `doc/*.pdf` and the four corpora under
`doc/corpora` — **1187 documents opened, 1178 first pages interpreted**, both arms built in one
sitting with the same `pdf-sandbox-worker` on disk, and the two files `diff` empty. Command count,
`Debug` length and hash equal on every line.

Beside it, every gate of `doc/todo/02` §2 was run after the last edit and none moved.

## The spec-driven half: §12.5.6.11, a refusal resting on a claim the table contradicts

The row was `reported`, with **no `code` and no `test`** — the same hygiene defect ADR 0423 found in
§10.8.3 the round before — and its note said "[t]he symbol itself is not stated anywhere".

Half of that is wrong, and Table 183 is where. The *caret* is stated nowhere, which is why the
refusal exists. The *symbol* is stated by name and by character: "P A new paragraph symbol (¶) shall
be associated with the caret". That is a `shall` and a code point — more than §12.5.6.4's seven
mandatory icons get, whose artwork `CLAUDE.md` uses as its standing example of a silence, and more
than §12.5.6.12's legends get.

**The refusal stands, on a better reason.** Trap 5's additive-or-substitutive test, read off the same
table: `/RD` measures a difference that "can occur. When a paragraph symbol specified by Sy is
displayed along with the caret" — so the pilcrow *accompanies* the caret rather than standing in for
it. Drawing it alone would put a mark on the page beside the mark nobody can derive, which is worse
than drawing neither, and it is ADR 0106's question answered in the direction that keeps a refusal
whole.

What is genuinely owed is a reader for `/Sy` and `/RD`: no source in this tree names either key. The
row stays `reported` rather than becoming `partial`, because both entries qualify a geometry that is
underivable — an entry that turns nothing on owes nothing.

The population is why nothing found this by looking at documents. `examples/witness_census` over the
974: `/Sy` is stated as a name by **none of them**, `/Caret` by four, and the corpus gate reports no
caret at all because every one carries an `/AP`. `a_caret_is_reported_whether_or_not_it_asks_for_a
_paragraph_symbol` now holds the refusal in all three shapes — no `/Sy`, `/Sy /P`, `/Sy /None` with
an `/RD` — and the row names it.
