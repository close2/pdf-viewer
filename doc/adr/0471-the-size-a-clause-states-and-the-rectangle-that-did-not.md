# ADR 0471 — The size a clause states, and the rectangle that did not

Status: accepted, 2026-08-21. Session 640. Takes `doc/todo/03` §25's named successor — the next
chunk of the SafeDocs crawl — and settles the lead that section left open. Amends §12.5.6.4's and
§12.5.3's ledger rows.

## The chunk, and why these archives

§25 leaves "13 944 crawled documents unranked … in archive-sized pieces". This round took **ten
whole archives — `2100`, `3990`, `4100`, `4605`, `6081`, `6100`, `6942`, `7065`, `7188` and
`7434`, 10 000 documents** — none of the fifty-two archives sessions 603, 613, 615, 619, 625, 631
and 636 ranked, and every remaining archive of a thousand but two. *Which* archives is immaterial
and that is ADR 0261's finding: the crawl is sorted by SHA-256 and cut into equal pieces, so an
archive is a hash bucket and any set of them is an unbiased sample.

The instrument is 603's, unchanged and reused: page one at 72 dpi from this tree and from
`pdftoppm`, `mutool` and `gs`, every invocation copied from `tools/pdfref/src/reference.rs` and
explicit about the page box (trap 3), ranked by our ink minus the lightest **live** reference's,
with each panel's raster size beside the number. **14 minutes 12 seconds for the ten thousand** at
fourteen workers; **9957 of the 10 000 produce a number and 43 do not**.

Checked before it was trusted, which is 619's and 624's lesson taken rather than re-learnt: both
binaries built (`--example render_at` *and* `-p pdf-sandbox --bins`), `target/release/examples/`
confirmed to hold no worker of its own, and §20's check run as the round's first command —
**31 checked, 0 absent, 31 rows, green**. Six documents named by earlier chunks were re-measured
before anything was read and reproduce to the thousandth.

## The head, at both ends, and what it says about the crawl

**The negative head is the shallowest any chunk has produced, for the second round running.**
Deepest row **−8.860**, against 636's −10.174, 631's −43.503, 625's −112.626, 619's −84.152 and
613's −20.341. That is a sequence and it is worth naming as one: five of the eight chunks put a
misread clause at the top of the ranking, and this one does not.

**What is at the top instead is this tree's own scan conversion, documented and priced.** Read
from the artefacts rather than from the label, three of the four deepest silent rows are one of the
departures `doc/todo/_scan-conversion.md` states:

- **`6942935.pdf` −8.691**, a hymn sheet whose staff lines and bar lines are drawn by the producer
  as **twelve abutting strokes of `.06 w` each**, offset `.06` apart. Twelve marks in one device
  pixel column unite by §11.3.7.3's "inverted multiplication" to `1 − 0.94¹²` = 0.52 where their
  area is 0.72, so the rules read grey where all three references draw them black. That is
  `doc/todo/11` item 5 exactly — ADR 0308's conflation — on a document nobody had to construct.
- **`7434231.pdf` −2.271**, with the three references inside **0.018** of each other, is the same
  subclause's first departure on a plain sub-pixel rule: a TeX double box drawn at about half
  coverage where §10.7.4's literal rule paints the whole pixel.
- **`6081615.pdf` −4.127** and **`4100532.pdf` +7.452** are departure 3, `Image::area_averaged`, on
  bilevel scans downsampled to page size — one lighter than the references and one heavier, which
  is what an averaging filter does against a decimating one in either direction.
- **`4100873.pdf` −7.922** and **`7188835.pdf` −5.129** are trap 9's family with the evidence
  beside them: four-component `ICCBased` `DCTDecode` photographs, the three references inside 0.3
  of one another and one colour library between them. The file even states
  `/Intent /RelativeColorimetric`, which is Table 51's default and ours.

