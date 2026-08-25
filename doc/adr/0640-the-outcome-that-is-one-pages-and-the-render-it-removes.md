# ADR 0640 — The outcome that is one page's, and the render it removes

Status: accepted, 2026-08-25. Session 740. Takes the one thing ADR 0633 stopped at and said why.
Cites no clause: this is `CLAUDE.md` principle 3's bound against principle 2's frame, and the
ledger is untouched.

## What was owed

ADR 0633 wired ADR 0626's codec into the frame path: a confined worker now compares two byte
counts per page and ships whichever is smaller, the marks or the pixels. It stopped one step
short, deliberately, and wrote the step down. **The worker still drew every page, including the
ones it shipped as marks**, and the reason was a vocabulary question rather than a rasteriser one.

`viewer-core` had one outcome meaning *no pixels here* — `Rendered::Presented` — and it is a
statement about the **viewer**:

- it sets `holds_rasters` false for *all* pages at once, so `Query::Frame` answers `Answer::None`
  and goes silent about the pages that must still cross as pixels;
- and `raster_budget()` becomes `u64::MAX`, so `MAX_PIXELS` stops bounding what the process is
  asked to draw.

The second is the one that decided it. Inside a confinement an unbounded raster is not a refusal:
it is the `RLIMIT_AS` abort ADR 0597 spent a round turning back into a sentence, and the
seven-hundred-and-nineteenth session had already found this boundary with no guard at all against
a length nobody could check. Saving a rasterisation by giving that up is the wrong trade.

## 1. The decision: one outcome, and it is about a page

`viewer_core::Rendered::Listed` — *the host took this request's own list*. It does exactly three
things and the third is the absence of a fourth:

- the page **holds its place**: `shown` records the target and revision, so the scheduler does not
  ask for it again — which is what `Presented` was doing right;
- the viewer **holds no pixels** of it, so `Answer::Frame` does not mention it, and `Event::Damage`
  follows, because taking a list is not the same as having drawn it;
- `holds_rasters` **does not move**, so `MAX_PIXELS` goes on bounding every request the host makes
  and `Query::Frame` goes on answering for the page's neighbours.

**This is the first change to `Rendered` since the vocabulary was frozen**, and the reason it was
needed rather than avoidable is stated in one sentence: this host is the first that keeps
*some* pages' marks and hands *others* back as pixels, and nothing in `Rendered` could say a thing
about one page. `doc/ui-boundary.md`'s standing rule was applied and it went the other way from
usual — the three mechanisms it prefers are all unavailable here. A *field* on `RenderRequest`
cannot express what the host did *after* the request. A variant **changing shape** would mean
`Presented` gaining a payload, and the two facts are not the same fact carried at different widths
— one is remembered for the life of the viewer and the other is forgotten with the page. And a
*question* is no use: this is an answer arriving unasked.

### What was checked before adding it

Three alternatives, and each fails on the budget rather than on taste:

- **Make `Presented` per page.** The state a viewer has to keep is then "which pages the host drew
  itself", and `raster_budget` would consult it — so the budget for a page that once answered
  `Presented` is unbounded, and a page that flips arms later (a layer switch turns a chart into a
  scan) is rasterised at an unbounded target. That is the hole, one indirection further away.
- **Let the worker answer `Presented` and re-tighten the budget in `viewer-confined`.** A second
  copy of the bound, in the crate that cannot see how the first is applied; `MAX_PIXELS` is public
  precisely so that there is one of it (its own doc comment says so).
- **Change `Answer::Frame` so a page can appear in it without pixels.** This was the design that
  looked cleanest and it is the one to argue against, because it costs the most for the least: it
  ships an unreachable arm to four hosts that never answer `Listed`, and the C ABI would need a
  frame that has a size and no bytes. `CLAUDE.md` forbids a path nobody takes. What the confined
  worker actually needed was an *origin* for a page it holds marks for, and the viewer answers that
  already — `Query::PageGeometry`, which says `Answer::None` for a page the arrangement is not
  showing and so serves as the store's eviction rule in the same question.

## 2. Where the merge lives, and why it is the transport's

`viewer_confined::protocol::Marks` now holds each page's origin beside its bytes, refreshed from
`Query::PageGeometry` after every command — because a scroll moves a page without redrawing it —
and `encode_answer` merges the two halves of a frame reply: the pixels the viewer is holding, and
the marks this store is holding. Keyed by page, so the reply keeps the page order it already had.

That is ADR 0633's own division kept: *which of two encodings a page crosses in is a fact about a
pipe*, so the place that knows both is the transport, not the boundary. The pixels are inserted
second, so a page both halves claimed would cross as the pixels the viewer is holding now — the
rule the older `Marks::of` stated as a size comparison, kept as a sentence now that the arms are
disjoint by construction.

`MAGIC` **does not move**: the wire format is byte for byte what `PDFVCF04` already was, and so is
the population it carries. What changed is which side of the worker each page's entry comes out
of. `fuzz/seed_confined_wire.py` reads `MAGIC` out of `protocol.rs` since ADR 0633 and is
unaffected; the target was re-run rather than assumed.

## 3. What it saves

