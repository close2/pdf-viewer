# 831 — A measure built from two inventions

Date: 2026-08-29. An instrument round in a worktree from `main` at `0b6709f7`, branch `round-831`.
One subject, one ADR (0759), one test file, two ledger notes, a trap instance and one todo
sentence changed, no pixel touched. `doc/rfc/` and `doc/todo/56` were not touched: both await the
owner. Not merged.

## The subject, as it was handed over

ADR 0756's closing section, and the reason the eight-hundred-and-twenty-seventh session left it:
the word-box gate's vertical-centre measure divides by the mean of two box heights, one of which
ADR 0323's Finding 3 already declares each extractor's own convention, and where the reference's
band collapses the ratio reads ≈ 1.0 whatever the word's position — while *every obvious repair
lands the two witnesses within a thousandth of the bound*.

That warning is exact and it was checked before anything else: this tree's em box is `(1.0, 0.0)`,
one em above the baseline and nothing below it, so dividing by our own height alone puts
`bug868745.pdf` at 0.497 against a bound of 0.5. Three thousandths of a quantity nothing derives.

## What the measure is asking

A word box on the cross axis is a baseline plus a band about it, so the difference of two centres
is the difference of two **baselines** plus half the difference of two **bands**. The first is
§9.4.4's arithmetic — the quantity the reading-axis bound is a tolerance on. The second is exactly
what ADR 0323 Finding 3 excluded from the verdict. The measure is their sum and cannot report
either alone.

The denominator turned out not to be the defect at all: the mean of two heights is precisely the
separation at which two bands stop overlapping, which is a well-formed quantity for a different
question. No denominator repairs a numerator, which is why every candidate lands on the witnesses
wherever the em box's own geometry puts them.

## What decides which of the two, and it is the file

§9.8.1's Table 120 defines both entries *about the baseline* — "[t]he maximum height above the
baseline reached by glyphs in this font", "[t]he maximum depth below the baseline reached by glyphs
in this font" — so a band built from the stated pair is a statement about where the baseline is,
and two readers obeying it differ only by their baselines. A band built from anything else says
nothing the file said. `the_band_the_file_states` asks `pdf_font::measured_extent` of every font a
page reaches, following a Type 0 font to its descendant's descriptor; a page that fails keeps both
reading-axis edges and loses the cross-axis measure, counted and printed. Type 3 is in the second
population by Table 120's own "Required, except for Type 3 fonts".

The same shape as ADR 0756's, one measure finer: a *file*-derived condition telling the instrument
which of its measures is about a quantity the file states, never a delta-derived exemption. It sets
aside a **measure** rather than a word, so neither the judged set nor the pair count moves.

## The evidence

Both halves are the runs' own and are not repeated in any instruction file; `tools/state.sh` and
the two invocations in `text_extraction.rs`'s doc comments reprint them.

Against `pdftotext`, the gate's cross-axis measure split in two: pages stating Table 120's pair
agree at **median 0.0000, p90 0.0000, p99 0.1041**, and pages stating no pair at **median 0.2540,
p90 0.3215, p99 0.9813**. Nine words in ten on a page that states the pair are placed by this tree
and by poppler at the same centre to the printed precision.

The derivation — poppler against mupdf, with this tree on **neither** side — was taught the same
split and confirms it: the centre bound rejects **0.15%** of the 10 073 pairs whose page states the
pair and **5.25%** of the 2705 whose page does not.

## What changed, and what it cost

`crates/pdf-model/tests/text_extraction.rs` and two ledger notes (§9.8.1 and §9.2.2), which the
ledger binary reformatted. Four documents leave `SELECTION_BELOW_FLOOR` — `bug868745.pdf`,
`issue1350.pdf`, `issue4665.pdf`, `issue6605.pdf` — and 143 matched words move into bounds, which
is exactly those documents' out-of-bounds words and exactly the verdict's own difference. All four
agreed to 0.00 pt on both reading-axis edges. `JUDGED_FLOOR` does not move and no refusal count
changes.

## What was refused

Moving `VERTICAL_CENTRE_BOUND`, though the derivation now shows a tighter bound available — a bound
retightened in the round that changed what it is applied to cannot be told apart from one fitted to
the corpus. And replacing the measure with the bands' *separation in points*, which is the only
cross-axis statement that survives both conventions: measured before being refused, the worst pair
in the whole corpus reads 0.9842 against a disjointness threshold of 1.0, so that verdict could
never fail anywhere. A bound that fires on a convention is bad; one that fires on nothing is worse.

## Calibration, and the floor it asked for

Trap 13, above the commit, both ways, both reverted — and **only one direction was caught**, which
is the round's second finding. With the set-aside disabled the gate fails loudly and names the four
documents that come back. With it widened to everything, the cross axis is judged nowhere at all
and **nothing fails**: the one document whose words fail that bound fails the reading-axis bound as
well, so no name enters or leaves the list, and the verdict, the judged set and the refusal table
all stay where they were. A measure evaporating in silence is exactly the failure the round was
about, one level up.

`CROSS_AXIS_FLOOR` is what closes it — the count of matched pairs the cross-axis bound is applied
to, ratcheted the way `JUDGED_FLOOR` ratchets the judged set — and it exists because a calibration
came back green in a direction it should not have.

## Gates and sweeps

The full §2 sequence and §4's sweeps, the latter diffed against a pristine `0b6709f7` checkout with
its own build directory, closed with it. Figures are the runs' own.

## What this round did not take

The em box's `(1.0, 0.0)` — the whole nominal line above the baseline, so a highlight over a
descriptor-less font covers no descender — which this round's arithmetic is what made visible, and
which belongs to a round that measures selection rectangles. `issue6127.pdf` is still undiagnosed.
