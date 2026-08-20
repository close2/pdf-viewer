# ADR 0454 — Four ceilings, none of which was this program's

Status: accepted, 2026-08-20. Session 619. Takes `doc/todo/03` §18's named successor — the next
chunk of the SafeDocs crawl — and fixes the four defects its eight thousand documents produced.
Amends §7.4.6's, §7.4.8's, §8.9.7's and §9.9's ledger rows.

## The chunk, and why these archives

§18 leaves "51 944 crawled documents unranked … in archive-sized pieces". This round took **eight
whole archives — `0546`, `1284`, `2022`, `2760`, `3498`, `4236`, `4974` and `5712`, 8000
documents** — none of the two session 603 ranked, none of the five session 613 did and none of
the seven session 615 did. *Which* archives is immaterial and that is ADR 0261's finding: the
crawl is sorted by SHA-256 and cut into pieces, so an archive is a hash bucket and any set of
them is an unbiased sample.

The instrument is 603's, unchanged and reused rather than rewritten: page one at 72 dpi from this
tree and from `pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box
(trap 3), ranked by our ink minus the lightest live reference's with each panel's raster size
beside it.

**It was checked against the previously-fixed documents before it was trusted, and the check
caught something** — not a neighbour's merged change, but the instrument's own setup. Sixteen
documents named by ADRs 0438, 0448 and 0451 were re-measured first, and seven of them came back
*worse than before their fix*: `2268946.pdf` at −15.621 where 615 recorded +0.035, `3252105.pdf`
at −156.436 where it recorded −6.390. The cause is a line worth writing down for the next chunk
round: **`pdf-sandbox-worker` has to be built into the same target directory as the example**, or
every codec that goes through the sandbox — JBIG2, CCITT, JPEG 2000 — refuses with
`the sandbox worker was not found` and the ranking measures a tree with no bilevel decoder in it.
`cargo build --release -p pdf-model --example render_at` does not build it; `cargo build --release
-p pdf-sandbox --bins` does. With it built, all sixteen reproduce to the thousandth.

## What the two ends said

**The negative head is four different defects of this tree**, and each of the four is a *bound*
that belongs to something other than this program: a decoder library's default, a window's size,
a filter parameter read as a disagreement, and a container's table list. Three of the four are
reported, which is trap 5 working — and a report is not a drawn page.

**The positive head is 613's finding and not ours**, down to +20: `poppler` alone drawing almost
nothing while `mupdf`, `ghostscript` and this tree agree, which is now a note in
`doc/traps/oracle-and-references.md` and was read there rather than derived again. Two rows above
it are the *other* reference failing, which the note's own sentence predicts from the other side:
`1284136.pdf` at +47.956 is `ghostscript` at 153.843 where ours, `poppler` and `mupdf` sit within
2.7 of each other at 201.8, 201.6 and 199.1; `1284295.pdf` at +28.502 is `ghostscript` rendering
a *different page box* — 612×792 against the 504×360 the other three agree on — which is trap 3
arriving as a size rather than as ink, and is exactly why the panel sizes are printed beside the
numbers.

Two rows of the negative head are neither ours nor a reference's: `0546561.pdf` at −30.018 and
`4974796.pdf` at −15.417 both reach `hayro-jbig2` 0.3.0's flat 10 000-instance cap, which
`doc/todo/_image-codecs-and-the-sandbox.md` §7 has been holding since session 613 and which
upstream has already replaced. That release now has **five** documents of 22 000 waiting on it.

## 1. A frame taller than a library was willing to try

`2022009.pdf` at **−84.152** is a full-page scan drawn as a blank sheet, with one report:
`Image height 28341 greater than height limit 16384. If use set_limits if you want to support
huge images`. That sentence is `zune-jpeg`'s, and 16384 is its `DecoderOptions` default.

§7.4.8 states no ceiling anywhere, and says whose the dimensions are:

> The values of these parameters, which include the dimensions of the image and the number of
> components per sample, are entirely under the control of the encoder and shall be stored in the
> encoded data.

ISO/IEC 10918-1 gives each axis sixteen bits, so 65535 is the largest a codestream can *say* and
there is nothing below it for a reader to enforce on the clause's behalf. What bounds this crate
is `MAX_SAMPLES`, which is an allocation this crate makes and has its argument written beside it
— and this page is 994 × 6802 at 72 dpi, so its 28341 rows are far inside it.

**A number that means "this library will not try" cannot also mean "this program will not
allocate".** That is 615's lesson about `/Mask`'s ceiling arriving from a different direction, and
the fix is the same shape: `jpeg_options` states the format's own maximum in each axis, both
`decode_jpeg` and `contradicted_frame` construct their decoder with it — the second mattered too,
since a frame past the default silently produced no *report* either — and `MAX_SAMPLES` is left to
do the job it documents. The bomb the default was written against is refused by it for the reason
it always was: 65535 by 65535 is 4.29 G samples against a budget of 268 M.

`2022009.pdf` −84.152 → **−0.105**, ours 84.047 against 84.542 / 84.152 / 84.196.

## 2. A length the window was too short to check

`3498294.pdf` at **−26.015** is an architectural drawing whose second inline image is 1024×716
`/DeviceRGB`, unfiltered. It drew 37% of the picture, reported that its "samples stop at 817411
bytes where 1024x716 at 8 bits and 3 component(s) needs 2199552", and then reported **fourteen
`Operator` lines whose names are the image's own samples** — because the content stream resumed
inside the data and 1.4 MB of a photograph was tokenised as a program.

`inline_image`'s design was already right and its module comment already said so: three answers
for where the data ends — `/L`, §8.9.3's arithmetic, and a forward search for `EI` — with the
search "only reached for *filtered* data with no `/L`". §8.9.7 is why the first two exist:

> The bytes between the ID operator and a white-space token, but before the EI operator shall be
> treated the same as a stream object's data ( see 7.3.8, "Stream objects"), even though they do
> not follow the standard stream syntax.

and §7.3.8.2 makes such an extent inferable rather than guessable:

> Finally, streams are used to represent many objects from whose attributes a length can be
> inferred. All of these constraints shall be consistent.

**What was wrong is not the order but the check.** Each derived end is verified against the `EI`
it predicts, and a page's `/Contents` arrives through a 64 KiB window that doubles (ADR 0365).
Inside a window shorter than the image the verification cannot happen at all — `terminator_at`
reads past the buffer's end and answers `None` — so both derived answers were dropped and the
search ran. Then the loop stopped growing the buffer, because the search had returned an *answer*:
`Interpreter::inline_image` grows only while the scan is an error. The module's own sentence about
answer 3 was therefore false for every unfiltered inline image larger than a window, which the
census says is a real population — 89 968 of 93 930 inline images are unfiltered.

**A derived end past the bytes held is a request for more bytes, not a failed check.** `scan` is
told whether its slice is all there is; where it is not, an unverifiable derived end is
`InlineImageError::Truncated`, which is what makes the caller double. Where the slice *is*
everything, an end past it is the file's own arithmetic overrunning its content stream and the
search still runs — which is what keeps a wrong `/L` a fallback rather than a blank page, and is
the behaviour `a_length_that_does_not_predict_the_terminator_falls_back_to_the_search` pins.

`3498294.pdf` −26.015 → **−0.106**, ours 33.276 against 35.065 / 33.383 / 36.420.

## 3. A width the filter was told to round up

`4236390.pdf` at **−15.235** and `2022430.pdf` at **−12.618** are scans whose images are refused
with `CCITTFaxDecode /Columns 872 is not the image's width 869` and `/Columns 896 … width 892`.
§7.4.6's ledger row called that refusal a decision and said why: "the row stride the filter
produces and the one §8.9.5.1 makes the unpacker read then differ and nothing here says which
wins".

