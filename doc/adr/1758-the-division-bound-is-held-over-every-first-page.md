# 1758 — The division bound is held over every first page, and the pages past it by name

Session 1461. Status: **accepted** and **built**. Takes up what ADR 1742 section 5 left: whether
`strip_parallelism.rs`'s two bounds are wrong, or whether the corpus pages past them have a second
mechanism. Supersedes nothing. Context: ADRs 0138, 0139, 0219, 1082, 1742; traps 1, 12b, 13, 66.
Code: `crates/pdf-model/tests/raster_golden.rs` (`DIVISIONS`, `ONE_LEVEL`, `RARE`, `PAST_THE_BOUND`,
`every_first_page_drawn_in_strips_is_the_page_drawn_whole_within_the_bound`).

## 1. The population, measured

Every tracked first page (967 drawn of 974 documents) drawn by the CPU backend at one strip and at
two, four, eight and sixteen, compared byte for byte (`scratchpad/r1461/division-<n>.tsv`):

| strips | pages that move | pixels moved | worst | pixels at 2+ levels | largest share of a page |
|---|---|---|---|---|---|
| 16 | 150 | 7 043 | 2 | 238 | 0.358% (`issue1350.pdf`) |
| 8 | 149 | 7 269 | **3** | 240 | 0.358% |
| 4 | 147 | 6 496 | **3** | 240 | 0.358% |
| 2 | 135 | 4 757 | **3** | 10 | 0.358% |

153 pages move at some division and 814 at none. The median page that moves moves 7 or 8 pixels;
40 move more than one pixel in ten thousand. Sixteen strips reproduce ADR 1742's figures (7 042
pixels on the 149 pages that open without a password, and one more on `issue21579.pdf`, which opens
with the password `corpus_passwords` publishes).

Eight pages pass `strip_parallelism.rs`'s bounds, and they are ADR 1742's eight: three move more
than one pixel in a thousand and five more than one level. **No other page passes either bound at
any of the four divisions.**

## 2. What moves, page by page (trap 1)

Each page was drawn with its moved pixels painted red, and magenta for two levels or more, and
looked at. Counted by horizontal run (`scratchpad/r1461/runs.py`):

- **A horizontal edge on a sample row** — ADR 0219's mechanism, one level: `issue1350.pdf` (1 078
  pixels on five rows, the form's boxes, runs up to 215), `pdfjs_wikipedia.pdf` (the heading rules
  and glyph tops, runs up to 351), `issue7020.pdf` (the flat tops of the Kannada glyphs).
- **Two marks' edges in one pixel** — two levels, one per edge: `issue7014.pdf` row 369, where the
  underline annotation's bar ends at 422.9688 and the highlight below it is clipped at 422.3106,
  228 pixels; `issue12810.pdf`, the pixel where a diagonal stroke meets a vertical one.
- **A glyph pixel under a highlight's Multiply composite** — two levels: `comments.pdf` and
  `highlights.pdf`, single pixels on the glyphs the annotations cover.
- **Inside a photograph under a blend mode** — up to three levels: `blendmode.pdf` draws one image
  over another in sixteen cells, one per §11.3.5 mode. Every moved pixel is single and inside a
  cell; at sixteen strips the Normal and Multiply cells move none, Saturation 18, Luminosity 16,
  Lighten 14. So `blendmode.pdf` is not on an edge, and the premise that every moved pixel is (ADR
  1742 section 2 said it of `issue1350.pdf`) does not hold of the population. The mechanism is the same shifted
  origin reaching an image's sample position rather than an edge's coverage; that a blend function
  can multiply a one-level change in its source (ColorDodge's slope is `1/(1 − Cs)`) is why the
  most is three. That reading is not proved here, and the gate does not rest on it: it holds the
  figure.

## 3. Why the constants stay and what was wrong with the argument for them

`strip_parallelism.rs`'s one-level bound is derived for **one coverage**: an `ulp` of `ty` moves an
exact converter's area by far less than a level, so one coverage crosses at most one rounding step.
That derivation is right, and it is the bound for the three pages it is asserted on. What it never
covered is a pixel built from several rounded results — two marks' edges, a mark under a composite,
a source through a blend — and the corpus holds each of those. Widening the constant to three would
admit a three-level move on any page, including the single-edge pages where one is the most the
mechanism can produce; that is a tolerance fitted to the worst page.

So the decision is the construction trap 66 asks for: **the property is held over the pages there
are**. Every first page must be within the two derived bounds, or be one of the eight pages
`PAST_THE_BOUND` names, each with the reading of section 2 beside it and held to its own ceiling —
the most pixels and the most levels it moves at any of the four divisions. A ninth page past either
bound fails naming its figure and its first pixel; a named page that moves more fails; a named page
that falls back inside both bounds is printed as able to leave. The test rides the golden's walk
(`--ignored` runs both): 20.9 to 27.9 s alone at four rayon threads, peak 1.6 GiB; with the golden,
31.6 s and a peak of 3.67 GiB, against the golden's 3.2 to 3.4 GiB alone.

## 4. Calibrated both ways (trap 13)

From an export of HEAD with this test and nothing else, its own `CARGO_TARGET_DIR`:

- **ADR 0219's defect put back** (the strip's offset folded into the page transform before a mark's
  is composed with it): exit 101, 7 pages named — six past their ceilings, `bug1146106.pdf` past
  one level — worst 4 levels. Under ADR 1082's area converter that defect is a few levels, not 16.
- **ADR 0138's defect put back** (`plan_strips` handed no unsplittable row, so curves are cut):
  exit 101, 14 pages named, worst **36 levels** (`bug1820909.1.pdf`).
- **The export unplanted**: exit 0, the figures of section 1.

## 5. What it costs, and what is left

- Nothing a window draws changed. `render-cpu`'s comments that quoted ADR 0219's three-page figure
  as the property's ("fewer than one pixel in ten thousand", "a handful in a million") now cite
  this measurement.
- The ceilings are a ratchet on another program's arithmetic as much as ours: a change that moves
  these eight pages' pixels can move their figures, and then the page is read again rather than
  its ceiling raised quietly.
- The fixture tests that call `CpuRasterizer::new()` keep the machine's count (ADR 1742 section 4)
  and none is moved by it today. The largest exposed one is `inline_image_abbreviations.rs`, a
  900-row page drawn in one strip on this 24-CPU machine and in two on two to eight CPUs. There, one
  label pixel at (269, 653) moves one level, outside the eight blocks the test compares.
  pdf-transform's oracle page, ISO 32000-2 p100 at 150 dpi, gets 14, 4 and 2 strips on 24, 4 and 2
  CPUs, and 277 to 397 pixels move between those counts. It holds exactly only because the program
  and its oracle ask the same machine; if either side states a count, the other must state the same
  one.
- The blend-mode reading of section 2 is a hypothesis about `tiny-skia`'s pipelines; a round that
  needs it proved draws `blendmode.pdf`'s Saturation cell at a ladder of offsets.
