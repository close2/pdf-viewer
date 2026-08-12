# 0289 — Three asks answered, and a defect only the second lane could reach

**Status.** Accepted.
**Context.** quorra released `74c4994d` → `a35dc703`: three commits, each answering one section of
`doc/QUORRA_FEEDBACK.md` — §20.4's page, §18's rule and §9's first frame.

## §20.4's page was a defect, and the instinct that found it was about the *metric*

`doc/QUORRA_FEEDBACK.md` §20.4 asked one question about `transparency_group.pdf`: its worst tile is
31.7 of 255 on the GPU lane, against the 1.5 to 12.2 quorra's own attribution reported, and
"[i]f sixteen samples is the whole explanation, the arithmetic should cap nearer 16 of 255 than 32".

It was not sixteen samples. `worst_tile_error` is a mean over a 32×32 tile and an edge crosses a
few percent of one, so **no sampling scheme reaches 31.7** — which is what turned the question into
a search. `ScratchPacker` packs rows at the device dimension and restrides them down to the width
the shelves reached; compaction leaves the old wide layout's bytes behind, and resizing straight to
the sheet's extent keeps whatever of that tail falls inside. On that page, 7 271 510 bytes of it —
670 rows of 10 853, exactly the shelf under the last CPU tile.

**It needed a shelf whose tiles write no CPU bytes**, which cannot exist until the device can take
a tile the atlas would have held — so it was unreachable for the life of quorra's ADR 0021 and
arrived with the lane's first corpus-scale run.

Reproduced here, same working copy, one pin apart:

| | agree | differ | refused |
|---|---:|---:|---:|
| gpu lane, 1× | 909 → **914** | 43 → **38** | 5 |
| gpu lane, 4× | 920 → **926** | 20 → **14** | 12 |
| **default lane, 1×** | 915 | 37 | 5 |

Five pages at 1×, six at 4×, **none going the other way**, and the lane this tree renders every
page on is unmoved — which is the ratchet passing rather than a null.

`transparency_group.pdf` is in both lists, and so are `issue17492.pdf` (both), `issue20489.pdf`,
`mixedfonts.pdf` and `textfields.pdf` at 1×, and `bug1922766.pdf`, `issue11473.pdf`,
`issue16038.pdf` and `issue7821.pdf` at 4×.

**What this says about the knob** built two rounds ago is more than the five pages: a lane no
instrument watches is a lane whose defects wait for one. The packer bug was in every release since
ADR 0021 and no gate on either side could see it.

## §18 was answered from the standard rather than from our reading of it

`doc/todo/11` recorded that `render-quorra` composed a clip chain by multiplying, inside the
graphics library, where `render-cpu` had taken `min` (ADR 0280) — so the two backends composed
clips by two different rules and **no gate could see it**.

quorra's ADR 0030 takes `min` there too, and reaches it from §8.5.4 directly:

> After the path has been painted, the clipping path in the graphics state shall be set to the
> intersection of the current clipping path and the newly constructed path.

The graphics state holds *one* clipping path; rasterising each link separately is a convenience,
and nothing in the standard composes two fractional coverages — the one genuine product of shape
values is §11.5's soft mask, which has its own clause. That is a stronger derivation than ADR
0280's, which argued from which estimator moves further from the clause.

Its measurement declines to arbitrate, as ours did: product chain, `min` chain, and `min` chain
with a `min` mark all give 915 / 37 / 5 over these 957 pages with no per-page line moving. **Both
sides still multiply where the clip meets the *mark*, and both now record that as a choice with the
same reason** — two unrelated boundaries in one pixel are the common case, and only a
conflation-free rasteriser answers the clause.

Three claims in this tree said quorra still multiplied. All three were true when written and are
corrected here: §10.7.4's ledger note, `doc/todo/11` item 4, and `doc/todo/_scan-conversion.md`.
**A claim about somebody else's code decays on their schedule**, which is ADR 0283's sentence
arriving for the second time in six rounds.

## §9's first frame, measured at the minimum because the effect is smaller than our spread

`a35dc70` timed the inside of `Device::render` and found **2.43 ms of the first frame was creating
that frame's own timestamp query** — a `QuerySet` and two buffers per frame, which the driver
charges for the first time and pools afterwards. One lives with the device now.

This side's instrument measures a whole `rasterize` on a machine with other work on it, and its
spread is three times the effect — three samples an arm showed nothing. So: **A/B/A, eight samples
an arm, read at the minimum**, sorted milliseconds for frame 1 of page 7 at 1× on the CPU lane:

```text
A (a35dc703)  12.77 13.31 16.19 16.59 18.87 19.15 20.46 25.62
B (74c4994d)  14.94 16.10 22.67 23.57 25.61 27.45 30.38 30.89
A (a35dc703)  12.47 13.51 14.12 16.34 18.35 18.53 18.70 21.92
```

**14.94 → 12.77 and 12.47**, which is 2.2 to 2.5 ms and reproduces the stated 2.43; both `A` arms
agree with each other, which is the drift control. The medians move the same way (24.6 → 17.7 and
17.3) and are worth less, because a median under load measures the load.

**The rest of §9 is not an optimisation.** About 6 ms remains inside `run_frame`, scaling with the
target — page-sized textures and the driver's first touch of a heap that size — and a warm-up
thread cannot allocate those before the viewport exists. quorra records it as the caller's contract
rather than taking it, which hands the question back, and §9.1 is this side's answer: **we are not
asking for `Device::warm_for` yet**, because a host that could call it would need its viewport
before its first frame and `viewer-ui` learns that from `Resized`. What would settle it is a launch
measured on the real adapter through a real window, and this machine cannot take one.

## Gates

Whole `tools/state.sh` sequence: every verdict identical — corpus 65 incomplete, oracle
905/68/786/1/2/14/18, both text gates, quorra 915/37/5/17, 1623 tests, ledger 875 rows.