Table 11's `/Columns` row says which wins, in its second sentence, and the row had only ever
quoted the first:

> The width of the image in pixels. If the value is not a multiple of 8, the filter shall adjust
> the width of the unencoded image to the next multiple of 8 so that each line starts on a byte
> boundary.

869 taken to the next multiple of 8 is 872. 892 is 896. **Both files wrote the adjusted width**,
which is the width this clause hands the filter — and the runs encoded in the data are for that
many columns, so believing `/Columns` is what decodes them at all. §8.9.5.1's `/Width` then says
how many of each line's samples are the image, and `ceil(869/8)` and `ceil(872/8)` are both 109,
so the stride the filter produces and the stride the unpacker reads are *the same number* and not
one sample moves. The two premises the refusal rested on were both false.

The relaxation is exactly Table 11's sentence and nothing wider: `columns == width` or
`columns == width.next_multiple_of(8)`. Anything further apart is still refused, because then the
runs are for a line this image is not and §7.3.8.2's "[a]ll of these constraints shall be
consistent" has been broken in a way that shifts every sample after the first run — which is what
`a_columns_that_is_not_the_padded_width_is_refused` holds.

`4236390.pdf` −15.235 → **+0.689**; `2022430.pdf` −12.618 → **+1.274**.

## 4. A table the container required and the clause did not

