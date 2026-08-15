# ADR 0365 — The window a content stream is read through, and the two things it cannot hold

Status: accepted, 2026-08-15. Session 530. Carries out `doc/todo/14`'s road D in the shipped
code: a page's `/Contents` reaches the lexer through a fixed window instead of being decoded
into one buffer. ADR 0343 wrote the producer, ADR 0362 measured the sink in a spike and said
yes; this is the rewrite, and it is the first entry of the order the project owner gave —
D, then B, then C.

## What changed, in one line

**A decompression bomb stops being an allocation.** §2's Bomb B — 1.85 MB of file inflating to
1.77 GiB — costs **8.6 MB** of peak resident memory where it cost **1032 MB**, and it now stops
at `MAX_OPERATIONS` rather than at `max_stream_len`, which is ADR 0362's predicted consequence
happening. The owner's 141 MiB witness is interpreted from **194 MB** where it took **381 MB**,
and draws the same page: the readback of ISO 32000-2's 1023 pages and the display list of every
pdf.js corpus document are byte-identical either way.

## The two numbers, and why they are those

Neither is invented, and both come from the census ADR 0362 built (`token_window_census`, one
root, 39 976 documents, 78 844 pages, 225 775 555 content-stream tokens):

- **`WINDOW` = 64 KiB.** The size at which refilling stops costing anything: 2 258 refills and
  7 694 re-lexed bytes over the witness's 141 MiB, against 36 156 and 120 367 at 4 KiB, at the
  same peak resident memory to within a megabyte.
- **`CEILING` = 1 MiB**, how far the window grows for one token. The census's own upper bound:
  the largest single token in the population is **390.16 KiB**, 233 pass 4 KiB, 2 pass 64 KiB
  and **none passes 1 MiB**. `Limits::max_string_len` is 2²⁶ and is 168 times anything measured,
  which is why it is not the bound here.
- **`SLACK` = 4 KiB**, which is not a bound at all but where the fast path ends: the buffer is
  refilled when fewer than this many bytes stand ahead of the cursor, so the second lexing pass
  a boundary needs is paid by 233 tokens in 225 775 555 rather than by all of them.
