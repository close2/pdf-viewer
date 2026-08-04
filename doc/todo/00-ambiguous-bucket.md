# Empty the oracle's ambiguous bucket

Status: **standing task**, since the hundred-and-seventy-sixth session. **87 pages left**, from 754.
Priority: 00 — the last large population where a defect can live without a name
Corpus: 786 ambiguous pages (749 on documents we call complete); 662 diagnosed, 87 held by name
Code: `crates/pdf-model/tests/oracle.rs`, `crates/pdf-model/tests/ambiguous_undiagnosed.txt`

## Why this is work rather than a caveat

`ambiguous` means "no two references agree closely enough for anybody to be called wrong". It is
the right verdict for the *ratchet* to reach and it is not the same as "right". `issue7406.pdf`
drew a JPEG cyan-on-black inside an `ambiguous` verdict for as long as anybody looked, and it is
correct now, and **nothing announced either event**. That is the bucket in one sentence:
unwatched in both directions. The project owner's judgement in the hundred-and-seventy-fifth
session is that the tree is far enough along for this to be the work.

## The instrument, in three parts

- **`AMBIGUOUS_*` groups** — a page with a written diagnosis, held by name, exactly the shape the
  contradicted list has had since the sixth session. A name that stops being ambiguous fails the
  build, because a diagnosis that outlives what it diagnosed is this project's oldest failure.
- **`tests/ambiguous_undiagnosed.txt`** — the rest by name, `include_str!`d and held to equality.
  A page arriving in it *used to agree*, which is the regression nobody could see before; a page
  leaving it has been fixed or diagnosed. Data rather than a `const` because the argument for
  each name is that there is not one yet.
- **A ranking the gate prints itself**, of the ten undiagnosed pages we sit furthest from the
  **nearest** reference on. Not the furthest: the printed per-page number is our distance from
  the *worst* reference, and on nineteen JBIG2 pages that is a `mupdf` which drew a black
  rectangle. `Distance::nearest` is the number that accuses us; `Distance::furthest` beside it
  says whether the references are the ones disagreeing.

## How to take one

1. **Read the ranking**, and prefer a page whose two numbers are close — that is the shape that
   says *we* are alone. `issue7229.pdf` sat at 77 from the nearest with the two nearly equal,
   and it was drawing the wrong page.
2. **Read the file's bibliography before opening anything.** Every pdf.js fixture is named after
   the issue that introduced it — `issueNNNN…pdf` → `github.com/mozilla/pdf.js/issues/NNNN`,
   `bugNNNNNNN…pdf` → `bugzilla.mozilla.org/show_bug.cgi?id=NNNNNNN` — and the issue says what
   the file was added to prove. A pair with a common stem is an A/B the corpus built for you
   (`issue7891_bc0` and `issue7891_bc1` differ in `/BC [0 0 0]` against `/BC [1 1 1]` and in
   nothing else). Two cautions: the issue describes **that reader's** defect, and an issue is
   evidence about a *file*, never about the clause.
3. **Open the side-by-side.** `<target>/tmp/oracle/<stem>/p<n>/` already holds our render, each
   reference's, a four-panel strip and a heatmap per reference. The picture has explained every
   page it was pointed at, and twice it named a defect the numbers could not (a shading painted
   as a square; a photograph rendered black).
4. **Ask what the page is made of before measuring anything.** `cargo run --release -p pdf-model
   --example open_one -- <file> 1` prints the command count. **One command has meant one image
   three times running** — `freeculture.pdf`, `issue5747.pdf` and `issue13372.pdf` — and
   `pdfimages -list -f 1 -l 1 <file>` then names it in a second. **Zero commands means a blank
   page reported complete**, which is the worst thing this bucket hides and has been a real
   defect every time (`issue13372.pdf`, `issue8372.pdf`, `issue13316_reduced.pdf`).
5b. **Ask the renderer under test the same question at rising resolution too.** Step 6 uses a
   *reference* at 8× to find the geometry; running `cargo run --release -p pdf-model --example
   render_at -- <file> <page> <scale> <out.png>` and measuring the same way says whether the
   difference is our scan conversion or our *shapes*. `bug1538111.pdf` is where it paid: our ink
   is 1.48 at 1×, 4× and 16× while `poppler`'s limit is 2.24, so the two draw different marks and
   no amount of anti-aliasing argument was going to explain it. (They are markup annotations
   whose artwork §12.5.6.10 does not state at all.)

