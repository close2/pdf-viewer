# Empty the oracle's ambiguous bucket

Status: **standing task**, since the hundred-and-seventy-sixth session. 715 pages left.
Priority: 00 — the last large population where a defect can live without a name
Corpus: 748 ambiguous pages on documents we call complete; 31 diagnosed, 715 held by name
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
5. **Measure with a closed form where the clause states one**, and with pairwise distances only
   as corroboration. `magick <png> -colorspace Gray -format "%[fx:(1-mean)*255]"` is the ink;
   `magick compare -metric MAE a.png b.png null:` is the pairwise number.

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

Ten sessions from the hundred-and-seventy-sixth, then eight more: **six defects found and all
six fixed** — a page one that was page two (ADR 0148), a photograph rendered black (0149), a
shading painted as a square (0150), a stencil that drew nothing (0151), a whole grid that
disappeared (0154), a sentence drawn as one Greek letter (0158) — plus the ten documents whose
substituted font drew none of its characters in silence (0152), the coverage rule that made
eight of them draw (0153), a pattern cell's clip worth 15% of a page's ink (0155), and a font
program that draws nothing now saying so (0157).

The bucket itself went 754 → 715, and that is the least interesting number in this file. *Six
defects nobody could see* is the one to watch.

## Its shape, measured

| | |
|---|---|
| distinct documents the pages come from | ~181 |
| `freeculture.pdf` (320) and `pdkids.pdf` (52) | **372 — two long books, half the bucket** |
| documents contributing exactly **one** page | ~154 |

Two books and a long tail of single pages. The books are set in fonts nobody embedded, so each
renderer substitutes differently and the *bound* is loose because the references disagree with
one another. **Take the tail first**: each of those is a file somebody added to a corpus for a
reason, and the reason is written down.

## The next names on the ranking

`issue7821.pdf` (5.44 from the nearest), `jp2k-resetprob.pdf` (5.03), `bug1799927.pdf` (4.57),
`issue1985.pdf` (4.10), `issue7200.pdf` (3.81), `issue18894.pdf` (3.50), `bug1863910.pdf`
(3.03), `issue21068.pdf` (2.82), `copy_paste_ligatures.pdf` (2.81), `radial_gradients.pdf`
page 5 (2.74 — and 2.74 from the furthest, which is the everybody-against-us shape).

**`issue8697.pdf` left this list in the hundred-and-ninety-seventh session and is the ranking's
own argument**: 3.52 from the nearest against 3.55 from the furthest, which step 1 says to
prefer, and it was drawing one Greek letter where the file states a sentence. ADR 0158.