`0546308.pdf` at **−6.785** and `3498231.pdf` at **−7.131** are the same producer's pages, each
losing about 1550 text operations to `font /T1_0 could not be parsed: units per em is zero`.
`/T1_0` is a `/Subtype /Type1` font whose descriptor's `/FontFile3` is `/Subtype /OpenType`, and
its table directory holds six tables: `BASE`, `CFF `, `GPOS`, `GSUB`, `OS/2` and `cmap`. There is
no `head`, which is where `skrifa` — and every sfnt reader — finds the em square.

An OpenType file without `head` is malformed by ISO/IEC 14496-22. **It is not malformed by
ISO 32000-2**, and §9.9's Table 124 says so twice. It lists what such a program owes:

> A Type1 font dictionary or CIDFontType0 CIDFont dictionary, if the embedded font program
> contains a "CFF " table without CIDFont operators. In addition to the "CFF " table, the font
> program shall include the "cmap" table.

and then states the exemption outright, which is the sentence that decides it:

> ISO/IEC 14496-22 describes a set of required tables; however, not all tables are required in the
> font file, as described for each type of font dictionary that can include this entry.

So these two files are *conforming*, and this tree refused them for want of a table the standard
says need not be there. The `CFF ` table is a whole font program — its own `FontMatrix`, charset,
encoding and charstrings — and this crate already has a complete bare-CFF reader for exactly that
shape, because §9.9's `/FontFile3` `/Type1C` subtype is the same program without the wrapper.
`program::extracted_cff` hands it over and nothing downstream knows a container existed.

**The condition is the absent `head` rather than the container**, and that is deliberate rather
than cautious: a program that states a `head` states its scale, and its `cmap` and `hmtx` are what
§9.6.5.4's route reads through `skrifa`, so it stays exactly where it was. What the change reaches
is the set of programs that could not be read at all —
`an_opentype_program_with_a_head_is_still_an_sfnt` is what says so.

`0546308.pdf` −6.785 → **−0.010**; `3498231.pdf` −7.131 → **+0.009**.

## What moved, measured on the population that found it

All **twenty-two** ranked archives — this round's eight, 615's seven, 613's five and 603's two —
were re-ranked whole with the fixed tree and diffed row by row against the ranking that named the
defects. **21 rows of 22 000 move**, and they divide in two.

**Ten are documents one of the four fixes is about**, and the four extra ones the six witnesses
did not name are what says the fixes generalise: `3375550.pdf` −7.099 → −0.085 and `5712943.pdf`
+2.451 → +1.785 are two more fonts with no `head`; `3498460.pdf` −1.971 → +0.322 is a `/Mask`
whose `CCITTFaxDecode` states `/Columns 680` against a width of 674; `2268541.pdf` −0.146 → +0.155
is a JPEG 23939 samples *wide*. **Two of the ten — `3375550.pdf` and `2268541.pdf` — are in 615's
own archives**, which is the third round running that a fix has reached an earlier chunk.