5. **Measure with a closed form where the clause states one**, and with pairwise distances only
   as corroboration. The ink is

   ```sh
   magick <png> -alpha off -channel R -colorspace Gray -format "%[fx:(1-mean)*255]" info:
   ```

   and `magick compare -metric MAE a.png b.png null:` is the pairwise number.

   **`-alpha off` is not optional and this file said so too late.** Our renders and `hayro`'s
   carry an alpha channel; `poppler`'s, `mupdf`'s and `ghostscript`'s do not. Without it
   `-colorspace Gray` averages alpha in as a second channel and returns **exactly half** the ink
   — so a comparison between our panel and a reference's compares half of one number with all of
   another, and the two renderers that "agree" with us are the two whose *file format* matches
   ours. Session 161 found this and wrote it in `CONTRADICTED_GLYPH_EDGES` and in the handover's
   Habits; the recipe here was not corrected, and the two-hundred-and-second session followed the
   recipe and drew two wrong conclusions from it (ADR 0163). **A lesson recorded in the place it
   was learned and not in the place it is used has not been recorded.**

6. **Where the difference is scan conversion, the closed form is the same page at eight times the
   resolution.** Ink is a geometric quantity and a renderer's departure from it shrinks as the
   pixels do, so `pdftoppm -cropbox -r 576` measures what the page's marks actually cover — no
   reference is being trusted, because the same renderer is being asked at two resolutions and
   only the *limit* is used. `bug1799927.pdf` is where this paid: at 72 dpi the five renderers
   span 5.94 to 13.40 and the limit is 10.8, which says which of them is measuring area.

   **`-cropbox` is not optional either, and it is `-alpha off`'s twin.** `pdftoppm` renders the
   **`/MediaBox`** by default; the oracle, `mutool draw` and this tree all render the
   **`/CropBox`**. On a document where the two differ the comparison is between two different
   pages, and the ink is wrong by the ratio of their areas — on `freeculture.pdf` that is 1.378,
   so a ladder taken without the flag put `poppler` at 9.10 against our 12.18 and would have
   manufactured a 34% defect on four pages that agree to **0.03 of 255** (the
   two-hundred-and-thirty-third session, `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`). The tell is the
   raster's size: `magick identify` every panel before believing any number, and if the
   dimensions differ the measurement has not started yet. `mutool draw -r N` needs no flag.

   **And the assumption inside it is checkable, because it has failed once.** The step assumes
   the reference *converges on the geometry* as the pixels shrink. On `issue2177.pdf`, a page of
   §8.7.3 tiling patterns, `poppler` goes 34.15 → 18.03 → 16.32 from 72 to 2304 dpi — its
   strokes get thinner rather than its edges getting sharper — and taking its limit would have
   said all five renderers paint two to three times the geometry. Ours is flat at 36.75, 37.37,
   37.25, 37.20 across four scales and `mupdf` at 8× is 37.20, which is the answer.
   **Take a second renderer's ladder, or ours beside one**: a limit is only a limit if the thing
   taking it is converging, and one ladder cannot tell convergence from drift.

## 7. Sweep the whole bucket for the one defect a distance cannot name

Steps 1 to 6 take a page at a time off a ranking. **The ranking cannot see missing content**,
because a page that draws less than everybody is not necessarily far from anybody: `issue19634.pdf`
sat at 0.85 with a quarter of its marks absent. What sees it is one number over the artefacts
already on disk — **our ink minus the lightest reference's** — and the sweep costs three minutes
because nothing has to be rendered again:

```python
# for every ambiguous page, over <target>/tmp/oracle/<stem>/p<n>/
live = [ink(r) for r in (poppler, mupdf, ghostscript, hayro) if ink(r) > 0]
gap  = ink(ours) - min(live)
```

sorted ascending. A large negative gap is content we are not drawing; a large positive one is
content nobody else is.

**Three corrections to that loop, all from the two-hundred-and-sixty-fifth and -sixth sessions,
and each of them changed what the sweep found:**

1. **Drop a reference that drew nothing before taking the minimum.** A blank is not a lower bound
   on the geometry, and leaving it in turns another program's failure into our surplus: four pages
   came back at +21 to +29 of 255 because `mupdf` draws nothing on `colorspace_sin.pdf`, `_cos`
   and `_atan`, and `hayro` nothing on `issue2840.pdf`. What the *positive* side is good for is
   exactly that — finding a reference that failed. Thirty-five such pairs across the bucket, most
   of them `ghostscript` on the JBIG2 fixtures.
