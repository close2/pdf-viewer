# 1577 — Every round of the turn gate starts on warm cores

Status: accepted. Session 1366. Amends ADR 1519 section 1, which declined a spin before a frame
as "work done to move a clock", and takes the decision ADR 1556 section 3 left to the gate's
method. Builds on ADRs 0916, 1513, 1537. Supersedes nothing.
Context: `CLAUDE.md` principle 2 ("Perf gates run in CI ... A regression fails the build");
trap 110; `doc/habits/measuring.md` 68.
Code: `crates/render-raster/tests/support/frame_cost.rs` (new: `WARM_SPIN`, `warm_the_cores`;
`round`), `crates/render-raster/tests/turn_path.rs` (module comment),
`doc/checks/turn-path.toml` (header, `personwithdog.pdf`'s `why`).

## 1. What a cold child measured

The batch's merge gate read `personwithdog.pdf`'s turn at 11.53 and 11.57 ms against a band whose
top is 10.80, judged once and outside, declined by the calibration probe the second time, and the
merge passed on the re-run. A row that fails on alternate runs by the processor's clock is not a
gate: what it says depends on how long the machine idled before the child, which the tree does not
decide.

## 2. Measured: both methods, interleaved, on every row

Six runs of the whole gate behind the heavy-walk lock, alternating, on an export of HEAD with a
knob for the spin, load 1.2 to 2.8 (the minimum of three children a row, ms):

| row | cold, three runs | warm, three runs |
|---|---|---|
| `personwithdog.pdf` turn | 11.17 (not judged), 11.68 OUTSIDE, 11.69 OUTSIDE | 9.34, 9.55, 9.78 |
| `personwithdog.pdf` step | 11.38 (not judged), 11.59, 11.76 | 10.23, 10.26, 10.45 |
| `issue19802.pdf` turn | 6.73, 6.80, 6.88, none of them judged | 6.66, 6.72, 6.73 |
| ISO 32000-2 p101 turn | 7.70 to 8.00 | 7.33 to 7.69 |
| the stroked Type 3 page, turn / step | 11.63–11.85 / 12.46–13.00 | 10.62–11.21 / 11.60–12.36 |
| `22060_A1_01_Plans.pdf` turn / step | 50.75–51.60 / 14.72–14.84 | 51.54–52.56 / 15.31–15.47 |
| `bug1743245.pdf` turn / step | 38.90–44.16 / 36.66–38.20 | 39.36–39.99 / 36.39–36.74 |
| `bug1721218_reduced.pdf` turn / step | 168.55–173.33 / 86.07–88.80 | 166.41–168.30 / 85.41–87.06 |

Cold, the three runs judged 20, 20 and 18 of 22 figures and two of them failed; the calibration
probe declined 24 of 90 children. Warm, every run judged 22 of 22 and passed; the probe declined 2
of 90. The heavy rows moved by under 5% and in both directions: the plan's step most, 14.72 to
15.31 at the minimum, slower warm, which is the package's power shared with 30 ms of eight busy
cores; `bug1721218_reduced.pdf` and `bug1743245.pdf` a little quicker. No band was near.

## 3. Decided: warm the cores, not wider bands

Every round of `frame_cost::round` — the gate's child and `examples/frame_budget`, which are one
method — begins by keeping each core the process may run on busy for 30 ms. The bands stay where
they are: every warm figure is inside them.

The other answer was to band the light rows with the clock's own spread. The mesh page's spread
cold and warm is 9.3 to 11.7 ms, and the file's rule (`high` the largest × 1.20) would put its top
at 14.1: a regression of a half would pass. A band that admits that is a gate for the governor and
not for the tree.

ADR 1519 argued the other way, that a real page turn comes after reading, which is the idle case.
That is true of what a person meets, and it is why the launch gate measures a first page from a
cold process. It is not what this gate is for: its figures are claims about what the tree costs,
compared run against run to catch a third's regression, and the idle clock is the one quantity in
them the tree does not decide. The other end of ADR 1556 — the cores the child is not pinned to
kept busy by a neighbour — is not removed by this, and the load ceiling and the device check are
still what answer it.
