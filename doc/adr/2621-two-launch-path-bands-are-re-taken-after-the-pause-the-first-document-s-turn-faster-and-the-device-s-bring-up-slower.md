# 2621 — Two launch-path bands are re-taken after the pause: the first document's turn, faster, and the device's bring-up, slower

The merge of batch 130 (sessions 1891–1900). Status: **accepted**. Context: `doc/checks/launch-path.toml`'s
own clock rule (three runs, clocks smallest × 0.85 to largest × 1.20), ADR 1969 (a band is moved by that
rule over HEAD's runs and by nothing else), ADR 2242 (a red clock gate is judged by a same-hold comparison
with HEAD before anything is bisected). It cites no clause and moves no ledger row.

## What was read

The `launch_path` gate was red at the merge of batches 129 and 130 and at the base's re-take in each
(the two merges' re-take logs, kept with the orchestrator's session records), on the same two figures whichever tree ran:

| figure | band | the base in one quiet hold | the batch in the same hold |
|---|---|---|---|
| `bring_up_ms` | 13.6 .. 20.6 | 22.6, 25.6 | 22.6, 23.3 |
| `PDF20_AN001-BPC.pdf` `turn_ms` | 2.13 .. 5.6 | 1.840, 2.109 | 1.885, 1.951 |

Three further runs of HEAD's export (ee5a2157) in one hold on 2026-11-02, one-minute load 1.5 to 2.9,
read `bring_up_ms` 19.99, 21.01 and 21.04 and the turn 1.929, 2.214 and 1.930. No package the figures
depend on changed since the bands were taken on 2026-10-05 and 2026-10-06 except Mesa and
`vulkan-radeon`, 26.2.3 → 26.2.4 on 2026-10-13, which this file already notes against another row.

## Decision

1. **Both bands are the file's rule over HEAD's three runs**, rounded outward to the file's decimals:
   `bring_up_ms = 17.0 .. 25.3` and the first document's `turn_ms = 1.64 .. 2.66`. The date, the hold's
   load, the three readings and the rule are written beside each row.
2. **The other figures outside at the edge stay banded as they are.** `first_page_ms` and
   `cold_first_page_ms` on the first document and on ISO 32000-2 read inside their bands in two of the three
   runs and just over in one; a band moved for one reading in three would be moved to make a gate pass,
   which ADR 1969 forbids.
3. **The direction is written down.** The turn is faster than it was on the same bytes, and the floor
   follows it so that a turn that slowed back to 2.2 ms would be legible. The device's bring-up is about
   four milliseconds slower on an unchanged shader set and adapter selection: the driver's share, which
   principle 2 made this gate for. A bring-up that returned to 17 ms would read below the new floor and ask
   for this band to move again.

## Consequences

- The gate is green on the base and the batch in the same hold under the new bands; the re-run after the
  edit is in the merge's record.
- A reader of the band file sees, beside each moved row, when and under what the figure was taken.
- Batch 131's brief need not carry a band review; the next red on these rows is a question about the tree.