2. **Run it over every ambiguous page and not only the undiagnosed ones.** Diagnosing a population
   takes its pages off `ambiguous_undiagnosed.txt`, and if the sweep reads that file then
   diagnosing 364 pages in one session removes 364 pages from the only instrument that sees
   content this tree is not drawing. The list of names to sweep is the gate's own output — every
   line it prints as `ambiguous` — which is 787 rather than 100.
3. **Read the result beside the corpus's incomplete list.** A page this tree *reports* is expected
   to be light: drawing less ink is what the report says, made visible.

**The full run, over all 787: twenty names at or past −1, seventeen of them documents this tree
already calls incomplete**, and the other three diagnosed and consistent with their diagnoses —
`issue16038.pdf` at −6.70 (`AMBIGUOUS_TILING_CELL_CLIP`, whose own note measures the interior
coverage 13% short), `issue12295.pdf` at −1.71 (`AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY`, where
every renderer paints more than the geometry and ours least, so a negative gap is the finding
rather than a defect) and `issue7821.pdf` at −1.00 (`AMBIGUOUS_GRADIENT_QUANTISATION`). **Nothing
unexplained anywhere in the bucket**, which is also the check the two long books' population
argument needed.

**Re-run in the two-hundred-and-sixty-fifth over the tail, and it produced a defect** —
`rc_annotation.pdf` page 1 at **−1.783 of 255**, past the −1 this file names as the alarm. The
page is one text annotation with `/Rect [50 50 50 50]`, this tree drew **nothing** for it, and
§12.5.6.4 says a text annotation is "attached to a point" and "shall appear as an icon". It sat at
0.73 from the nearest reference — a nearly blank page resembles a nearly blank page — so no
ranking would ever have produced it. **This is the instrument's first positive result and the
reason it exists.**

**And the sweep itself had a defect the same run exposed**: `min` over the references includes a
reference that drew *nothing*, and a blank is not a lower bound on the geometry. Four pages came
back at +21 to +29 of 255 — `mupdf` draws nothing on `colorspace_sin.pdf`, `_cos` and `_atan`, and
`hayro` nothing on `issue2840.pdf`. Drop a zero-ink reference before taking the minimum; what the
positive side is *good* for is finding a reference that failed.

**Run in the two-hundred-and-fortieth session over all 493 names it produced a negative result,
and the negative result is the finding**: the whole bucket lies between **−0.84 and +0.42 of
255** of every reference. After ADR 0173 and 0174 there is no ambiguous page left where this tree
draws materially less than the lightest of four other renderers. That is the class of defect the
bucket was most likely to be hiding — `issue19634.pdf` was −4.76 before ADR 0173 — and it has
been swept for.

What the sweep's own head is worth reading anyway, because a small gap can still be a clause:
`jpx_smaskindata.pdf` at −0.84 (`AMBIGUOUS_MATTE_WITHOUT_A_SOFT_MASK_IMAGE`), `issue16473.pdf` at
−0.72, `issue7454.pdf` at −0.15 but with the *references* spread over 9.3, and `bug1308536.pdf`
at +0.42.

**Re-run it after any round that changes what gets drawn**, and expect it to stay empty; a name
appearing at −1 or beyond is a regression no other gate would report as one.

**Run in the two-hundred-and-ninety-first**, after three rounds that changed pixels — a `Tf`
naming `/Helvetica` (ADR 0183), a written `/Differences` (0184), §9.6.5.2's `.notdef` (none, as it
turned out). All 786 ambiguous pages, and **correction 3 is worth doing inside the loop rather
than beside it**: filtering the corpus's incomplete list out first turns two lists into one, and
what is left is the only list that can hold a surprise.

```text
on documents we report (10 of the 12 largest gaps)   −19.4 to −6.0, every one of them
                                                     "a substitute cannot be addressed (§9.10.2)"
on documents we call complete, 742 pages:
  −6.700  issue16038.pdf p1        AMBIGUOUS_TILING_CELL_CLIP, 13% short by its own note
  −1.712  issue12295.pdf p1        AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY
  −1.000  issue7821.pdf p1         AMBIGUOUS_GRADIENT_QUANTISATION
  −0.840  jpx_smaskindata.pdf p1   AMBIGUOUS_MATTE_WITHOUT_A_SOFT_MASK_IMAGE
  −0.717  issue16473.pdf p1
  −0.535  blendmode.pdf p1   −0.470  issue7339_reduced.pdf p1   then nothing past −0.29
```

