# ADR 0168 — An annotation that does not zoom, and the one thing it costs

Status: accepted, 2026-08-03. Session 217. `doc/todo/25-view-dependent-annotations.md`'s second
half.

## What the clause asks

ISO 32000-2 §12.5.3, Table 167 bits 4 and 5:

> If the NoZoom flag is set, the annotation shall always maintain the same fixed size on the
> screen and shall be unaffected by the magnification level at which the page itself is
> displayed. Similarly, if the NoRotate flag is set, the annotation shall retain its original
> orientation on the screen when the page is rotated (by changing the Rotate entry in the page
> object; see 7.7.3, "Page tree").

and, in the same subclause, the fixed point and the thing not to touch:

> In either case, the annotation's position is defined by the coordinates of the upper-left
> corner of its annotation rectangle, as defined by the Rect entry in the annotation dictionary
> and interpreted in the default user space of the page. … However, it shall not actually change
> the annotation's Rect entry, which continues to describe the annotation's relationship with the
> unscaled, unrotated user space.

§12.5.5 says where the adjustment goes:

> The annotation may be further scaled and rotated if either the NoZoom or NoRotate flag is set
> (see 12.5.3, "Annotation flags"). Any transformation applied to the annotation as a whole shall
> be applied to the appearance within it.

So: one similarity about the `/Rect`'s upper-left corner, in default user space, composed *before*
the page's own transform — which is what lets it undo that transform — and applied to the whole
annotation, §12.5.6.19's highlight included.

## Why the ledger said it could not be done, and why that was two claims

The row read: "NoZoom and NoRotate make an appearance's size or orientation depend on the view,
which a resolution-independent display list cannot express — neither is applied nor reported."
True-sounding, and it is a reason about this project's *architecture* rather than about the
standard, which `doc/todo/01` says to read with suspicion. Split in two, it is one sentence that
was never true and one that costs less than it sounds.

- **`NoRotate` depends on `/Rotate`, which is in the file.** The page's rotation is not a property
  of the view at all; it is §7.7.3.3's entry, which `base_transform` has read since the first page
  tree. So this half needed no new vocabulary, no re-interpretation and nothing from a host — it
  is as pure a function of the document as every other mark on the page.
- **`NoZoom` depends on the magnification, which is the reader's.** That is a real dependency and
  the display list genuinely cannot carry it.

## Where the magnification enters, and why it is `ViewState`

`CLAUDE.md`'s rule 1: `pdf_syntax::Document` is immutable and `interpret` is a pure function of
the document and the view state. A magnification is a statement about the view, so it goes in
`ViewState` beside the layers a person switched and the value they typed — not as a fourth
argument to `interpret`, which would have made every caller state one.

**`None` is not 1.0, and the distinction is the whole reason the gates did not move.** It means
*nobody has said*, which is what the corpus gate, the oracle, `render-quorra`'s comparison and
every caller of `ViewState::of` mean: they render a page at its own scale and have no opinion
about a zoom. Under `None`, `NoZoom` changes nothing. Reading an unstated magnification as 100%
would have made three gates assert a zoom nobody chose.

**Logical pixels per user unit, not device ones.** "The same fixed size on the screen" is a size a
person sees; on a doubled display the annotation should be drawn *sharper* rather than half as
large, so `viewer-core` divides the device magnification by the display's scale before handing it
over. A choice, and the one the sentence supports.

## The cost, and how it is confined

A zoom re-rasterises the same display list without re-interpreting, and that property — asserted
by pointer equality in `zooming_rasterises_again_without_interpreting_again` — is what makes
zooming cheap. `NoZoom` is the only thing in the standard that breaks it.

`Interpretation::view_dependent` is what keeps the breakage where the clause put it: it is true
exactly where a *drawn* annotation sets `NoZoom`, and `Viewer::settle` re-interprets only when the
magnification changed **and** the last interpretation said the page would notice. Measured over
the corpus with `mutool show`: 124 annotations in **51** documents set `NoZoom` — 82 of them
popups, which §12.5.6.14 makes this tree draw nothing for — so 923 documents pay nothing at all.
`NoRotate` is deliberately not part of that flag, for the reason above.

## What no corpus document can show, measured rather than assumed

Trap 8's third shape. 127 annotations in 51 documents set `NoRotate`, and **not one of them is an
annotation this tree draws on a page with a non-zero `/Rotate`**: the 82 popups are not drawn, and
the three documents that carry both a `NoRotate` annotation and a rotated page — `inks.pdf`,
`pr20043.pdf`, `rotated_ink.pdf` — put the flag only on popups. So the corpus cannot exercise this
half at all, and the corpus and oracle verdicts are unchanged to the page.

That is a reason to write the test by hand rather than a reason not to implement it: §6.3.2.2
ranks a rendering processor's obligations and a corpus cannot rank a requirement no file
exercises. `a_no_rotate_annotation_pivots_about_its_own_corner` is a 100×100 page at `/Rotate 90`
with a `/Rect [40 40 70 70]` whose appearance fills only the left half of its box, so the mark is
asymmetric and a rotation of it is visible. Every number in it is one composition of two matrices:

```text
the mark, in default user space         x 40..55   y 40..70
/Rotate 90 alone, (x, y) -> (y, 100-x)  x 40..70   y 45..60
pivoted first, (x, y) -> (110-y, x+30)  x 40..70   y 70..85
  and then rotated                      x 70..85   y 30..60
```

Both tests were confirmed to fail with the adjustment stubbed to the identity.

## What is still owed

`/FixedPrint` (Table 193, §12.5.6.22's watermark annotations) is the remaining view-dependent
entry and it is a *printing* decision — the clause's own words are "printed at a fixed size
relative to the target media" — so it waits on a printing path rather than on this one.
`doc/todo/25` keeps `/Fo` and `/Bl`, which want a keyboard-focus model `viewer-core` does not have.
