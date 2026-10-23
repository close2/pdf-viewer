# 2245 — The warm-open excursions of batches 106 to 108 are the machine's, and the bands stay

Session 1705, the measure round of batch one hundred and nine. Status: **accepted**. Context:
habit 101; ADR 1969, the rule for re-taking a sub-millisecond band; ADR 1621, warm cores; ADR
2244, the memory half of the same red gate; review 1704's finding 1. Check file:
`doc/checks/launch-path.toml`. No band moves. Cites no clause.

## 1. What the gate said

At the merges of batches 106, 107 and 108, `t2-launch_path` failed the warm open on up to four
rows: `opt_demo.pdf` at 0.144, 0.199 and 0.182 ms against 0.08 .. 0.14, and `PDF20_AN001-BPC.pdf`
at 0.478 and 0.450 against 0.26 .. 0.40. Batch 108's merge also failed `opt_demo.pdf`'s script
spawn and first run, at 0.904 and 0.598 ms. In that run, every failing child's calibration read
0.711 to 0.758 ms.

## 2. What two holds measured (`scratchpad/r1705/hold1`, `hold2`)

- **The instructions are the same.** An open executes 3972.5 thousand instructions on the
  five-page row in `5ded4916` and 3972.6 in HEAD, and every row agrees to two hundredths of a
  per cent. The harness of ADR 2244 adds 2.3 thousand to every row, which is the test binary's own
  start (trap 98).
- **The base fails the same way.** In one hold of nine runs (base, HEAD and HEAD with ADR 2244,
  three each, interleaved, at a load of 2.9 to 4.7), the base read `opt_demo.pdf` at 0.168 once
  and HEAD read the 57-page row at 1.236 once. Every other warm open, script spawn and first run
  of all three trees sat inside its band. Batch 108's own ABBA control had already shown the base
  failing the same bands (`/home/AI/launch-ab-1698/analysis.md`).
- **In the merge's own order the gate passes.** The second hold ran the merge's line — touching
  `viewer-core`, rebuilding and then measuring — and it exited 0. The run before that rebuild read
  `opt_demo.pdf` at 0.149, with that child's calibration at 0.718 ms. The calibrations in these
  holds read 0.669 to 0.704 ms in most children. The merges' failures came at 0.711 to 0.758, a
  processor 5 to 10 per cent slower on the probe that guards the clock, on a laptop whose `Tctl`
  read 72 to 75 °C during the hold.

## 3. The decision

- **The failures are the machine's, by habit 101.** The instructions, the bytes and the read calls
  match HEAD and the base, and the base fails in the same mode.
- **The bands stay.** ADR 1969 re-takes a band when HEAD's own quiet runs leave it. Here they sit
  inside it, apart from one excursion in nine. A re-take by the file's rule over HEAD's three runs
  would raise all five ceilings, by 0.01 to 1.08 ms. Even then it would still fail `opt_demo.pdf`'s
  0.182 and 0.199 and `bug1815476.pdf`'s 0.376 and 0.419 from the merges. That mode stays a
  failure on the gate's line and is judged as ADR 1969 section 3 says.
- **The gate was red for two causes, not one.** ADR 2244's memory figure failed every run of the
  base and of HEAD today, at the original path as well as at a copy. That figure is mended. The
  timing figures are the machine's, judged on the gate's line.

## 4. Not done

The calibration band, 0.62 .. 0.78, admits a processor a tenth slower than these holds measured.
Whether it should decline at 0.72 is a change to the guard, and it is argued on its own.