**Four names past −0.7 and all four already diagnosed**; the alarm at −1 holds. The negative head
of the *unfiltered* run is entirely `doc/todo/21` item 2's population — composite fonts naming an
`Identity` ordering, which report and draw nothing — and that is the sweep working rather than a
finding: a page this tree reports is expected to be light.

The positive side did its job too: `bug920426.pdf` at **+21.07** is one reference drawing a row of
empty boxes where we and the other three draw *Checkliste Service*.

## What a group must say

**`ambiguous` is the gate's verdict and never the answer.** The owner put it in one sentence:
*even if the oracle cannot agree, we should be able to determine what is actually true, based on
the spec.* A group whose whole argument is "we sit inside their spread" has answered the easy
question. Every group must say **what the specification determines**, and there are three
shapes, all of them findings:

- **The clause determines it and we can be checked against it.**
  `AMBIGUOUS_SHARED_JBIG2_DECODER`: ISO/IEC 14492 defines the decoding exactly, and
  `tests/jbig2.rs` checks us against the corpus's own invariant — ninety-six encodings of one
  image, byte-identical — with no reference involved at all.
- **The clause determines that everyone here is departing from it.**
  `AMBIGUOUS_IMAGE_REDUCTION`: §10.7.4 says "there shall not be averaging over the pixel area",
  all five renderers average, ours is ADR 0025's documented departure. The finding is the
  departure; the spread is corroboration.
- **The clause puts the answer beyond itself, and says so.**
  `AMBIGUOUS_DEVICE_CMYK_CONVERSION`: §10.4.2.1 ranks two answers, §10.3.1 makes the destination
  profile "beyond the scope of this document", and its NOTE names "assumptions made by the PDF
  processor software". Say which clause leaves it open, and name the assumption this tree makes.

A fourth shape is not acceptable: a group that names no clause. And a group may say **"we are
wrong"** — `AMBIGUOUS_ZERO_AREA_FILL` did, for two sessions, before the fix.

## What has come out of it, so far

Ten sessions from the hundred-and-seventy-sixth, then twenty more: **ten defects found, eight
of them fixed** — a page one that was page two (ADR 0148), a photograph rendered black (0149), a
shading painted as a square (0150), a stencil that drew nothing (0151), a whole grid that
disappeared (0154), a sentence drawn as one Greek letter (0158), a stamp's gradient painted flat
(0160), and two coverage losses that moved the oracle's own headline (0165: a `/BBox` clip on a
widget border's own edge, and a miter bound on a comb field's separators). The ninth was found and not fixed for
twenty-six sessions and is fixed now: §8.7.4.5.4's greatest admissible root, which every backend
got wrong from the same place because every gradient library gets it wrong the same way
(ADR 0171).

Beside them: the ten documents whose substituted font drew none of its characters in silence
(0152), the coverage rule that made eight of them draw (0153), a pattern cell's clip worth 15% of
a page's ink (0155), a font program that draws nothing now saying so (0157), **thirteen JPEG 2000
codestreams that decode to the wrong samples** (0161, `doc/JPEG2000_FEEDBACK.md`), and a
measuring command that had been halving our own ink for two sessions (0163).

The tenth is found and not fixed either: a stroke under a pixel wide loses the half of
`tiny-skia`'s hairline smear that falls outside the raster's top edge, which is `doc/todo/11`
item 3 and was found by a synthetic ladder rather than by a reference.

The bucket itself went 754 → 704 undiagnosed, and that is the least interesting number in this
file. *Nine defects nobody could see* is the one to watch — **and one gate that found thirteen
more.** `jp2k-resetprob.pdf` sat at the top of the ranking with a name that named its own
hypothesis, and checking the hypothesis meant building `tests/jpeg2000.rs`: every corpus
`JPXDecode` stream against ISO/IEC 15444-5's reference software. It ruled the codec out for that
file — the decode is byte-identical — and found thirteen of the other twenty-nine codestreams
wrong (ADR 0161). **A page on this list is sometimes a question about an instrument that does not
exist yet.**

## Its shape, measured

| | |
|---|---|
| distinct documents the pages come from | ~181 |
| `freeculture.pdf` (309) and `pdkids.pdf` (52) | **361 — two long books** |
| **one paper under twelve names** (`tracemonkey.pdf` and eleven copies) | **154, diagnosed in the two-hundred-and-thirty-third session** |
| documents contributing exactly **one** page | ~154 |

