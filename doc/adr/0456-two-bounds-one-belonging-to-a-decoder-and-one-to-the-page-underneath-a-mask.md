# ADR 0456 — Two bounds: one belonging to a decoder, one to the page underneath a mask

Status: accepted, 2026-08-20. Session 621. Closes
`doc/todo/_image-codecs-and-the-sandbox.md` §7's `hayro-jbig2` item and the two rows session 615
left at the head of `doc/todo/03` §18. Amends §7.4.7's, §8.7.2's, §11.6.5.1's and §8.6.5.8's
ledger rows, and trap 9.

Two items, both handed over as "images", and they turned out to be one release, one clause and one
correction to a diagnosis nobody had measured.

## 1. The decoder's bound: `hayro-jbig2`'s ten thousand symbol instances

`hayro-jbig2` 0.3.0 refuses a §7.4.7 text region whose header declares more than a flat 10 000
symbol instances, under the comment "[a]rbitrarily chosen, but we need some limit to prevent
timeouts". A full page of scanned text has more. Three documents of the SafeDocs crawl pay it, and
each pays it *silently*, because a refusal inside `pdf-sandbox`'s worker is one image failing rather
than one page: the image is dropped, the page draws around it, and what the reader sees is a blank
sheet.

Upstream replaced the constant in `1be7ab10` (`hayro-jbig2: Use heuristic for maximum symbol
instances (#1278)`) with pdfium's heuristic — `segment_data_len × 32` — which is a bound
proportional to the bytes the segment actually carries, and so cannot refuse a region a small
segment could not have encoded. On `1653119.pdf` it admits 852 096 where the file declares 13 264.

**No release carries it.** `cargo search hayro-jbig2` still answers 0.3.0, and the crate has two
tags, `v0.1.0` and `v0.3.0`. `doc/environment.md` says what to do about that — `tmp/hayro` is a
checkout of the whole workspace with the owner's fork as `origin`, and a fix goes on a branch there
rather than being waited on — and here even that was not needed: the commit is already in
`upstream/main`, and `64efcaca`, the revision this tree pins for `hayro-jpeg2000`, already contains
it. So the whole of the change is naming that revision for two more crates.

`hayro-ccitt` is the second, and only because `hayro-jbig2` names it by path inside that workspace:
one crate from two sources compiled into one binary is a version nobody chose. It brings #1304 with
it, which folds `Decoder::push_pixel` and `push_pixel_chunk` into a single
`push_pixels(white, count)` — the caller no longer splits a run at byte boundaries, so the packer
does. `PackedRows::push_run` counts pixels for both filters now: the bits that finish the byte in
progress, the whole bytes as a `memset`, the bits that start the next one. The middle part is the
one that matters on a 600 dpi scan line and the two ends are the ones that shift every pixel after
the first run if they are wrong.

**What the three documents do now**, page one at 72 dpi, our ink minus the lightest live reference's:

| | before | after | what it is |
|---|---|---|---|
| `1653119.pdf` | −35.695 (ours 0.000) | **+0.012** | a 1991 *El País* page, 4256×6258; blank sheet → the whole broadsheet |
| `3375154.pdf` | −16.417 | **+0.032** | a Russian newspaper scan whose `/Mask` is a 9364×13030 stencil |
| `3252105.pdf` | −6.390 | **−0.215** | a book cover whose foreground layer is another stencil |

Nothing was reported before the change and nothing is reported after: `unsupported []` on all three,
which is the point — the bound was never this program's to see.

**The next bound did not appear**, which was the thing to check rather than assume: session 615
found exactly that shape, two documents arriving from behind a ceiling a previous round lifted.
Here the removed cap exposes nothing.

**And nothing on the curated corpus moved.** `examples/raster_digest` over the 125 pdf.js documents
that name `CCITTFaxDecode`, `JBIG2Decode` or `JPXDecode` — `filter_census`'s own list — is
byte-identical before and after, all 125 lines. That is what says the release is about the crawl,
and it is also the check the `push_run` rewrite needed: the corpus's 172 CCITT images and the
`bitmap-*` suite's 97 encodings of one drawing pack to the same bytes they did.

`deny.toml`'s `allow-git` entry names the second reason now, so the entry does not read as the JPEG
2000 fork alone: these two go back to crates.io on a release, independently of that fork.

## 2. The page's bound: a shading pattern inside a soft mask

Session 615 left two rows below −8 and called both "trap 9's family" — the trap that says two
references can agree because they share code or data. One is. One is a defect of this tree, and the
difference is the finding.

### `5589519.pdf`, filed as `/DeviceCMYK` JPEGs, is not that

The file names no `ICCBased` space, no `/DefaultCMYK` and no output intent, so on its face it is ADR
0048's `CONTRADICTED_DEVICE_CMYK_CONVERSION` again. **A probe says otherwise.** A four-object PDF
holding nine plain `DeviceCMYK` patches, rendered by all four programs, puts this tree and `poppler`
on *identical* values across the range and `mupdf` and `ghostscript` in the other camp — ADR 0048's
own two-cluster signature, from the side that clears the page rather than convicts it. Whatever
`5589519.pdf` was, it was not the conversion.

`pdfref-hayro` settled the direction: a fourth interpreter, sharing no colour code with the other
three, agrees with them and not with us. Then a bisection of the page's own content stream — 51 105
lines, rebuilt at the same byte length so every cross-reference offset stays valid, binary-searched
on one pixel — named the operator, between lines 36 944 and 36 945:

```
q 0 417 m 0 1134 l 418 1134 l 418 417 l h W n
  q /R120 gs      % /SMask << /S /Luminosity /G 101 0 R >>
    /R122 Do      % the photograph
  Q
Q
```

