# 519 — The bound a `Vec` doubled past, and the copy a newline bought

**Finding.** `doc/todo/10` §5's four roads were re-read against today's code to be re-priced, and
the re-reading turned up a defect in the code they are priced against. **A bound that caps an
allocation is a claim about `Vec`'s growth policy, and `Vec::reserve` is amortised**: `inflate`
computed exactly the right step — the capacity, or the room left under the ceiling, whichever is
smaller — and handed it to a method documented to take `max(2 × capacity, len + additional)`. Below
the bound the two agree; at the bound they disagree by a factor of two, so a gibibyte ceiling bought
a 1804 MiB buffer and §2's Bomb B peaked at **1811 MB** where the number promises 1024 MiB. The
comment above the loop said the buffer never grows past the ceiling and had said so since the loop
was written. `massif` then named the rest of the peak, which `doc/todo/10` §1 had recorded as
unattributed for four rounds: the buffer's slack is resident and `Arc<[u8]>` is a copy beside it, so
a whole decode cost capacity + length — up to 3*L* where ADR 0306's derivation of the gibibyte
assumed 2*L*. **No constant moved. The code now costs what the number was derived from.**

**Date.** 2026-08-14.
**ADR.** [0354](../adr/0354-the-bound-a-vec-doubled-past.md).
**Touched.** `crates/pdf-syntax/src/filter.rs` (`inflate` split into `inflate` and
`inflate_buffer` with a `Stopped` outcome, `reserve_exact`, `shrink_to_fit`, and the unit test that
reads `capacity`), `crates/pdf-model/src/page.rs` (the separator reserved with the part),
`doc/conformance/ledger.toml` (§7.4, §7.4.4.1, §7.7.3.3), `doc/todo/10` (the header, §1's residue,
§2's table and bound table, §3's residue, and §5 — a price table for all four roads and the
sections corrected where they describe code that no longer exists), `doc/adr/0354-*` (new), this
file.

## What the roads cost, which is what the round was for

`doc/todo/10` §5 now carries the table; the two findings behind it are these.

**D is half-built and nobody set out to build it.** §5 D's case rested on "`filter::flate` already
holds a *streaming* decoder — `flate2::read::ZlibDecoder`, an `io::Read` — and then calls
`read_to_end`". The adapter left in the five-hundred-and-eighth session (ADR 0343) and what replaced
it is a pump: a `flate2::Decompress` held across iterations, its own input cursor, and three named
stopping conditions. The producer half of a window-fed decoder is written, tested and shipped; what
is left is the sink, the lexer's `&[u8]` and `inline_image::scan`'s lookahead.

**And D's prize shrank by a third in the same measurement.** It would take Bomb B from 1031 MB to a
window and the witness from 381 MB to about 315 MB, where before this round the same two numbers
were 1811 and 429. B got cheaper for a reason that is not about B: its standing objection that the
4 GiB ceiling is smaller than what one stream can demand is answered by the bound obeying its own
arithmetic — Bomb B's `VmPeak` is 1041 MB against that ceiling, where it was 1821 MB. A and C are
untouched and nothing went near `Interpreter::run`.

## The measurements, and the instrument this machine does not have

`/usr/bin/time -v` is not installed here. `os.wait4` gives the child's own `ru_maxrss`, which is the
counter it prints as *Maximum resident set size*, and `VmPeak` polled from `/proc` is the quantity
`RLIMIT_AS` is compared against — the second is the one the confined worker's ceiling reads and no
round had reported it before. Three runs each, quiet machine, both binaries `--profile gates`.

| `pdf-retrieve page … 0` | before | after |
|---|---|---|
| **Bomb A** 0.39 MB → 400 MB | 0.83–0.84 s, **1145 MB**, VmPeak 1158 MB | 0.77–0.79 s, **768 MB**, VmPeak 777 MB |
| **Bomb B** 1.85 MB → 1.9 GB | 2.08–2.18 s, **1811 MB**, VmPeak 1821 MB | 1.16–1.22 s, **1031 MB**, VmPeak 1041 MB |
| **`Entwurf.pdf`** | 1.04–1.21 s, **429 MB**, VmPeak 531 MB | 0.94–0.99 s, **381 MB**, VmPeak 390 MB |
| the same through `render_at … 1 1.0` | 1.29–2.13 s, **429 MB** | 1.53–2.96 s, **381 MB** |

**The bombs were rebuilt from `doc/todo/10` §2's description for the third time and came out
389 317 and 1 847 467 bytes, both 1029:1** — the sizes that file records, to the byte, which is what
makes this a measurement rather than a memory.

**Callgrind, `RAYON_NUM_THREADS=1`.** Bomb B: **22 228 599 946 → 12 620 142 664 instructions,
−43.2%**, because `decompress_vec` fills all the spare capacity it is given and the oversized buffer
was an instruction to inflate 1.76 GiB before the loop could notice a gibibyte had passed. The
ordinary paths: `pdf-syntax`'s `callgrind_open` **858 461 443 → 857 214 808 (−0.145%)** and
`pdf-model`'s `callgrind_interpret` **1 184 734 743 → 1 183 362 007 (−0.116%)**, both slightly
cheaper, which is the measurement that had to be taken before `shrink_to_fit` could stay.

## Nothing drawn moved, and the artefacts say so

`display_list_digest` over all 974 pdf.js documents is **byte-identical**. `render_at` writes a
**byte-identical PNG** for the witness. The corpus gate's incomplete count is 61 both ways and the
oracle's verdicts are 906/67/786 both ways, each run on this tree rather than read off a document.
`doc/todo/00` step 7's ink sweep is therefore not owed.

## The test, and why it had to read a `capacity`

`tests/stream_length_bound.rs` checks what a bomb is *told*, and that was right the whole time: the
refusal is the same `FilterRefusal::TooLarge` whether the buffer stopped at the bound or at twice
it. **Nothing observable outside the function changes when a bound over-allocates**, which is how
the defect survived the round that wrote the loop and the round that measured its output. So
`inflate` was split from `inflate_buffer` and `an_inflate_never_buys_a_buffer_past_the_bound` reads
`Vec::capacity` directly; it was confirmed to fail with `reserve` put back — *a bound of 65536
bought a buffer of 130784*.

## Gates

`fmt` clean. `clippy --workspace --all-targets` silent (the `viewer-qt` `cargo:warning=` lines are
gcc's on a cold build, `doc/todo/02` §2). `nextest --workspace` **1879 tests run: 1879 passed, 15
skipped** — 1878 at the base, plus the one above. Doctests pass. Corpus **974 documents in 4.1s: 0
unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 61 incomplete, 0 slow**. Oracle **1794
pages, 906 agrees, 67 contradicted, 786 ambiguous, 1 our geometry, 2 reference geometry, 13 not
comparable, 19 no render**, undiagnosed list empty. Text extraction **10969/11163 words in bounds
(98.26%), 486 of 508 documents fully in bounds**, and the frozen PDFBox comparison beside it. Dates
**1514 of 1545 conform (97.99%)**. XMP, JPEG 2000 green. Quorra corpus **956 pages compared in
34.7s: 934 agree, 20 differ, 2 refused, 18 not comparable**. Conformance **875 subclauses — 422
implemented, 235 partial — 7904 citations, 759 quotations**, 0 unreviewed in every clause.

Fuzzing, for a change on the decode path: `object` and `document`, 50 000 runs each, no crash. They
are the two targets `doc/verify.md` lists that reach `filter::flate`; there is no `flate` target and
this round did not add one, because the defect it found is invisible to a fuzzer — an
over-allocation produces the same output as a correct one.