**A quarter of the bucket was one document wearing different names, and nothing said so.**
`tracemonkey.pdf` is pdf.js's canonical fixture and eleven other corpus documents are the same
fourteen pages with an annotation added; `pdftotext` on page 9 gives the same md5 for all of
them. One measurement settled 154 names — and the number to report is *one finding*, not 154,
which is why `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE` says so in its first line. **Before taking a
name off this list, check what else in it is the same file**: `pdftotext -f N -l N | md5sum`
across the documents sharing a page count costs a second and can be worth a hundred names.

**Both books were taken as populations in the two-hundred-and-sixty-second session**, which is
what took the undiagnosed list from 489 to 136 — and the method is the part worth keeping. Six
pages had been measured one at a time over three sessions, all the same way, so the question
stopped being "what is wrong with page 329" and became "is this book one finding or three
hundred". Twelve more pages spread through both books, with two ladders each, put ours within
0.012 of `poppler`'s own limit every time; then the *whole* population's printed metrics were read
as a band, and it is one band with no gaps.

**The two-hundred-and-sixty-third took the next two populations the same way**:
`TAMReview.pdf`'s 22 pages, which are one band (mean 4.05 to 9.96, similarity 0.7722 to 0.9214)
and four ladders inside `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`'s own finding; and `calrgb.pdf`'s
eight, which are the bucket's sharpest instance of shape 3 — §8.6.5.3 defines the components-to-XYZ
arithmetic exactly, the sheet's first page states an identity so the file is naming XYZ values
directly, several of them are outside any gamut, and §10.3.1 says in one sentence that how a
processor gets from there to a pixel "is beyond the scope of this document".

**The band caught the page the sample would have buried.** `freeculture.pdf` page 171 has a worst
tile of 81.57 where nothing else in the book exceeds 29.09: its cartoon is a one-bit stencil that
`ghostscript` thresholds to a black blob, and it belongs to `AMBIGUOUS_IMAGE_REDUCTION`. **A
population argument needs the population's own numbers and not only a sample's** — read the band,
then look at whatever sits outside it.

Two books and a long tail of single pages. **The books are not what this file said they were.**
It read "set in fonts nobody embedded, so each renderer substitutes differently", and `pdffonts`
says `freeculture.pdf` embeds all four of its fonts — nothing substitutes on any of its pages
(the two-hundred-and-twenty-ninth session, `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`). What they are is
dense text at book size, which earns the page the *text* tolerance: 0.90 similarity, measured
over 153 reference-against-reference pairs because five rasterisers cannot agree more closely
than that about small glyphs. The bound is loose for a reason that was measured, not for a reason
about the file. **Take the tail first**: each of those is a file somebody added to a corpus for a
reason, and the reason is written down.

## The next names on the ranking

**The head went in the two-hundred-and-ninety-fifth, and it produced a mechanism this bucket had
not named.** `issue19971.pdf` pages 5 and 6 are one document — a specimen of lists, headings,
paragraphs and four scripts — and they came apart into two findings:

- **Page 6** is 456 commands of text in four scripts and no image at all. Two ladders agree at 8×
  to 0.0055 of 255, ours climbs onto the limit ending 0.025 short, and a four-by-four grid of tile
  means says the residual is spread over every tile in proportion to its ink.
  `AMBIGUOUS_GLYPH_SCAN_CONVERSION`, with no new argument needed.
- **Page 5** is the same text plus one 2500 × 1750 `DCTDecode` photograph in an `ICCBased` space,
  and it is a new group. The two ladders agree to **0.0008 of 255** — the tightest limit this
  bucket has produced — and ours stops 0.155 short, of which the photograph is 57% on 12% of the
  page.

**And step 6's own assumption failed for the second time, in the direction that is a finding.**
The step works because a renderer's departure from the geometry shrinks with the pixels. Rendering
the same page at **16×**, where the image is enlarged rather than reduced, gave a per-channel
difference identical to 8× to three decimal places — so it is neither scan conversion nor the
reduction, and the only two places left are the decoder and the colour space. Decoding the
extracted codestream twice ruled out the first (under 0.2 of 255, mixed in sign, against a uniform
lift six times larger), which leaves `pdf_model::icc` and `lcms` evaluating one 296-byte
matrix-shaper profile — §10.3.1's "beyond the scope of this document", one colour space over from
`AMBIGUOUS_DEVICE_CMYK_CONVERSION`. `AMBIGUOUS_ICC_MATRIX_PROFILE`.

