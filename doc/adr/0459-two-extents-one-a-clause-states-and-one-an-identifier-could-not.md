# ADR 0459 — Two extents: one a clause states, and one an identifier could not

Status: accepted, 2026-08-20. Session 625. Takes `doc/todo/03` §19's named successor — the next
chunk of the SafeDocs crawl — and fixes the two defects its ten thousand documents produced.
Amends §9.9's and §8.7.3.1's ledger rows.

## The chunk, and why these archives

§19 leaves "43 944 crawled documents unranked … in archive-sized pieces". This round took **ten
whole archives — `0669`, `0915`, `1530`, `2391`, `3129`, `4113`, `5220`, `6204`, `7311` and
`7926`, 10 000 documents** — none of the twenty-two sessions 603, 613, 615 and 619 ranked.
*Which* archives is immaterial and that is ADR 0261's finding: the crawl is sorted by SHA-256 and
cut into pieces, so an archive is a hash bucket and any set of them is an unbiased sample.

The instrument is 603's, unchanged and reused rather than rewritten: page one at 72 dpi from this
tree and from `pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box (trap 3),
ranked by our ink minus the lightest live reference's with each panel's raster size beside it.

**It was checked against the previously-fixed documents before it was trusted**, which is 619's
discipline and 619's warning: the sandbox worker was built explicitly
(`cargo build --release -p pdf-sandbox --bins`) beside the example, and all **26** documents named
by ADRs 0438, 0448, 0451, 0454 and 0456 reproduce their recorded numbers to the thousandth —
`0100223.pdf` −0.158, `3252105.pdf` −0.215, `2022009.pdf` −0.105, `5589519.pdf` +0.713 and the
rest.

## What the two ends said

**The negative head is two defects of this tree, and both are about an *extent*** — where
something ends, and which thing owns the answer. One is stated by a clause and was being decided
by a filter; the other cannot be stated at all by the quantity that was deciding it.

**The positive head is 613's finding and not ours**, down to about +20: `poppler` alone drawing
almost nothing while `mupdf`, `ghostscript` and this tree agree — eight of the ten deepest
positive rows are exactly that shape — which is a note in `doc/traps/oracle-and-references.md` and
was read there rather than derived again. `2391466.pdf` at +23.749 is the other half of that note:
`ghostscript` rendering a *different page box*, 612 × 792 against the 504 × 360 the other three
agree on, which is trap 3 arriving as a size.

## 1. A program the standard measures and a filter was measuring instead

`0669424.pdf` at **−7.223** is a text page — ours 2.774 against 14.217 / 9.997 / 14.160 — that
reports three fonts and draws none of them:

```
font /F14 could not be parsed: /FontFile2 decoded only as far as its damage
(Truncated, 87764 bytes): a prefix of a font program is a directory describing bytes
that are not there
```

941 text operations were lost to it. **87764 is the stream's own `/Length1`**, exactly, and so are
the other two files' 62548 and 61016. Each of the three `/FontFile2` streams is a whole TrueType
font — thirteen tables apiece, every one of them ending inside the decoded bytes — whose Flate data
simply stops before RFC 1951's final block.

ADR 0343's rule is right and stays: a prefix of a font program is a directory describing bytes that
are not there, and reading one draws glyphs the producer never wrote. What it presumes is that the
bytes *are* a prefix, and §9.9's Table 125 says whether they are:

> Length1 | integer | ( Required for Type 1 and TrueType font programs ) The length in bytes of the
> clear-text portion of the Type 1 font program, or the entire TrueType font program, after it has
> been decoded using the filters specified by the stream's Filter entry, if any.

The entry is stated in **decoded** bytes, which is what makes it usable here rather than merely
true: a decode that produced that many bytes has produced every byte of the program, and what
stopped short is the *filter's* own end-of-data marker, which is outside the program. That is ADR
0356's rule read one clause along — ask whether the standard states the thing's extent before
asking whether a filter failed.

`program::whole_program` asks it, and `program::stated_extent` is the clause: `/Length1` alone for
`/FontFile2` ("the entire TrueType font program"), the sum of the three sections for `/FontFile`,
and **nothing at all for `/FontFile3`**, of which §9.9 says the three lengths "are not needed in
that case and shall not be present". A compact program short of its data is still a prefix and is
still refused. `sfnt::truncation` is untouched and still catches a directory that overruns the
bytes that arrived, which is the structural half of the same question.

**And the length is not the whole condition — the corpus said so before any gate did.** The first
version of this fix asked about the extent and nothing else, and `cargo nextest run --workspace`
came back with `silent_fonts::a_font_that_draws_none_of_its_codes_is_reported` failing on
`issue13316_reduced.pdf`, which is ADR 0343's own witness. Its `/FontFile2` decodes to **168 808
bytes under a `/Length1` of 168 808** — it reaches its stated extent *exactly* — and read as a whole
program it draws **A C E F** where the file's six CJK glyphs belong.

The two damages are not two grades of one thing, and `Damage`'s own documentation already said so.
`Truncated` is the encoded data running out before the filter's end-of-data, and every byte it
produced is what the producer's compressor emitted from bytes the producer wrote — §7.4.1's
"convert the information back to its original form", achieved as far as it goes. `Corrupt` is the
input violating the filter's grammar at a definite point, past which nothing is the producer's.
Table 125 says how many decoded bytes the program is and says nothing about whether they are the
right ones, so **both conditions are asked**: the damage is a truncation, and the extent is reached.
`0669424.pdf` is `Truncated` three times over; `issue13316_reduced.pdf` is `Corrupt`.

**The population is measured, over the whole crawl, with an instrument that is not this tree's**
(trap 8): a Python walk over all 65 944 documents that finds every embedded font stream whose Flate
data ends before the final block and compares what `zlib` produced against the lengths the
dictionary states. **140 streams in 8 documents**, 138 of them `/FontFile2` and two `/FontFile`;
**every one of the 138 reaches its `/Length1`**, and the two `/FontFile`s fall *short* of
`/Length1 + /Length2 + /Length3` — 8923 against 12882 and 9838 against 10639 — so the Type 1 arm is
exercised in the negative direction by a real document and `7557616.pdf` stays refused.

Six documents move, all from a deficit to agreement:

| | before | after |
|---|---|---|
| `6942406.pdf` | −15.171 | **−0.033** |
| `6696243.pdf` | −8.218 | **+0.058** |
| `0669424.pdf` | −7.223 | **+0.181** |
| `4100967.pdf` | −6.410 | **+0.112** |
| `7680832.pdf` | −2.645 | **+0.212** |
| `3990014.pdf` | −0.551 | **−0.080** |

**Two of the six are in archives an earlier chunk ranked** — `6696243.pdf` in 615's `6696` and
`7680832.pdf` in 603's `7680` — which is the fourth round running that a fix has reached back.

## 2. A clip the cell built, decided by where its identifier sat

`4113230.pdf` at **−112.626** is the deepest row of the ten thousand and is silent: 162 commands,
no report, ours 44.712 against 157.601 / 157.338 / 157.821. The three references agree to half a
level and draw a photograph of a car's interior; this tree draws a photograph of a building.

The page fills one path twice, with two tiling patterns in turn. Both patterns are the same
`/BBox [0 0 1536 864]` under the same `/Matrix [0.62562 0 0 0.62597 -0.96 540]`, and each cell is
one `Do` of one full-bleed image. The building is the first fill; the car is the second and should
cover it.

**The diagnosis is from the artefacts rather than from the dictionary** (trap 9's last bullet), and
it took bisecting the content stream at unchanged byte length, which is session 621's method:

- swapping the two pattern names moves the page to 157.202, so *each* fill draws correctly alone;
- pointing the first fill at an undefined pattern gives 157.202 and pointing the second at one
  gives 44.712, so the second fill contributes **nothing**;
- nudging the second fill's path by one unit — `960 540 l` → `960 538 l` — makes it draw, so the
  loss depends on the two paths being *identical*;
- replacing the second cell's `Do` with a plain red rectangle changes the page's ink not at all,
  so it is not about the image;
- and the display list holds 162 commands either way, fifteen of them the second tiling's, so the
  interpreter built the marks and something later dropped them.

What drops them is `pdf_render::Cell`. Every site of a tiling is the cell's commands displaced, and
a clip the cell *built* has to be displaced with them while a clip that was already in force is
shared — `Displaced` decided which by comparing the clip's identifier against the number of clips
the list held when the cell began. **`DisplayList::add_clip` interns**: it hands back the identifier
of an equal clip already in the table. So the second cell's box — the same rectangle, the same
matrix, the same parent — was handed the *first* cell's identifier, which is below that mark, and
was read as a clip that was already in force. Every site of the second tiling therefore kept the
first cell's first-site box; that site is off the top of the page, so the second photograph painted
nothing and the first stayed visible under it, in silence.

The question is provenance and it is asked as one now. `Cell::drawn` is given the clip the tiling
was drawn *inside*, and `Displaced::is_the_cells_own` answers by walking **that** clip's chain: the
clips in force are exactly the base and its ancestors, a short closed list, and every other clip a
cell's commands name is one the cell put there. That is true however an identifier was minted,
which is the property a position cannot have. `Mark` no longer counts clips at all; it still counts
soft masks, because `add_soft_mask` appends unconditionally and a position is a sound answer there.

**The obvious inverse — "does this clip descend from the base" — was written first, and the oracle
caught it.** `issue8565.pdf` went newly contradicted at 63.57 nearest against 63.80 furthest: a
soft mask's group is interpreted in a clip context of its own, so a clip the cell genuinely built
inside one descends from nothing the tiling was given, and the narrow test left it where the first
site had put it. Asking which clips were *in force* is the same question from the side that can be
enumerated, and the page agrees again.

§8.7.3.1's own picture is why this is a defect rather than a saving:

> the effect is as if the figure were painted on the surface of a clear glass tile, identical
> copies of which were then laid down in an array

A copy that keeps another tile's boundary is not an identical copy of anything.

`4113230.pdf` −112.626 → **−0.103**, ours 157.235 against 157.601 / 157.338 / 157.821.

## What each fix is pinned by, run against the defect first

Trap 13, both times — the test was watched to fail against the tree before the fix and to pass
after, with the other tests in its module unmoved.

- **`a_font_program_that_reaches_its_stated_length_survives_a_truncated_filter`**, with
  `a_font_program_short_of_its_stated_length_is_still_refused`,
  `a_compact_font_program_has_no_stated_extent_and_is_refused` and
  `a_corrupt_program_of_the_stated_length_is_refused` as its three negative twins — the last of
  which is `issue13316_reduced.pdf`'s rule on a fixture, and the only one of the four a length test
  alone would let through. The fixture needs no compressor: RFC 1951's stored block is a header
  byte, a length and its complement, so `zlib_stored` writes the program as one non-final stored
  block with no adler32 after it — every byte present, the filter's end never written, which is
  `0669424.pdf`'s shape exactly; the corrupt twin follows that block with a header whose type is
  RFC 1951's reserved `11`, so the decode reaches the stated length and *then* meets a grammar
  violation, and asserts that the refusal names `Corrupt` rather than a shortfall. Calibrated
  twice: making `stated_extent` answer `None` fails the first test with the document's own message
  and leaves the others passing, and dropping the truncation condition fails the corrupt twin —
  which is how the *first* corrupt fixture was caught passing for the wrong reason, its NLEN
  corruption having stopped the decode short of the length as well.
- **`a_second_cell_stating_the_first_cells_box_still_moves_it`**, which builds two cells in
  succession whose boxes are equal and asserts that the table interned them (`second_box ==
  first_box`) before asserting that the second tiling's copy still gets a clip of its own.
  Calibrated by restoring `Mark`'s clip count and the position test: this test fails —
  `ClipId(1) == ClipId(1)` — and the module's other four pass.

## What moved over the corpus

Thirty-two archives, 32 000 documents, ranked whole before and after with the same instrument and
diffed row by row: **39 rows move**, and they divide.

**Four are documents one of the two fixes is about**, and two of the four are in archives an
earlier chunk ranked:

| | before | after | fix |
|---|---|---|---|
| `4113230.pdf` | −112.626 | **−0.103** | the cell's clip |
| `6696243.pdf` | −8.218 | **+0.058** | `/Length1` (615's archive) |
| `0669424.pdf` | −7.223 | **+0.181** | `/Length1` |
| `7680832.pdf` | −2.645 | **+0.212** | `/Length1` (603's archive) |

The font fix's other three witnesses — `6942406.pdf` −15.171 → −0.033, `4100967.pdf` −6.410 →
+0.112 and `3990014.pdf` −0.551 → −0.080 — are in archives nobody has ranked, so they were measured
one at a time rather than through the sweep.

**Twenty-three are tiling-pattern pages moving by at most 1.34**, every one of them silent and
every one carrying more than one `PatternType 1`: eighteen move toward agreement, the largest
`3375503.pdf` at −1.307 → +0.033, and five move away by at most 0.64 (`3129707.pdf` −0.352 →
−0.988). That is the same fix on pages where the lost pattern was a hatching rather than a
photograph.

**The other twelve are the instrument rather than the tree, and that is measured rather than
asserted**: nine have our own panel identical to the thousandth with a *reference* panel differing
between the two runs, and three had a panel absent from the earlier run altogether. Thirty seconds
is the per-renderer bound and a sixteen-way run on a loaded machine is what these are;
`doc/traps/oracle-and-references.md` carries the shape and session 619 measured it before.

## What the head still holds, named rather than taken

- **`7926872.pdf` at −41.731** is the module comment of `pdf_model::inline_image` coming true:
  answer 3, the forward search for `EI`, is "the one guess in the module … wrong exactly when the
  compressed bytes contain a whitespace-`EI`-delimiter sequence". The image is
  `/W 1200 /H 1790 /CS /RGB /BPC 8 /F /FlateDecode` with no `/L`, and the first `EI` token stands
  24822 bytes into 2.9 MB of Flate, so 477 217 samples of 6 444 000 are drawn and the rest of the
  photograph is tokenised as operators. **The clause has a third answer nobody has used**: §8.9.7
  makes the bytes "a stream object's data", and every filter it admits states its own end-of-data
  — RFC 1951's final block, §7.4.4.2's code 257, §7.4.5's length 128 — so a *filtered* extent is
  derivable rather than searchable. `pdf_syntax::Pump` already carries a `consumed` count on its
  Flate engine and does not expose it; `DCTDecode` and `CCITTFaxDecode` do not go through it at
  all, which is what makes this a round of its own rather than a line.
- **Five silent rows diagnosed no further than their numbers**, each in `doc/todo/03` §20 so the
  next round does not re-derive them: `6204475.pdf` −12.710, `5220184.pdf` −8.911,
  `3129942.pdf` −6.879, `0915159.pdf` −4.244, and the two positives where three references agree
  and we are far above them, `1530098.pdf` +47.699 and `7926547.pdf` +44.797. A structure count is
  evidence about where to look and never about who is right (trap 9), so none of them is called a
  family here.
- **`1530064.pdf` −15.950 is `doc/todo/49`'s** — `MAX_TILES` reached, and a stroke whose colour is
  a tiling pattern, which §8.7.3's row already prices.
- **65 rows of the 10 000 produce no number**, the same three shapes 613, 615 and 619 opened by
  hand.

## The clauses

`spec-errata emit` over the specification PDFs has **no annotation on §9.9 at all**, and two on
§8.7.3.1 — an "(implementation dependent)" insertion and a one-word caret — neither of which
touches the cell, its box or its lattice. Both ledger rows are amended with the reading above;
§9.9 stays `implemented` and §8.7.3.1 stays `partial` for the stroke §8.7.3's row already names.
