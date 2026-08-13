# ADR 0317 — The font program a thousand pages inflate

Status: accepted, 2026-08-13 (session 482).

## Context

Two items had stood open beside each other for long enough to be worth re-measuring rather than
re-reading. `doc/todo/47` says a *cold* document-wide search is unchanged after ADR 0256 made a
repeated one 750× cheaper, and names the decoded stream chain as the thing nobody has priced;
`doc/todo/41` is that chain from the other end — **a decoded-stream cache, priced at 0.7% of
interpretation over the pdf.js corpus and not taken**, with the file's own lesson written above
the number: *price an item on the corpus, not on the page the profiler happens to open.*

The lesson is right and the price was still wrong, because **0.7% is a measurement of a corpus
walked one page per document, and a decode repeats between pages**. What a reader does — turning
pages, and searching a document end to end — is the population the item was never priced on.

## What one cold sweep actually spends on §7.4

A temporary counter build in `Document::decoded_stream_data_reported`: wall clock in the call,
the call sequence keyed by the address and length of the encoded bytes, and the decoded length.
Not in the tree, for ADR 0260's reason — a permanent timer on that path is the unmeasured cost
this project exists to avoid — and what replaces it is this table. ISO 32000-2, 1023 pages,
`--profile gates`, one sweep of `interpret_with` over every page:

| | |
|---|---|
| calls to `decoded_stream_data` | 12 734, of which 12 586 have a filter chain |
| distinct streams behind them | 3 936 |
| calls that decode something already decoded | **8 798** |
| of the sweep's wall clock, spent decoding | **24.6%** |
| of the sweep's wall clock, spent decoding *again* | **23.4%** |
| decoded bytes produced | 877 MB, of which **830 MB is re-inflation of 46 MB** |

Three streams are 3.2 s of the 3.9: 193 KB inflated 1993 times, 136 KB inflated 1486 times,
96 KB inflated 808 times. They are the document's fonts, and a font program is decoded once per
*use* rather than once per document.

**The counts are deterministic and the two shares are one run's ratio**, taken inside the process
so that both halves meet the same machine; a quieter earlier run of the same build put them at
29.5% and 28.1%. Which is another way of saying that the shares are the reason to look and the
callgrind table below is the evidence.

Replaying that recorded sequence through a least-recently-used simulation says what a bound
gives up, which is what makes the budget below derived rather than chosen:

| budget | saved, as a share of the sweep | evictions |
|---|---|---|
| 1 MiB | 21.4% | 4 568 |
| 4 MiB | 23.1% | 3 715 |
| 64 MiB — the whole working set is 46.6 MB | 23.4% | 0 |

**The knee is at a megabyte.** The repeats are a handful of large shared resources, so almost all
of the saving is available inside a bound small enough not to argue about. The simulation charges
an entry its *decoded* length where the implementation charges decoded plus encoded, so what
shipped holds slightly less than these rows do — which is why the row that decides anything is the
callgrind one below, measured on the code as written.

## Decision

**`pdf_syntax::Document` memoises decoded streams, under `DECODED_BUDGET` = 4 MiB, evicted
least-recently-used.** It sits beside the two memos already there — the object cache and the
expanded object streams — behind the same kind of `RwLock`, and it changes no answer: the same
bytes with the same filter chain decode to what they always did.

**One line of it is about §7.6 rather than about speed.** `authenticate` already empties the object
cache and the expanded streams once the file encryption key exists, because anything read on the
way to the key was read as ciphertext; the decoded memo is emptied in the same place and for the
same reason, and a cache that missed that line would hand a filter chain's opinion of ciphertext to
the first page that asked.

### The budget is derived from a stated band and a measured knee

4 MiB is the project owner's "below 10 MB is definitely ok" (ADR 0256) **less the 4 MiB the
readback cache already spends on an open document**, so the two per-document caches together are
8 MB and inside the band; and it is 0.3 percentage points short of an unbounded cache on the
largest document this project owns. Measured, not assumed: peak resident memory over a full sweep
of ISO 32000-2 is **213.6 and 214.0 MB with the cache against 211.6 and 209.6 without**.

### The key is an address, and an entry holds the allocation that address names

The signature is `decoded_stream_data(&self, stream: &Stream)` — there is no object number in it,
and there could not be, since `pdf-model` builds `Stream`s that were never objects. So the key is
the address and length of the encoded bytes, which is round 462's construction: identity by an
`Arc`'s address, made sound by holding the `Arc`.