**And the next name down, in the two-hundred-and-ninety-ninth, is the shape a wide ratio is for
and the reason step 3 exists.** `issue19326.pdf` page 1 sat at 0.65 from the nearest reference and
**11.06 from the furthest**. The ink says almost nothing — ours 46.25 against `ghostscript`'s
47.64, which on a page of black letterforms reads as an edge difference — and the picture says
everything: ours, `poppler`, `mupdf` and `hayro` draw the letters *JPX*, and `ghostscript` draws a
band of scrambled blocks with about the same coverage. **A reference that decoded an image wrongly
can have the right amount of ink**, so no metric on that page would have produced it and the
side-by-side did in one look. `AMBIGUOUS_A_REFERENCE_DECODED_THE_IMAGE_WRONG`, with the honest
caveat written into it: `tests/jpeg2000.rs` declines this codestream because it is sixteen-bit, so
the evidence is four decoders agreeing rather than ISO/IEC 15444-5's reference software, and it is
recorded as the weaker kind.

**And four in the three-hundredth, off one document, by this file's own instruction.**
`issue12963.pdf` had four pages on the undiagnosed list and two more already inside
`AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY` — so step 1's ranking was pointing at page 5 while the
answer was already written down two pages over. **Check what else on the list is the same file**
paid four names for one measurement: the two ladders agree to **0.0004 of 255**, the tightest
limit this bucket has produced, on each of the four independently.

**The lesson is about the ladder rather than the page**: a limit that a renderer does not approach
*at all* is not a loose limit, it is a difference in a different quantity, and one more rung is
what tells them apart. The two-hundred-and-sixteenth found the same step failing the other way, on
a reference that drifted instead of converging.


**Two off the head in the two-hundred-and-seventy-ninth, and both were one shape apiece.**
`issue7769.pdf` page 1 — 0.67 from the nearest and 0.97 from the furthest, the tightest ratio the
tail had left, which step 1 reads as *we are alone* — is 24 commands setting one sentence on a
153 × 63 page, so its mean is its glyph coverage: two ladders agree to **0.003 of 255** and ours
climbs onto the limit from 0.5 below it (`AMBIGUOUS_GLYPH_SCAN_CONVERSION`). `issue11473.pdf`
page 1 is four hatch swatches whose tiling cell is a **0.3985-unit stroke** — 0.4 of a device
pixel — where `ghostscript` paints 60% more than the geometry, `poppler` 46% more and ours 10%
less (`AMBIGUOUS_SUB_PIXEL_LINE_WORK`). **Neither was a defect and both were a width**: the tail's
head is now populations of *scan conversion*, which is the same result the ranking reached one
level up in the two-hundred-and-fifteenth.

**Two more in the two-hundred-and-eighty-sixth, and both joined an existing group with no new
argument needed.** `two_pages.pdf` page 1 is **one command** — step 4's "one command has meant one
image" for the fourth time — a 512 × 543 JPEG with a JPEG soft mask reduced by a third, where ours
is flat to four decimal places and two ladders land 0.011 of 255 around it
(`AMBIGUOUS_IMAGE_REDUCTION`). `textfields.pdf` page 1 is six empty fields on a letter page whose
whole ink is one-point borders and comb cells: the two ladders agree to **0.0025 of 255**, and at
the page's own scale `ghostscript` is 27% over the geometry and `hayro` 19% under — the same two
outliers in the same two directions as `bug1863910.pdf`'s 28% and 22%, which is why it is that
page's group rather than a new one (`AMBIGUOUS_WIDGET_BORDER`).

**The ranking is a different list now.** With the three populations gone the head is 0.76 and
below, and the two-hundred-and-sixty-fourth session took four of it: `issue11913.pdf` page 1,
where the two ladders and ours agree to **0.024 of 255** — the tightest three-way agreement the
bucket has produced — `issue1350.pdf` pages 1 and 3, and `ZapfDingbats.pdf` page 1, whose eight
fonts are all standard 14 with nothing embedded and whose 0.60 of 255 is Foxit's outlines against
URW's. What is left below them: `issue12963.pdf` page 7 (0.76 / 1.92), `issue17065.pdf` page 1
(0.73 / **14.86** — a ratio of twenty, which step 1 says is a page about the references),
`issue16473.pdf` page 1 (0.72 / 2.77), `issue19971.pdf` page 6, `textfields.pdf` page 1 and
`issue11473.pdf` page 1.


