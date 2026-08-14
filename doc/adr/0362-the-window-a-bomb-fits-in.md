# ADR 0362 — The window a bomb fits in, and the peak nobody had attributed to the right block

Status: accepted, 2026-08-14. Session 527. Carries out what `doc/todo/14` says a round taking
road D owes **first — the measurement, not the rewrite**. Decides that the rewrite is worth
doing and hands the next round numbers instead of an estimate. Touches no shipped code path:
what lands is two instruments and a corrected arithmetic.

## The question, and the answer in one line

**Does a `Lexer` fed from a fixed window hold a decompression bomb at the window, and read an
honest document at a comparable cost?** Yes, on both, measured: Bomb B costs **6 MB** of peak
resident where the shipped path costs **1030 MB**, and the witness's page lexes to the same
20 834 587 tokens and 3 185 295 operators for **+4.10% instructions** and **98 MB instead of
380 MB**.

## The instrument

`crates/pdf-model/examples/window_lexer_spike.rs`, two arms over one page:

- `whole` — today's route: `Page::content` decodes every `/Contents` part into one `Vec` and
  the lexer walks it.
- `window <bytes>` — the road: `flate2::Decompress` writes into a fixed buffer, the lexer runs
  over what is in it, the unconsumed tail is compacted to the front, the pump refills.

It **reuses `pdf_syntax::Lexer` rather than reimplementing one**, which is what makes the two
arms comparable: the spike is the *feeding*, and the tokeniser under both is the shipped one.
Both arms count by `content_budget_census`'s rule — a keyword token at array depth zero is an
operator (§7.8.2, §7.3.6) — and **the counts must agree**, which is the only check that says a
window boundary was handled rather than papered over.

Peak resident is `ru_maxrss` from `os.wait4` *and* `VmHWM` polled from `/proc`, because the
first has a floor this round found the hard way (below). `VmPeak` is the quantity `RLIMIT_AS`
compares against. Callgrind counts instructions with `RAYON_NUM_THREADS=1`; the spike is
single-threaded, and pinning is `doc/habits.md`'s rule rather than an option.

## What it measured

`--profile gates`, three runs each, both bombs rebuilt from `doc/todo/10` §2's description and
**389 317 and 1 847 467 bytes to the byte**, and the witness the project owner's `tmp/Entwurf.pdf`.

| decode + lex only | `whole` | `window 65536` |
|---|---|---|
| **Bomb A** 0.39 MB → 400 MB | 765–766 MB, 2.6–2.9 s, 200 M tokens | **4 MB**, 2.5–3.7 s, 200 M tokens |
| **Bomb B** 1.85 MB → 1.9 GB | 1030 MB, 1.8–2.3 s, **0 tokens** | **6 MB**, 10.9–16.6 s, **950 M tokens** |
| **`Entwurf.pdf`** 141 MiB of content | 379–380 MB, 0.8–1.4 s | **98 MB**, 0.7–1.8 s |

Bomb B's `whole` row is the shipped refusal doing its job: `max_stream_len` stops the decode at
a gibibyte, so the arm spends 1030 MB and produces **not one token**. The `window` arm reads all
950 million of them in a buffer that never grows. That is road D's whole claim — the *kind* of
the quantity changes, from an allocation nothing can take back to time somebody can stop — and
it is now a measurement.

**The window's own size is noise.** On the witness, 4 KiB costs 36 156 refills and 120 367
re-lexed bytes, 64 KiB costs 2 258 and 7 694, 1 MiB costs 142 and 453; peak resident is 98, 98
and 99 MB. Re-lexing at 64 KiB is 0.005% of the stream.

**Callgrind, `RAYON_NUM_THREADS=1`, the witness:** whole **8 972 848 710**, window
**9 340 854 748** — **+4.10%**. Wall clock did not distinguish them, which is what
`doc/habits.md` says to expect and why the counter is quoted.

**And it reads the same page.** Page one of every document in the pdf.js corpus through a
deliberately small **512-byte** window: **948 agree, 0 disagree**, 10 skipped for a filter chain
the spike does not pump, 10 unopenable, 6 pageless. A 512-byte window puts a boundary through
almost every token and comment in the corpus, and no page lexed differently.

## The prize is bigger than `doc/todo/14` said, and `massif` says why

That file predicted the witness would fall from 381 MB to "about 315 MB", on the reasoning that
what is left is "the display list and the raster". **The peak is not the display list.**
`valgrind --tool=massif --time-unit=B` on `pdf-retrieve page … 0` names four blocks alive at it:

| block | bytes |
|---|---|
| `filter::flate`'s inflate buffer | 147 972 800 |
| the `Arc<[u8]>` the decode is handed over as | 147 972 263 |
| the file, as `Document::open_with_password` keeps it | 49 679 528 |
| the *encoded* stream, copied out of the file by `Parser::parse_stream_data` | 49 678 824 |

395 MB of useful heap, 446 MB with the allocator's own. The first two are **two copies of the
same 141 MiB of decoded content**, and they are exactly what a window replaces. The display list
is visible later in the same profile — the heap grows from 247 MB once the decode settles to
346 MB while the page is interpreted — so **it is about 99 MB and it is not at the peak**.