**The render that does not happen**, and it is the whole of the buy. `examples/confined_page`,
four documents, two workers built from this tree with and without the skip, run interleaved in one
sitting, five runs apiece, **load average 4.1 to 4.5** — not the 1.5 to 2.4 ADR 0633's table was
taken at, three neighbouring rounds being what a load average is made of, which is why the two
*controls* below matter more than the two subjects.

Milliseconds from `Command::Open` to the events coming back — an interpretation, a payload choice,
and on the pixel arm a rasterisation:

| page | arm | before, median (min) | after, median (min) |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` p1 | marks | 8.73 (5.56) | **2.82 (2.41)** |
| ISO 32000-2 p1 | marks | 48.86 (46.72) | **40.38 (36.50)** |
| `scan-bad.pdf` p1 | pixels | 71.60 (67.43) | 70.78 (67.60) |
| `issue12841_reduced.pdf` p1 | pixels | 178.72 (169.01) | 171.43 (164.90) |

**The controls are the argument.** The two pixel-arm documents are the pages this change does not
touch, and they do not move — which is what says the two rows above them are the rasterisation
leaving rather than the machine wandering. On the sparse page every "after" sample is below every
"before" sample and the open is **three times faster**; on ISO 32000-2's densest first page four of
five are, for about **8.5 ms**.

**And the extreme case is the one worth quoting**, because a page's marks say nothing about what
they cost to draw. `tests/support/amplification.rs` builds a 1.5 kB document that draws ten
thousand page-covering fills through nested form XObjects. At a 900×1200 window its marks are 990
kB against 4.19 MB of pixels, so it crosses as marks — and it rasterises, release, one strip as the
worker uses, in **26.5 s**. The confined worker now opens, interprets and ships it in **18 ms**.
That is `tests/confined.rs`'s `a_page_whose_marks_cross_is_shipped_without_being_drawn`.

## 4. What it costs, stated rather than discovered

**The cancel covers the work this process does, and on the marks arm that is now the
interpretation.** ADR 0241's cancel is a kill of the worker; what it stops is whatever the worker
is doing. A page shipped undrawn is a page whose *rasterisation* the worker never begins, so there
is nothing of ours left for a cancel to be about — the drawing is the host's, on the host's device,
where ADR 0607 put it.

This is worth being precise about, because it sounds worse than it is. **The host had to draw those
marks either way**: since ADR 0633 the marks are what crosses, so the host's rasterisation was
already unbounded and already outside the confinement. What the worker was contributing was a
*second* copy of the same work, inside the confinement, thrown away — and the fact that a cancel
could stop that copy protected nothing, because the host would then draw the page itself with no
cancel at all. So what this decision removes is duplicated work, and what it exposes is a cost ADR
0607 had already accepted: **a host that takes marks owns their draw**, and needs its own answer
for a page that will not finish. That answer is not in this tree yet and `doc/todo/15` now says so.

What is emphatically *not* given up is the memory bound. `MAX_PIXELS` bounds every request this
process makes, exactly as before, which is the whole reason `Rendered::Listed` exists rather than
`Rendered::Presented` being reused.

**One fixture had to grow a level, and finding out why is the finding.**
`tests/support/amplification.rs` builds the hostile document the cancel test cancels, and at four
levels its marks are the smaller payload — so after this change the worker shipped it undrawn and
the test's premise ("the work had not finished after two seconds") failed in thirty-one
milliseconds. The document is five levels deep now, which puts its list at 9.9 MB — larger than any
raster this boundary permits — so it crosses as pixels, the worker draws it, and the cancel is
again about something. **The premise is now checked rather than assumed**: the test asks
`wire::crossing` which arm the page takes before it blocks on one, so a fixture that drifts under
the choice fails saying so instead of racing a pipe.

## 5. What says it is on

- `viewer-core`'s `taking_one_pages_list_does_not_lift_the_raster_budget` — the same shape as the
  tier-2 test beside it with `Listed` in `Presented`'s place, and the opposite verdict: the 40×
  zoom that a presented host sails through is still refused by name.
- `viewer-core`'s `a_page_the_host_took_the_list_for_leaves_its_neighbours_answerable` — a column
  of pages, one answered `Listed` and the rest with pixels, and `Query::Frame` answering for
  exactly the rest.
- `viewer-confined`'s `a_page_the_viewer_holds_no_pixels_of_still_crosses_as_marks` — an *empty*
  `Answer::Frame` producing a reply that carries a page, which is what could not happen before,
  plus the eviction rule in the same test.
- `a_sparse_page_crosses_the_confinement_as_marks_rather_than_as_pixels`, which is now structural
  as well as descriptive: the pixels win the merge, so a worker that drew this page anyway would
  ship the raster it drew and this assertion is what fails.
- `a_page_whose_marks_cross_is_shipped_without_being_drawn` — the clock, honestly used, over a
  three-orders-of-magnitude gap with nothing in between.
- And the oldest one, unchanged and still the strongest: the page a host draws from a list it did
  not interpret is byte-identical to the page the unconfined viewer draws.

## 6. Consumers

`viewer-ui` failed to compile, which is what nothing in `viewer-core` being `#[non_exhaustive]` is
for, and it is the only one that did: it matches `Rendered` exhaustively in two places and now
names the outcome it never produces. GTK, Qt and the C ABI *construct* `Rendered` and never match
it, so none of them moved and `PDFV_EVENT_KIND_COUNT` stayed where it is — an outcome is not an
event.
