# 2742 — The RX 580's launch and toolkit bands are re-taken whole, over quiet unpinned runs

Status: accepted. Session 1964, a single measure round. Context: ADR 1969 (the band rule), ADR 2662
(bands per machine), ADR 2667 §5 (a warm open quicker than its floor), ADR 2730 (the RX 580's device
figures re-taken unpinned), ADR 2741 (a retake keeps only the readings the gate would judge). It
cites no clause and moves no ledger row.

## What was owed

ADR 2730 re-took `bring_up_ms` and the first pages unpinned and kept every other `launch_path` band
from the pinned retake of 2026-11-03, taken at one-minute loads of 15 to 25 from another account.
The `toolkit-launch` bands came from that pinned retake too. ADR 2741 changed which readings a retake
keeps and left the committed bands for the next quiet retake. The hang marker stopped both.

## What was measured

Six `launch_path` runs, unpinned, each its own `--clock` hold, at one-minute loads of 6.5 to 10.5:
five exit 0, and run 3 exits 101 on one figure. The five-page row's warm open read 1.300 ms against
a floor of 1.63, below ADR 2667 §5's allowance of 1.386. The open's counted work was inside its bands
(2201 k instructions, 88.5 KiB, 25 read calls). So the clock did not stop before the work did: the
machine was quicker than the band's loaded runs. Round 1958's pinned run read 1.291 on the same row.
The row's six quiet warm opens are 1.30 to 1.99 ms, and the 2026-11-03 readings were 1.92 to 2.34.
That row was the band's error, not the program's.

Three `toolkit-launch` runs, unpinned, at loads of 5.9 to 6.3: `quorra-gtk`'s `device_up_ms` read
74.9 to 77.9 against the pinned band's ceiling of 72.4, and `quorra-qt`'s 64.0 to 69.4. This is ADR
2730's finding in the toolkit windows: unpinned, the loader reads every ICD the machine has.

## Decision

**Both machine files are re-taken whole**, by `tools/bands.py` over this round's quiet unpinned
runs. The six launch runs judged 347 of their 348 figures, so each figure has six readings, except
the script worker's start, which has five: run 6 declined one, its child off the machine's clock.
The toolkit runs give three readings for each figure, all judged. The 2026-11-03 readings are
dropped from the opens, turns, probes and script stage. They were pinned, and taken at two to three
times this round's load, and a floor taken from loaded readings sits above what a quiet minute
reads: the warm open is that case. The font-walk row is re-taken like the others: its six unpinned
first pages are 97.5 to 110.2 ms, and the 134 ms reading that ADR 2730 kept it for is from the
loaded pinned retake. `turn-path.toml` is not re-taken. Its one quiet run judged all 33 figures
inside their bands.

A judged run of each gate under the new bands exited 0, at loads of 4.9 and 6.2.

## Cost

Several bands narrow. The widest moves are the font-walk row's first page (161.0 to 132.2) and the
tagged row's turn (20.2 to 16.3). Each band now holds six quiet readings with ADR 1969's margins, and
the gate declines a loaded run's figures rather than judging them. A narrower band can still fire on
a run that is quiet but slow. That is the price of bands taken where the gate judges, and the answer
is the band rule over more quiet runs, never a hand-widened edge.
