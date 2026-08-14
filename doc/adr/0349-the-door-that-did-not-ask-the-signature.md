# 0349 — The door that did not ask the signature, and the swatch that killed a label

**Status.** Accepted.

## Context

The oracle's **contradicted** bucket is the strongest signal the raster instrument produces: pages
where the reference consensus disagrees with this tree. `doc/HANDOVER.md`'s standing warning about
it is that **a page's group names a hypothesis and has been wrong nine times out of nine**, and
that a label this project wrote is still a label.

Two things about the bucket were true when this round opened and neither had been written down.

**The list is ranked by distance and read by ratio, and the two readings disagree at the head.**
`rank_the_contradicted` prints the ten pages furthest from their *nearest* reference. Ranked
instead by `doc/habits.md`'s own instrument — our worst measurement over the bound it is held to —
the head of the sixty-eight is a page that ranking never prints:

| | ratio | bounds failed | in a group? |
|---|---|---|---|
| `xobject-image.pdf` p1 | **127.75×** | all four | **no** |
| `issue5751.pdf` p1 | **12.66×** | all four | **no** |
| `bitmap-halftone-composite.pdf` p1 | 33.47× | all four | yes |

**And the reason the two are in no group is structural rather than an oversight.** `check_the_ratchets`
filters on `Examined::complete`, so a page on a document this tree *reports* cannot fail the
contradicted ratchet — which is right, and is the module's own stated argument: a page whose
interpretation reports an unsupported font is expected to differ, and `corpus.rs` owns it. What
nobody had noticed is the consequence: **the two largest disagreements on the whole list were
therefore outside every diagnosis in the file**, one of them by a factor of ten over anything gated.

## Decision

### 1. `/FontFile` asks the signature, like the other two doors

`pdf_font::program`'s first paragraph says the reader is chosen "by the bytes' own signature rather
than by the key's spelling or by the `/Subtype` name, because a producer can mislabel a stream and
cannot mislabel a leading `OTTO`". §9.9's ledger row asserts the same of all three of Table 124's
keys. **It was true of `/FontFile2` and `/FontFile3` and false of `/FontFile`**, which took the
pairing from the key and handed every stream to the Type 1 reader.

`issue5751.pdf` is the witness, and it came off the oracle rather than off a re-reading. Its
descriptor is a `CIDFontType0`'s, it writes `/FontFile`, its stream states none of Table 125's
`/Length1`, `/Length2` or `/Length3` — all three "Required for Type 1 font programs" — and its
first bytes are `01 00 04 03` followed by the Name INDEX string `MyriadArabic-Regular`. It is a
bare CFF, which Table 124 puts under `/FontFile3` with `/Subtype /CIDFontType0C`.

So the file makes three statements that disagree with the key it wrote, and the bytes are the only
one of the four a producer cannot fake. `embedded_program` now applies `is_bare_cff` to `/FontFile`
as it already did to `/FontFile3`. Neither of Type 1's own packagings can collide with the test — a
PFA begins `%!`, a PFB `80 01` — so this reroutes a program that could not be read at all and
leaves every readable one where it was.

The clause states no recovery, because the clause is addressed to the file: each row of Table 124
ends "[t]he font program provided as the value of this key shall conform to" the format's own
specification, and ISO 32000-2 says nothing about a reader that meets one which does not. This is
`CLAUDE.md`'s second question — robustness — answered the same way §9.9's row already answers it
for a Private DICT that will not parse.

### 2. Two contradicted pages that no ratchet can hold get a group anyway

`CONTRADICTED_ON_A_PAGE_WE_REPORT` carries the diagnosis for a page this tree reports, held by a
staleness assertion rather than by a ratchet: a name that stops being contradicted fails the build.
The arrival direction needs nothing, because a page that stops *reporting* while still contradicted
joins the gated list and fails there. The module's argument for gating only complete pages is not
reopened; what changes is that its largest casualties are now diagnosed instead of invisible.

`issue5751.pdf` left the list under decision 1. `xobject-image.pdf` stays, and its entry is below.

### 3. `CONTRADICTED_CALIBRATED_COLOUR` is empty, and it is ten for ten

ADR 0296 closed by naming one unmeasured sentence: `issue9940.pdf`'s claim that "`mupdf` and
`ghostscript` take its components for `DeviceRGB`", which had just been disproved on `calrgb.pdf`
and might still have been true of a `CalRGB` reached through an `/Indexed` `/DeviceN` alternate.

It is not true there either, and the instrument is a fixture rather than the page: 100 × 100 points
filled with `0.5 0.25 0.75 sc` in *that file's own* `/CalRGB` dictionary. Both readings are closed
forms taken from the file, with no renderer consulted — §8.6.5.3's decoding gives
`X Y Z = (0.20686, 0.04737, 0.57830)` and IEC 61966-2-1's transform on it gives **(151, 0, 205)**,
while taking the components for `DeviceRGB` gives **(128, 64, 191)**.

| | centre pixel |
|---|---|
| **§8.6.5.3 + XYZ → sRGB** | **(151, 0, 205)** |
| ours | (151, 0, 205) |
| `poppler` | (151, 0, 205) |
| `mupdf` | (166, 0, 205) |
| `ghostscript` | (166, 0, 207) |
| **components as `DeviceRGB`** | **(128, 64, 191)** |

Nobody assumes. Ours and `poppler`'s are the closed form exactly, and the pair that contradicts us
is 15 levels away **in red alone** where the `DeviceRGB` reading would have moved all three
channels by −23, +64 and −14. The page says the same over 484 704 pixels: `R − G` is −2.05 for
ours, `poppler` and `hayro` and +2.02 and +1.72 for `mupdf` and `ghostscript`, with the green and
blue means agreeing across all five to 0.6 of 255.