**The positive head above +16 is 613's note and nothing else** — `poppler` alone at 0.1 to 4.5
while `mutool`, `gs` and this tree agree within 3 — read out of
`doc/traps/oracle-and-references.md` rather than derived again. The one row that is not is
**`7188579.pdf` +19.856**, and it is the opposite of a defect: a linearised file that states
`/L 2236960` and is 310 952 bytes long, which `poppler` reconstructs to a 1×1 raster and `mutool`
and `gs` refuse outright, while this tree draws the part of the scan the file carries and reports
the shortfall in §7.3.8.2's own words.

## The second instrument, because ink had gone quiet

A ranking by ink cannot see a page that is wrong in a way that costs no levels, and after the head
above it was worth asking the ten thousand a different question: **what does this tree report?**
`examples/open_one` over all of them, page one — **101 documents of 10 000 report anything at
all**, and two of those hold nine tenths of the reports. Both are damaged files where the three
references do worse than we do: `7188417.pdf` is the truncated linearised file above, whose 1002
`/ImNNNN is not a stream` reports are the missing half of the file; `4605705.pdf` states eight
`/Contents` parts, every one of them a Flate stream that decodes cleanly for tens of kilobytes and
then into garbage, with `poppler` refusing it and `mutool` and `gs` disagreeing with each other by
78 levels of 255 about what it says.

**One thing found there is worth a successor and is not taken here**: on that garbage,
`render-cpu` refuses the **whole raster** for one command — `page_to_path` cannot invert a singular
transform, so `CpuRasterError::UnsupportedPaint` propagates out of `rasterize` and the 293
commands that did draw are lost with it. Whether a paint that cannot be positioned should cost its
own mark or the page is a question about `Rasterizer`'s contract rather than about this document,
and it is recorded in `doc/todo/11` rather than answered on a file whose content stream is noise.

## The defect: an icon that took its size from a rectangle that does not state one

**`1407194.pdf` −6.304**, seven commands and silent — the lead §25 named and left open. A book
cover, and over the top-left quarter of it a pale yellow note **250 units square**. The annotation
is `<< /Subtype /Text /Border [0 0 0] /C [1 1 0.5] /Rect [0 542 400 792] /Contents (…) >>`: no
`/AP`, no `/Name`, so Table 175's default `Note`, and this tree inscribed its icon in the largest
square that fits inside a 400 × 250 rectangle. All three references draw a small note at the
rectangle's upper-left corner and agree within 0.6.

### What the clauses say

§12.5.6.4 opens with the sentence the tree already quotes:

> A text annotation represents a "sticky note" attached to a point in the PDF document. When
> closed, the annotation shall appear as an icon

and gives the size in the next but one:

> Text annotations shall not scale and rotate with the page; they shall behave as if the NoZoom
> and NoRotate annotation flags (see "Table 167 -Annotation flags") were always set.

§12.5.3 says what that flag means:

> If the NoZoom flag is set, the annotation shall always maintain the same fixed size on the
> screen and shall be unaffected by the magnification level at which the page itself is displayed.

A size fixed **on the screen** is by definition not `/Rect`'s, because `/Rect` is stated in default
user space and every extent in that space scales with the magnification. And Table 166 does not
make `/Rect` a size either — its whole row is "The annotation rectangle , defining the location of
the annotation on the page in default user space units". What turns `/Rect` into a size is §12.5.5,
and only for an annotation that has a stored appearance: the algorithm there "shall be used to map
from the coordinate system of the appearance XObject … to the annotation's rectangle in default
user space", step 2 computing a matrix that "scales and translates the transformed appearance box
to align with the edges of the annotation's rectangle". An annotation with no `/AP` states no such
box, so there is nothing for that algorithm to scale, and the subclause's own sentences are all
that is left.

The corner is Table 167's, in the `NoZoom` row: "The location of the annotation on the page
(defined by the upper-left corner of its annotation rectangle) shall remain fixed, regardless of
the page magnification."

