# 2394 — A page whose Adobe-Identity text no reader can name is photographed as the window draws it

Status: implemented. Session 1780, the archive slot of batch one hundred and eighteen. Builds the
second of owner answer A570's two remedies, after ADR 2378's re-keying. Context:
`doc/questions/Q570`, `A570`; ADR 0270, ADR 2162, ADR 2378.

## The reading

§9.7.5.2: "The Identity-H and Identity-V CMaps shall not be used with a non-embedded font." Under
`Adobe-Identity` a CID indexes the glyphs of the program that defined it (§9.7.4.2). Where the font
states no `/ToUnicode`, or one re-keying refuses, nothing in the file says which glyph a CID is,
and the viewer draws those marks blank (ADR 0270). A570: the image branch chooses no font, because
the per-CID correspondence is the missing piece; it photographs the window's own rendering, blank
marks included, and its worth is page salvage.

## The decision

`archive::fonts::photographed`, reached from `composite::unnamed` after the pack and re-keying.

1. **The font is deferred, not refused**, and once every font is in, each page whose content
   selects it (`pdf_model::content::interpret_ledgered`'s `Font` lookups) is interpreted once with
   its annotations left out. That one interpretation is the gate's evidence and the picture.
2. **The owner's gate** — only where the input's text was never machine-readable — refuses the page
   by name where its readback names any character (§9.10.2's routes and §14.9.4's replacement text,
   every font of the page), where the interpreter reports anything but the fonts it could not draw,
   and in a document stating a structure tree (§14.7) or optional content (§8.11). A font that keeps
   a `/ToUnicode` keeps re-keying's own sentence where its map makes the page readable.
3. **The picture**: the oracle backend at 300 dpi, a documented choice no clause states, written as
   one `ICCBased` sRGB image placed through the inverse of `base_transform`, so `/Rotate`, the crop
   box's origin and `/UserUnit` stay the page's. The page's `/Contents` and `/Resources` are
   replaced and its `/Group` goes; its annotations stay, drawn over the picture as before.
4. **The loss `text-as-image`**, in the one decision at `fonts/font-programs-embedded`. Both shipped
   profiles carry it: `as-if-printed` through the site's `discard`, `as-shown` through a new row key,
   `text-as-image = "discard"`, beside its `stop`, which leaves the site's other three losses
   stopped. Offered at 2b and at every part 4 flavour; at 2u and 2a the font keeps its refusal,
   because those levels exist to keep a page's text.
5. **Reported**: `Conversion::photographed` per page (number, resolution, pixels, fonts), the JSON
   report's `photographed_pages`, and the file's own `xmpMM:History`.

## What this is not

Not a font substitution and not a flattener (RFC 0006 section 5.2's option is untouched): the marks
are the producer's as this reader draws them, nothing is composed, and a page with anything a
reader could extract is never taken. A570 places the construction; `CLAUDE.md`'s list of the two
ratified amendments on the near side of the authoring line does not yet name it, which is the
owner's file to amend.

## Consequences

- Of the 19 pdf.js documents — A570's 18 and issue5244 — 12 convert at 2b and 13 at part 4 under
  each shipped profile; the rest keep a refusal by sentence: five documents whose page also shows
  extractable text, one stating a structure tree, and one stopped at 2b by its own metadata.
  `doc/history/1780` has the census and the veraPDF figures, and the one output veraPDF fails at
  part 4, on the source's packet rather than on the page.
- A drawn output is the source's raster at 300 dpi to zero levels; at another resolution it is
  resampled, which is the loss.
- Refused still, by sentence: a font only an annotation's appearance shows; a page this reader
  recovered rather than one the tree states; a page past the pixel budget at 300 dpi.