**`chrome-text-selection-markedContent.pdf` left it in the two-hundred-and-fifty-ninth**, and it
is the cleanest instance of shape 1 so far: the whole difference is **one level of green over a
third of the page**, the file states every number in the fill that produces it, and §11.3.6's
arithmetic on those numbers gives 235.569 — which is 236, which is ours. Both references give 235.
`AMBIGUOUS_EIGHT_BIT_COMPOSITING`, and the way in was step 6's two ladders saying *not scan
conversion* (ours flat at 26.95 while both references climbed onto 27.21) followed by a
three-by-six grid of per-tile differences, which put the whole of it in two columns, followed by a
per-channel mean, which named the channel. **Localise before explaining**: a page-level number
said "0.25 low everywhere" and the truth was "one level low on one third".

**`bug1703683_page2_reduced.pdf` and `issue2884_reduced.pdf` went in the two-hundred-and-sixtieth**,
both to existing groups and both by the same instrument: two reference ladders, and ours beside
them. The first is one indexed image with a JPEG soft mask reduced by four, where `poppler`
descends onto 5.3695 and ours is flat at 5.364 — **0.006 of 255 apart, the tightest agreement
`AMBIGUOUS_IMAGE_REDUCTION` has produced** — while `mupdf` is flat 0.14 below both and is the
reference the page is about. The second is a 169 × 19 crop box holding one line of Japanese, whose
mean *is* its glyph coverage: the ladders agree to 0.018, ours climbs onto the limit from below,
and at eight times the two panels are indistinguishable. `AMBIGUOUS_GLYPH_SCAN_CONVERSION`.

Then **`freeculture.pdf` for seven of the next eight** — pages 163, 165, 184, 172, 156, 160 and 325,
from 0.86 down to 0.77.

**Every name above 0.75 has a furthest at least twice its nearest.** Step 1 says to prefer a page
whose two numbers are *close*, because that is the shape that says we are alone, and there is no
longer one on the ranking. That is a result about the list: its head is pages where the
*references* disagree, and its tail is the long book, which
`AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` has already measured twice.

**So step 7 is where the next defect is more likely to be than step 1.** The ranking has been
worked down to a population; the sweep is what looks at all 492 at once.

**And the two-hundred-and-thirty-fourth session took the two with the widest ratio, 8.01 and
9.63, and both were exactly that.** `issue21436.pdf` is 450 bytes whose catalogue's `/Pages`
names a `/Type /Page`: `mupdf` refuses the document, `ghostscript` paints a one-unit stroke 27%
over its geometry, and ours is 4.5836 at 1× against 4.5900 at 8× — the geometry itself.
`issue11931.pdf` is a `DCTDecode` image whose `SOF0` identifiers are the letters R, G and B:
`ghostscript` obeys Table 13's default and paints the band magenta at six and a half times the
page's ink, and the other four read the codestream. **Both are a clause read correctly by the
renderer that is alone**, which is the shape a wide ratio is *for* — and both produced a
correction to a ledger row rather than a change to any pixel.

**And the two-hundred-and-thirty-seventh took the next one down and it was ours.**
`issue19634.pdf` sat at 0.85 / 5.96 — Skia's own `blurSmallRadii`, five renderers giving five
answers between 2.87 and 47.98 — and the picture said what no number could: **we drew none of
the red text**. §8.6.8's uncoloured restriction was still in force inside the soft mask's own
group, so a `d1` glyph procedure that set a `/Luminosity` mask had its mask evaluated to zero.
Ink 2.87 → 8.03 against `mupdf`'s 7.63 and `hayro`'s 8.11. ADR 0173. **A five-way spread is
never scan conversion**, which is the reading to take from the ratio rather than "the references
disagree, so it is not ours".

**Four `freeculture.pdf` pages and one paper under twelve names left it in the
two-hundred-and-thirty-third**, which is 158 of them, and the shape of that result is in the
section above: a quarter of this bucket was one document.

**`issue4402_reduced.pdf` left the list in the two-hundred-and-thirty-first session**, and it is
the clearest instance so far of shape 3 — the clause puts the answer beyond itself and says so.
The page is a 215 × 28 crop box holding one line of eight-point text and a rule, so its mean *is*
its glyph coverage, and §10.7.4's last sentence is "[s]can conversion of character glyphs may be
performed by a different algorithm from the preceding one". The two ladders agree to 0.012 of 255
and ours climbs onto the limit (55.41 → 56.71 → 56.78 → 56.91 against 56.98), while at the page's
own resolution the five renderers spread by 3.0 of 255. `AMBIGUOUS_GLYPH_SCAN_CONVERSION`.

