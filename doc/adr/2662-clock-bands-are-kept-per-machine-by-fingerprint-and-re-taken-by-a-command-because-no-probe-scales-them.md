# 2662 — Clock bands are kept per machine, found by a fingerprint and re-taken by a command, because no baseline probe scales them

Session 1914, a single round (measure + instruments). Status: **accepted**. Context: the owner's
question of 2026-11-03 ("run a few basic tests as baseline and determine the other values as factors
from these baseline tests? (If this is not realistically possible, that's fine as well. It just seems
wrong)"), ADR 1969 (the band rule), ADR 2660 (the server VM's bands), ADR 2158 (a cold first page
takes its row's warm band), ADR 2663 (what round 1913's bands were drawn on). It cites no clause and
moves no ledger row.

## 1. The question

Round 1913 re-took every clock band of `launch-path.toml`, `turn-path.toml` and `toolkit-launch.toml`
by hand for the server VM. The owner asked that a change of machine or GPU not need such a round
again, and proposed bands expressed as factors of a few baseline probes the gate takes itself — a
processor probe (`calibration_ms` already is one), a disk probe (`io_ms`, `io_latency_ms` already
are), and a graphics device probe that would have to be new.

## 2. What was measured

A factor band works for a class only if every figure of the class moves between machines by the
same ratio as the class's probe, within the band's own slack. Take a fifth either way as that slack:
then **the largest ratio of a class over its smallest may not pass 1.2 / 0.8 = 1.5, whatever the
probe is**, because one number has to be within a fifth of all of them. That test needs no probe
built, and the round applied it to these points: the notebook against the VM (the ratio of each
band's floor, which is the ratio of the quickest runs), and three in one sitting on the VM — the
RX 580 through OpenGL (the bands' own device, ADR 2663), the RX 580 through Vulkan (RADV), and
OpenGL with the children confined to two CPUs — and a fifth, lavapipe, from its retake's runs. The table, per class, is the round's record; its
ends are:

| class | probe's own ratio | the class's figures | largest ÷ smallest |
|---|---|---|---|
| processor, notebook → VM | `calibration_ms` 2.71 | warm open 3.32 – 5.22, script spawn and first run 3.39 – 3.72, a toolkit page one on the processor 8.29 – 9.06 | 2.7 |
| processor, all CPUs → two | 1.00 | turn rows 0.70 – 6.42 (`bug1743245.pdf` 5.5×, `issue14415.pdf` 3.0×, `22060_A1_01_Plans.pdf` 2.0×) | 9 |
| disk, notebook → VM | `io_ms` 5.10, `io_latency_ms` 6.30 | cold open 2.85 – 4.21 | 1.5, and outside both probes |
| device bring-up, notebook → VM | — | `bring_up_ms` 3.28, GTK `device_up_ms` 2.84, Qt 1.97 | 1.7 |
| device bring-up, OpenGL → Vulkan | — | launch 0.53, GTK 0.66, Qt 0.74 | 1.4 |
| device work, RADV → lavapipe | bring-up 0.97 – 1.15 | step 2.4 – 12.4, toolkit page one on the device 3.2 – 3.5, turn 0.90 – 1.54 | 14 |
| page one and turns, notebook → VM | — | first page 4.49 – 5.51, toolkit page one on the device 6.08 – 6.68, turn rows 0.85 – 5.00 | 5.9 |

No class passes. The calibration probe reads the same on two CPUs and on thirty-two, because it is
one thread, while a turn the pool divides reads up to six times slower; the notebook's Zen 5 and the
EPYC's Zen 1 differ by a different factor for a scalar open than for an image decode; and the device's
bring-up is the driver's, which moved by a factor of two with the backend on one card. A device probe
would be one more number with the same problem: the figures it would scale are each a different mix
of processor, memory, driver and device.

The noise floor says the same from the other side: the same configuration run twice in that sitting
(a point meant for lavapipe drew through OpenGL on the card, section 4) read single figures 0.65 to
1.55 times each other, so even a perfect factor could not be validated to a fifth from a run or two.

## 3. The decision

1. **The clock bands stay absolute and are kept per machine.** `doc/checks/machines/<name>/` holds a
   `fingerprint` and one file per gate stating, row by row, only the clock bands with the runs each
   rests on. The three band files keep the counted figures, the structure and the prose, and carry
   `clock_bands = machine` where each row's clocks go. `tools/conformance/test-support/machine_bands.rs`
   puts the machine's lines in before the gate's own parser reads the file; the counted figures stay
   absolute and are not this ADR's (ADR 2660).
2. **The fingerprint is the processor's model, the processors this process may run on, and the
   adapter the gate draws on as its own bring-up names it** (name, type, backend). It is taken by the
   harness, never by the environment: the launch and turn gates bring a device up in the parent and
   read its description, the toolkit gate reads the windows' own `graphics device up in` line.
   `vulkaninfo` named lavapipe where the gate drew through OpenGL on the RX 580 (section 4).
3. **A machine with no directory fails the gate by name**, with the command that fixes it. It is not
   judged against another machine's numbers and it is not let through unjudged (ADR 2661's lesson).
4. **`tools/bands.sh retake` takes a machine's bands**: it asks the harness for the fingerprint, does
   nothing if the machine has a directory (`--again` re-takes it), builds each gate's binary outside
   any lock, then runs each gate three times with `PDFVIEWER_BANDS_RETAKE=1` — every clock band
   unbounded, so every figure is printed and none judged — each run its own `--clock` hold and its own
   `with-gpu-lock` (none where the adapter is a processor's), and `tools/bands.py` writes the
   directory by ADR 1969's rule, the disk probes by ADR 2660's shape, a cold first page by ADR 2158.
   A band for another fingerprint is never touched; a figure no run kept stops the write by name.
5. **The split by the device's clock level (`*_lowest_ms`, `*_raised_mhz`, ADR 2402) is not
   re-taken**: a retake has no run sorted by level, so a retaken machine judges every turn figure
   against its main band until a round takes the split by hand.

## 4. What it costs

- A new machine, a new card or a new driver costs one command and about twenty minutes of the
  machine: `tools/bands.sh retake`, then the directory reviewed and committed. The driver's version is
  not in the fingerprint (the adapter description does not carry it), so a driver upgrade that moves
  the figures shows as a red gate and is answered by `retake --again`.
- The comments under each band, the runs, now live in the machine's file; the argument for each
  figure stays in the band file.
- Every gate run brings one device up in its parent before it starts (tens of milliseconds, outside
  every figure), or reads its bands after the windows have launched (the toolkit gate).