- **`LOOKAHEAD` = 16 MiB**, for §8.9.7's inline images — the one construction in a content
  stream that is not a token. The clause recommends 4096 bytes twice over ("it should be used
  only for small images (4096 bytes or less)", "[t]he value of the Length key should not exceed
  4096 bytes") and a `should` binds nobody, so the number is the population's: **1.78 times the
  largest inline image of the 93 930 measured**, which is 9.01 MiB.

## The two exceptional cases are loud, which is the whole of ADR 0306's lesson

A fixed buffer can fail to hold two things, and neither is silently truncated:

- **A token longer than `CEILING`** is `ContentIssue::TokenTooLong { limit }`, and the reader
  steps over the token to the next white-space byte rather than handing the lexer a prefix. A
  cut token is not a smaller token: it is bytes the file never wrote, which is exactly the
  silent clamp ADR 0306 removed one layer down.
- **An inline image whose data outruns `LOOKAHEAD`** is
  `InlineImageError::Unbuffered { bound }`, kept apart from `NoTerminator` because the two are
  opposite statements: that one says the file states no `EI`, this one says this reader stopped
  looking. The lookahead starts at the window's own size and doubles only while the answer may
  have been cut by it; where the stream ends inside what was buffered, the answer is the answer
  it always was.

Both have a test, and each was confirmed to fail with its report taken out — `TokenTooLong` and
`Unbuffered` removed one at a time, the two tests failing, and passing again with them back.

## Where it applies, and the one place it deliberately does not

**A page's `/Contents` is read through the window. The four other content streams §7.8.2 names
— form XObjects, patterns, Type 3 glyph descriptions, annotation appearances — are decoded
whole exactly as before.** That is a decision rather than an omission, and `doc/todo/14`'s own
criterion is what decides it: the good case is a stream read "once, forwards", and those are not
it. §11.6.6's paired runs interpret the *same* form twice and sometimes three times —
`group_commands` runs it for the subtractive half, again for the black half, and
`rerun_on_device` runs it a third time — so a window would have to inflate it again for each,
where `decoded_stream_data`'s memo hands back the bytes it already has (ADR 0317). The bound
those streams keep is `max_stream_len`, unchanged.

What follows honestly: **a bomb hidden in a form XObject still costs its gibibyte.** It costs
exactly what it cost before this round, and `doc/todo/14` carries it as what is left rather than
this ADR claiming it away.

## What the reader is

`crates/pdf-model/src/content/reader.rs`, and three sentences of construction:

- **`pdf_syntax::filter::Pump`** is the producer half made resumable — `flate2::Decompress` held
  across turns, writing into a caller's slice, saying `Wrote`, `Ended` or `Damaged` each time.
  It shares `turn()` with `inflate_buffer`, so RFC 1951's three outcomes are classified in one
  place and the whole-buffer route and the window route cannot drift apart.
- **`Document::stream_source`** decides how a stream is to be read: a single `FlateDecode` with
  no predictor is pumped, and every other chain comes back whole by the route it always took,
  cache and bound included. That is a route decision and never a silence — the bytes are the
  same bytes and a refusal is the same refusal.
- **`ContentReader`** holds the window, the parts and the report. `Page::content_with_report` is
  now a drain of it rather than a second assembly of Table 31's parts, because two assemblies of
  one entry is the second decode path trap 6 is about.

**A token is lent, not given.** `with_token` hands the lexer's `Token` to a closure and takes it
back: a `Token::Keyword` borrows the buffer, and under a moving window those bytes stop existing
at the next refill. `doc/todo/14` asked for an API where the compiler enforces that rather than a
comment, and this is it — nothing that borrows the window can escape the closure. What the
interpreter needs afterwards is the operator, which goes onto the stack as `Word`'s fifteen
inline bytes; ADR 0341's finding that a heap allocation per token was a fifth of interpreting a
dense page is kept rather than undone, because a memcpy of fifteen bytes is not an allocation.

## The cost, measured, because principle 2 requires the number

Callgrind, `RAYON_NUM_THREADS=1`, `--profile gates`:

| | before | after | |
|---|---|---|---|
| ISO 32000-2 page 101, interpreted 50 times | 1 190 383 283 | 1 258 702 834 | **+5.74%** |
| the witness's one page, 141 MiB of content | 12 972 272 610 | 14 279 218 721 | **+10.08%** |

`Lexer::next_token` and `read_regular_run` are unchanged to five significant figures — the same
bytes are lexed once each — and `zlib_rs::inflate_fast_help_avx2` costs 1.4% more for being
asked for 64 KiB at a time. What the two rows carry is the reader's own per-token bookkeeping,
about sixty instructions of it: the buffer is a field rather than a local, so the lexer is
rebuilt over it for each token where the old loop kept one.

Three things were tried and the table shows the best of them. A version handing back an *owned*
token cost the same as the borrowing one, which is what says the copy was never the price; the
fast path that skips the boundary machinery when 4 KiB stand ahead of the cursor is worth 1.0
and 4.4 points; pushing an operand into the pending list inside the closure, rather than
carrying it out through an enum, is worth another 0.3 and 1.9. The remainder is the cost of
reading through a window at all, and it buys the memory below.

**What it buys**, `VmHWM` from `/proc` because `ru_maxrss` has a floor equal to the spawning
process's own resident set (ADR 0362), `pdf-retrieve page … 0`:

| | before | after |
|---|---|---|
| **Bomb B**, 1.85 MB → 1.77 GiB | 1032 MB, `TooLarge { part: Some(0) }`, **nothing drawn** | **8.4 MB**, `MAX_OPERATIONS` |
| **Bomb A**, 0.39 MB → 400 MB | 768 MB, `MAX_OPERATIONS` | **5.6 MB**, `MAX_OPERATIONS` |
| **the witness**, 141 MiB of content | 381 MB | **193.7 MB** |
| ISO 32000-2 page 101, the control | 42.4 MB | 42.4 MB |

Both bombs were rebuilt from `doc/todo/10` §2's description for the fifth time and came out
**389 317 and 1 847 467 bytes**, the sizes that file records, to the byte.

The witness's 194 MB is what ADR 0362 predicted to the megabyte — "about 193 MB against 446 MB
of heap today" — because what is left after the window is the file, the encoded stream's copy
and the display list, and the window replaced the two copies of the decoded content.

**And the bombs are faster as well as smaller**, which is not the road's claim and is worth a
sentence: a gibibyte that is never allocated is a gibibyte that is never faulted in, memcpy'd
into an `Arc`, or freed. **The witness's wall clock does not separate the two arms** — 2.9 to
4.8 seconds before and 1.6 to 4.6 after, on a machine with a neighbour's build running — which
is what `doc/habits.md` says to expect and why the instruction counter above is quoted instead.

## Output identity, as bytes

`CLAUDE.md`'s rule 1 makes interpretation a pure function of the document and the view state, and
this round changed how the bytes reach it. So the artefact is compared rather than the verdict:

- **`examples/readback` over all 1023 pages of ISO 32000-2**, concatenated: 2 730 201 bytes,
  `sha256 ed074b1c…`, identical on both arms — and identical to the figure session 500 recorded
  thirty rounds ago.
- **`examples/display_list_digest` over every pdf.js corpus document's page one**: 975 lines,
  `sha256 3d82288f…`, identical on both arms.
- **Every gate's own output was captured on both arms and diffed as a sorted set** — corpus,
  oracle, text extraction, dates, XMP, JPEG 2000 and both quorra lanes, 1422 lines of verdicts
  between them, **identical**. What differs is two lines the machine wrote rather than the
  code: each quorra lane's "median page: quorra takes N× the CPU backend's time, over M pages
  above a millisecond", which is a wall clock on a machine that had a neighbour's build on it.

## Consequences

- **`max_stream_len` stops being the load-bearing bound for a page's content**, as ADR 0362 said
  it would. Under a window there is no per-part allocation to bound, so what survives is Table
  31's own sentence — the array's streams "form a single stream" — counted as the reader
  produces bytes and reported as `ContentIssue::TooLarge { part: None }`. A part decoded *whole*
  (any chain but a plain `FlateDecode`) keeps its own bound and its own `part: Some(index)`.
- **`MAX_OPERATIONS` is load-bearing again on a bomb**, which is the same ADR's other
  consequence: the counter is reached a few megabytes into Bomb B's 1.9 GB instead of after the
  whole of it has been spent. The bound did not become a good bound; it became one that fires
  before the damage instead of after it.
- **A page's content stream is no longer memoised.** The pumped route does not go through the
  decoded-stream cache, so a page interpreted twice inflates its content twice. ADR 0317's
  measurement is what makes that acceptable: over a document-wide sweep the repeats are
  *resources* — font programs inflated 1993, 1486 and 808 times — and content streams are read
  once each.
- **`Page::content_with_report` still exists and still returns a `Vec`**, because tests, censuses
  and examiners want the bytes. It is the caller that pays the allocation the window avoids.
