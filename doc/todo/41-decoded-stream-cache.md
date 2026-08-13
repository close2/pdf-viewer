# A decoded-stream cache

Status: **taken in the four-hundred-and-eighty-second session** (ADR 0317), after being priced
twice on populations that could not see it. What is left is one narrower question, at the bottom.
Priority: 41
Code: `crates/pdf-syntax/src/document.rs` (`DECODED_BUDGET`, `DecodedStreams`),
`crates/pdf-syntax/src/filter.rs`

Measured over one interpretation of every corpus page (session 128): 6220 inflations of 38.08 MB.
Among the streams above 4 KB — 722 calls, 35.0 MB, 92% of the bytes — **35 are repeats costing
925 KB, 2.6%**. So a decoded-data cache is worth about **0.7% of interpretation**, against a real
memory cost, a bound to argue for, and a liveness invariant to write down.

Below 4 KB the count is worthless: an address freed with one document is handed to the next, so
"the same stream twice" cannot be told from "two streams at one address".

**The benchmark page is not representative**, and that is the lesson worth more than the number:
one 88 KB font program inflated twice is 58% of *that* page's inflation and 2.6% of the corpus's.
Price an item on the corpus, not on the page the profiler happens to open.

Take it only if something else makes a decoded-stream table exist anyway — a moving window of
interpreted pages (`doc/RENDER_LIBRARY.md`'s residency plan) would.

## And the price was taken on the wrong population, which is this file's own lesson one step out

**`3129278.pdf` from the `SafeDocs` corpus spends 78% of its 380 G interpretation instructions in
`filter::flate`, inflating one `ICCBased` profile 1053 times** — once per `cs` on a page of 1053
distinct axial shadings (ADR 0288). The 0.7% above is right about the 974 and says nothing about
that page.

So the rule this file states — price an item on the corpus rather than on the page the profiler
opened — needs one more clause: **the corpus is not the web either.** 974 documents assembled from
bug reports have their own shape, and 65 944 crawled pages have another.

**What was taken instead is narrower and is why this item is still open**: a memo of the *parsed
profile*, keyed by the stream's `ObjectId`, in `Interpreter::icc_spaces`. It needs no eviction
argument, no liveness invariant and no bound on decoded bytes — the three things this item still
owes — and it took that page from 34 450 ms to about 1 550. A general decoded-stream cache would
have taken it further, and the same distance costs the three arguments above.

## And the third population is the one a reader is in, which is where it was worth taking

**0.7% was measured over a corpus walked one page per document, and a decode repeats between
pages.** Nothing in a one-page walk can show a font program being inflated once per page that
uses it. Over one sweep of ISO 32000-2's 1023 pages, 23.4% of the wall clock is decoding
something already decoded — 830 MB of re-inflation against 46 MB of first decodes, and three
streams are 3.2 s of the 3.9. ADR 0317 has the census, the budget's derivation and the callgrind
A/B (−36.5% of a hundred-page sweep's instructions, −2.4% on a two-page document with two
repeats in it).

**The three arguments this file said were owed are the three the ADR makes**: the budget is
derived from the owner's stated band less what the readback already spends, eviction is
least-recently-used and counted, and the liveness invariant is an entry that *holds* the
allocation its key names — which is what makes the address a key rather than a guess, and is
exactly the hazard the paragraph above met at 4 KB.

## What is still open, and it is one line rather than a design

**A refusal is not memoised.** `FilterRefusal::TooLarge` costs up to `Limits::max_stream_len` of
inflation to reach, and a document naming one bomb stream from every page pays that per page —
an amplification that predates this cache and that this cache is placed to remove. It is left out
because a refusal holds no decoded bytes, so charging it to a *byte* budget needs a per-entry
overhead constant, and this project does not invent constants. Whoever takes it owes that
derivation, and a document that does it to measure against.