### What was wrong with the code, which is narrower than it looks

`annotation::anchored_icon` had made **exactly this derivation** in the two-hundred-and-sixty-fifth
session, for `rc_annotation.pdf`, and its own doc comment said "a fixed size, which is by
definition not `/Rect`'s" — and then applied it under `if subtype != b"Text" || !is_empty(rect)`.
The condition is the whole defect. None of the three sentences above mentions the rectangle's area;
the second half of that `if` was the witness that was available in the four-hundred-and-something
corpus, not a reading of anything. **A derivation and the condition it is written under are two
separate claims, and only the first was checked.**

The fix is the removal of `|| !is_empty(rect)`. Every text annotation with no stored appearance now
gets the same twenty-unit square hung from `/Rect`'s upper-left corner, degenerate rectangle or
four-hundred-unit one. **How big** is still a choice and still twenty units, recorded beside
`crate::icon`'s artwork as the same kind of invention.

**§12.5.6.15's file attachment and §12.5.6.16's sound are deliberately untouched**, and that is
what keeps this from being a rule about icons in general: neither clause states either sentence —
neither annotation is attached to a point, neither is held at a fixed size — so
`appearance::symbol_icon` still inscribes those in `/Rect`, and the test that used to pin the
inscribing behaviour through a *text* annotation now pins it through a file attachment, which is
the subtype whose clause actually states it.

## The erratum, which says the size is ours to choose

`spec-errata emit` over all fourteen documents before writing, which is `doc/errata-read.md`'s
standing rule. **§12.5.6.4 carries no annotation at all and §12.5.5 carries none**; §12.5.3 carries
**Issue #34, `Review/Completed`**, and one half of it had never been read here — a pure addition,
which `check` cannot see because it compares the tree's quotations against *struck* passages:

> When an appearance dictionary is not present, the rendered appearance will be implementation
> dependent.

The other half of the same issue — the struck "without regard to any other keys", which had `/BM`
ignored on every stored appearance stream — was read in the four-hundred-and-eighteenth session and
already has a row, and that is why this half went unnoticed for two hundred and twenty rounds: **an
issue number that already carries a verdict stops being looked at.** What the sentence settles is
not a behaviour but a direction of inference. Everything this tree constructs for an annotation
with no `/AP` — the seven icons, their size, §12.5.6.8's inscribed rectangle — has rested on
principle 5's "where the specification genuinely defines nothing, make a documented choice". This
sentence is the standard saying so itself, for the whole population at once.

## The population, measured before the change

Trap 11's rule, with an instrument that is not this tree's (trap 8): a hand-written scanner over
each file's own bytes and over every Flate stream in it, tracking `<< >>` depth while skipping
`(…)` strings, `<…>` hex strings and `%` comments.

**Over 67 193 files — the whole crawl, `doc/corpora` and `doc/pdf.js` — there are 185 `/Text`
annotation records in 67 documents; 80 of them in 18 documents state no `/AP`; and 7 in 6 documents
state no `/AP` and a `/Rect` with a side over twenty units.** The six are `1407194.pdf` and
`6573247.pdf` (both `[0 542 400 792]`, one producer), `7557734.pdf` (105.81 × 128.25),
`2100517.pdf` (100 × 100), `6696835.pdf` (28.35 × 19.84) and `1038997.pdf` (284 × 0, which the old
condition already caught because its area is zero). **The curated corpora carry not one**: every
`/Text` annotation in `doc/pdf.js` and `doc/corpora` either states an `/AP` or is
`rc_annotation.pdf`'s degenerate rectangle, so no gate in this tree could show this defect and none
moves because of the fix.

The other 73 records with no `/AP` state rectangles at or under twenty units, and they move too —
each icon grows to the stated size — which is why the reach below is measured rather than reasoned
from the six.