So the arithmetic road D leaves on the witness is the file (47 MB) plus the encoded stream's
copy (47 MB) plus the display list (99 MB) plus the window: **about 193 MB against 446 MB of
heap today**, where `doc/todo/14` predicted 315. The spike's window arm measures the first two
of those three directly — 98 MB, which is the two 47 MB copies and nothing else.

**Two other things fell out of that profile, and both are the road's business:**

- **The encoded stream is resident twice**, once in the file and once as `Stream::data`'s owned
  `Arc<[u8]>`. A window-fed reader that takes its input from `Stream::data` pays 47 MB on this
  witness before it has decoded a byte — which is what the spike's 98 MB is. Feeding the pump
  from a *borrowed* slice of the document's bytes would take it off, and that is a decision for
  the rewrite rather than a defect here.
- **`doc/todo/10` §1 and `doc/todo/14` both call the witness's content stream 66 MB.** It is
  **147 972 263 bytes**, one part, which is what `lexer.rs`'s own comment has said since ADR 0341
  and what both spike arms print. Corrected where it was written.

## The two design questions, answered with a census

`crates/pdf-model/examples/token_window_census.rs`, one root — `doc` holds pdf.js, the four
corpora and this project's own documents — **40 388 files found, 39 976 opened, 78 844 pages,
225 775 555 content tokens**:

**How large a window does the largest single lexical object need?** The largest token in the
whole population is **390.16 KiB** — a string, `219789.pdf` page 9. The largest name is 176 B,
the largest number 11.43 KiB, the largest keyword 19.39 KiB. **233 tokens pass 4 KiB, 2 pass
64 KiB, none passes 1 MiB.** `Limits::max_string_len` is 2²⁶ = 64 MiB, which is 168 times the
largest thing any of these documents states — so the bound a window needs is not that one, and
the rewrite's choice is between growing the window for one token and refusing with a report.

**What does `inline_image::scan`'s unbounded `EI` lookahead cost a bounded window?** Of
**93 930** inline images read, **90 304 state or imply their length before their data is read**
— 336 by §8.9.7's `/L` and 89 968 by §8.9.3's sample arithmetic — and only **3 455 need the
forward search**. The largest image in the population is 9.01 MiB; **the largest one that needed
the search is 2.99 KiB.** So the lookahead is a bounded buffer's problem for 3.7% of inline
images, and in this population 64 KiB would cover every one of them with the refusal §8.9.7
already has (`InlineImageError::NoTerminator`) for what it does not.

**One caution about the instrument, since the first run of it hit this**: the walk is recursive,
so naming `doc` *and* `doc/pdf.js` counts every file under the second twice. The figures above
are one root.

## A note about the instrument, which cost an hour

**`ru_maxrss` from `wait4` has a floor, and the floor is the spawning process's own resident
set.** `posix_spawn` clones the parent's address space until the `exec`, and the child inherits
the parent's high-water mark, so a Python harness measuring a 4 MB program reported **13–14 MB**
— three times the truth. `/proc/<pid>/status`'s `VmHWM` reads 4 MB for the same run, and the two
agree exactly wherever the number is large (765 vs 765, 1030 vs 1030, 379 vs 379). ADR 0354's
figures are unaffected, all being far above the floor. **A round measuring a small footprint
reads `VmHWM`.**

## Decision

**Road D is measured, and the rewrite is the next round's.** The verdict `doc/todo/14` asked for
is yes on both halves, and the file now carries the numbers, the design's four open decisions and
what the spike leaves undone. What lands here is:

- `examples/window_lexer_spike.rs` — the experiment, kept because a measurement nobody can
  re-run is a memory. It lexes and does not interpret, and says so.
- `examples/token_window_census.rs` — the census, which is an instrument in its own right: it
  answers "how large is the largest token" and "how do real inline images state their length"
  for anything that asks later.
- `doc/todo/14`, amended with all of the above, and its 66 MB corrected.

**Nothing in the shipped path moved**, which is why no gate could have changed: the two examples
are dev targets and reach no library code.

## Consequences, and the one that changes another road

**`MAX_OPERATIONS` becomes load-bearing again on the bomb.** `doc/todo/10` §2's table says the
bound "guards none of" a cycle, a decode or an allocation, because on Bomb B "the memory is
already spent" before the first operator is counted. Under a window there is no memory to spend:
the counter is reached at operator four million, a few megabytes into a 1.9 GB stream, and the
bomb stops there rather than after eleven seconds — which is what the spike, having no operator
bound, actually took. The bound does not become a good bound; it becomes one that fires *before*
the damage instead of after it.

**And `max_stream_len` stops being the load-bearing bound for content streams.** Its whole
justification (ADR 0306's derivation, ADR 0354's arithmetic) is the size of an allocation that a
window-fed route does not make. The resource paths — fonts, images, ICC profiles, xref streams,
JBIG2 globals — keep it, because they are random-access parses that want the whole buffer, and
`doc/todo/14` already says so.
