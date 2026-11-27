# 3826 — The field-script walks and three archive walks are declared long

Session 2281. Status: **accepted** and built in `tools/state.sh`. Context: ADR 1869 (a small walk is
one that ends within 600 s, and while a clock run waits for the first lane a small walk is still
granted the second), ADR 1756 (a long run is pinned to the second lane and is not granted while a
clock run holds the turn).

## Context

`section_scripts` declared both of its walks `small`, and each runs for over an hour. Under ADR
1869's rule a small walk asked while a clock run waited for the first lane was granted the second.
The clock run then waited for that walk to end, so rounds' launch and turn gates queued 60 to 90
minutes behind the train. The ceiling is stated, not enforced. A walk that outruns it must say so
itself.

## Decision

Each `walk small` in `state.sh` was read against its holds in `heavy-walk.log` since 2026-11-20
(these are the train's runs and rounds' runs, with each hold measured) and against the newest train
log's `bounded:` lines. Five are past 600 s and now declare `long`:

| walk | holds (s) |
|---|---|
| `pdf-model --test script_corpus` (Tier 0) | median 4 589, max 7 409 |
| `pdf-script --features engine --test script_corpus` (Tier 1) | median 4 289 (train 4 126 and 4 289), max 7 375 |
| `pdf-archive --test cross_check` | median 8 664 (train 4 742 and 5 320) |
| `pdf-transform --test archive_corpus` (the converter's census) | median 1 668 (train 1 099 and 1 668) |
| `pdf-transform --example archive_unbuilt` | median 1 110, max 1 690 |

The others stay `small`. The longest of them is `viewer-confined --test awkward_classes`, with a
median of 510 s and the train's latest at 538 s. One hold reached 1 279 s under load, so it is the
next to read if its median moves. Next come `pdf-archive --test corpus` (median 458 s) and
`viewer-core --test accessibility_census` (median 236 s).

## Consequences

A clock run that holds the turn is no longer queued behind an hour-long census on the second lane.
The train's long walks queue behind one another on that lane. The train runs them one after another
anyway, so it loses nothing.
