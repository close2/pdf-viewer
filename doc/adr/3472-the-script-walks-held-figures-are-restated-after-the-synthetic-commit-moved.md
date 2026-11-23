# 3472 — The script walks' held figures are restated after the synthetic commit moved

Session 2192. Status: **accepted**. Builds on ADRs 3453 and 3454 (the held figures at `1d73fc0f6`), 3469 (the
walk's sequence budget), 3470 (the selection entered) and 3471 (the held-only walk).

## Tier 0

The whole walk in its update mode at this round's head, over 100 312 PDFs (462 not opened, 382 with a field
script): six lines moved, none added or removed, every one in the `untaken` column and none in the digest — the
values displayed are the ones held.

| document | untaken | why |
|---|---|---|
| `FOP-2216-4`, `-5`, `-7`, `-8`, `-9` | 1 → 0 | `Country`, a combo box without bit 19 and no `/V`, entered as the empty selection (ADR 3470) |
| pdf.js `pdf.js-8048-0` (tika `batch4`) | 6 → 3 | three selection-only fields entered by their selection |

The held-only walk read the same 382 documents and moved the same six.

## Tier 1

- **The held documents** (`HELD_ONLY_FIGURES`): 21 162 runs, 9 315 throws, 8 unparsed, nothing stopped, the same
  unpinned and pinned to one processor (ADR 3469). The floor is the runs measured; the ceilings are the
  census population's, which the held documents reach.
- **The census population** (`WHOLE_FIGURES`): the whole walk, 100 312 PDFs, ran the same 21 162 times — only the
  held documents carry a field script — threw 9 315 and left 8 unparsed, nothing stopped; the floor is raised to
  21 162, 41 above ADR 3454's whole walk at its head; ADR 3470's selections are the one change to what runs. It failed on one run over a
  budget, `Wall`, which the walk's runs no longer meet (ADR 3469 decision 4); the held-only walk re-run under it
  is the check, and the train's whole walk the confirmation.

## Decision

Both held files are as their update modes and the measured walks left them. The ten fields ADR 3453 named: five run
again; the other five hold a `/V` no option names, and stay untaken (ADR 3470).
