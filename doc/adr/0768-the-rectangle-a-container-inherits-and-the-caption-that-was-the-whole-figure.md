# ADR 0768 — The rectangle a container inherits, and the caption that was the whole figure

Status: accepted, 2026-09-01. Session 841. Amends §14.8.3.3's, §14.8.5.4.5's and §14.8.5.4.3's
ledger rows. Extends ADR 0486's content rectangle with the two derivations §14.8.5.4.5 states that
nobody had read as a viewer's; changes nothing ADR 0214, 0301, 0325, 0338, 0394 or 0425 decided.
The second half is about the oracle and settles nothing: it records why the head of the
contradicted list keeps its verdict, and what would move it.

## The question

`doc/todo/31` carried two remainders about where an element is, and both were phrased as questions
about what this program had left over rather than about what the clause says:

- **the elements whose sequences marked nothing** — 1021 of them when ADR 0486 counted, "because
  their sequences marked nothing at all … No clause derives a rectangle from no marks, so that is an
  answer rather than a debt";
- **whether a stated `/BBox` should win over the shapes that were drawn**, on the mixed element: "a
  `Figure` holding a caption *and* a picture has text quads covering only the caption while the
  attribute covers both … Nothing has measured how often that happens or by how much."

Both were the same defect of reading, and it is the one `CLAUDE.md` warns about: a claim that the
specification states nothing, made without reading the clause's neighbours. §14.8.5.4.5 has five
bullets and ADR 0486 read two of them.

## What the standard determines

**§14.8.3.3**, unchanged and still the `shall` this family turns on:

> The content rectangle shall be derived from the shape of the enclosed content and defines the
> bounds used for the layout of any included child elements.

**§14.8.5.4.5** divides that derivation by structure type, and the whole of it matters:

> - For a table cell (structure type TH or TD), the content rectangle is determined from the
>   bounding box of all graphics objects in the cell's content …
> - For any other BLSE other than TH and TD, the height of the content rectangle shall be the sum of
>   the heights of all BLSEs it contains, plus any additional spacing adjustments between these
>   elements.
> - For an ILSE that contains text, the height of the content rectangle shall be set by the
>   LineHeight attribute. The width shall be determined by summing the widths of the contained
>   characters …
> - For an ILSE that contains an illustration or table, the content rectangle shall be determined
>   from the bounding box of all graphics objects in the content …
> - For an ILSE that contains a mixture of elements, the height of the content rectangle shall be
>   determined by aligning the child objects relative to one another … and finding the extreme top
>   and bottom for all elements.

So the two questions are answered, in bullets ADR 0486 called "a layout engine's":

- **the mixed element is bullet five.** An element holding a caption and a picture takes "the extreme
  top and bottom for all elements", not the caption's. This program was answering with the text
  quadrilaterals wherever it had any, so the *presence* of one glyph decided that nothing else the
  element enclosed counted;
- **the container is bullet two.** A block-level element's rectangle comes from the elements it
  contains. An element whose own sequences marked nothing has a content rectangle whenever what it
  contains has one — a `TD` whose only content is a widget annotation, a `Div` around a `Figure` that
  states a `/BBox`.

**Bullet two was called a layout engine's for a true reason that had expired.** It is a layout
engine's while the operands are: a processor reflowing a file computes its children's heights and
sums them. After ADR 0486 the operands are rectangles *this* program has already derived, and
summing them is arithmetic rather than layout. That is the same correction ADR 0486 made one bullet
over, arriving on schedule.

## What is a choice, and it is one thing

