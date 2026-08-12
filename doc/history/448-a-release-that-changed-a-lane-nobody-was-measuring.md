# 448 — A release that changed a lane nobody was measuring

**Finding.** quorra's `74c4994d` moves nothing on the lane this tree draws with, and takes
**24 refusals off the lane a person reaches by zooming** — 20 of them landing in agreement with
the CPU oracle. Neither half could be seen before this round, because the corpus gate had only
ever run one of the two lanes.

**Date.** 2026-08-12.
**ADR.** [0283](../adr/0283-a-release-that-changed-a-lane-nobody-was-measuring.md).
**Touched.** `Cargo.lock` (`c1f6e2f4` → `74c4994d`),
`crates/render-quorra/tests/corpus.rs`, `crates/render-quorra/examples/first_frame.rs`,
`doc/quorra-gpu-coverage.md`, `doc/QUORRA_FEEDBACK.md` (§20), `doc/verify.md`,
`doc/todo/02-every-round.md`, `doc/adr/0283-*`, this file.

## What was asked and what was built

The project owner asked what a new quorra release changes here. The release is three commits and
every line of all three is inside quorra's **GPU coverage lane** — which `render-quorra`'s corpus
gate had never run, and which `viewer-ui` puts a person on past ten times magnification. So the
question could not be answered before the instrument existed: `PDFVIEWER_QUORRA_COVERAGE=cpu|gpu`
on the gate and `FIRST_FRAME_COVERAGE` on `examples/first_frame`, both skipping the ratchets and
both panicking on a value that is neither rather than measuring the default under the other lane's
heading.

## What the measurements said

| | agree | differ | refused |
|---|---:|---:|---:|
| default lane, 1×, `c1f6e2f4` | 917 | 35 | 5 |
| default lane, 1×, `74c4994d` | 917 | 35 | 5 |
| gpu lane, 1×, `c1f6e2f4` | 904 | 44 | 9 |
| gpu lane, 1×, `74c4994d` | 908 | 44 | 5 |
| gpu lane, 4×, `c1f6e2f4` | 900 | 16 | 36 |
| gpu lane, 4×, `74c4994d` | 920 | 20 | 12 |

The four pages that left the 1× refused list — `bug1703683_page2_reduced`, `issue12810`,
`issue1905`, `issue9418` — all agree with the oracle; quorra had named two of them. The
rasterisation total rising 4.68 s → 6.13 s is those four joining the timed set, measured alone at
1.26–1.27 s. The release's headline first-frame improvement does **not** appear on a page of dense
text, three samples each way, which is what its own commit message predicts.

## What it corrected

`doc/quorra-gpu-coverage.md`'s "what the lane still does not do" had two of its three bullets
overtaken by quorra: the lane chooses per *command* now, and an atlas does stand in front of it for
tiles a page places more than once. Struck rather than deleted, because a document about somebody
else's code decays on their schedule.

## Gates

Whole `tools/state.sh` sequence, green and unmoved: 1619 tests, corpus 974/65 incomplete, oracle
905 agree / 68 contradicted, text 99.2% and 99.8%, quorra 917/35/5/17, ledger 875 rows, 6559
citations, 631 quotations.
