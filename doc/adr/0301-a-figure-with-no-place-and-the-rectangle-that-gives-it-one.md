# ADR 0301 — A figure with no place, and the rectangle that gives it one

Status: accepted, 2026-08-13. Session 466. Amends §14.8.5.4.3's, §14.8.5.4.6's and §7.9.5's ledger
rows. Extends ADR 0214's bridge and ADR 0300's reading of §14.8.5; changes nothing either decided.

## The question

ADR 0300 left this tree's only `silent` ledger row. `viewer_core::AccessibilityNode::quads` is
built from the text layer, so an element whose content drew no *text* — a `Figure`, a table cell
holding an image — crossed the boundary with an empty list of shapes, and an assistive technology
was handed a node with **no place at all**. Nothing said so.

Table 379 states the rectangle that would fill it. The question is whether any document states one,
whether it says what a magnifier needs, and what happens when it says something absurd.

## What the standard states

§14.8.5.4.3, Table 379:

> An array of four numbers in default user space units that shall give the coordinates of the left,
> bottom, right, and top edges, respectively, of the structure element's bounding box (the
> rectangle that completely encloses its visible content).

and the condition on when a producer writes one:

> The BBox attribute should be present for structure elements whose content does not lend itself to
> reflow or any other visual rearrangement of the content inside it.

with NOTE 1 naming which elements those are — "Figure and Formula structure elements" — and the
EXAMPLE naming the content: "Formulas, graphic art, vector drawings, images". **That is the same
population the text layer cannot place**, stated by the standard for its own reasons. Twelve of
Table 379's thirteen entries describe the layout *process* that produced an appearance this reader
already has; this one describes a result.

## The population, counted before anything was built

`crates/pdf-model/examples/element_bounds_census.rs`, over the pdf.js corpus, `doc/corpora/` and
`doc/`:

```sh
cargo run --release -p pdf-model --example element_bounds_census -- \
  $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') $(find doc/corpora -name '*.pdf') doc/*.pdf
```

| | |
|---|---|
| documents read / with a structure tree | 1080 / 117 |
| structure elements | 133 114 |
| documents stating a Table 379 `/BBox` | **28** |
| elements stating one / stating one that is not a rectangle | **132** / 0 |
| by type | `Figure` 77, `Table` 51, `P` 3, `Formula` 1 |
| elements whose content items produced **no text** | **1700** |
| of those, stating a `/BBox` | **61** — `Figure` 60, `Table` 1 |
| of those, reaching the page only through §14.7.5.3's `/OBJR` | 576 |
| `/BBox` rectangles reaching outside their page's crop box | **8** |

So the attribute is real but not common, and where it appears it is overwhelmingly on exactly the
elements that need it: **60 of the 61 rescued elements are `Figure`s**, which is NOTE 1's own
sentence coming back as a measurement. It also says what this does *not* fix: 1639 placeless
elements state nothing, and `doc/todo/31` now carries them with the strongest route named.

**Two things the census had to be corrected on before its numbers meant anything**, both trap 11's
shape. It first interpreted only the documents that stated a `/BBox`, which made every element of
every other tagged document count as marking no text — a denominator taken from a different
population than the numerator, and the placeless figure came out 4163 instead of 1700. And
`document.get_key(element, "Pg")` *resolves*, so `as_reference()` on it is always `None` and the
overreaching count was 0 until the raw entry was read: a measurement whose instrument silently
answered "no" to the question it was asked.

## What was decided

**1. The rectangle crosses as a second, separate statement.** `AccessibilityNode::bounds` sits
beside `quads` rather than merging with them, because they are different kinds of fact: the quads
are shapes *this program* found by drawing the element's text, and this is what the producer wrote
down. A host takes whichever it has.

**2. It crosses in the viewport's device pixels**, the space `quads` are already in, and the
mapping is `viewer-core`'s because only `viewer-core` holds the magnification, the centring and the
scroll. The attribute is stated in *default user space*, so it is one transform longer than a
selection's shapes: `pdf_model::content::page_space_at` first — §7.7.3.3's `/Rotate` and the crop
box's origin — and then the y flip, which belongs to `TargetSpec::for_page` and not to the page
(trap 12a, ADR 0118). The test states the expected numbers from the clause's own space on a page
with `/Rotate 90`, and removing `page_space_at` fails it.

**3. The result is intersected with the page, and that is a reading rather than a tidy-up.**
§14.8.5.4.3's rectangle encloses "its **visible** content"; §14.11.2.1 says what of a page is
visible — the crop box "defines the region to which the contents of the page shall be clipped
(cropped) when displayed or printed". A rectangle beyond the page therefore encloses nothing
anybody can look at. This is not hypothetical: `doc/PDF20_AN001-BPC.pdf` states
`[-32768 -32768 32767 32767]` for one figure — the whole representable plane, a producer writing
"somewhere" — and unclipped that reached the bus as a node 55 045 pixels square, which would win
every hit test on the page. Where the rectangle does not meet the page at all the answer is
nothing, because then the document has said nothing about where *on this page* the element is; a
rectangle that merely touches an edge crosses degenerate, since §7.9.5's own NOTE is that
"[r]ectangles can have a width of zero or height of zero".

