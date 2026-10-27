# 2454 — A photographed page's substitutes are the fonts the output holds

Status: implemented. Session 1810, the substitutes half of the archive slot of batch one hundred and
twenty-one. Context: ADR 2394 (the photograph), ADR 2445 (this defect, deferred), ADR 2162 (a direct
font written out as an object).

## The defect

A photographed page is given one image for content and resources that name that image alone, so a
font only that content selected is in no object the output holds — the writer copies what the
catalog and the information dictionary reach (`rewrite::convert`). The fonts were embedded first,
page by page, and each `Substitutes::done` entry named the font only by its resource name and its
`/BaseFont`, so nothing could tell which of them the photographs had taken away: on ADR 2445's plant
(a Helvetica drawing a space beside the unnamed `Adobe-Identity` font) the report and the file's
`xmpMM:History` both said Helvetica was embedded from Liberation Sans while the output held no
Helvetica at all. The report listed what was planned, not what was written.

## The decision

1. **Each entry names the font it substituted** (`SubstitutedFont::font`), in the form the
   photograph's deferred fonts already use: the font's object, or for a font written directly in a
   resource dictionary its value as `Debug` prints it — `fonts::selected`, shared by both.
2. **Once a page is photographed, the entries are kept only for fonts the output still holds.**
   `photographed::still_held` walks the source's objects from the trailer's `/Root` and `/Info`,
   each once and with an explicit stack, with every photographed page's `/Contents`, `/Resources`
   and `/Group` left out — the keys the rewriter replaces or removes there — and keeps an entry
   whose font it reaches. Reachability rather than selection, because the question is what the
   output holds: a font an annotation's appearance on the photographed page selects, one another
   page selects, and one an inherited resource dictionary or a form's `/DR` still names are all in
   the output with their faces, and the report names them.
3. The losses, the operator's supplied facts and the history entry are all read off the kept list,
   so a face the output does not carry costs nothing and is recorded nowhere.

ADR 2445's design read the selections instead (keep a font another page or an annotation selects);
reachability answers the same two cases and a third it would have missed — a font the output
carries that nothing selects — which is why it was taken.

## Consequences

- The plant: the report lists no substituted font, and the output names no Helvetica in any object,
  report or history; its control (the same Helvetica also shown by a `Square` annotation's
  appearance on the photographed page) keeps the entry, and the font in the output has its program.
- No corpus document reaches it: none of the photographed conversions lists a substituted font (ADR
  2445), and the census is unchanged by this half.
- The walk is a prediction made in preparation, over the source's graph, because the history entry
  is written into a packet the writer's own walk copies. A remedy that removes the last route to a
  font for a reason of its own — a forbidden annotation whose appearance alone shows it, on a page
  that is also photographed — would leave that font over-reported the same way; no file in either
  corpus holds that shape, and it is the sentence a later round reads if one does.
