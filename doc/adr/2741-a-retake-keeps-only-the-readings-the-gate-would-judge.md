# 2741 — A retake keeps only the readings the gate would judge

Status: accepted; built in `tools/bands.py`. Session 1963, a single round. Context: ADR 2662 (bands
per machine, re-taken by a command), ADR 1969 (the band rule over kept readings), ADR 2730 (the RX
580's unpinned retake). It cites no clause and moves no ledger row.

## What was inconsistent

`bands.py` turns a retake's logs into bands. Every band it writes says "over N kept reading(s)". For
`launch_path` it dropped a figure whose probe line gave either of two reasons: the load was above
one task per physical core, or the child was not on the machine's own clock. Every other reason the
gate gives for "not judged" was kept. The thread that waited for a processor (`Declined::ItWaited`)
was the one round 1958 saw. The first pass outside its band, the carve-out, the disk and a cache that
stayed were also kept. `turn_path` keeps only its fit runs, and `toolkit-launch` drops every run it
printed `NOT JUDGED`.

## The decision

By the file's own rule, a band is taken over **kept** readings, and a kept reading is one the gate
would judge. The gate gives each of its declines the same meaning: this reading is not this
machine's clock. A band built from such a reading would judge the next run's readings by one that
the gate itself refused to judge. So `launch()` now drops every figure whose probe line says
`not judged:`, for any reason. The gate already prints each reason (ADR 0903), so the filter matches
that one phrase rather than a list of reasons that can fall behind the gate's enumeration.

## What it moves

The new rule, run on the logs of round 1958's merged retake, keeps fewer
readings. Of `bring_up_ms` it keeps 2 of 3 readings: 45.0 .. 64.8 against 44.7 .. 64.8. Of the disk
probes it keeps 25 of 28. Three rows' first pages lose one reading each, and two of their bands
narrow: 114.1 to 112.6, and 108.2 to 91.3. The committed bands are left as ADR 2730 wrote them. The
next `tools/bands.sh retake` of the RX 580 applies the rule. That retake needs the card and a quiet
machine, and a band over one reading is the thin derivation that a quiet retake exists to replace.

## Cost

A loaded retake keeps fewer readings, and where none is left, `bands.py` refuses the figure by name
and writes nothing. That is its existing behaviour, and the answer is a quieter run.
