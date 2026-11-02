# 2616 — Every distinct place a requirement failed at is kept to a bound of its own, and six more sites take it

Session 1891, slot 1 of batch 130, an "archive" round.
Status: accepted; built in `crates/pdf-archive/src/finding.rs` (`Named::places`),
`crates/pdf-transform/src/archive/sites.rs` (`places_failed`) and the six sites below.
Context: ADR 2602 (two sites took the whole population), trap 289 (a remedy over the kept prefix),
trap 305 (a lifted cap takes a bound of its own). ADR 2617 is unused unless a later part of this
slot needs it.

## 1. The premise, and where it did not hold

Round 1884 named six converter sites that iterate the thirty-two places `Outcome::Failed` keeps
and ignore `total`: `boundaries::removal`, `external::external_stream_data`,
`remedies::embedded_streams`, `transcode::prepare`, `font_widths::restated` and
`prepare::unicode_fonts`. The brief's hypothesis was that `sites::page_resources`' survey would
serve four of them and the survey's font set the other two. It does not: none of the six is about
a page's content streams, which is the population the survey holds. Four read only the object a
place names, and `pdf_archive::Named`'s object set would have served them; two read more than the
object — the boundary site a page and the entry it failed at, the embedded-file site a file
specification and the `/EF` key under which the failing stream sits — and `Named` keeps neither.

## 2. What is kept, and its bounds

`Named` now keeps, beside its object set, **every distinct place** (`Where`: object, page, name)
in the order first found, so a remedy does its work in the order a remedy over the kept prefix
did and an output's object numbering is unchanged where the population was within thirty-two.
The list is bounded twice and says when either bound stopped it (`places_cut_short`):
`MAX_NAMED` (4096) places, and `MAX_PLACE_NAME_BYTES` (1 MiB) of their names, because a name can
be a producer's string and 4096 places alone do not bound the bytes. One helper,
`sites::places_failed`, hands a site the list or refuses with `PLACES_CUT_SHORT`. All six sites
read it; one helper serves six rather than four.

## 3. The bounds the lift needs at each site (trap 305)

- **Boundaries**: the pages are enumerated once (`Pages::every`) rather than looked up per place,
  and each page asks which object states each removed entry once rather than once per holder;
  the holders are now up to 4096 where they were up to 32.
- **External data** and **embedded files**: each stream is taken once by a set, not by a scan of
  what is held, which over thousands of places was quadratic. An embedded-file place list cut
  short leaves the site unanswered, so the requirement's own refusal stands.
- **Widths**: `MAX_READ_WIDTHS`, 256 entries for each of 4096 fonts, counts the entries read,
  because fonts can share one `/Widths` array by reference and each font's reading and copy of a
  long one would cost fonts times the array.
- **JPEG 2000 transcode**: `MAX_TRANSCODED_SAMPLES` is thirty-two images at the ordinary
  decoder's largest picture, which is what the site decoded at most while it read the prefix, so
  no document within the old population is refused by it.
- **`/ToUnicode`**: the survey is read once into a map by object, where each font searched the
  survey's fonts and a part 2 restatement walked the whole document again per font.

## 4. Why this is not a raised `MAX_KEPT`, and what it costs

The brief kept `MAX_KEPT` out of this slot because the report's list is the reader's and stays
bounded. It does: `Outcome::Failed { places, total }` is still built from `Findings::kept`, the text
report still prints at most thirty-two places and "… and N more", and `Judgement::named` is read only
by converter sites, never printed or serialised. What changed is the bound a *remedy* reads: where it
read thirty-two places it now reads up to `MAX_NAMED` (4096), which is ADR 2602's bound for the
object set beside it, so the two populations a remedy may be handed have one ceiling.

The cost is one `Where` held twice (the ordered list and the set that keeps it distinct) and its name,
per distinct place. Measured by `getrusage` (peak resident set of the converter's process tree) on a
document of 4096 pages each failing section 6.1.13 at an unauthorised crop box, so that both trees
refuse at the same decision after the same check: HEAD 31 804 to 32 640 KiB, this tree 33 016 to
33 224 KiB over three runs each, about 1 MiB more for 4096 places, a quarter of a kilobyte a place.
With every requirement of a file failing at 4096 places this is a few hundred mebibytes in the worst
case, which `MAX_PLACE_NAME_BYTES` keeps from becoming gigabytes through long names.

## 5. Evidence

- **The fixtures past thirty-two** (`tests/archive/whole_population.rs`): 40 pages with an over-sized
  crop box (80 findings), 40 forms stating `/FFilter` and no `/F`, 40 fonts with a `/Widths` array too
  long, 40 Type 3 fonts drawing `uni0041` with no `/ToUnicode`, each converting and held to the
  target again; all four fail at HEAD. The embedded-file and JPEG 2000 sites read only
  `places_failed`, whose bound is tested at 4096 and 4097 places (`sites::tests`).
- **Each bound at its bound**: two fonts sharing one array of 524 288 entries convert and three are
  refused with `WIDTHS_OVER_BUDGET`; `with_samples` at `MAX_TRANSCODED_SAMPLES` and one past it; the
  place list at 4096 places and at a mebibyte of names (`finding::tests`).
- **The plant**: `MAX_KEPT` at 2 in an export: every whole-population test passes; the one red,
  `statement::a_halftone_statement_that_cannot_be_composed_says_why_beside_the_discard`, is the
  halftone sites, which refuse by name when the kept list was shortened — outside these six, and the
  refusal is the designed one.
- **1 000-item hostile inputs, `getrusage` beside HEAD** (debug profile, peak resident set, exit):
  boundaries 22 736 KiB exit 4 / 21 672 KiB exit 0; external data 20 388 / 20 924; widths (1 000
  fonts sharing one 1 000-entry array) 23 584 / 24 192; `/ToUnicode` 30 964 / 34 024; embedded files
  at 4f under a `supply` table 25 960 / 26 576; JPEG 2000 (1 000 bare codestreams, `keep-everything`)
  26 084 / 35 108. HEAD refuses every one with the requirement still failing past the kept places;
  this tree converts every one.
- **veraPDF** on the freed outputs: PASS at 2b (four), 2u (one), 4 (four), 4f (one); the planted
  output, one page's media box narrowed to 2 units, FAIL.