Mask group 101's whole content is a `/DeviceGray` axial pattern; its `/Matrix` puts the axis from
`y = 600.195` to `y = 526.179`, extended both ways. The page's first operator is
`0.72 0 0 0.72 0 0 cm`. Under the mask's own space that axis is device rows 427 to 480, below the
photograph, so the photograph is opaque. Under the *page's* default space it is rows 259 to 333,
through the middle of the photograph — and that is exactly where this tree's render fades, measured
row by row: identical to `poppler` at row 255, the page's own green mixing in from row 260, pure
background green by row 334.

**§8.7.2 says which it is**, and names the case outright:

> Similarly, if a pattern is used within a form XObject (see 8.10, "Form XObjects" ), the pattern
> matrix maps pattern space to the form's default user space (that is, the form coordinate space at
> the time the form is painted with the Do operator).

A soft mask's `/G` is a form XObject. What stands in for its `Do` is §11.6.5.1's own sentence, which
this tree already quotes in `build_soft_mask`: the group's `/Matrix` concatenated with the transform
in force at the `gs`. `draw_xobject` has swapped the interpreter's `base` for exactly that since the
clause was first read; `build_soft_mask` never did. Two lines, and the fix is the same two lines the
other door already had.

**This is the fourth way of becoming a parent content stream, and the fourth time the sentence was
wrong about one.** §8.7.2's ledger row already records three: the page (session 52), the form (ADR
0160), the tiling cell (ADR 0430). The lesson written down after the second — *a rule about "the
parent content stream" needs a test per way of becoming a parent* — was right and the enumeration
was short. `a_pattern_in_a_mask_group_maps_through_the_groups_own_default_space` fails in both
directions: 121 and 0 with the clause, 188 and 61 without, both measured rather than derived.

`5589519.pdf` goes from **−8.212 to +0.713**.

### `6696954.pdf` is trap 9, and the mechanism is a library's default argument

A union newsletter whose page group and images all run through one embedded CMYK press profile —
`prtr`, PCS `Lab`, an `A2B0` shared with `A2B2` and a *different* `A2B1`. The three references agree
with each other to four levels and sit up to twenty from us on the dark areas.

The data is not shared: it came out of the file. What is shared is the engine. `objdump -p` — what a
binary asks for, not `ldd`'s closure — says `libpoppler` and `libgs` both link `liblcms2.so.2`, and
`libmupdf` links neither and *defines* 445 `lcms2mt_*` symbols, Artifex's fork of the same code
statically linked. Same family, three copies, one library. And `INTENT_PERCEPTUAL` is 0, which is
what a caller passing nothing passes.

The probe again — patches in an `ICCBased` space built from the document's own profile — and this
tree's own evaluator pointed at both tables:

| CMYK | our `A2B1` + BPC | our `A2B0` | poppler | mupdf | gs |
|---|---|---|---|---|---|
| `0 0 0 0` | 255,255,255 | 255,255,255 | 255,255,255 | 255,255,255 | 255,255,255 |
| `0 0 0 1` | 43,41,42 | **35,31,32** | **35,31,32** | 34,30,31 | 35,31,32 |
| `1 1 1 1` | 0,0,0 | 0,0,0 | 0,0,0 | 0,0,0 | 0,0,0 |
| `.75 .68 .67 .9` | 21,23,23 | 2,3,2 | 0,0,0 | 2,3,4 | 1,2,2 |

Our `A2B0` reproduces `poppler` byte for byte on the K-only black. So the three references are
reading the profile's **perceptual** table where nothing in the document asked for one. Setting the
profile header's rendering-intent field to 1 changes none of their answers, so it is the library's
default and not the file's request.

**The specification says RelativeColorimetric three times.** Table 51 gives the graphics state
"[i]nitial value: RelativeColorimetric"; §8.6.5.8 repeats it for a name a processor does not
recognise; and §11.4.7 says it a third time for precisely the construction this document has — the
page carries `/Group << /CS <the profile> /S /Transparency >>`:

> If the page group needs to be converted to the colour space of the output device, the colour
> conversion shall use a rendering intent of RelativeColorimetric unless the processor has an
> implementation-dependent way of specifying it otherwise. Additionally, the use of black point
> compensation in this colour conversion process is implementation-dependent.

`A2B1` is that table, and it is what `icc.rs` selects. **The page is left contradicted, with the
evidence beside it**, which is what principle 5 requires: agreement among three programs running one
library at its default is not evidence about the clause, and the clause is not silent here.

The second sentence of §11.4.7's paragraph is worth keeping too, because it is the standard granting
outright what §8.6.5.9's row argues for at length: black point compensation in this conversion is
implementation-dependent.

## What this round did not do

**§8.6.5.8 stays `partial`.** A document that *does* ask for `Perceptual` or `Saturation` still gets
`A2B1`; selecting a profile's table by intent is unimplemented and the row has said so for a long
time. What is no longer owed is any suggestion that the *default* is wrong — which was the open
question this page looked like, and is not.

## Consequences

- Three crawled documents that drew blank or short now draw, and no document of the curated corpus
  moves a pixel for it.
- A shading pattern inside a soft mask's group is anchored to the mask's own space, so a page that
  transforms its own coordinates before setting a mask no longer bleeds what is under it.
- Trap 9 gains a third mechanism (a shared *default argument*), an instrument (the patch probe), and
  a warning that the trap's name is a hypothesis rather than a diagnosis.
- The oracle's 1794 per-page lines are byte-identical with and without the mask fix, so
  `doc/todo/00` step 7's sweep cannot have moved: no ambiguous page's ink changed.
