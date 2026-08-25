# ADR 0646 — The fact a window had, and did not hand back

Status: accepted, 2026-08-25. Session 742. Takes `doc/todo/41`'s remainder, which ADR 0587 sent
back there smaller than it left. Amends the ledger rows for §7.4, §7.4.1 and §7.8.2.

## What changed, in one line

**A decompression bomb inside one of §7.8.2's self-contained content streams stops being inflated
once per page that draws it.** Twenty pages drawing one `[/ASCIIHexDecode /FlateDecode]` form
`XObject` whose encoded bytes fit the decoded-stream budget cost **14.32–17.99 s** and now cost
**186.16–287.23 µs**, at unchanged peak resident memory. The corpus's display lists do not move.

## The defect, and why nothing could see it

ADR 0365 put a page's `/Contents` behind a fixed window and ADR 0427 did the same for §7.8.2's
other four — a form `XObject`, a tiling cell, a Type 3 glyph description, an annotation
appearance. The routing rule is the decoded-stream memo's own condition, and it rests on a
premise ADR 0427 wrote down:

> A stream the memo would decline is **pumped through a window**. Re-reading it costs a
> re-inflation, which is what it cost before as well — a decode the memo declines is re-run on
> every read today — and the gibibyte it used to command is not spent at all.

That premise was true when it was written and stopped being true two sessions later, in a place
nobody looked: **ADR 0437 made the memo remember a *refusal*.** From then on a decode the memo
declined was not re-run on every read — on the buffered route it was answered from the memo the
second time and every time after. The windowed route kept re-running it, because

> **a window makes no `TooLarge`: the aggregate bound is the reader's** ([`Document::pumping`]'s
> own doc comment, and it is correct)

so the window reaches the same fact by a route that had nothing to write it down with. ADR 0587
measured the pair and recorded the distance — **154.98 µs against 14.62 s** on twenty pages — and
attributed it correctly to *a refusal remembered against a refusal re-reached*, then handed the
line back to `doc/todo/41`.

**No gate could see it and no gate should have.** The bytes are the same bytes, the display list
is the same display list and the report is the same report; what differs is a gibibyte of
inflation per read, on a document class no corpus contains. It is trap 1's shape one directory
over: an instrument that says the picture is right says nothing about what the picture cost.

## What travels, and why it is two facts rather than one

The obvious fix is to record `FilterRefusal::TooLarge` when a window reaches the bound. It is
wrong, and the reason is worth more than the fix:

**A window hands over everything up to the bound and *then* says it stopped.** `Window::refill`
writes `limit` bytes into the reader and raises `ContentIssue::TooLarge` when the next byte would
pass it, so a stream whose prefix marked the page has marked the page. Replacing that with a
refusal on the second read would make the first read draw a prefix and every later read draw
nothing — a page that is a function of whether the cache still holds an entry.