The standard gives a block-level container's **height** from the elements it contains and its width
from the reference area — which a viewer does not lay out, having been handed the result. So the
union in both directions is this program's decision, and it is written down as one: it errs in the
direction the rest of this rectangle already errs in (ADR 0486: "it is a bound, never an
underestimate"), and a rectangle narrower than the content would point a magnifier at part of an
element.

Two smaller things are decided rather than derived, and both were already this tree's practice:

- **`/LineHeight` is not read.** Bullet three derives a text element's rectangle from the line height
  and the summed character widths; this program holds the boxes those produce and takes them. A file
  whose attribute disagrees with what it drew is answered by what it drew, which is ADR 0301's
  argument.
- **The enclosure is asked last**, after the element's own marks and after the producer's own
  rectangle. A derivation from below is the weakest of the three statements and must not displace
  either of the others.

## What it does not change

**The residue is still an answer.** An element enclosing nothing that was drawn, stated or placed
still has no rectangle, and no clause derives one from nothing. 1082 of the corpus's elements are
that shape and 253 of them are not — the difference is exactly the elements whose descendants are
placed by a route that does not propagate through the marks.

**An untagged page is still untagged.** Nothing here invents an element or a rectangle for one: the
enclosure route unions the places of elements that already exist, and a page with no structure tree
has none.

## Where it lives, and why it is one function

`viewer_core::places` — a free function over one page's answer, not a field on
`AccessibilityNode` and not a method.

- **Not a field**, because a host can compute it: the parents are indices into the same slice and
  every rectangle is already there. `doc/ui-boundary.md`'s test for what crosses is what a host
  cannot work out, and this is not that.
- **Not each consumer's own arithmetic**, which is what it was. The precedence was written twice —
  in `viewer_accessibility::tree::place`, which is what a screen reader is told, and again in
  `viewer-core`'s accessibility census, which is what says whether that is any good. Two copies of a
  precedence is how an instrument comes to price a program nobody runs, and the census's copy would
  not have moved with this change at all.

`viewer_accessibility::tree` calls it once per page rather than once per node, because the third
route is a walk of the whole answer.

## The population, measured before either half was built

`pdf-model --example element_bounds_census`, extended this round with the two counts that decide it,
over the pdf.js corpus, `doc/corpora/` and `doc/`:

```sh
cargo run --release -p pdf-model --example element_bounds_census -- \
  $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') $(find -L doc/corpora -name '*.pdf') doc/*.pdf
```

| | |
|---|---|
| documents read / with a structure tree | 1245 / 153 |
| structure elements | 166 724 |
| elements enclosing **both** text and other marks | 119 530 |
| of those, whose marks reach outside their text | **3032** |
| of those, by more than a tenth of the area | **1885** |
| elements no route places | 1082 |
| of those, **enclosing an element some route places** | **253** |

By role, the 253 are 195 `Div`, 23 `P`, 18 `TD`, 6 `TR`, 4 `Sect`, 3 `Document`, 2 `Part`, 1 `Art`,
1 `Figure` — containers, which is what the clause's second bullet is about. The documents that
carry them are the form-shaped ones: `prefilled_f1040.pdf` 205, `annotation-text-widget.pdf` 10,
`annotation-choice-widget.pdf` 8, `smaskdim.pdf` 8.

The mixed-element half is largest on the standards themselves: `ISO-14289-2-2024` 743 elements (534
widely), `ISO_32000-2_sponsored_EC3` 557 (332), `ISO-TS-32004-2024` 277 (216). By role over the whole
population it is 1195 `P`, 316 `Span`, 230 `TD`, 223 `LI`, 221 `LBody`, 133 `Link`, 116 `TR`, 77
`Figure`.

**Two figures moved since ADR 0486 for a reason that is not this change**, and re-deriving them is
`doc/habits.md`'s rule paying off: that ADR counted 349 elements placed by their own marks alone and
1021 by nothing, and this run reads 288 and 1082 over an identical 1245/153/166 724. The 61 are
ADR 0488's — the census example matches a sequence per content stream, as §14.7.5.2 requires, and
`viewer-core` recovers 61 elements of two documents through the one-other-stream inference that
example does not implement.

## What it costs, and what the census says

The accessibility census (`doc/todo/02` §2's line, a ratchet since ADR 0425) over its 988 documents:
**elements with no place, 1091**, and **245** placed by the new route, under a name of its own —
`placed by §14.8.5.4.5's derivation from what they enclose, and by nothing else`. The two are the
before and after of one run rather than two runs, and the arithmetic is exact by construction: the
route is asked only of an element with no quadrilaterals, no marks and no stated rectangle, so every
element it places is one the old condition counted as placeless. 1091 + 245 = **1336 without the
change**, which is the figure trap 16's table recorded independently when it held the census's
binary fixed. Every floor
held to the unit: elements reached 102 853, placed 7538, placed by their own marks 93 267, cells with
headers 16 617, controls 272, a caret's 57 116 elements and 2 974 185 characters. **The new count is
printed and not ratcheted**, on `doc/todo/05`'s rule that a number enters a gate once it has held
across rounds.

The mixed-element half moves no count in that census at all, and that is correct rather than
disappointing: it changes *which* rectangle 3032 elements answer with, and the census counts whether
an element has one. `element_bounds_census` is the instrument that prices it, which is what
`doc/todo/31` said it would be.

The arithmetic is one reverse pass over the page's own answer, and the answer is bounded at 8192
nodes.

## The second track: the head of the contradicted list keeps its verdict

`rank_the_contradicted`'s head is `bitmap-symbol-context-reuse.pdf` page 1 at 28.91 bounds from its
nearest reference, failing the **worst tile** at 28.91×. It is diagnosed —
`CONTRADICTED_SHARED_JBIG2_DECODER`, ADRs 0499 and 0513 — and this round re-derived it rather than
taking it on trust. The note carries the whole of what follows; two things are worth having here.

**The rule that would move the page was written, tested and reverted.** Where *no* reference drew
marks and the flat sheets disagree with each other, none of them is a reading of the page: one page
has one appearance, a white sheet and a black sheet are not it, so a consensus made of two of them is
two failures agreeing at a spread of zero. The argument is not circular — the disagreement is the
evidence and our own render never enters it — and it is a *different* predicate rather than a looser
one, which is what ADR 0513 thought the case needed. What refutes it is `pdfref`'s own test suite:
`a_two_of_three_majority_forms_the_consensus` and
`references_disagreeing_among_themselves_is_not_our_failure` are two uniform white rasters against a
uniform black one, which is this page's shape exactly. **A genuinely blank page with one broken
renderer and a page nobody decoded with one broken renderer have the same rasters**, and a predicate
that fires on both would forgive a render of ours that painted marks on an empty sheet.

**What would separate them is the renderer's own words, and the harness throws them away.** All three
logs say *failed to decode*; `Reference::render` captures both streams into the work directory, but
`cache::render` returns on a hit without calling it, so on a run with a 100% hit rate the verdict is
taken from rasters while the diagnosis comes from files an earlier run happened to leave behind.
Remembering the log beside the raster would let an abstention rest on a renderer's testimony instead
of on its pixels. It costs a `cache::FORMAT` bump — every cached entry re-rendered once, 6707 on this
corpus, about a thousand seconds — and it is the one route this page has left.

**One fact this round added to the note**: `hayro`'s raster is byte-identical to ours here, and it is
not a fourth reading. `pdf-sandbox` decodes §7.4.7 through `hayro-jbig2`, so the two rasters are one
decoder run twice — trap 9's standing rule about `hayro` in a sharper form than a font page's. What
does establish our decode is ADR 0381's argument, which uses no renderer as truth: the `bitmap-*`
family is one drawing through nearly every path ISO/IEC 14492 defines, and this tree returns one
image where `jbig2dec` returns six.