**4. Where an element has both, the measured shapes win.** `viewer_accessibility::tree::place`
prefers the quads and falls back to the rectangle. The marks are what is on the screen; the
attribute is a claim about a layout this program has already carried out, so it answers where there
is nothing to compare it against. The case it under-serves — a `Figure` holding a caption and a
picture, where the quads cover only the caption — is written down in `doc/todo/31` as unmeasured
rather than decided by taste.

## How it was verified, and it is the bus rather than a `TreeUpdate`

`doc/verify.md`'s recipe: `dbus-run-session`, `at-spi-bus-launcher`, `at-spi2-registryd` with a
`DISPLAY` of its own, `Xvfb`, and a client walking `org.a11y.atspi.Accessible` from the registry
root, asking `org.a11y.atspi.Component.GetExtents` at every node. The same binary twice, one field
of difference — the A/B was taken by making `finish` answer `None` and rebuilding.

`doc/PDF20_AN001-BPC.pdf`, page one:

```text
        [documentFrame] '' (35, 236, 428, 578)
          [image] 'PDF Association logo'   before: —      after: (35, 181, 105, 48)
          [paragraph] 'A ppl ication Note' (282, 236, 181, 32)     — unchanged
          ...
            [image] 'Creative Commons'     before: —      after: (0, 146, 500, 707)
```

The logo's stated `/BBox` is `[42.365 741.715 168.265 799.525]` on an 841.89-unit page drawn at
0.8398 device pixels per unit, which is `(35.6, 181.6)` and `105.7 × 48.5` — the numbers on the bus.
The Creative Commons badge is the ±32768 one, and what comes back is the page: `(0, 146)` is where
the raster sits and `500 × 707` is its size. **Before, neither node implemented `Component` at all**
— a client asking where they are got an error, which is what "no place" looks like from the outside.

Two facts about the instrument, because the next round will want them and ADR 0300 half-recorded
them. An accessible's *name* is a D-Bus **property** and not a method, so a walker calling
`GetName` reads every node as `''` and looks exactly like a bridge that lost its labels. And the
adapter is inactive unless `org.a11y.Status IsEnabled` is true on the bus the viewer connects to —
which it is on a desktop session and is **not** inside a fresh `dbus-run-session`, where the whole
application subtree comes back empty with nothing saying why.

## What it costs

Nothing measurable. `Tree::bounds` is one `Tree::attribute` per element, and `Tree::attributes`
returns an empty list at once for an element stating neither `/C` nor `/A`, which is nearly all of
them. A/B in one sitting, in release, best of five over three runs of `Query::AccessibilityTree` on
ISO 32000-2's 1023 pages: **54.0–75.7 ms with the read, 54.2–77.0 ms without** — inside the
instrument's own spread, and inside the 67–91 ms ADR 0300 recorded for the same question.

That measurement is now `crates/viewer-core/examples/accessibility_cost.rs` rather than a stopwatch
somebody held: `doc/todo/31` has carried "this question costs eighty milliseconds" since the last
round with no way for anybody to check it, which is the shape `CLAUDE.md`'s "what is written down
is the command that counts it" exists to prevent.

## What this does not do

- `/Width` and `/Height` are still not read. They are results rather than process, like `/BBox`,
  but they give an extent and **no origin**, so they cannot place anything; §14.8.5.4.3's row stays
  `partial` and says so.
- Nothing reports an element that has neither a `/BBox` nor any text. That is not a silence about a
  requirement — the attribute is optional and the standard states no other answer — so the row is
  `partial` rather than `silent`, and the population is `doc/todo/31`'s to argue about.
- AT-SPI's `Table` and `TableCell` interfaces are still not implemented by
  `accesskit_atspi_common`, so a cell's coordinates still have nowhere to arrive. Unchanged by this.

## What it corrected on the way

§7.9.5's ledger row quoted "[a]lthough rectangles are conventionally specified by their lower-left
and upper-right corners, it is acceptable to specify any two diagonally opposite corners", and
**ISO 32000-2 does not contain that sentence** — it is ISO 32000-1's wording for the same rule.
Checked against `pdftotext` on the PDF as well as against `doc/md/`, so it is not a conversion
artefact. The current wording is weaker and more useful: a rectangle is "an array of four numbers
giving the coordinates of a pair of diagonally opposite corners", and `[llx lly urx ury]` is what
the clause says the array "[t]ypically" takes rather than what it requires — which is exactly why
`normalised_rectangle` sorts the corners instead of trusting them. Found by writing the same
quotation into a comment and checking it before committing it, which is the fifth sweep's method
applied one file over.

§14.8.5.4.6's row said `/BBox` was among the attributes that clause restricts. It is not: that
clause states three conditions on `/Height`, `/Width` and `/BaselineShift` under Table 377's
`/Placement`, and the `/BBox` a `Figure` states is Table 379's.