**The other eleven are the instrument rather than the tree, and that is measured rather than
asserted.** Eight have our panel identical to the thousandth with a *reference* panel absent from
one of the two runs; three have **our** panel absent, and re-measured alone at three workers
instead of sixteen all three reproduce their earlier number exactly — `1161651.pdf` +1.537,
`6327464.pdf` +0.481, `1161228.pdf` −0.033. Thirty seconds is the per-renderer bound this harness
sets and a sixteen-way run over 1000 documents is a loaded machine;
`doc/traps/oracle-and-references.md` already carried the same shape from the references' side, and
it now carries it from ours. The full table is in `doc/history/619-*.md`.

Each fix is pinned by a test that was **run against the defect first** (trap 13): the JPEG budget
by a generated 8 × 20000 frame, above the library's default and far below `MAX_SAMPLES`, which
needed `dct_components.rs`'s hand-written codestream to learn a second dimension; the inline
image by a fixture two windows long whose samples spell an `EI` in the first one, with a marker
rectangle after the real `EI` so that a wrong resume is visible as a missing command; the CCITT
padding by four T.4-encoded lines of sixteen columns under a `/Width` of twelve, plus its own
negative twin at `/Columns 24`; and the font by an `OTTO` wrapping this repository's own
`FoxitSerif.pfb`, with and without a `head`.

**No gate number moves**, and that is the fourth round running the crawl has said so: no document
of the 974 states a JPEG frame past 16384 rows, an unfiltered inline image larger than a window, a
`/Columns` that is its width padded, or an OpenType program with no `head`. `doc/todo/00`'s step 7,
re-run over all the oracle's artefacts because this round changes what gets drawn, reproduces
session 598's head and tail to the thousandth — `issue12418_reduced.pdf` −19.447, `issue4722.pdf`
−13.810, `issue15977_reduced.pdf` −12.927, `bug1050040.pdf` −11.272, `issue5801.pdf` −8.991, and
on complete documents `issue16038.pdf` −5.737, `issue12295.pdf` −2.363, `issue14297.pdf` −1.130.

## What the head still holds

- **Five documents of 22 000 waiting on `hayro-jbig2`'s flat 10 000-instance cap**, two of them
  found this round: `0546561.pdf` −30.018 and `4974796.pdf` −15.417, beside 613's `1653119.pdf`
  and 615's `3375154.pdf` and `3252105.pdf`. `doc/todo/_image-codecs-and-the-sandbox.md` §7.
- **Four silent rows this round did not take**, each named in `doc/todo/03` §19 with what is known
  about it: `2022794.pdf` −12.743, `4236552.pdf` −10.930, `4236836.pdf` −10.001 and
  `2022216.pdf` +20.141.
- **615's two are still open**: `6696954.pdf` −10.252 and `5589519.pdf` −8.212, both trap 9's
  family.
- **44 rows of the 8000 produce no number**, the same three shapes 613 and 615 opened by hand:
  crawl artefacts saved under a `.pdf` name, truncated files, and documents this tree refuses on a
  clause.

## Consequences

- A `DCTDecode` frame is bounded by this crate's own sample budget and by ISO/IEC 10918-1's
  sixteen bits, never by a decoder library's default.
- An unfiltered inline image's derived length is checked or asked about again, never abandoned for
  a search because the window was short.
- A `CCITTFaxDecode` `/Columns` that is the image's width taken to a byte boundary is Table 11's
  own arithmetic and decodes rather than being refused.
- A `/FontFile3` `OpenType` program whose outlines are a `CFF ` table is read as that program when
  the tables Table 124 exempts are absent.
- Four clause rows are amended; `doc/todo/03` gains §19 and **43 944 crawled documents remain
  unranked**.
