# ADR 0163 — The ink recipe was halving our own page

Status: accepted, 2026-08-03. Session 202. A correction to two of this session's own findings,
and a repair to the method file that caused them.

## What happened

`doc/todo/00-ambiguous-bucket.md` is the method for §3a's work, and step 5 said how to measure
ink:

```sh
magick <png> -colorspace Gray -format "%[fx:(1-mean)*255]"
```

**Our oracle artefacts carry an alpha channel and `poppler`'s, `mupdf`'s and `ghostscript`'s do
not.** `-colorspace Gray` on an RGBA image produces grey *plus alpha*, and `%[fx:mean]` averages
both channels — so for an opaque page the result is `(255 − grey) / 2`, **exactly half** the ink.
The three C references are measured correctly and we and `hayro` are measured at half.

Sessions 197 and 199 followed that recipe and each drew a conclusion from it:

| ADR | as written | corrected |
|---|---|---|
| 0158, `issue8697.pdf` | ours 9.22, `hayro` 9.08 │ `mupdf` 18.40, `ghostscript` 18.62, `poppler` 18.75 | ours **18.43**, `hayro` **18.16**, and the other three unchanged |
| 0160, `issue7821.pdf` | ours 22.9, `hayro` 24.2 │ `mupdf` 47.1, `poppler` 46.7, `ghostscript` 50.0 | ours **45.71**, `hayro` **48.33**, and the other three unchanged |

Both wrote the same diagnosis: *trap 9's third shape — three references linking one
`libfreetype` against two Rust renderers that do not.* There is no such split on either page. On
`issue8697.pdf` all five renderers are within **0.6** of one another, which is ink conserved with
the difference confined to where the glyphs are; on `issue7821.pdf` the spread is 9% and
`ghostscript` is the outlier rather than us. Both ADRs and both group comments now carry the
correction beside the claim.

## Why this is not just a slip

**Session 161 found this exact defect and wrote it down twice.** It is in
`CONTRADICTED_GLYPH_EDGES`:

> The first measurement put our ink at exactly half the three C renderers' on ten pages running,
> a ratio of 2.00 to three significant figures — which is not what hinting does and is what a
> broken instrument does. … the tell here was that the two renderers agreeing with us were the
> two whose output format matched ours.

and in the handover's Habits, under "a suspiciously clean measurement is a reason to check the
instrument".

It is *not* in the file that tells the next session how to measure. So the lesson was recorded in
the two places where it was learned and not in the one place where it is used, and the next
session to reach for the instrument reached for the broken one.

**A lesson recorded where it was learned and not where it is used has not been recorded.** That
is the finding, and it is about this project's own documents rather than about PDF.

The tell was there both times and was not read: on `issue8697.pdf` the ratio between our number
and each of the three C references was 2.00, 2.02 and 2.03. Session 161's own sentence — a clean
ratio is a broken instrument — was in the tree and did not fire.

## What changed

- Step 5 of `doc/todo/00-ambiguous-bucket.md` now says `-alpha off -channel R`, with the reason
  and the two-renderers-agreeing tell beside it.
- A **new step 6**: where the difference is scan conversion, the closed form is the same page at
  eight times the resolution. Ink is geometric, a renderer's departure from it shrinks with the
  pixels, so the *same* renderer asked at two resolutions gives a limit no reference's verdict is
  borrowed for.
- ADRs 0158 and 0160, and `AMBIGUOUS_SUBSTITUTED_FACE`, carry the corrected tables.

## The page that found it

`bug1799927.pdf` sat first on §3a's ranking at 4.57 bounds. It is an A4 CAD drawing whose text is
not text: **2 156 of its 2 331 commands are inline one-bit stencils**, 2 153 of them 7×10 samples
at 116 ppi on a 72 dpi page, so every sample is well under a device pixel.

The first measurement said ours was 5.47 against the references' 11.7 to 13.4 — half, again,
and this time the number was so far from plausible that the instrument got checked.

With the instrument fixed it becomes the cleanest measurement this group has:

```text
ours 10.94 │ ghostscript 11.70 │ poppler 12.64 │ mupdf 13.40 │ hayro 5.94
           └ poppler at 576 dpi: 10.82   mupdf at 576 dpi: 11.40
```

The same renderer at rising resolution converges — `poppler` gives 12.64, 11.39, 10.82 at 72, 288
and 576 dpi — because scan conversion stops rounding as the pixels shrink. **We are the only one
of the five already at the geometry**, within 1% of that limit, while each C reference deposits 8%
to 24% more at the page's own scale than it deposits at eight times it.

That is not an accusation. §10.7.4 asks for exactly what they do — "any pixel whose half-open
square region intersects the shape, no matter how small the intersection is" — and ours is ADR
0025's documented departure from that sentence. On this page the departure is what lands on the
truth, which is `AMBIGUOUS_IMAGE_REDUCTION`'s standing argument with a number attached for the
first time.

## Alternatives rejected

- **Write the oracle's artefacts without alpha.** It would fix the recipe by removing the
  difference, and it would also remove information: a transparent background is what
  `shadings.rs` relies on to tell "painted white" from "not painted", and the tell that saved
  session 161 was precisely that our format differs. The instrument should handle both formats.
- **Leave the two ADRs and note the correction only here.** A claim and its correction have to
  sit together or the claim outlives it — which is the failure mode `AMBIGUOUS_SUBSTITUTED_FONT`
  demonstrated for twenty-seven sessions.
