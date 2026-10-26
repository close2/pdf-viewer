# 2402 — A turn figure carries the device's clock level, and a band is a claim about one level

Status: implemented. Session 1784, 2026-10-26, an "instruments" round under the project owner's switch
to code improvements of 2026-10-14. Context: trap 274 and ADR 2388 (the 890M's two speeds are its clock
levels), ADR 1537 (a turn is judged only on a quiet device), the band rule of
`doc/checks/turn-path.toml` and ADR 1969, ADR 2242 (a red gate is judged, re-taken or asked). Code:
`crates/render-raster/tests/support/device_clock.rs` (`Sampler`, `Residency`), `support/frame_cost.rs`
(`round` keeps each timed frame's span, `keep` merges the readings), `tests/turn_path.rs` (`Level`,
`level`, `band_for`, the child's `*_sclk*` fields), `examples/frame_budget.rs` (a `device clock` line per
row). Tests: `a_figure_is_judged_against_the_band_of_its_clock`, the two of `device_clock::tests`, and
`a_planted_busy_device_is_seen_as_busy`, which now also asserts that the planted device left its lowest
level. It cites no clause and moves no ledger row.

## 1. What the gate could not tell

ADR 2388 found `issue19802.pdf`'s zoom step at about 5 ms at the device's lowest clock and 1.7 ms at a
raised one, and the gate read neither: a red on that row could be the tree or the clock.

## 2. The instrument

A thread in each round reads the starred line of `pp_dpm_sclk` every 2 ms, from before the first device
to after the last frame. Each timed frame keeps the readings inside its own span; a figure's residency is
counted over the five rounds' frames, because a frame of a millisecond or two holds one reading or none.
The child prints the counts and the megahertz range per level with its figures, and the parent prints
them beside every child and every figure (`sclk 600 MHz 100% (14 readings)`). A read costs about 16 µs
of one core. Where no device publishes the table, the header line says so and nothing else changes.

**Measured, one `--clock --nice 0` hold, HEAD's export and this one's interleaved three times**, every arm
started at a one-minute load of 2.97 to 4.24 (the 60 s cap reached on four of seven), busy 5 to 14 %, no
render-node holder of this account. Against HEAD's second and third arms, raised by their step figures,
this export's first two, raised by its readings: over the 33 figures the quickest of each side differ by
a median of +1.1 % (−5.3 to +4.4 %), inside the spread between two arms of one side.
The sampler moves no figure that the gate can see.

## 3. What the clock does

The hold read the device at its lowest level for one HEAD arm and a part of the third pair, and at the
table's middle line for the rest — every frame of two whole arms, and of all three arms of a second hold
of this export alone — at 605 to 1 687 MHz, mostly 1 100 and above, while the busy share read 4 to 14 %.
So ADR 2388's "raised only while another process keeps the device 30 to 49 % busy" does not hold: both
levels come with a busy share the device check admits, and for minutes at a time. Most figures read
inside one band at either. Four steps do not: `issue19802.pdf` (5.07 ms at the lowest in HEAD's first
arm, 1.57 to 1.90 raised), `ISO_32000-2_sponsored_EC3.pdf` p101 (3.80 against 1.67 to 1.99),
`images.pdf` (11.85 and 12.24 against 10.0 to 12.1) and `issue14415.pdf` (9.90 and 10.76 against 8.28
to 10.20).

## 4. The decision: a band per level where the level moves the figure

- **"Not judged" for every figure off the lowest level was built first and rejected.** It judged 0 of
  33 figures in every arm that ran raised, five of six: a gate that goes quiet whenever the device is raised, which it was for
  most of the evening, is the silence trap 5 describes.
- **A row states `<figure>_lowest_ms` for a figure the lowest level moves out of `<figure>_ms`**, taken
  at the lowest level by the check file's own rule (smallest × 0.85, largest × 1.20). A figure read
  wholly at the lowest level is judged against it; one whose clock moved between the two levels through
  its frames is not judged, said out loud, where the row states both; every other figure is judged
  against `<figure>_ms`, as before the clock was read. A figure is the quickest fit child's, with that
  child's residency.
- **`issue19802.pdf`'s `step_lowest_ms = 4.3 .. 7.3`**, the rule over ADR 2388's six arms at 600 MHz,
  5.10 to 6.02 ms. HEAD before that fix read 9.07 to 9.59 at the same clock and fails it. Widening
  `step_ms` to hold 5 ms would have taken it to 1.1 .. 7.3, which holds the regression's raised 2.74 to
  3.28 as well — the reason a band is never widened over two levels.
- **The level is the table's own line**, not a megahertz threshold: a reading on the middle line counts
  as raised even at 605 MHz, as the table says, and the printed range shows how far above the lowest it
  was. Drawing a threshold through a continuum would be a constant tuned to this device.

## 5. Not done

- **The three other rows' lowest bands are owed.** Each has one or two figures at the lowest level, and
  the rule is over runs. Until a hold sees the lowest level three times, a red on one of them at the
  lowest prints `sclk 600 MHz 100%` beside it, and ADR 2242 judges it.
- **No performance level is forced.** `power_dpm_force_performance_level` needs root, and a pinned clock
  measures a state no reader's device is in; a band per level asks the same question without either.
  Q650 is not asked.
- The gate's own run on the per-level judgement was not taken: the hold ran this export with the sampler
  and the printing, before the judgement was changed; `band_for` is held by its unit test.

## Costs

One thread per round sampling at 2 ms, about 0.8 % of one core; three to sixteen more `key=value` fields
on each child's line; a `device clock` line under each row of `frame_budget`.
