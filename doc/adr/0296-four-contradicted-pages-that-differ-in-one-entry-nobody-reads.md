# ADR 0296 — Four contradicted pages that differ in one entry nobody reads

Status: accepted, 2026-08-13. Session 461. Amends `oracle.rs`'s `CONTRADICTED_SUBSTITUTED_FONT`
and the `/BlackPoint` paragraph in `colour.rs::cie_to_srgb`; does not amend ADR 0012, which it
supplies the missing measurement for.

## The question

`oracle.rs` has held `calrgb.pdf` pages 1, 5, 11 and 12 in `CONTRADICTED_SUBSTITUTED_FONT` since
the sixth session, on that group's membership rule — *the page names a font nobody embedded* —
which the group's own first paragraph calls the weakest in the file. What kept them there was two
sentences with no number behind them:

> The four that remain differ by about ten levels in one channel against `mupdf` and `ghostscript`,
> while agreeing with `poppler` exactly. That is a residue of colour management rather than of
> fonts, and small enough that closing it would mean choosing whose arithmetic to copy.

A note that names *another* group's mechanism is a page in the wrong group, and `doc/HANDOVER.md`'s
trap 1 was eight for eight on a contradicted page's group naming a hypothesis. This session measured
them.

## What the file states, which nobody had read

`calrgb.pdf` is seventeen pages of `CalRGB` patches, 850 × 1100 points, so the oracle's raster is
one pixel per point. Each page states its own space in a header and then eighty swatches labelled
with the `A, B, C` that produced them. **The four contradicted pages state the same space in three
of Table 63's four entries — `/WhitePoint [1 1 1]`, `/Gamma [1 1 1]`, the identity `/Matrix` — and
differ only in the fourth**: `/BlackPoint` at `[0 0 0]`, `[1 1 1]`, `[8 8 8]` and `[50 50 50]`.

They are the only four of the seventeen that vary in `/BlackPoint` alone; the eight in
`AMBIGUOUS_CALRGB_TO_SCREEN` vary the white point, the gamma or the matrix, and the remaining five
agree.

## What was measured

Four instruments, none of which trusts a reference.

**1. The four pages are one page.** Below the header — device rows 150 to 1090, `md5` of the raw
RGB — our raster, `poppler`'s, `mupdf`'s and `ghostscript`'s are **byte-identical across all four
pages**. Four renderers read `/BlackPoint` and none lets it move a colour. `hayro` is the only one
that does, and it does not vote: it sits 0.87 of 255 from us on page 1 and **16.54** on page 12.
The gate's own line agrees without being asked — mean 1.38, worst tile 14.16, differing 11.23%,
similarity 0.9908 on three of the four, and the fourth's worst tile is 13.86.

**2. That one moving figure is the header, not the page.** The worst 32-pixel tile against
`ghostscript` is at (192, 0) on all four, which is the line printing the `/BlackPoint` values, and
page 12 prints `[50.00000 …]` where page 1 prints `[0.00000 …]`. So on this page the worst tile is
measuring the label font and the differing fraction is measuring the swatches — and only the second
decides the verdict.

**3. It is not the font.** 76.6% of the page is flat in all five renderers — a pixel whose 7 × 7
neighbourhood is one colour in every one of them — and that region holds no glyph.

| | mean over the flat region | share of the page's total difference inside it |
|---|---|---|
| `poppler` | **0.004** of 255 | 0.5% |
| `mupdf` | 1.677 | 67.0% |
| `ghostscript` | 1.362 | 56.6% |

Against `poppler`, **not one channel of the swatch interiors moves by more than four levels**, and
`poppler` substitutes a *different* serif face from ours: the labels are `/Times-Roman` with no
`/FontFile`, ours is `FoxitSerif` (ADR 0133), the C references resolve `NimbusRoman` through
fontconfig. Two renderers with different faces agreeing to 0.004 of 255 over three quarters of the
page is ADR 0267's serif finding arriving on a fifth document.

**4. Nobody assumes `DeviceRGB` here.** Page 1's space is the identity in all three transformation
entries, so §8.6.5.3's stage is the identity and the file states XYZ directly. A processor taking
the components for `DeviceRGB` would paint `0.75 0.00 0.00` as `(191, 0, 0)`:

| | ours | `poppler` | `mupdf` | `ghostscript` | `hayro` |
|---|---|---|---|---|---|
| `0.75 0.00 0.00` | 255, 0, 62 | 255, 0, 62 | 255, 0, 65 | 255, 0, 66 | 255, 0, 60 |
| `0.01 0.00 0.00` | 50, 0, 2 | 50, 0, 2 | 19, 0, 2 | 35, 0, 2 | 49, 0, 2 |
| `0.50 0.50 0.50` | 188, 188, 187 | 188, 188, 188 | 193, 187, 188 | 196, 187, 188 | 194, 184, 188 |

All five convert. What separates them is the shadow end and the neutral axis, which is where a
white point is adapted and a transfer function applied — §10.3's half, not §8.6.5.3's.

## Two camps of two, and the reference that agrees with us is further out than we are

The gate's differing fraction counts channels moving by more than four levels of 255 over four
channels per pixel, alpha included. Every pair on page 1, in the gate's own units:

```text
  ours        <-> poppler        1.62%     <- the closest pair on the page
  mupdf       <-> ghostscript    4.41%     <- the consensus; the bound is twice it, 8.82%
  ours        <-> mupdf         11.12%
  ours        <-> ghostscript   11.23%     <- the figure the gate prints for us
  poppler     <-> mupdf         11.21%
  poppler     <-> ghostscript   11.65%
```

**`poppler` is further from the consensus pair than we are, on both of its members.** The verdict
"`mupdf` and `ghostscript` agree, we differ" would read identically with `poppler` in our place and
by a larger margin. The gate's printed figure and its printed bound both reproduce exactly from
this table, which is what makes the table a check on the gate rather than a measurement beside it.

## The decision

**The four pages move to `CONTRADICTED_CALRGB_TO_SCREEN`, a new group whose diagnosis is the
clause.** §8.6.5.3 defines components-to-XYZ exactly and all five renderers agree there — on page 1
it is the identity, and the swatch table is what an identity looks like when everybody applies it.
The other half is stated to be open, in §10.3.1:

> The specific method by which the CIE-based destination colour space is established is beyond the
> scope of this document, but may include the use of Output Intents

That is `doc/todo/00`'s shape 3, and it is the same reading `AMBIGUOUS_CALRGB_TO_SCREEN` carries
over eight *other* pages of this same document. These four are that finding at a tighter bound
rather than a second finding: they are contradicted instead of ambiguous only because the two
references that share a camp happen to agree to 4.41%.

**Nothing about the rendering changes**, and that is the outcome rather than a shortfall. This is
the third time the picture has rejected a label rather than finding a defect, and the ninth time a
group's name has named a hypothesis.

## What it also fixed, which was a claim rather than a page

`colour.rs::cie_to_srgb` argues that `/BlackPoint` is read and deliberately not applied, and closed
with "`calgray.pdf` page 3 and `calrgb.pdf` page 14 are the corpus's only examples". **There are
eleven**, all in those two files. `crates/pdf-model/examples/black_point_census.rs` is the command
that counts them — every object the cross-reference table lists, so that a `/BlackPoint` inside an
`/Indexed` base or a `/DeviceN` alternate is counted like one in a page's own `/ColorSpace` — and
over the 964 corpus documents it opens, 21 Cal spaces state the entry at all and 11 state anything
but `[0 0 0]`. `CLAUDE.md`'s rule applies: the number is not written down again, the command is.

And the corpus supplies the A/B the decision never had. `a_cal_spaces_black_point_does_not_move_its_colours`
pinned the choice on `CalGray` only, while its own name and comment said "a Cal space"; it now
carries the four black points `calrgb.pdf` states, on the swatch whose value the rasters give, and
it fails on the first of them when a stretch is reintroduced on the `CalRGB` path — checked by
reintroducing one.

## What this does not settle, and where a round should look next

`CONTRADICTED_CALIBRATED_COLOUR`'s single page, `issue9940.pdf`, says `mupdf` and `ghostscript`
"take its components for `DeviceRGB`". That is not what they do on `calrgb.pdf`, where the identity
swatch above shows all five converting. The claim may still be right about `issue9940.pdf` — a
`CalRGB` reached through an `/Indexed` `/DeviceN` alternate is a different path — but it is now the
only unmeasured sentence in the neighbourhood, and its entry says so.
