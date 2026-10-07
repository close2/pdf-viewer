# 1621 — The launch gate's warm open is taken from warm cores

Status: accepted and **built**. Session 1392. Takes for `launch_path`'s warm arm the decision ADR
1577 took for the turn gate, on the evidence ADR 1606's round gathered with
`PDFVIEWER_LAUNCH_WARM_CORES`. Amends ADR 0884's method for that one figure. Supersedes nothing.
Context: `CLAUDE.md` principle 2; trap 110.
Code: `crates/viewer-ui/tests/launch_path.rs` (`phase_open`, the `warm-open` phase, `WARM_CORES`),
`doc/checks/launch-path.toml` (the header, every row's `warm_open_ms`).

## 1. What the warm open was measuring

A warm open is the open's own work with the disk taken out, 0.1 to 5 ms, in a child born idle. On
a figure of a few tenths of a millisecond the child lives its whole run at the clock an idle core
wakes at, a quarter below a working core's on this governor (trap 110). Ten runs on 2026-10-07 at a
load of 1.54 to 1.68 read `bug1815476.pdf`'s warm open at 0.288 to 0.307 ms in ten of ten with a
30 ms spin first and at 0.267 to 0.329 in five and 0.402 to 0.610 in five without, every
calibration in its band.

## 2. Measured again, beside HEAD

Three runs of this tree, the warm arm spinning, interleaved with three of HEAD's, load 4.5 to 11.5,
then three more of this tree at 1.1 to 2.0; the minimum of nine pinned children each:

| row | warm cores, six runs | HEAD, idle-born, three runs | band before | band now |
|---|---|---|---|---|
| `PDF20_AN001-BPC.pdf` | 0.31–0.33 ms | 0.29, 0.56, 0.73 | 0.27 .. 0.48 | 0.26 .. 0.40 |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 0.94–0.98 | 0.97, 1.36, 1.42 | 0.77 .. 1.40 | 0.79 .. 1.18 |
| ISO 32000-2 | 4.37–4.61 | 4.36, 4.75, 4.96 | 3.7 .. 6.5 | 3.7 .. 5.6 |
| `bug1815476.pdf` | 0.274–0.290 | 0.26, 0.26, 0.68 | 0.23 .. 0.35 | 0.23 .. 0.35 |

In each of HEAD's three runs at least one idle-born warm open read outside its band (0.73, then
0.56 and 0.68, then 1.42); its other failures were `open_peak_mib` figures of the snapshot's own
build, about 9.6 MiB, and are a reading of neither method. Every run of this tree judged every warm
open inside. The new bands are the file's clock rule over the six,
smallest × 0.85 and largest × 1.20, rounded outwards: three ceilings came down, because a band derived from an
idle-born arm had the idle clock's spread in it.

## 3. Decided

**The warm arm's child keeps every core it may run on busy for 30 ms before it opens**, as every
round of the turn gate starts (ADR 1577); it is the `warm-open` phase. **The cold arm stays born
idle**: a cold open is what a person meets, mostly round trips to the disk, and the reason for it is
the person rather than the governor. `PDFVIEWER_LAUNCH_WARM_CORES` now asks the same spin of the cold
arm, the instrument for whether a cold figure that reads high is the idle clock: one run of this
tree at a load of 1.6 read `bug1815476.pdf`'s cold open at 0.529 ms against 0.32 .. 0.50, its child's
calibration in band, which is the shape trap 110 has, and which a later round may measure with it.

## 4. The Type 3 page's excursion, read again

ADR 1607's first gate run read all three of `ContentStreamNoCycleType3insideType3.pdf`'s rows at
13.0 to 14.5 ms, at a load of 3.8 to 6.0, and a re-take minutes later 11.2 to 11.5. Ten runs of
HEAD's turn gate one after another on a quiet machine, load 0.8 to 2.7, one binary by `md5sum`: turn
10.67 to 11.29 ms, seventh frame 10.29 to 10.89, step 11.45 to 11.75 — every one inside, the turn
1.9 ms under its ceiling. A child already on warm cores did not reproduce it on a quiet machine,
which is not the idle clock; it came with the load and went with it, which is the busy end of trap
110, as ADR 1607 read it. Nothing showed a third thing. The band keeps its reason. The same ten runs
read `personwithdog.pdf`'s turn at 11.26 ms against 7.30 .. 10.80 once, at a load of 1.6 with the
cores warmed: the light row's idle-clock end is not wholly closed by the spin either.
