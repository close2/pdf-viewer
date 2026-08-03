# ADR 0160 — A pattern in an annotation's appearance

Status: accepted, 2026-08-03. Session 199. Found by opening the *first* page on the ambiguous
ranking, at 5.44 bounds from the nearest reference.

## The page

`issue7821.pdf` is 166×55 points — a page cropped down to one stamp annotation reading
*APPROVED* in a rounded green box. Ten commands, one clip, `unsupported: []`, and the picture
looks entirely right: the box is in the right place, the border is drawn, the word is drawn.

The box is filled flat. Every other renderer grades it.

```text
along the box's top edge, our render against poppler's

ours       241,246,235  241,246,235  241,246,235  241,246,235   ← flat
poppler    242,246,235  238,245,232  235,242,228  231,240,224   ← a gradient
```

## What the file says

The appearance stream sets `/Pattern cs /P0 scn` and fills a rounded rectangle with it. `P0` is
a `PatternType 2` shading pattern over a `ShadingType 2` axial shading whose `/Coords` are
`[163.729 415.523 315.349 374.897]` — an axis running diagonally across the box, from a
near-white `(0.949, 0.969, 0.922)` to a pale green `(0.812, 0.878, 0.776)`, with
`/Extend [true true]`.

Those coordinates are in the *appearance stream's* space: its `/BBox` is
`[157.551 370.547 321.051 420.047]` and its `/Matrix` is `[1 0 0 1 -157.551 -370.547]`, which is
what §12.5.5 then maps onto the annotation's `/Rect` at `[447.998 2.737 611.498 52.238]`.

## What was wrong

§8.7.2 says where a pattern is positioned:

> Similarly, if a pattern is used within a form XObject (see 8.10, "Form XObjects" ), the pattern
> matrix maps pattern space to the form's default user space (that is, the form coordinate space
> at the time the form is painted with the Do operator).

`content.rs` carries that space in `self.base`, and `run_form` swaps it for the form's own
transform — since the fifty-second session, with a ledger row and a test.

**`draw_appearance` is the other way into a form's content, and it never did.** So the shading's
axis was placed by the *page's* default transform. The page's crop box starts at x 445.966 and
the axis runs 163.729 to 315.349, so the whole of it fell off the left of the visible page — and
`/Extend [true true]` obligingly painted the entire box the colour at the far end of an axis
nobody could see. A flat fill nobody would question.

The fix is one `std::mem::replace` around the `self.run`, exactly as `run_form` does it.

## What it moved

Our distance from the nearest reference went **5.44 → 1.79** bounds and the page's worst mean
12.83 → 8.50. **Exactly one page of the 1794 changed**, checked by diffing every verdict line
against a run with the fix stashed — so one corpus document's page one has a pattern inside an
annotation appearance, and the rest of the aggregate is untouched: 893 agree, 78 contradicted,
788 ambiguous, before and after.

It is **still ambiguous**, and honestly so: five renderers substitute five faces for a font the
file does not embed, and the box is drawn at 166×55 points where a rounded corner is two pixels.

**The ink table this section first carried was wrong and the correction is ADR 0163's.** It read
"ours 22.9, `hayro` 24.2, `mupdf` 47.1, `poppler` 46.7, `ghostscript` 50.0" and called the split
trap 9's third shape — three references sharing one `libfreetype` against two Rust renderers.
There is no split: our artefacts and `hayro`'s carry an alpha channel that the measuring command
was averaging in, halving both. The five are ours **45.71**, `hayro` 48.33, `poppler` 46.69,
`mupdf` 47.08, `ghostscript` 50.00 — a spread of 9%, with `ghostscript` the outlier rather than
us.

## The lesson, and it is the third time this row has been wrong

§8.7.2's ledger row has now been false twice about the same sentence:

- The fifty-second session found `base` staying the page's throughout a form, which put a type 5
  shading pattern 180 points below where it belonged and drew `issue6231_1.pdf` as bare axes.
  The row had claimed the rule for twenty sessions.
- This session found the appearance path, which is the *other* way a form's content gets run.
  The comment two lines above the `self.run` in `draw_appearance` says "an appearance stream is
  a form `XObject`" — the code knew, and the knowledge had not reached the paint.

**A rule about "the parent content stream" needs one test per way of becoming a parent.**
`tiling.rs::a_pattern_inside_a_form_is_anchored_to_the_forms_space` guarded one of the two, and
its passing is what made the row look maintained.

`shadings.rs::a_pattern_in_an_annotation_appearance_is_placed_in_the_appearances_own_space` is
the second. Its fixture separates the two spaces the way the real file does — the appearance
draws at x 200–300 and its `/Matrix` brings that back to the page's 0–100, with the shading's
axis stated at 200–300 — so under the page's transform `/Extend` would paint one flat colour.
Confirmed to fail with the swap removed.

## Alternatives rejected

- **Set `base` in `run` itself.** `run` is called for the page's own content too, where `base`
  must stay the page's; making it derive the base would have to distinguish the cases, which is
  what the two callers already do.
- **Make `Appearance` carry the base.** It already carries `transform`, and the base *is* that
  transform. A second field holding the same value is a second thing to keep in step.
