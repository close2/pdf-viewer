# Q570 — May the converter answer an unembedded `Adobe-Identity` font: re-keyed text where ToUnicode is trustworthy, a rasterized page where it is not?

Source: the owner's word on Q569, 2026-10-23 (A569 quotes it in full). Evidence: session 1706's
census (the 37 pdf.js composite refusals, half `Adobe-Identity`); ADR 2162 (the `machine-face`
loss and its profile split); ADR 0270 (the viewer's drawing route).

## The fact

An `Adobe-Identity` CID is an index into the producer's own program (§9.7.4.2), so no shipped
face can be ordered by those numbers — but the viewer draws these pages today all the same
(ADR 0270): it takes a face of this machine's for the font's name and maps each CID through the
font's ToUnicode to a glyph. That extraction aid is also something the converter already derives
where absent and checks where present — ISO 19005-2 §6.2.11.7.2 asks a CMap only of Level U and
above, Part 4 states it as a `should`, and the row `fonts/to-unicode-values-are-usable` refuses
a map whose values are garbage (the census's U+0007 and `#` documents) rather than trusting it.
So two cases exist, and the tree already tells them apart:

- **ToUnicode usable** — the viewer's own drawing is a CID→Unicode→glyph mapping the converter
  can reproduce, and the page's text is machine-readable in this reader.
- **ToUnicode absent or unusable** — nothing says what the CIDs mean; the viewer cannot draw the
  marks, and the input's text was never selectable or extractable in any reader.

## The owner's two remedies (proposed in conversation, A569)

1. **A font keyed from the ToUnicode.** Where the values are usable, embed a synthesized
   CID-keyed program whose glyphs are a face's, selected by those values and renumbered to the
   document's CIDs: the CIDs are untouched, so positions are exact; text stays text; the written
   ToUnicode states the values it was keyed from. The owner called it a "pixel font"; the
   embedded form is outlines, because a raster glyph program is the Type 3 form and PDF/A
   prohibits Type 3 (ISO 19005-1 §6.3.7's rule carried through the parts; the build verifies
   Part 4's clause number when it files the row). The loss is two things at once: the shapes are
   a substitute's, which is `machine-face` at composite scale, and the glyph choice is inferred
   from an extraction aid, which is a new named loss of its own.
2. **A rasterized page.** Where no trustworthy ToUnicode exists, rasterize the page and emit an
   image-only PDF page — conforming at Level B, which asks no text of a page — with no
   selectable text. The owner's gate: last resort, and only where an image is an equivalent in
   usability to what the input represents, which is exactly the branch condition — the input's
   text was never machine-readable anywhere, so the image discards nothing the input offered.

## Recommendation

Take both, in the owner's order, each behind a named loss the levels attach to:

1. `--font` first — the producer's program is the only faithful answer.
2. The shipped collection face (Q569's four faces) for the collection-named half.
3. **Re-keying** for `Adobe-Identity` where `fonts/to-unicode-values-are-usable` passes — the
   trust spent is the same trust the viewer's own drawing already spends, and the check that
   gates it already exists. Embed the shipped CJK face's glyphs for those values, uniform with
   ADR 2162's ruling for simple fonts: `as-if-printed` discards the losses, `as-shown` stops
   (shapes differ from the window's), and embedding this machine's own face instead — the one
   form that would satisfy `as-shown` — is not taken now, because ADR 2162 declined it for
   simple fonts and a machine face's embedding permission (`OS/2` `fsType`) is not this
   program's to assume.
4. **The rasterized page** where the values fail — the same row that refuses today now routes
   the page to an image. A print never had a text layer either, so both profiles may carry it
   as a named loss (`text-as-image`) a configuration can withhold, and `as-shown` may proceed
   when the owner's gate holds, because the image is the window's own pixels.

## What the tree does meanwhile

Refuses each by name (`COMPOSITE_NOT_SUBSTITUTED`, with the `--font` sentence;
`fonts/to-unicode-values-are-usable` keeps refusing garbage maps). Nothing is built.