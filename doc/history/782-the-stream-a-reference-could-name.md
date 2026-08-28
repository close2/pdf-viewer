# 782 — The stream a reference could name

2026-08-28. The batch's general-improvement round, subject chosen by argument.
Decision: [ADR 0719](../adr/0719-the-stream-a-reference-could-name-and-the-appearance-that-dropped-its-name.md).

## What was chosen and why

The `entries` sweep printed §14.7.5.2 — an **implemented** row — with Table 357's `/StmOwn`
"named nowhere at all, AND THE NOTE DOES NOT NAME IT", the sweep's own read-first shape, and
`doc/todo/31` carried the matching remainder with its design already argued: an `/MCR` whose
`/Stm` names an appearance stream found nothing, what would close it is the `/AP` reference
carried through `Appearance`, and `/StmOwn` is the same item from the structure side, "take the
two together or not at all". No corpus or crawl document exercises it (`mcid_stream_census`), so
it is the coverage track's to reach — the batch's siblings held the errata ranking, the oracle's
contradicted pages and the confined-boundary host, and this touches none of their lanes.

## What landed

- `annotation::Appearance::source`: the `/AP` entry's indirect reference — from either of
  §12.5.5's two forms, the direct entry and the state subdictionary — captured before
  resolution, the same shape as `draw_xobject`'s unresolved lookup. The interpreter seats
  `ContentStream::Object(id)` for a stored appearance's run; `Unnameable` keeps the directly
  written stream and the §12.7.4.3 construction. A regeneration keeps the identity, because the
  splice leaves every sequence outside the `/Tx` region byte-identical.
- `structure::Child::MarkedContent` gains `owner` — Table 357's `/StmOwn`, read as the reference
  it is — and `viewer_core::accessibility` feeds it into the `objects` channel §14.7.5.3 built,
  so such an element is placed by the widget's `/Rect`, says which control it is, and names the
  annotation a click goes to.
- `Tree::appearance_owners` — the finding the round did not go looking for: the structural
  population walk asked the page's `/StructParents`, its annotations' `/StructParent` and the
  `XObject`s the page's *resources* name, and an `/AP` entry is not a resource, so an element
  whose only content item lives in an appearance stream was reachable by no route and pruned.
  Found by the round's own end-to-end test failing; the fifth route in a function whose fourth
  (ADR 0488) was the same shape.
- A narrowing that fell out: ADR 0488's exactly-one-stream recovery could not tell two
  appearance streams apart while both were `Unnameable`; named apart, two streams sharing an
  identifier answer nothing, which is what the condition always said.
- Ledger: §14.7.5.2 and §14.7.5.4 notes extended (both stay `implemented`), tests added to both
  rows. `doc/todo/31`'s item struck through as taken; `doc/todo/README.md`'s line updated.
- Tests: three in `marked_content_scope.rs` (fixture: a stamp whose appearance stream marks
  `/MCID 0` against the page's own `/MCID 0`), one end-to-end in `headless.rs` (a check box
  reached through `/Stm`+`/StmOwn` with no `/OBJR` anywhere). Each calibrated per trap 13 —
  interpreter seating forced back to `Unnameable`, the consumer push dropped, the population
  walk reverted — failed under every plant, restored, green.

## Gates

Full §2 sequence (pdf-model touched): fmt clean; clippy `-D warnings` clean after three of its
own findings (a similar-names binding, `walk` over the line limit — resolved by extracting
`names_object`, which the two arms now share — and a doc-markdown line split); nextest 2721
passed, 18 skipped; doctests ok; fuzz targets check clean; corpus, oracle (3 passed),
text-extraction (4 passed), selection census, accessibility census (ratchets held), dates, xmp,
jpeg2000, quorra corpus, fixed-documents (40 checked, 0 absent), conformance all green.
`spec-errata emit` over §14.7.5.2: the errata there (Issue #431) reword `/Pg`'s requiredness and
bind nothing new on `/Stm`/`/StmOwn`.

## Sweeps

Baseline and after in `tmp/sweeps-782-*.log`; every delta accounted: `entries` −1 hit (the
subject retired), `pointers` +3 live (this round's documents), `tables` +16 agreeing citations,
`counts` +12 sentences, `overtaken` +1 decision record (ADR 0719), rest unchanged. One
intermediate delta was this round's own note reading as an unread claim to the `unread` sweep
("off the unread list"); reworded, and the sweep returned to baseline.

## Contradictions with the briefing

None. CI red on origin/main is the pre-existing owner-arc failure, untouched here.
