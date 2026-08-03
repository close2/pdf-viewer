# ADR 0172 — Two `shall`s that cannot both hold, and which one a strike-out obeys

Status: accepted, two-hundred-and-thirty-sixth session.
Closes `doc/todo/13-a-strikeout-that-does-not-follow-its-text.md`, opened one session earlier
from a report by the project owner.

## Context

> on page 285 of the `ISO_32000-2_sponsored_EC3.pdf` is some strikethrough text. When zooming
> the lines stay in place. I am pretty sure they should move with the text.

Checked rather than guessed. Page 285's `/Annots` holds fourteen annotations; object `11702` is
`/Subtype /StrikeOut` with `/F 220` — Locked, ReadOnly, **NoRotate**, **NoZoom**, Print — a
`/Rect [86.155 714.216 487.863 738.456]` and eight `/QuadPoints` over two lines of text. So the
file asks for what is being seen, and this tree started obeying it in the
two-hundred-and-seventeenth session (ADR 0168, §12.5.3's `NoZoom` and `NoRotate`). Before that
the annotation scaled with the page because nothing read the flag.

**It is not a rendering accident. It is one clause obeyed, and the question is whether a second
clause says it should not have been.**

## The conflict, and it is between two `shall`s

§12.5.3 is unconditional about the flag:

> If the NoZoom flag is set, the annotation shall always maintain the same fixed size on the
> screen and shall be unaffected by the magnification level at which the page itself is
> displayed.

and names the fixed point: "the coordinates of the upper-left corner of its annotation
rectangle", adding "The PDF processor shall perform this alteration automatically." Our
`ViewGeometry::adjustment` does exactly that. At 200% a strike-out is drawn half-size about
`(86.155, 738.456)`, so its left end stays on the text and its right end falls short by half the
line — which is the report, seen from the other side: the lines *are* anchored, and they no
longer span the words.

§12.5.6.10 says what a text markup annotation is, and it is a `shall` too:

> Text markup annotations shall appear as highlights, underlines, strikeouts (all PDF 1.3), or
> jagged ("squiggly") underlines ( PDF 1.4 ) in the text of a document.

with Table 182 fixing it to the text:

> An array of 8×n numbers specifying the coordinates of n quadrilaterals in default user space.
> Each quadrilateral shall encompasses a word or group of contiguous words in the text underlying
> the annotation.

At any magnification but 1 these cannot both hold, and **the standard states no precedence
between them.** §12.5.6.4 is the proof that it legislates these flags per subtype when it means
to — text annotations "shall behave as if [both] were always set" — and it says nothing of the
kind here, in either direction.

## What was counted before anything was decided

Every `Highlight`, `Underline`, `Squiggly` and `StrikeOut` in the corpus and in `doc/`, by
`mutool show … grep`:

```text
511 text markup annotations, in 34 documents
    /F   0 :   1        /F   4 :  65        /F 132 : 234        /F 220 : 211
211 carry NoZoom — every one of them a StrikeOut, and every one of them in
    ISO_32000-2_sponsored_EC3.pdf
```

**One document, one subtype, one flag value on all 211.** That is a producer emitting a template,
not 211 decisions — which is evidence about intent and, on its own, not a reason to disobey a
clause. What it *does* establish is the cost of the change: no other document in the corpus or in
`doc/` is touched by it, in either direction.

## Decision

**§12.5.3's `NoZoom` and `NoRotate` do not reach §12.5.6.10's four subtypes.**

The argument is about what each clause is doing. §12.5.6.10 says what the annotation *is* — an
appearance in the text of a document, with its geometry stated by reference to that text.
§12.5.3 offers a display option that annotations have in general. **A general option does not get
to make an object stop being what its own subclause defines it as**, and the reverse reading has
no equivalent support: nothing in §12.5.3 says it overrides a subtype's definition, and §12.5.6.4
shows the standard writing that kind of override explicitly when it wants one.

`NoRotate` goes with `NoZoom` and for the same reason: a page's `/Rotate` turns its text, so an
upright strike-out over rotated text is detached exactly as a half-size one is.

**This is recorded as a choice, not as a derivation.** `CLAUDE.md`: where the standard genuinely
settles nothing — and two `shall`s with no precedence rule is that — say so plainly, choose, and
document it as a choice. Revisit it by argument.

## Consequences

- `ISO_32000-2_sponsored_EC3.pdf`'s 211 strike-outs follow their text at every magnification.
- **A page of them stops being view-dependent**, which is a second, unlooked-for gain:
  `Interpretation::view_dependent` is set from the same flag, and it is what forces a
  *re-interpretation* rather than a re-rasterisation on every zoom step. The clause's whole cost
  was being paid on the largest document in the tree — 1023 pages, 101 318 objects — for an
  annotation that should not have been paying it.
- No gate moves. The oracle and the corpus render every page at its own scale, where
  `ViewGeometry::magnification` is `None` and both flags change nothing by construction, which is
  why nothing could see this and why `annotations.rs` is where the test goes.
- §12.5.3 keeps its behaviour for every other subtype, which
  `a_text_markup_annotation_scales_with_the_text_it_marks` pins with a `Square` carrying the same
  `/F 220`. Without that case the test would be indistinguishable from a repeal of the clause.
