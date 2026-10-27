# 2421 — A raised band states the slowest clock its runs were read at

Status: implemented. Session 1793, 2026-10-26, an "instruments" round. Context: ADR 2402 §4 ("the level
is the table's own line, not a megahertz threshold"), ADR 2420, trap 275. Code:
`crates/render-raster/tests/turn_path.rs` (`Bands`, `band_for`, `slowest_raised_mhz`, the
`<figure>_raised_mhz` keys). Tests: `a_figure_is_judged_against_the_band_of_its_clock`,
`the_check_file_states_a_band_for_every_row`. It cites no clause and moves no ledger row.

## 1. The defect

The first run of ADR 2420's hold, HEAD's gate, failed `issue19802.pdf`'s step at 5.33 ms read at 601 to
602 MHz on the table's middle line. ADR 2402 judges a middle-line figure as raised, so it met
`step_ms = 1.1 .. 1.8`; but 5.33 ms is the lowest level's figure (that row's 600 MHz figures in the same hold,
declined by the calibration probe, read 4.89 to 5.02). The middle line is the clock of the moment, and that evening it read from 601
MHz up — below trap 275's 605 — and EC3 p101's step read 5.50 ms there at 608 to 620 MHz. A row with a
lowest band was failed by the clock, not the tree.

## 2. The decision

A row that states `<figure>_lowest_ms` states `<figure>_raised_mhz` beside it: the slowest clock its
`<figure>_ms` runs were read at, observed with them, as the band is. A figure wholly at the table's
first line is judged against the lowest band; a figure read on a raised line below that clock is not
judged, said out loud with both clocks; every other figure is judged as before. A row with one band is
judged as before, whatever its clock. The check file's own test fails a row that states one key and
not the other, and fails once a planted row omits it.

- **Counting a middle-line reading near 600 MHz as the lowest was rejected.** The bound would be a cut
  through a continuum chosen to sort figures, the constant ADR 2402 §4 declined; and `images.pdf` read
  11.88 ms at 603 to 608 MHz, a raised band's figure, so no such cut holds for every row.
- **The bound is the band's own observation**: `issue19802.pdf` 860 MHz, EC3 p101 796 MHz, from ADR
  2420's hold. Where a run lands between the lowest level and it, no band was taken there, and the
  gate says so rather than judging it against either.

## 3. Measured

Re-judged offline over the hold's 18 runs, the rule changes one verdict: run 1's 601 MHz figure is no
longer judged, and with ADR 2420's EC3 band runs 1 and 18 turn green; 6 of 18 stay red, none of them on
a row with a lowest band at its lowest level. The gate's own run on the new judgement was not taken, as
the round's clock figures are one hold; the merge's `t2-turn_path` is its first.

## Costs

112 lines added and 27 removed in the harness; one key per banded figure; a figure between the two clocks is silent
where the row states both, which is the price of not judging it against a band taken elsewhere.
