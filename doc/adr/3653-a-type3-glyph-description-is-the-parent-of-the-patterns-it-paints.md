# 3653 — A Type 3 glyph description is the parent content stream of the patterns it paints

Session 2238. Status: accepted. Context: review 2237 (B01, D02), ISO 32000-2 §8.7.2, §9.6.4 and
Table 110, Errata Collection 3's Issue #128 (§7.8.3).

## Decision

`Interpreter::draw_type3_glyph` swaps `Interpreter::base` for the glyph description's initial CTM
— §9.6.4's `FontMatrix × text space` — around the description's run, and restores it after, as
a form's `Do`, an annotation appearance, a tiling cell, a soft mask's group and an imported page
already do. A pattern a glyph description paints with is therefore placed against "the default
coordinate system of the pattern's parent content stream" (§8.7.2), which is the glyph's.

**Whatever dictionary the resource search ends in.** A Type 3 font stating no `/Resources` reads
the page's by Issue #128's last step, and review 2237 left open whether a pattern found there is
placed against the page's space or the glyph's. The glyph's: the search decides *where a name is
looked up*, not which stream uses it, and a form stating no `/Resources` is treated the same way
(its `Do` swaps `base` unconditionally). One rule for every nested stream is the reading a person
can predict; the alternative would place one pattern differently according to which dictionary
happened to state it.

The `base` field's documentation and the comments over a shading pattern's and a tiling's
placement now say "the parent content stream's default space" (D02).

## Cost

None measurable: one `Transform` swapped per glyph description run. A page whose Type 3 glyphs
paint with a pattern moves; `raster_golden` names any such page.
