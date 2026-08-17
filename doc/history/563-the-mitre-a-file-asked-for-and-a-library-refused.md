# 563 — The mitre a file asked for and a library refused

**Finding.** `doc/todo/11` §6's prediction was wrong in the way that matters. It said a fix for
`LargeMitreLimit.pdf`'s bevelled mitre "is three strokers rather than one", because Vello and quorra
"have their own strokers with their own thresholds"; the ladder that did not exist when that was
written says **both device backends already draw the tip within a pixel of §8.4.3.5's own
arithmetic**, at every ratio the file's limit admits, and exactly one stroker was wrong.
`tiny-skia`'s `dot_to_angle_type` classifies a join by a normals' dot product before it reads the
limit, so its `SCALAR_NEARLY_ZERO` of `1/4096` is the mitre-length ratio `1/sqrt(1/8192)` = 90.51 in
disguise. So this is ADR 0226's shape — the oracle being brought up to the device — rather than a
three-backend change: `pdf_render::mitre_wedges` states which joins the limit admits and where each
tip goes, from the clause's formula alone; `render-cpu` passes in the one ratio that is a fact about
its library and fills the bevelled outline with the mitres appended in **one** path; and the gate
holds all three backends to the closed form rather than to each other's pixels.

**Date.** 2026-08-17.
**ADR.** [0398](../adr/0398-the-mitre-a-file-asked-for-and-a-library-refused.md).
**Touched.** `crates/pdf-render/src/mitre.rs` (new — the clause, the wedge, the join walk),
`crates/pdf-render/src/lib.rs` (two exports),
`crates/render-cpu/src/lib.rs` (`draw_long_mitres` and `BEVELLED_BY_THE_STROKER`),
`crates/render-quorra/tests/mitre_limit.rs` (new — the gate, on all three backends),
`crates/render-quorra/examples/mitre_ladder.rs` (new — the instrument),
`crates/pdf-model/examples/long_mitre_census.rs` (new — the population),
`doc/conformance/ledger.toml` (§8.4.3.5, §8.4.3.4),
`doc/todo/11-shapes-that-still-disappear.md` (§6 closed, item 4's stroke line corrected, header),
`doc/todo/README.md`, `doc/adr/0398-*` (new), this file.

## The clause, and the closed form everything here is written against

§8.4.3.5 bounds "the ratio of the miter length to the line width" and converts the join to a bevel
when the limit is exceeded; the ratio is `1 / sin(φ/2)`, which `doc/md/` cannot carry — its line
reads `formula-not-decoded`, and `pdftotext -layout` over the PDF prints it. The clause's own EXAMPLE
checks the reading (1.414 under 90°, 2.0 under 60°, 10.0 under approximately 11.5°), and its NOTE is
one line: "Very large miter lengths are allowed." The ratio spans the whole join, so **the tip sits
`(w/2) / sin(φ/2)` from the vertex**. On the witness's join φ = 0.687516°, the ratio is 166.676 and
the tip is 833.378 units above it.

## The population, which nobody had

`long_mitre_census` over first pages: **2 of 1441** — the two `LargeMitreLimit` files, 8 strokes,
sharpest ratios 166.677 and 111.125, none dashed and none at or under a device pixel, which are the
two cases the construction declines. Nothing in the 974 pdf.js documents, the fourteen
specifications, `format-corpus`, `pdf20examples` or `pdfbox`. **So no gated page moves, and the
verification is the clause's arithmetic rather than the corpus** — which the gates below confirm
rather than assume.

## The cost

Callgrind, `RAYON_NUM_THREADS=1`, one arm each side of the render-cpu diff:

| page | before | after | |
|---|---:|---:|---:|
| `issue12295.pdf` p1 × 3 — 65 859 strokes, the corpus's stroke-heaviest | 8 172 069 281 | 8 172 464 967 | **+0.005%** |
| ISO 32000-2 p101 × 20 | 5 514 759 861 | 5 514 875 808 | **+0.002%** |

The reason it is that small is the entry test rather than luck: a join needing a wedge has a ratio
over the caller's threshold *and* at or under the file's limit, so `mitre_wedges` answers a stroke
whose `M` is at or under 90.51 without walking the path, and Table 51's initial limit is 10. The
first row also says something about itself — `issue12295.pdf`'s strokes are all under a device pixel,
so they take §10.7.4's substitution and never reach this test at all; the second is the honest one
for a page of wide strokes.

## Every gate

- `cargo fmt --all --check`: silent. `cargo clippy --workspace --all-targets`: silent — **and it
  was not on the first run**: the new files carried six lints (`neg_cmp_op_on_partial_ord`,
  `manual_midpoint`, `manual_let_else`, two `useless_conversion`, `type_complexity`) and three
  unfulfilled `cast_precision_loss` expectations. Fixed here, and worth the sentence because a
  `#[expect]` that is not needed is a lint of its own.
- `cargo nextest run --workspace`: 2088 → **2096**, all passing, 15 skipped. The eight are six unit
  tests in `pdf-render::mitre` — the tip's position, the limit's conversion, the caller's own
  threshold, the three cases the clause excludes, a join that doubles back, and the property that
  ties this module's join walk to `outline.rs`'s — and two cross-backend scenes.
- `cargo test --workspace --doc`, `cargo test -p conformance`: pass.
- **corpus**: `974 documents in 3.5s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 65
  incomplete, 0 slow`, with the three silence lines at 5 codes over 2 documents, 57 over 9 and 1226
  over 41. Every field unchanged.
- **oracle**: `agrees` 906/862, `contradicted` 67/66, `ambiguous` 786/753, our geometry 1/0,
  reference geometry 2/2, `not comparable` 13/7, `no render` 19/0 — **no verdict moves**, which the
  census predicted before the run: no gated document states a mitre this construction touches.
- **text_extraction** (two gates): PDFBox `overall 99.8% (14257/14281 words)` against both orders,
  4 below 90%; pdf.js `overall 99.2% (22836/23015 words)`, 22 below 90%. Both unmoved. The word-box
  gate: `10969/11163 matched words in bounds (98.26%)`.
- **dates**, **xmp**, **jpeg2000**: pass.
- **render-quorra corpus**, default lane: `956 pages compared in 39.7s: 931 agree, 23 differ, 2
  refused, 18 not comparable`. Unchanged.
- **quorra `gpu` lane at 4×**: `951 pages compared in 446.0s: 937 agree, 10 differ, 4 refused, 23
  not comparable`, ratchets off as that lane always reports. Unchanged.
- **`doc/todo/00` step 7's ink sweep**, over the gate's own 786 `ambiguous` lines: **786 measured,
  0 skipped**. Twenty at or past −1 and **sixteen of them documents this tree calls incomplete**;
  the four complete ones are `issue16038.pdf` −5.655, `issue12295.pdf` −2.827, `issue14297.pdf`
  −1.129 and `issue7821.pdf` −1.000, all four diagnosed and all four within a hundredth of the
  five-hundred-and-fourteenth's run bar `issue16038.pdf`'s 0.08. The alarm holds. Nothing this
  round drew could move a line here — no ambiguous page has such a join — and the run is what says
  so rather than the argument.

## What the two witnesses look like, because a page nobody has looked at is trap 1

Both `LargeMitreLimit` documents were rendered at scale 1 and **looked at**. Four spikes each, rising
out of the joins at the page's own `Y = 0` and thinning to nothing near the tip; the article's grid
lines every 100 units are what they are measured against, and the tips land where
`(w/2) / sin(φ/2)` puts them — 833 on the straight-line file, 556 on the Bézier one. Beside them:
`mutool` reaches 819 and `ghostscript` 810 by the same crude colour test that gives ours 823, all
three converging on the arithmetic from below because the last units of a spike are thinner than one
level of 255; `poppler` puts its highest ink at the join itself on the first file.

## The discrimination check

`git apply -R` of the `render-cpu` half alone, with the gate and the clause untouched:
`every_backend_draws_the_mitre_the_limit_admits` fails at its first assertion with the processor at
**0.00 of the wedge's own 4166.97 device pixels**, while
`every_backend_bevels_a_ratio_over_the_limit` still passes — so the two tests fail in opposite
directions and neither is satisfied by "draw a spike whenever a join is sharp".
