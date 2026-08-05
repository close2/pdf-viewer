# ADR 0193 — A construction is not a form XObject

Status: accepted, 2026-08-05 (session 314).

## Context

Found by drawing something else. ADR 0192 gave §12.5.6.7's line endings a size, which took
`issue13447.pdf` off the corpus's incomplete list, which let the oracle judge its first page for
the first time — and the side-by-side showed **two whole annotations that four other renderers draw
and this tree drew nothing for, with no report**.

The file states, on two of its line annotations:

```text
/Rect [598.31 146.63 537 316.13]     % x 537 … 598
/L    [176.63 45 177.94 154.5]       % x ≈ 177
```

The rectangle does not contain the line. Both of its polylines are the same: `/Rect [667.5 250.69
567.75 344.44]` — beyond the page's own 612-unit width — against `/Vertices` at x 250 to 350.

This tree drew neither, because a *constructed* appearance was clipped to `/Rect`. The clip was
inherited from the stored-appearance path, where it is correct and required: §12.5.5 makes an
appearance stream a form XObject, and §8.10.2 says of a form's `/BBox` that it is

> ... expressed in the form coordinate system. This bounding box shall be used to clip the form
> `XObject` and to determine its size for caching.

A construction has no `/BBox`, because it has no stream in the file; this tree invented one, and
what it invented was `/Rect`.

## Decision

**A constructed appearance is bounded by `/Rect` only where the clause it is built from bounds it
by `/Rect`.**

Four subtypes state their geometry in *default user space* — the page's space, not a box's — and
those are unbounded:

| clause | entry | the words |
|---|---|---|
| §12.5.6.7 | `/L` | "specifying the starting and ending coordinates of the line in default user space" |
| §12.5.6.9 | `/Vertices` | "the coordinates of each vertex, in default user space" |
| §12.5.6.10 | `/QuadPoints` | "the coordinates of n quadrilaterals in default user space" |
| §12.5.6.13 | `/InkList` | "each array … a stroked path … in default user space" |

Every other construction *derives* its geometry from `/Rect` — an icon on the largest square inside
it, a border along it, a widget's background filling it, a field's value laid out in it — so
bounding those changes nothing, except in the one case where the bound is the point: §12.7.4.3's
value is clipped to the field it does not fit in.

So the rule is per clause rather than per annotation, and it is written where the clause is:
`appearance::Constructed::bounded`.

**Why not simply widen the box to hold both?** Because that answers a different question. A union
of `/Rect` and the marks would still be a box this program invented, and it would still clip
something one day; the honest statement is that the clause states where the marks go and nothing
in the standard says a constructed appearance is clipped at all.

**What is *not* excepted is §12.2's `/ViewClip`.** Where a document has narrowed what the screen
shows, an annotation is drawn over the page and is not exempt from what the page is clipped to, so
an unbounded construction still inherits it. Only the invented box goes away.

## Consequences

- **Four annotations on one corpus document draw that drew nothing before**, and the change is
  visible: our ink on that page goes 24.13 → 24.81 of 255 against the three C renderers' 25.5 to
  25.8. The rest of the gap is ADR 0192's size choice, measured in
  `AMBIGUOUS_LINE_ENDING_SIZE`.
- **It was a silence, which is the part worth keeping.** Nothing was reported, because nothing
  refused: the marks were made and then clipped away by a box a *different* clause justifies. Trap
  5's rule is about refusals; this is the shape it does not cover — a mark lost between two correct
  pieces of code — and the only instrument that found it was the picture (trap 1).
- **The oracle judges one more page**: 749 → 750 ambiguous on documents we call complete, because
  a document that stops reporting starts being judged. The denominator moving in that direction is
  the gate working, and it is why the ratchet counts both.
- **`hayro` draws no line ending at all**, which the same measurement found. Not a defect of ours
  and worth knowing when reading that panel: on this page the two renderers below the C three are
  the two written in Rust, and for different reasons.