This tree has already written that rule down, for the other half of the same memo
([`Outcome::Decoded`]'s `damage` field):

> A hit that answered `None` here would make a damaged stream report on its first reading and stay
> silent on every later one, which is a report that depends on the cache's budget.

So what is recorded is **too large *and empty*** — the bound was reached and the run added not one
operator. That conjunction is exactly reproducible: the same bytes under the same chain under the
same bound decode to the same tokens, so a second read of a stream that produced nothing produces
nothing again. `Interpreter::run` is the one place both halves are visible at once, and is
therefore the caller.

**How reachable the discarded half is, measured rather than asserted.** `MAX_OPERATIONS` is four
million and counts operators, so a stream reaching `max_stream_len` at all must average more than
268 bytes per operator — real content streams average tens. A window that reaches the gibibyte is
a stream that is overwhelmingly not instructions. The guard is kept anyway, because "no real
document does this" is not a reading of anything, and because a test that pins it costs nothing.

## The shape

Four pieces, each on a boundary that already existed:

- **`pdf_syntax::Decoding`** — the pair the memo keys an entry by, the encoded bytes and §7.4's
  chain with its parameters, handed out so that a reader which exhausts a bound can name *which*
  decode it exhausted. Opaque: a caller has no reason to read a chain apart, and the type exists so
  that the fact travelling back cannot be about some other stream.
- **`StreamSource::Refused { limit }`** — a third arm that is not a third behaviour. It is the
  second arm's answer, remembered.
- **`Document::window_found_nothing(&Decoding, under)`** — the hop. It writes the same
  `Outcome::Refused` under the same key that the buffered route writes, so nothing about the
  budget, the charge, the eviction or the liveness invariant changes; ADR 0586 declined a new
  charge, a new ceiling and a digest key, and none of them is here.
- **`Window::refused(limit)`** — a window with no parts and the issue already raised, so a read
  served from the memory says what the read that learnt it said, in the same words and with the
  same number.

`Document::pumping` now hands back the chain it read. Deciding whether a stream pumps means
reading `/Filter` and every `/DecodeParms`, which is also exactly what identifies the decode — so
a caller that needs both gets both from one pair of reads instead of resolving those indirect
references twice. `decoded_chain` is `decoded_under` with the chain already in hand.

**And the memo is asked once rather than twice.** `nested_content_source` now puts the new lookup
and the allowance under one write guard: both are questions about the same entry, and this
cache's own documentation says the exclusive acquisition is the expensive part of a hit. The
route decision therefore costs one acquisition where it used to cost one, not two.

## The measurement

`viewer-core/examples/find_cost` over twenty pages each drawing one hex-armoured form `XObject`,
both arms built in one sitting from one patch, alternating, `RAYON_NUM_THREADS=1`, peak from
`VmHWM` sampled every 20 ms. ADR 0586's witness pair rebuilt from its generator **to the byte** —
4 174 537 and 12 523 517 encoded bytes, which is what says the fixture is the same one.

| twenty pages, one form | before | after |
|---|---|---|
| 4 174 537 B encoded (under the budget) | 14.32–17.99 s, peak 22 292–22 436 kB | **186.16–287.23 µs**, peak 22 200–22 316 kB |
| 12 523 517 B encoded (over the budget) | 14.24–16.89 s, peak 55 108–55 248 kB | 14.18–16.47 s, peak 55 284–55 352 kB |

Three runs an arm. **The wall clock is printed rather than argued from**: the machine carried a
load average of 6 to 18 from three parallel rounds, which is what the ranges are. The peaks are
deterministic and they match ADR 0587's after-column, which is the second thing that says this is
the same witness.

**The second row is unchanged on purpose and is ADR 0586's refusal, still standing.** A refusal
whose *encoded* bytes do not fit `DECODED_BUDGET` is not remembered, because
`DecodedStreams::keep` declines what it cannot hold beside its own key — and letting one entry
exceed the budget would let the bomb evict everything around it. `doc/todo/41` keeps that line.

`callgrind`, `RAYON_NUM_THREADS=1`, one sweep of ISO 32000-2's 1023 pages: **37 299 983 484 →
37 301 137 814 instructions, +0.0031%.** The saved second read of `/Filter` and `/DecodeParms` is
slightly smaller than the memo lookup and the one chain clone that replace it; both are noise, and
the honest number is the one printed rather than the sign somebody hoped for.

## What was not taken, and why

**A page's own `/Contents` keeps the window with nothing remembered.** Table 31 makes an array's
parts "a single stream" and the bound is over the concatenation, so a window that reaches it has
not named which part outgrew it — and a page's `/Contents` is read once per render rather than
once per site, which is the population §7.8.2's other four are in. `Window::push` drops the
`Decoding` and says so.

**`NestedContent::damage` still drives a pump to the end.** §12.5.5's appearance asks whether a
stream is damaged, and for a windowed stream that is a whole pass with no allocation past the
window — which is the point of ADR 0359's construction, and is still a gibibyte of inflation on a
bomb. It has no `&Document` to record against and is a different question from this one;
`doc/todo/41` carries it.

## Tests

`a_window_that_found_nothing_refuses_the_read_after_it` — the route asserted before the first read
and after it, and the report asserted to be the same sentence both times, because a report that
appeared only on the first read would be a report that depends on the cache ·
`a_window_that_drew_something_is_read_again_rather_than_remembered` — the other half, and the half
that makes the memory sound · `the_routing_question_answers_under_its_bound_and_moves_neither_tally`
· `the_routing_question_answers_only_for_a_bound`.

**Both halves were planted before they were believed** (trap 13), against a scratch copy of
`run.rs`: with the hop removed the first test fails on the route, and with the emptiness guard
removed the second fails on the second read's display list. The other five tests in that file pass
under both plants, which is what says the two new ones are the discriminating pair.