So the page is `CONTRADICTED_CALRGB_TO_SCREEN`'s mechanism — §10.3.1's "specific method by which
the CIE-based destination colour space is established is beyond the scope of this document" — and
has moved there. **Tenth for ten on a group's name naming a hypothesis rather than a diagnosis.**

## What `xobject-image.pdf` turned out to be

The page that heads the ratio ranking by a factor of ten is 1462 hand-written bytes, and its five
panels are five flat rectangles: ours and `hayro`'s red, `mupdf`'s black, `poppler`'s and
`ghostscript`'s white. Asked in words rather than in pixels, **the three references fail at two
different things**:

```text
poppler  Syntax Error (1274): Missing 'endstream' or incorrect stream length
         Syntax Error: Unknown operator 'endstream'
mupdf    warning: PDF stream Length incorrect
         warning: padding truncated image
gs       Incorrect /Length for stream object ... recoverable image error ... bad DecodeParms
```

The content stream is `500 0 0 400 0 0 cm\n/SomeImage Do\n`, 33 bytes, under a `/Length 14` that
stops in the middle of the `cm`. So `poppler` **never reaches the image at all**: it reads fourteen
bytes, meets `endstream` where an operator belongs, and stops. Its blank page is about Table 5's
`/Length` and about nothing else. `ghostscript` repairs the length, reaches the image and refuses
*that*. The two renderers whose agreement contradicts us agree on white for two unrelated reasons —
trap 9's fourth shape sitting on its second — and `mupdf`, the one reference that both repairs the
stream and draws the image, produces a third picture neither of them produces.

The image itself is the standard's own gap: the XObject states `/Width 200 /Height 100` and its
`DCTDecode` data is a **1 × 1** JPEG of one red sample. §8.9.3 says "[t]he image dictionary shall
specify the width, height, and number of bits per component explicitly" and Table 87 makes both
required; §7.4.8 says the encoder's parameters, "which include the dimensions of the image …, shall
be stored in the encoded data" and that "DCTDecode may obtain the parameter values it requires
directly from the encoded data". In a conforming file the two agree. Where they do not, no clause
states a recovery, and each of the three pictures is one coherent choice:

| | reads the image as | visible result |
|---|---|---|
| ours, `hayro` | the codestream's 1 × 1 grid | the red sample over the whole region |
| `mupdf` | the dictionary's 200 × 100 grid, padded | black — the visible corner is all pad |
| `ghostscript` | neither: it refuses the image | white |

`mupdf`'s own log names its choice and the arithmetic confirms it: the `cm` maps the unit square to
500 × 400 on a 200 × 100 page, so the visible part is source rows 75 to 99 and the one real sample
— row 0, column 0 — falls outside it. This tree's choice is ADR 0340's, and it draws *and* reports:
`corpus.rs` says in words what the picture cannot, that a 200 × 100 picture was described and one
red sample supplied.

## The four sans pages whose cap rows were never measured

`CONTRADICTED_SUBSTITUTED_FONT`'s table had seven pages under "the substitution costs one number"
and cap-row measurements for three of them; the other four carried the diagnosis on their
`/BaseFont` and their ink — the membership rule that group's own history keeps catching. Measured,
one capital per page at 8×:

| page | glyph | ours | `poppler` and `mupdf` | 0.942857 × theirs |
|---|---|---|---|---|
| `bug847420.pdf` | `T` | 77 | 82 | 77.3 |
| `bug850854.pdf` | `B` | 110 | 117 | 110.3 |
| `issue6069.pdf` | `M` | 77 | 82 | 77.3 |
| `issue11403_reduced.pdf` | `E` | 99 | 105 | 99.0 |

Each capital sits on the same baseline as the references' and is short only at the top, which is
what a cap height is and what a smaller point size would not be. **Two had to be measured twice**,
and the reason is `doc/todo/00`'s own warning that a band of rows is a hypothesis about what is in
it: `issue6069.pdf`'s whole-line ink box is 106 rows against 107, no difference at all, because the
line's tallest ink is an ascender and the dot of an `i`; and `issue11403_reduced.pdf`'s leading
`2.` reads 101 against 104, a ratio of 0.971 that fits nothing, because a digit's height is not a
cap height in either face. A page-level box cannot test a per-glyph metric — ADR 0174's lesson in a
different instrument.

## Consequences

- One page of 1794 moved, and the oracle's own before/after says so line by line: `issue5751.pdf`
  page 1 left the contradicted list and agrees; every other page's verdict and every printed metric
  is byte-identical. The corpus gate's incomplete list is one shorter for the same reason.
- `doc/todo/00`'s step 7 was re-run whole over all 786 ambiguous pages, because a round that changes
  what gets drawn owes it. Twenty names at or past −1, sixteen of them documents this tree calls
  incomplete, and the other four are `issue16038.pdf` −5.734, `issue12295.pdf` −2.823,
  `issue14297.pdf` −1.145 and `issue7821.pdf` −1.000 — the same four names in the same order the
  four-hundred-and-forty-fourth session recorded, each already diagnosed. Nothing unexplained.
- The contradicted list is 67 where it was 68, and 66 of those are on complete pages, unchanged.
- **What this round did not do** is move the ranking the gate prints. `rank_the_contradicted` still
  orders by distance from the nearest reference, and the two pages this ADR is mostly about are not
  on it. Ranking by the ratio is one more sort in the same function and is left for a round that
  wants it, with the argument recorded here rather than the code written: the distance ranking is
  the ambiguous bucket's instrument borrowed unchanged, and it is the *bound* rather than the
  distance that says a page is accused.
