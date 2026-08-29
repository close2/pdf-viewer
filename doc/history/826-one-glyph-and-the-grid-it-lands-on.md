# 826 — Four contradicted pages were one glyph, and why two of their neighbours agree was the grid

Date: 2026-08-29. Branch `round-826`, from `main` at `3c259925`. Parallel round, worktree `r826`,
beside 824 (errata), 825 (the cms corpus, fuzzing) and 827 (general).
ADR: [0755](../adr/0755-one-glyph-and-the-grid-it-lands-on.md).
Touched: `crates/pdf-model/tests/oracle.rs` (`CONTRADICTED_SUBSTITUTED_FONT`'s note),
`crates/raster-compare/src/lib.rs` (`DEFAULT_TILE`'s doc comment and one new test),
`doc/traps/oracle-and-references.md` (new trap 26), `doc/HANDOVER.md` (the trap index),
`doc/conformance/ledger.toml` (§9.5's note, written back through `--bin ledger`), and two new
files, `doc/adr/0755` and this one.

## The subject

The oracle's *contradicted* pool — 61 pages — rather than the ambiguous bucket. Round 780 had
taken 32 of them down to one mechanism (ADR 0717: the differing fraction, convicted by the
`poppler` + `mupdf` pair that hints through a single `libfreetype.so.6`). The brief was the
remaining 29: which fall to a cause nobody has diagnosed, and which of those is sharpest.

## The pool read as a population

The instruments first, all off one run of the gate. `--bin unpriced` reports every failing bound in
the pool named by the note that holds its page, 0 not, and 0 notes naming a measure none of their
pages fail; the one page whose printed line cannot distinguish its own margin is still
`issue6069.pdf`, six channels of eighty thousand, which is ADR 0606's finding and not a new one. So
the bookkeeping the sweep can see is clean, and the answer had to come from reading the pool a
level finer than the sweep does: **per page rather than per note.**

Tabulating the gate's own lines against the twelve groups, the pool is 42 pages furthest outside on
the differing fraction (32 of them the `libfreetype` pair's), and the rest fall to notes that price
their mechanism by ablation in the measure that fails —
`CONTRADICTED_DEVICE_CMYK_CONVERSION` (100% of every failing measurement on all five, ADR 0510),
`CONTRADICTED_LINK_BORDER` (all three inside both bounds with `/Border [0 0 0]`, ADR 0499),
`CONTRADICTED_CALRGB_TO_SCREEN`, `CONTRADICTED_REFERENCE_GLYPH_WIDTHS`,
`CONTRADICTED_TIGHT_CONSENSUS`. One group does not: `CONTRADICTED_SUBSTITUTED_FONT`'s four
`pdfbox/PDFBOX-2984-rotations.pdf` pages fail on the **worst tile alone**, at 1.10 times their
bound, under a note whose whole account of them is a cap-height ratio measured in cap rows and
whole-page ink. Four pages of one document, falling together, on a measure nobody had priced. That
is what the round took.

## What it was

One glyph. `raster_compare` records where the worst tile is, and on all four pages — ours against
each of the three references and between each pair of references — it is the same tile, `(480, 64)`,
holding the line's `registered` sign. The closed form settles it with no renderer in the
measurement: `LiberationSans-Regular.ttf` states that glyph's net outline area as 664 570.5 units²
over a 2048 em and `NimbusSans-Regular.otf` as 228 762.3 over a 1000 em, which at 50 pt are
396.11 px² and 571.91 px², against 395.73 and 572.08 measured at eight times the resolution. Every
renderer paints the area its own font program states. The advance is Adobe's published 737 in both
faces and `standard_metrics.rs` answers it, so the layout is the document's and only the drawing
differs — §9.5 NOTE 5 exactly. A §7.5.6 update replacing the sign with a space takes all four pages
inside every bound.

**A vindication**, then, and the mechanism is the group's own. What the round adds is the mechanism
priced in the measure the row is ranked on, which is ADR 0688's rule applied to the contradicted
pool.

## And the finding underneath it

The note also explained why pages 5 and 6 of the same document *agree*: "because their consensus
pair happens to sit further apart and the bound derived from it is wider". That is trap 12's shape
and it would have been a good answer. Measured, it is false in both halves — the pair on 5 and 6
sits **closer** (25.33 against 28.40) so the bound is **narrower** (50.66 against 56.81) — and what
differs is our own number, 35.32 against 62.57, on a page carrying the same face and the same
glyph.

The whole of that is the tile grid. `raster_compare` lays its 32-pixel tiles from the raster's
origin, and the sign occupies device columns 484–519 on page 1 and 526–561 on page 5: 28 of its 36
columns inside one tile there, split 18 and 18 across two here. Over the glyph's own columns the
difference is 75 004 level-pixels against 78 212 — the same picture to four percent — and the
measure reads 62.57 against 35.32. One page contradicted, one agreeing, on where a grid happened to
fall.

That is trap 26 now, `DEFAULT_TILE`'s doc comment carries it beside the constant, and
`the_same_difference_reads_half_as_much_when_it_straddles_the_tile_grid` pins the arithmetic. **The
measure is not changed** and ADR 0755 says why: a sliding maximum is a different instrument, larger
on every page, and every bound in `Tolerance` was measured against this one.

## Calibration, gates and sweeps

The new test is calibrated against the absence of the property (trap 13): run its two bodies
through a 16-pixel grid, on which both placements are aligned, and it fails on the halving
assertion by name, reporting 191.25 where it wants half — not on the mean, and not through a
catch-all.

Full §2 sequence, all green, and **no verdict moved**: the gate's per-page lines are byte-identical
before and after. The §4 sweeps were run against a pristine `main` in its own checkout with its own
build directory (`r899`, closed with it): every difference is a line-number shift, a +1 in the count
of files naming an ADR, the trap index in `--bin parts`, and three quotation counters.
`--bin quoted` moved 237 → 239 figures read and 123 → 125 confirmed with **contradicted unchanged
at 101** — the first draft of the note had added three, a per-tile table using the gate's word
`mean` for a figure the gate does not print, which is that sweep's own noise class manufactured by
hand and was relabelled rather than explained away.

The reference-render cache is per-worktree and was cold here; it was seeded from the main
checkout's before the first run, which costs `pdftoppm`, `mutool` and `gs` invocations and changes
no verdict (the cache key is derived from the invocation). The machine was loaded throughout by
three sibling rounds, one of them fuzzing — which is why no duration in this round is quoted as a
measurement of anything.