**That pin is the whole invariant and it is not a copy of anything.** A freed buffer's address is
handed to the next allocation, so an entry that did not hold its key's allocation could answer a
lookup for a *different* stream with the first one's bytes. `doc/todo/41` met exactly this at
small sizes and had to throw away its counts below 4 KB. Holding the `Arc` makes the collision
impossible rather than unlikely: the allocation cannot be freed while the entry naming it exists.
`a_stream_cannot_inherit_the_decoded_bytes_of_one_whose_buffer_it_reuses` is the test, and it has
teeth — replacing the pin with a copy of the same bytes fails it on the second iteration.

The filter chain is compared as well, because one buffer can carry two decodes:
`pdf_model::thumbnail::significant` builds a second `Stream` over another's `data` with Table 87's
insignificant entries removed. Today that copy keeps `/Filter` and `/DecodeParms`, so the chains
agree — the comparison is there so that the cache stays correct when some future caller's does
not.

### What it costs, and what it buys, in instructions rather than in seconds

The machine carried a load average of 20 to 30 throughout this session (five other rounds), and a
wall clock A/B on it is not evidence: seven interleaved samples an arm gave medians of 9.75 s
against 6.73 s with ranges of 9 s and 8 s. So the number this ADR rests on is callgrind's, which
counts instructions and does not move with the machine. Two binaries from one tree, the constant
above set to 0 for the off arm, `find_cost … split 100` — a hundred pages of §7.7.3.2 walk and
`interpret_with`, which is what a search step is:

| ISO 32000-2, 100 pages | instructions |
|---|---|
| no cache | 4 933 481 135 |
| 4 MiB | **3 133 405 696** — **−36.5%** |
| 4 MiB, repeated | 3 132 945 345, 0.015% from the first |

The readback both arms produce is byte-identical (280 504 bytes), which is the gate `doc/todo/47`
states: a search that returns different results is a defect and not a speed-up.

And on a document with almost nothing to repeat — `issue6961.pdf`, 2 pages, 276 decodes of which 2
are repeats — **942 918 105 → 920 371 576, −2.4%.** *Two* hits already pay for 274 misses, which
is what the shape of the overhead predicts: it is one hash lookup and one `Vec` of filter names
per decode, so it scales with the *number* of streams while what it saves scales with their
*size*.

## What was considered and not done

- **Bypassing the memo for §7.5.7's object streams**, whose decoded bytes are already memoised as
  *objects* by `expanded_streams` and are therefore pure eviction pressure. The simulation counts
  them and the budget still lands within 0.3 points of unbounded, so the exception would buy a
  fraction of a percent and cost a rule with a condition on it.
- **Memoising a refusal.** A `FilterRefusal::TooLarge` costs a gibibyte of inflation to reach and
  a hostile document could name one from every page, so this is worth revisiting — but a refusal
  holds no decoded bytes, and charging it to a *byte* budget needs a per-entry overhead constant
  that would be invented rather than derived. Left out deliberately, and recorded in `doc/todo/41`.
- **An atomic recency stamp so a hit could share the read lock.** A hit takes the write lock, which
  adds 12 586 exclusive acquisitions to a sweep already taking 61 836 for the object cache (ADR
  0260 §1). It would buy a *parallel* sweep something, and ADR 0260 declined the parallel sweep on
  memory rather than on locking.

## What this does not decide

`doc/todo/10` §5's four roads are the project owner's, and road D — a streaming lexer — is the one
this measurement bears on, because a stream held decoded and a stream never held at all are
opposed designs. The evidence is written into that file's road D and the decision is not taken
here.

## Cost, written down

- `document.rs` is **+395 / −5 lines**, and the split is the point: **126 of the additions are the
  five tests and their helper**, and **107 of the rest are doc comments** carrying the arguments
  above beside the code they justify. What executes is two structs, five short methods, one free function and
  one lookup on a path that was already building the values it is keyed on.
- One public type, `DecodedStreamCache`, and one method, `Document::decoded_streams`, which
  `viewer-core/examples/find_cost` prints after a split. An instrument rather than a `Query`, for
  ADR 0256's reason.
- `find_cost` gained a page limit, so that the split can be run under callgrind at all.
