# 2420 — The three step rows' lowest-clock bands are taken in one hold, and the lowest level comes with the processor's load

Status: implemented for `ISO_32000-2_sponsored_EC3.pdf` p101; deferred for `images.pdf` and
`issue14415.pdf`. Session 1793, 2026-10-26, an "instruments" round under the project owner's switch to
code improvements of 2026-10-14. Context: ADR 2402 §5 (three step rows owed a `_lowest_ms` band), ADR
2388, ADR 1537, traps 262, 274 and 275, habit 186. It cites no clause and moves no ledger row.

## 1. The hold

One `--clock --nice 0` hold of HEAD's release build (88339689), 18 runs of
`cargo test --release -p render-raster --test turn_path -- --ignored --nocapture` back to back, each
started once the one-minute load was under 3 or after 60 s; 2 929 s. The first run is HEAD's gate as
the batch 118 merge ran it, and what each row's line judged is in the record of session 1793.

## 2. What the clock did

The device's level followed the processor's load, not its own work. Over the hold's 594 children, none
of the 231 taken at a load under 3 ran wholly at 600 MHz; 5 of 117 at 3 to 6 did, 47 of 111 at 6 to 9,
and 80 of 135 at 9 and more. So trap 274's "alone on the device it holds 600 MHz" did not hold that
evening, and habit 186's quiet start cannot see the lowest level at all: every lowest-level figure is
a loaded machine's, and the gate's calibration probe (0.62 .. 0.78 ms) declines most of them.

## 3. The decision, row by row

- **EC3 p101's step: `step_lowest_ms = 4.7 .. 6.9`**, the check file's rule over the two runs the gate
  admitted wholly at 600 MHz, 5.57 and 5.68 ms at loads 7.5 and 9.8. Three more at 600 MHz, 5.46 to
  5.63, were declined by the calibration probe and lie inside it. The raised figures, twelve runs at
  1.65 to 1.94 ms from 796 MHz up, stay in `step_ms = 1.3 .. 2.1`, which does not move. Two runs, not
  three: the lowest figure varies by 4 % over every load the hold saw, so a third would add a margin
  the rule already gives, while leaving the row unbanded keeps a red on every loaded run (two of 18).
- **`images.pdf` and `issue14415.pdf` gain no band.** In 18 runs the gate admitted no figure of either
  at 600 MHz; their figures there, 12.55 to 17.89 ms, all came from children the probe declined, so a
  band from them would be a claim about a loaded processor, not the device. Their raised runs — 14 of
  `images.pdf` at 10.25 to 11.88 ms, 13 of `issue14415.pdf` at 8.25 to 9.54 — sit inside their bands,
  and the lowest level, where it is admitted, will print `sclk 600 MHz 100%` beside a red for ADR 2242
  to judge. The trigger to take them: a hold in which the gate admits three runs of each at 600 MHz.

## 4. The hypothesis

"Each of the three reads two to three times its raised figure at 600 MHz" held for EC3 (5.6 against
1.7, 3.3 times) and not for the other two, whose 600 MHz figures are within 1.2 to 2.1 times their raised
ones and inseparable from the load that brought the clock down. "None moves its own band" held.

## Costs

Two lines and two comments in the check file; a row banded from two runs, said beside it.
