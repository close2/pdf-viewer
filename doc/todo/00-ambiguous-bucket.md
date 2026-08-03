# Empty the oracle's ambiguous bucket

Status: **standing task**, since the hundred-and-seventy-sixth session. 511 pages left.
Priority: 00 — the last large population where a defect can live without a name
Corpus: 788 ambiguous pages (749 on documents we call complete); 238 diagnosed, 511 held by name
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

`pr12564.pdf` (0.99 from the nearest, 2.30 from the furthest),
`chrome-text-selection-markedContent.pdf` (0.98 / 2.15), `standard_fonts.pdf` pages 8 and 7
(0.96 / 1.42), `bug1703683_page2_reduced.pdf` (0.91 / 2.02), `issue2884_reduced.pdf`
(0.90 / 4.53), `issue9291.pdf` (0.87 / 1.49), `freeculture.pdf` page 163 (0.86 / 2.09),
`issue19634.pdf` (0.85 / **5.96**).

**Every name above 0.85 now has a furthest at least 1.4× its nearest.** Step 1 says to prefer a
page whose two numbers are *close*, because that is the shape that says we are alone — and there
is no longer one on the ranking. That is a result about the list: the head of it is pages where
the *references* disagree.

**And the two-hundred-and-thirty-fourth session took the two with the widest ratio, 8.01 and
9.63, and both were exactly that.** `issue21436.pdf` is 450 bytes whose catalogue's `/Pages`
names a `/Type /Page`: `mupdf` refuses the document, `ghostscript` paints a one-unit stroke 27%
over its geometry, and ours is 4.5836 at 1× against 4.5900 at 8× — the geometry itself.
`issue11931.pdf` is a `DCTDecode` image whose `SOF0` identifiers are the letters R, G and B:
`ghostscript` obeys Table 13's default and paints the band magenta at six and a half times the
page's ink, and the other four read the codestream. **Both are a clause read correctly by the
renderer that is alone**, which is the shape a wide ratio is *for* — and both produced a
correction to a ledger row rather than a change to any pixel.

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