**And the pairwise matrix is worth taking on a page like this**, because it answers a question
step 5 cannot: ours against `hayro` is 0.0219 MAE, the *smallest* pair among all ten, and every
pair involving `ghostscript` is larger than our worst. A page where the references disagree with
each other more than they disagree with us is not a page about us.

**`issue18529.pdf` left the list with a difference, not with an explanation**, and it is the one
name here worth coming back to: ours and `hayro` are both 5.8% under the high-resolution limit on
a 65×50 page that is one §8.7.4.5.3 gradient, and the three C renderers are on it. 1.3 of 255,
and the two renderers on one side of it are the two that share no library with the other three.

**The whole of the list above 1.6 went in the two-hundred-and-fifteenth session**, six pages in
one sitting, and the shape of that result is worth as much as the pages: **the ranking's top is
now populations rather than defects.** Two were a face nobody ships, two were one word on a page
the size of a postage stamp, one was two hairlines, one was an eight-bit ramp — and the only new
*defect* among them is a rasteriser property that a synthetic page found in ten minutes
(`doc/todo/11` item 3).

**`issue4706.pdf` is already known to be about *where* rather than *how much*.** It and
`stamps.pdf` come out within 0.12 and 0.2 of every renderer on ink *and* against the
high-resolution limit, so whatever separates them is placement. That is worth knowing before
opening one: **step 5's closed form answers "how much" and is silent on "where"**, and a page
where everybody's ink agrees needs the heatmap instead.

**A page can be fixed and stay on this list, and this file said otherwise.** The paragraph below
recorded that `issue7821.pdf` "left it in the hundred-and-ninety-ninth from the top of the list".
What left was its *position*: ADR 0160 took it from 5.44 to 1.79 and it sat at the top of the
undiagnosed ranking for fifteen more sessions, because a fix is not a diagnosis and only the
second takes a name off `ambiguous_undiagnosed.txt`. It has one now
(`AMBIGUOUS_GRADIENT_QUANTISATION`). **When a session fixes a page on this list, write its group
in the same session** — the same lesson the text gate's ratchet taught in the hundred-and-sixty-
sixth, one list over.

**Fifteen names left the list in the two-hundred-and-fifth to -eleventh sessions**, and the shape
of the result is the argument for the tail: two were defects in this tree (`bug1863910.pdf`'s
`/BBox` clip and `issue21068.pdf`'s miter bound, both ADR 0165), one was a defect with its own
file (`radial_gradients.pdf`, fixed in the two-hundred-and-thirty-second session, ADR 0171),
one is a clause `poppler` does not honour
(`bug1552113.pdf`'s 112-unit border), and the rest are scan conversion or artwork the standard
does not state.

**And the sixth defect the bucket has produced came out of the next name down.** `bug1863910.pdf`
was two empty text fields, and its one-point borders carried 22% less ink than their geometry —
an anti-aliased `/BBox` clip lying exactly on the stroke's outer edge, which is ADR 0155's finding
one path over. Fixing it moved the oracle's own headline: **agrees 849 → 851, contradicted 70 →
68** (ADR 0165). Two of the three pages the ranking has produced since the instrument was repaired
were defects.

**Step 6 emptied the top of the list in one session.** Four of the five names above 3.5 were
image reductions whose whole difference is scan conversion, and the high-resolution limit settled
each in minutes: `bug1799927.pdf`, `issue1985.pdf`, `issue7200.pdf` and `jp2k-resetprob.pdf`.
The fifth, `issue18894.pdf`, was a file that had broken Table 73's operand count. None was a
defect; all five now say *what the clause determines* rather than sitting inside a spread.

**`issue8697.pdf` left this list in the hundred-and-ninety-seventh session and is the ranking's
own argument**: 3.52 from the nearest against 3.55 from the furthest, which step 1 says to
prefer, and it was drawing one Greek letter where the file states a sentence. ADR 0158. And
`issue7821.pdf` was **fixed** in the hundred-and-ninety-ninth from the *top* of the list, where
it had been for four sessions: 5.44, and the picture was a stamp anybody would have accepted
(ADR 0160). It left the list itself only in the two-hundred-and-fifteenth, which is the
distinction the section above draws.
`jp2k-resetprob.pdf`, `S2.pdf` and `issue5475.pdf` left it in the two-hundredth, all three
through `tests/jpeg2000.rs`. ADR 0161.
