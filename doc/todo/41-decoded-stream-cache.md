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

## The refusal is memoised now, and the shape of the item had moved before it was taken

**Taken in the six-hundred-and-second session; ADR 0437 has the measurement and the argument.** A
refusal is an outcome beside the bytes rather than an absence: `DecodedEntry` holds an `Outcome`,
the entry records the **bound the refusal was reached under** — a `TooLarge` under
`nested_content_source`'s smaller allowance is not an answer under the document's own bound — and
the per-entry overhead this file said was owed is `size_of::<DecodedEntry>()`, charged to every
entry rather than only to refusals, so nothing was invented.

**Three sessions had moved the item under it and the round checked first.** All five of §7.8.2's
content streams have a window now (ADRs 0427, 0429, 0430), and `Document::pumping` granted one only
to a *single* `FlateDecode` or `LZWDecode` with no predictor — so a bomb in a page's `/Contents`, a
form or a pattern cell costs kilobytes and was never what remained. What remained is everything the
window declined: a chain of two filters (`[/ASCIIHexDecode /FlateDecode]` — **§7.4.1's worked
arrangement rather than §7.4.7's, which this line said for two sessions**; §7.4.7 is `JBIG2Decode`
and its example is `[/ASCIIHexDecode /JBIG2Decode]`, while §7.4.1 EXAMPLE 3 is
`[/ASCII85Decode /FlateDecode]` over a page's own marking instructions), a predictor, and every
stream that is not content at all — a font program, an
`ICCBased` profile, a cross-reference stream — read whole from every page that names them.

The witness this file asked for is that shape: Bomb B inside a form `XObject` that twenty pages
draw, hex-wrapped so no window can take it. One cold sweep of it went from **5.92–6.12 s to
2.76–3.23 ms**, three runs an arm, alternating.

## The image route joined it, and the reason it was outside was half right

**Taken in the seven-hundred-and-twelfth session; ADR 0585 has the measurement and the argument.**
This file used to say `image_stream` decoded "outside this one by construction (a codec's bytes are
not a filter chain's)". That is true about the codec and false about everything Table 5 lets
`/Filter` put in *front* of it — and for a codec-less image, which is most of them, "in front of it"
is the whole chain over the samples themselves. 2420 of the pdf.js corpus's 2997 image `XObject`s
run a filter there (`crates/pdf-model/examples/image_prefix_census.rs` is the instrument).

`Document::chain_over` is now the one place a §7.4 chain is run and both routes take it, so the
image route gained no key, no budget and no constant of its own. What it did gain, and what the
price re-derivation found by reading rather than profiling, is that **one `Do` asked `image_stream`
four times** — three reports that are each about one codec, decoding the chain before asking which
codec it was, plus the samples. `Document::image_codec` reads Table 5 instead, and the three decline
before spending anything.

The lesson this file keeps from it: **a memo and the redundant calls it hides are two different
defects, and the second is the cheaper one to find.** −60.5% on twenty pages that repeat one image;
−2.35% of a thousand-page sweep from the reordering alone, against −2.06% from the memo alone,
because a document that repeats no image pays the memo's displacement and collects nothing.

## What is left, and it belongs to `doc/todo/14` rather than here

**A refusal whose *encoded* bytes do not fit the budget is still re-run per read**, because the
entry pins them and the cache declines what it cannot hold. At `DECODED_BUDGET` that is a stream
above 4 MiB of encoded data.

**The document that reaches it exists and is one hex digit away from ADR 0437's own witness**:
`ASCIIHexDecode` costs an author two bytes per one, so a bomb large enough to command the gibibyte
`max_stream_len` allows is already within a factor of two of the budget, and padding is free. Twenty
pages drawing one such form `XObject`: **257 µs under the budget, 6.93 s over it — about 25 000×**.
The generator is in `doc/history/712-…`.

**ADR 0586 declined the construction this file asked for and says why**: charging a refusal nothing
loses the ceiling that is this cache's whole shape, letting one entry exceed the budget makes the
bomb evict everything around it, and keying on a digest trades a decode for a hash and a collision
for content dropped in silence.

**Its redirect was taken in the seven-hundred-and-fourteenth session and was half right, which is
what this line now records.** `doc/todo/14`'s chain pump landed (ADR 0587) and the gibibyte is
gone — 1 070 828 KB of peak resident memory against 22 608 on the same twenty pages. What did not
follow is the sentence ADR 0586 wrote beside it, that a chain pump "would take this document to
kilobytes on **every** read": kilobytes of *memory*, but the read is still the bomb's whole decode,
and the 25 000× was the distance between a refusal **remembered** and a refusal **re-reached**. A
window re-reaches it. Measured after the pump: **154.98 µs against 14.62 s** over twenty pages, the
first arm being the buffered route with ADR 0437's memo behind it.

**So the item is back here, and it is smaller than the one that was declined.** Nothing has to be
charged differently, exceed the budget or be keyed on a digest: the refusal a window reaches is the
*same* `FilterRefusal::TooLarge` under the same key as the one the buffered route reaches, and the
reader already knows it has read `max_stream_len` decoded bytes out of one stream. What is needed
is that fact travelling one hop back to `Document`, for the single-part case where the stream it is
about is unambiguous (`Window::single`, which is every one of §7.8.2's nested content streams).
Whoever takes it owes the hop, not a new number — and the witness and both arms are ADR 0587's.

**Taken in the seven-hundred-and-forty-second session, and the hop was one fact short.** ADR 0646
has the measurement and the argument: 14.32–17.99 s against 186.16–287.23 µs on ADR 0586's own
witness rebuilt to the byte, at unchanged peak resident memory, and +0.0031% instructions over ISO
32000-2's 1023 pages. What the paragraph above got wrong is worth keeping, because it is the
difference between a memo and a page drawn by a memo: **a window hands over everything up to the
bound and only then says it stopped**, so "too large" alone is not a fact a second read may be
answered from — a stream whose prefix marked the page owes those marks to every later read.
What travels is *too large **and empty***, which is exactly reproducible and which
`Interpreter::run` is the one place able to see both halves of. It is the rule
`Outcome::Decoded`'s `damage` field already states for the other half of this cache.

Two lines are left, and neither is the one that was owed:

- **A refusal whose encoded bytes exceed `DECODED_BUDGET` is still not remembered** — the second
  row of ADR 0646's table, unchanged at about 14–17 s. That is ADR 0586's argued refusal and it
  stands: `DecodedStreams::keep` declines what it cannot hold beside its own key, and letting one
  entry past the budget would let the bomb evict everything around it.
- **`NestedContent::damage` still drives a pump to the end**, once per read, which is a gibibyte on
  such a stream. §12.5.5's appearance is the caller and ADR 0359 is why the question is asked where
  the stream is read; the method has no `&Document` to record against, and what it asks is a
  different question from the one above — *is this stream damaged*, not *is it too large*. Whoever
  takes it should check first whether §12.5.5's route still needs the answer before the run.
