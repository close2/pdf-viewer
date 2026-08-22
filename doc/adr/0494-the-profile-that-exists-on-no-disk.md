# ADR 0494 — The profile that exists on no disk

Status: accepted, 2026-08-22. Session 668. Rewrites `CONTRADICTED_CALRGB_TO_SCREEN`'s note around
measurements; amends the §10.3.1 and §8.6.5.3 ledger rows and traps 9 and 12. **No pixel moves and
no list changes.**

## How the group was chosen, and the criterion is the fifth

Four rounds have worked the oracle's contradicted pool and each left a better way of choosing than
it found: 643 wrote a page's closed form and ranked five renderers against the geometry; 651 found a
group's *name* holding while everything under it was wrong; 656 asked how many of a group's own
members its note measures; 662 asked how many clauses of ISO 32000-2 its note cites. That last one
is a principle-5 question about one of the two claims a contradicted verdict makes. **This round
audits the other one.**

> A contradicted verdict says two things: that the *standard* rather than the consensus decides the
> page, and — ADR 0005's premise — that **the agreement which outvotes us is evidence**. 662 audited
> the first. For each group, does the note name a mechanism for the two voting references agreeing,
> and is that mechanism *verified* rather than asserted?

`doc/traps/oracle-and-references.md`'s trap 9 is the catalogue of ways an agreement can be
manufactured, and its own fourth bullet says the sentence "this is trap 9's family" is a hypothesis
rather than a diagnosis. So the audit has three outcomes, not two. Read over all fourteen non-empty
`CONTRADICTED_*` lists:

```text
  verified against a binary, a source file, a data file, a log or a ladder   10
    NEGATIVE_LINE_WIDTH        a ladder at nine widths and nine angles
    DEVICE_CMYK_CONVERSION     md5 byte identity, objdump, two profiles' desc tags
    SHARED_JBIG2_DECODER       objdump, both programs' logs, the corpus's own invariant
    VISIBILITY_EXPRESSION      the FIXME and the WARNING, in both sources
    REFERENCES_DREW_NOTHING    both logs, and gs's only with -q removed
    ON_A_PAGE_WE_REPORT        three logs saying three different things
    LINK_BORDER                mupdf's switch, and gs redrawn with /F 4
    SUBSTITUTED_FONT           fontconfig, and the two font programs' own charstrings
    GLYPH_EDGES                FreeType's hinting, via a page that draws one glyph twice
    TIGHT_CONSENSUS            §10.7.4's clip-as-a-set, split by row and column

  named and inferred from the picture                                         3
    IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE   "a consensus that averages" — poppler's pixel is
                                       measured, mupdf's is not
    SUBPIXEL_IMAGE                     "the shape rule applied to an image", from a
                                       coverage table
    REFERENCE_GLYPH_WIDTHS             "the /DW default of 1000", from ink columns

  no mechanism at all                                                         1
    CALRGB_TO_SCREEN           "they are contradicted instead of ambiguous only because
                               the two references that share a camp happen to agree"
```

**One group of fourteen calls the agreement its verdict rests on a coincidence**, in a note that
otherwise measures a great deal. That is the group. The criterion is spent by being applied, like
its four predecessors, and the next round owes a new one.

## The five pages, and what actually separates the camps

`calrgb.pdf` pages 1, 5, 11 and 12 are one 850 × 1100 sheet four times — eighty `/CalRGB` swatches
under a header, `/WhitePoint [1 1 1]`, `/Gamma [1 1 1]`, the identity `/Matrix`, differing only in
`/BlackPoint` — plus `issue9940.pdf` page 1, whose space is a D65 white with gamma 2.2 reached
through an `/Indexed` `/DeviceN` alternate.

**The four are the only pages of that document's seventeen where §8.6.5.3 has nothing left to
decide.** With all three transformation entries the identity, the space *is* XYZ; every page where
`/Gamma` or `/Matrix` does real work is `ambiguous` or agrees. So the split is downstream of
everything the cited subclause defines, which is a fact about the file rather than an inference.

### Ranked against the closed form

Bradford-adapt the stated `/WhitePoint` onto sRGB's white, apply IEC 61966-2-1's matrix and transfer
function — written in a script holding the published constants and none of this crate's code, and
sampled at all eighty swatch centres, in levels of 255:

| | mean | worst swatch |
|---|---|---|
| `poppler` | **0.013** | 1 |
| ours | **0.025** | 1 |
| `hayro` | 2.150 | 8 |
| `ghostscript` | 4.300 | 15 |
| `mupdf` | 4.838 | 31 |

Two implementations sharing no line of code are on that arithmetic to one level of 255 on every
swatch, and the two that vote are the two furthest from it. §10.3.1 leaves the *destination* open,
so this ranks rather than conforms — what it establishes is that the camps are a fact about
implementations and not a coin toss.

### The mechanism, from the binaries

- **`ghostscript`** carries `gsicc_create_from_cal`, `./base/gsicc_create.c` and the parameters
  `CalRGBProfile`/`CalGrayProfile` among its internal names — strings rather than symbols, since
  `libgs.so.10` exports 242 dynamic symbols and no `gsicc_*` is among them; `NEEDED liblcms2.so.2`,
  22 `cms*` undefined.
- **`mupdf`** *exports* `fz_new_cal_rgb_colorspace` and `fz_new_icc_data_from_cal`, carries the
  message *CalRGB profile creation failed; bad values*, asks for no colour library and **defines
  437 `lcms2mt_*` symbols** — Artifex's fork of Little CMS, compiled in.
- **`poppler`** has its own `GfxCalRGBColorSpace` (the typeinfo name is in the binary) and exports
  `make_GfxLCMSProfilePtr`, the entry to `liblcms2` for an `ICCBased` stream.

So the two that outvote us **synthesise an ICC profile from Table 63 and run it through Little
CMS**, and they are one house's two programs over two builds of one CMM. That `ghostscript`'s
default is the managed path and not a reading of the components is checkable in one run:
`gs -dUseFastColor=true` paints the component times 255 with no transfer function at all — 3 for
`0.01`, 127 for `0.50 0.50 0.50` — a mean of 75.51 of 255 and a worst pixel of 173 from its own
default rendering of the same file.

### The profile, obtained

`gs -sDEVICE=pdfwrite` writes the page back with the `/CalRGB` array replaced by an `ICCBased`
stream, so `gsicc_create_from_cal`'s output is **585 bytes on disk**: a version 4.2 `scnr` profile,
RGB to XYZ, `wtpt` the D50 connection white, three `curv` TRCs of gamma 1.0, and colorants

```text
  rXYZ = [1.006409, 0.000000, 0.000000]
  gXYZ = [0.000000, 1.019592, 0.000000]
  bXYZ = [0.000000, 0.000000, 0.827927]
```

A Bradford adaptation of the identity `/Matrix` from `[1 1 1]` onto D50 is not diagonal — it is
`[0.997781, −0.009757, −0.007429]`, `[−0.004152, 1.018325, 0.013463]`, `[−0.029429, −0.008567,
0.818866]`, whose columns sum to D50 exactly. The synthesised profile keeps three numbers, and its
colorants sum to (1.0064, 1.0196, 0.8279) against its own `wtpt` of (0.9642, 1.0, 0.8249) — 4.4%
adrift in X, which is a space whose white does not map to its own white point.

**Then ADR 0048's instrument, on data that was not on any disk until it was asked for.** Over the 72
swatches outside the deepest shadow, in levels of 255:

```text
  ours, /CalRGB                vs ghostscript, /CalRGB    4.15  max 11
  ours, ghostscript's profile  vs ghostscript, /CalRGB    0.07  max  1
```

**This tree, handed the profile `ghostscript` builds out of Table 63, reproduces `ghostscript`'s
rendering of Table 63 to one level of 255.** And the confirmation from the other side — each
renderer's own `/CalRGB` answer against its own answer to the rewritten page:

```text
  ghostscript   0.03  (max  1)    it was already using this file
  mupdf         0.83  (max  3)    its own synthesised profile is effectively this one
  ours          4.17  (max 11)    it moves us onto their answer
  poppler       4.24  (max 11)    and poppler with us
```