**The instrument was itself wrong twice before it was believed**, which is trap 1 one directory
over and is why this paragraph exists. Its first version matched dictionaries with a regular
expression and missed `rc_annotation.pdf`, whose `/RC` entry holds `<p>Hello World!</p>`; its
second blanked stream data with a pattern that also matched the `stream` inside `endstream`, and
blanked every object between two streams — which lost `pr12564.pdf`'s thirteen annotations. Both
were caught by asking the census for documents whose answer was already known.

## The reach, measured over our own panel

631's rule, and it is not a preference: a reference renderer's panel cannot depend on our build and
has been measured to differ between two runs of the same tree, so a reference-side comparison
invents movement. The panel is **62 009 documents** — all sixty-two archives any chunk round has
ranked, plus every document `doc/checks/fixed-documents.toml` names and every document the
annotation census reached — rendered by this tree before and after, with the fix reverted by patch
for the first pass and re-applied for the second.

**Four rows of the 62 009 differ, and no row differs for any other reason** — the machine was
quiet enough that no render lost a budget, which is the failure mode 631 measured and 633 met.

| document | ours before → after | references | gap before → after |
|---|---|---|---|
| `1407/1407194.pdf` | 39.468 → 45.804 | 46.307 / 45.932 / 45.772 | **−6.304 → +0.032** |
| `6573/6573247.pdf` | 11.241 → 2.805 | 2.995 / 3.010 / 2.977 | **+8.264 → −0.172** |
| `7557/7557734.pdf` | 27.993 → 27.414 | 27.855 / 27.389 / 28.651 | +0.604 → +0.025 |
| `2145/2145632.pdf` | 141.6065 → 141.6074 | 141.963 / 138.047 / 141.558 | +3.559 → +3.560 |

**Three of the four are in archives an earlier chunk took** — `1407` is 636's and it is §25's own
open lead, `6573` and `2145` are 631's — which is the **eighth round running** that a fix has
reached back into an earlier chunk. The fourth is in an archive no chunk has ranked and reached the
panel only because the census named it.

`6573247.pdf` is the sharper of the two visible ones and was on the **positive** side: the same
producer's note over a nearly blank page, where 250 units of pale yellow was most of the ink, at
+8.264 against three references agreeing within 0.04. It is a row 631 did not name because it sat
well below that chunk's head.

**`2145632.pdf` is the only one that does not move toward the lightest reference**, by nine
ten-thousandths of a level. Its twenty-seven text annotations state no `/AP` and rectangles at or
under twenty units, so each icon grows a little; the lightest reference there is `mutool` at 138.047
against `poppler`'s 141.963 and `gs`'s 141.558, and this tree sits between the other two. It is the
row that says what the change costs where a rectangle was already about the right size: nothing
that can be seen.

**`2100517.pdf` does not move at all**, and the reason is worth recording because the census named
it as a witness: its `/Text` annotation with `/Rect [0 0 100 100]` is object 2, and no page in the
file has an `/Annots` array at all — so it was never drawn, before or after.

**`6696835.pdf` does not move either**, and its reason is the third different one: its four notes
each state `/CA 0`, which Table 166 makes the opacity of "all visible elements of the annotation in
its closed state", so they draw nothing at any size. Three witnesses out of six that the census
named turn out not to reach the page at all — which is the difference between a population and a
reach, and the reason the second is measured rather than inferred from the first.

## What is not taken

- **`doc/todo/11` item 5's conflation**, witnessed again by `6942935.pdf` and priced in that
  section: the cure is a conflation-free rasteriser or an *N*× box filter, and both cost their most
  on the frame time-to-first-page is measured on.
- **A singular transform costing the page rather than the mark**, above.
- **The `/Text` annotations whose `/Rect` is between one and twenty units.** The clause's answer is
  the same — the size is not `/Rect`'s — and the code now gives them the twenty-unit square too.
  What is *not* claimed is that twenty is right; it is the same invention it always was, and the
  erratum above is the standard agreeing that it is an invention.
