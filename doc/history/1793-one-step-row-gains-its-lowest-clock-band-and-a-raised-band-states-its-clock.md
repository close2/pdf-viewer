# 1793 — One step row gains its lowest-clock band, and a raised band states the clock it was taken from

An "instruments" round of batch 119, slot 6, 2026-10-26; ADRs 2420 and 2421; Q659 unused. I worked in
`scratchpad/r1793/`. No ledger row moves; no clause is cited; the change is `turn_path`'s judgement.
**The contract.** The `_lowest_ms` bands ADR 2402 left owed for three step rows (EC3 p101, `images.pdf`,
`issue14415.pdf`), taken in one `--clock --nice 0` hold of HEAD's build, the hold's first run HEAD's gate.
**What the merge's gate judged.** Batch 118's `t2-turn_path` kept only its summary (exit 0, 2 passed,
95.04 s). The hold's first run, the same gate on the same commit at a load of 6.66 (cap reached): 33
figures banded, 24 judged, 22 inside, 2 outside, 9 not judged by the calibration probe; the two reds were
EC3 p101's step, 5.57 ms wholly at 600 MHz, and `issue19802.pdf`'s, 5.33 ms at 601–602 MHz on the
table's middle line, which ADR 2402 judges as raised. The other rows read at 600 to 1 139 MHz, inside.
**Measured.** 18 runs, 2 929 s, each started at load under 3 or after 60 s, nice 0; one child of 594 (run 2,
declined by the probe) had a `raster_gpu` test of this account on the render node, the rest none. The lowest level followed the processor's load: none of 231 children
at a load under 3 ran at 600 MHz, 80 of 135 at 9 and over did (ADR 2420 §2).
**Built.** EC3 p101 `step_lowest_ms = 4.7 .. 6.9` over its two admitted 600 MHz runs (5.57, 5.68 ms).
`images.pdf` and `issue14415.pdf` gain none: no 600 MHz figure of either was admitted in 18 runs, and their
raised runs sit inside their bands (ADR 2420). A row with both bands states `<figure>_raised_mhz`, the
slowest clock its raised runs were read at (EC3 796, `issue19802.pdf` 860), and a figure read on a raised
line below it is not judged; the check file's test requires the two keys together (ADR 2421). Re-judged
offline, runs 1 and 18 turn green and 6 of 18 stay red, none on a lowest band. `doc/verify.md`'s turn
paragraph described the rule ADR 2402 rejected and now states the one in force.
**Premises.** Arms started under load 3 see the lowest level: did not hold, none did. The hypothesis held
for EC3 (3.3 times) and not for the other two (1.2–2.1 times, under load). "None moves its own": held.
**Found, for others.** `issue19802.pdf`'s raised step left `1.1 .. 1.8` in three of twelve quiet runs
(1.81–1.97 ms at 889–1 422 MHz); `bug1743245.pdf`, `issue12841_reduced.pdf`, the Type 3 page and two
turns went red at 600 to 628 MHz under loads of 8 to 11, rows with no lowest band.
**Unfinished.** The two rows' bands, until a hold admits three 600 MHz runs of each; the gate's own run
on the new judgement, which is the merge's `t2-turn_path` (the round's clock figures are one hold).
**Gates.** The hold's 18 runs of the turn gate: exit 101 in runs 1, 2, 5, 6, 11, 14, 16, 18, exit 0 in
the other ten; the hold `bounded` exit 0. `render-raster` in the export of this change: `cargo fmt
--check` exit 0; clippy `--all-targets -D warnings` exit 0; `cargo nextest run` 112 passed, 3 skipped,
exit 0; `the_check_file_states_a_band_for_every_row` failed once on a planted missing key, as it should.
`cargo test -p conformance --no-fail-fast` exit 101, its one red `records` on session 1790's record, not mine.