The two that outvote us do not move when handed the other's profile; the two that agree with us
both do. **The whole verdict is one 585-byte file**, and it is a file no dependency graph shows, no
digest comparison finds and neither binary contains, because each of the two manufactures it from
the document. Trap 9's eighth mechanism, and the only one for which the tree's existing instruments
— `objdump -p`, the embedded-profile scan, the `desc` tag — all return empty.

### And the agreement is thinnest where the page's difference is largest

Maximum channel difference at each of the eighty swatch centres:

```text
                                  max   mean   swatches over the gate's four levels
  ours        <-> poppler          1    0.01     0 of 80
  mupdf       <-> ghostscript     16    2.35     8 of 80
  ours        <-> ghostscript     15    4.31    36 of 80
  ours        <-> mupdf           31    4.85    34 of 80
```

On the 41 swatches where the camps differ at all, the voting pair is a mean 3.78 apart with a
maximum of 16. At `0.01 0.00 0.00`, which carries the page's largest difference, ours is 50,
`poppler` 50, `ghostscript` 35, `mupdf` 19 — **we are nearer `ghostscript` there than `mupdf` is**.
Their 4.41% is an average over a sheet three quarters of which no camp disputes. Trap 12 gains the
population: a bound derived from an aggregate is not a bound on the pixels the aggregate is made of.

## Was the deciding clause the one the group cited?

**No, and it is one sentence away.** The entry read *the difference is the half of the journey
§10.3.1 puts beyond itself*, and quoted the sentence that puts it there. The next sentence of the
same subclause is a `shall`: conversion from a CIE-based source colour to a CIE-based destination
colour is to be performed based on the appropriate ICC specification. (Prose rather than a
quotation because Errata Collection 3's Issue #181, `Review`/`Completed`, strikes that sentence's
dated *ISO 15076-1:2010 (ICC.1:2010)*; `spec-errata emit` files it under §10.4.1's heading and
§10.3.1's ledger row carries it. `emit` over the whole document files nothing on §8.6.5.3 or
§10.3 at all, so both subclauses stand as printed.) So the *destination* is a choice — Artifex's
own sRGB file for `ghostscript`, IEC 61966-2-1 for us — and the *route* is the referenced
standard's, whose media-relative colorimetric intent adapts the source white onto D50 by the
transform ICC's `chad` tag carries. `colour::BRADFORD` has cited exactly that since ADR 0012 and
had no corpus witness; it has one now.

§8.6.5.3's own sentence was being read for one of its two subjects. It says the `WhitePoint` **and**
`BlackPoint` entries shall control the overall effect of §10.3's gamut mapping function, and this
group quoted it under `/BlackPoint`. On these four pages `/BlackPoint` moves nothing in four
renderers, while `/WhitePoint` is the whole question — because it is what the adaptation is *from*.

That makes it three rounds running, and now four, in which the deciding clause was in a different
row than the group cited.

## Consequences

- `CONTRADICTED_CALRGB_TO_SCREEN`'s note is rewritten around the measurements above; its title now
  states the mechanism rather than the symptom.
- §10.3.1's ledger row gains the corpus witness for its `shall`; §8.6.5.3's records that its quoted
  sentence names two entries and the row read one.
- Trap 9 gains an eighth mechanism and trap 12 the population distinction.
- **Two stale numbers corrected by re-running them**: the entry said `hayro` sits 16.54 of 255 from
  us on page 12, where `examples/compare_rasters` now prints 12.47, and 0.87 on page 1 against
  0.90. Trap 1's cheapest tell, again.
- The verdict is unchanged, no page leaves the list, no pixel moves and the oracle's lists are
  byte-identical.

## Owed

- A criterion for the next round; this one's is spent.
- `mupdf`'s synthesised profile was inferred rather than obtained — `mutool convert` flattens the
  space instead of writing it back — so the claim that it is effectively `ghostscript`'s rests on
  0.83 of 255 and on the symbol, not on two files side by side.
- Neither camp's *shadow* behaviour is explained. Handed one profile, the four still spread 19 to 54
  at the darkest swatch, which is a question about four CMM pipelines rather than about the clause,
  and it is where the page's largest single difference lives.
- 0489's owed item stands: nothing links a group's note to the code or to the gate figures it
  quotes, and this round found two more stale numbers by hand.
