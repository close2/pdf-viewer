# 454 — Three asks answered, and a defect only the second lane could reach

**Finding.** quorra's `a35dc703` answers three sections of `doc/QUORRA_FEEDBACK.md` at once, and
the one this side asked as a *question about a metric* was a real defect: a coverage-sheet
compaction bug reachable only on a shelf whose tiles write no CPU bytes, which cannot exist until
the device can take a tile the atlas would have held.

**Date.** 2026-08-12.
**ADR.** [0289](../adr/0289-three-asks-answered-and-one-defect-only-a-lane-could-reach.md).
**Touched.** `Cargo.lock` (`74c4994d` → `a35dc703`), `doc/QUORRA_FEEDBACK.md` (§9.1, §20.5),
`doc/quorra-gpu-coverage.md`, `doc/conformance/ledger.toml` (§10.7.4),
`doc/todo/11-shapes-that-still-disappear.md`, `doc/todo/_scan-conversion.md`,
`doc/todo/42-the-launch-path.md`, `doc/adr/0289-*`, this file.

## Measured

| | agree | differ | refused |
|---|---:|---:|---:|
| gpu lane, 1× | 909 → **914** | 43 → **38** | 5 |
| gpu lane, 4× | 920 → **926** | 20 → **14** | 12 |
| default lane, 1× | 915 | 37 | 5 |

None went the other way, and the lane this tree renders on is unmoved.

First frame, A/B/A with eight samples an arm because the effect is smaller than this instrument's
spread, read at the minimum: **14.94 ms → 12.77 and 12.47**, reproducing the stated 2.43 ms.

## Corrected

Three places in this tree said quorra still multiplied a clip chain. All three were true when
written; quorra's ADR 0030 takes `min` there too, read off §8.5.4 rather than off ADR 0280's
argument. A claim about somebody else's code decays on their schedule — ADR 0283's sentence, twice
in six rounds.
