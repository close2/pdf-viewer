# 2667 — A first page's memory is judged beyond the device's own allocation, and the device's figure is the driver's

Session 1918, a single round (instruments). Status: **accepted**. Context: ADR 0910 and ADR 0911 (the
anonymous figures), ADR 1902 (small pages), ADR 2660 §2.6 and §3 (the server VM's memory findings),
ADR 1969 and ADR 2242 (the clock rule and a red gate), ADR 2666 (its companion for the count). It
cites no clause and moves no ledger row.

## 1. What the figures contained

`bring_up_anon_mib` read 8.9 MiB on the RX 580 through RADV against a band of 4.8 .. 7.5, and four
rows' `peak_anon_mib` read 0.8 to 6.9 MiB past theirs. The bring-up child's own `/proc/self/smaps`,
dumped at its exit by a preloaded destructor, holds 10.5 MiB of anonymous pages:

| mapping | anonymous, KiB | whose |
|---|---|---|
| `libLLVM.so.19.1`, which Debian's RADV links | 5564 | the loader's relocation of the driver's library |
| other libraries of the driver's chain (`libz3`, `libstdc++`, `libxml2`, …) | about 450 | the same |
| unnamed anonymous mappings | 4328 | the heaps: RADV's, wgpu's and this program's |
| the test binary's own relocated data | 324 | this program's |

A first-page child of the smallest row holds the same library pages and 8920 KiB of heap. Neither
the rayon pool's width (4 against the machine's 32) nor the huge pages (8.90 against 8.92 MiB) moved
the bring-up figure.

## 2. The evidence that the excess is the device's

Each row's small-pages first page less the bring-up child's figure, on the server VM, against the
notebook's small-pages children (`launch-path.toml`, ADR 1902's twelve per row) less the notebook's
bring-up of the same week, 5.309 MiB:

| row | server VM, MiB | notebook, MiB |
|---|---|---|
| `PDF20_AN001-BPC.pdf` | 12.2 | 12.1 .. 12.8 |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 15.1 .. 15.3 | 15.3 .. 16.3 |
| ISO 32000-2 | 20.9 .. 21.0 | 20.6 .. 21.4 |
| `xfa_filled_imm1344e.pdf` | 5.5 | 5.2 .. 6.0 |
| `opt_demo.pdf` | 8.8 .. 8.9 | 8.7 .. 9.3 |
| `bug1815476.pdf` | 30.1 .. 37.9 | 38.0 .. 47.7 |

Five rows agree to a few tenths; the sixth spreads by eight mebibytes on both machines. And
`open_anon_mib`, the open with no device in the process, held on every row on both.

## 3. The decision

1. **Each row's `peak_anon_mib` is its small-pages first page less the device's own allocation**,
   the bring-up child's figure from the same run. With no bring-up figure the row's memory is not
   judged and the run says so.
2. **The rows' bands are the notebook's less the notebook's bring-up**, 5.309 MiB, rounded outwards
   to half a mebibyte: no reading of the server VM is in them.
3. **`bring_up_anon_mib` is `none`**: printed on every run, and not banded as this program's. It is
   the driver's build: the libraries the driver links and how the loader relocates them, and a Mesa
   upgrade moved it by a fifth on the notebook with the tree still.

## 4. What it costs

An allocation this program makes while bringing the device up — `raster`'s pipeline store, the
device's own buffers — is in the bring-up figure and in every row alike, so the subtraction takes it
out of both and no band holds it. A band for it belongs with the driver's figures in
`doc/checks/machines/`, which `tools/bands.sh retake` does not yet take.

## 5. A warm open just under its floor, with its open's work done, is printed rather than failed

Three of ten gate runs on the server VM put one warm open under its floor: `PDF20_AN001-BPC.pdf`
1.471 ms against 1.70 and `bug1815476.pdf` 1.018 against 1.08 in round 1917, `PDF20_AN001-BPC.pdf`
1.508 against 1.63 in this round's second run, 4 to 8 % under, at a one-minute load of 15.5 where
the band's seven runs had read 1.921 to 2.339 at 15 to 25. In each the row's `read_kib` and
`read_calls` were inside and its instruction count read what every run read: the open did all its work, and the machine was
quicker than any of the band's runs. A floor that fires on three runs in ten is a floor whose red
nobody reads, which is the always-red gate this round exists to end.

What a floor still sees that nothing counted does is a clock that stopped before the work did, and
that reading is far under. So:

1. **A warm open under its floor by no more than the band rule's own lower margin** — at least
   0.85 of the floor — **whose row's `open_kinstructions`, `read_kib` and `read_calls` were judged
   inside in the same run is printed, counted in the summary, and not failed.** Further under, or
   with any of the three outside or not counted, it fails as before, and ADR 1969 §3 and habit 101
   judge it.
2. The cold open keeps its floor whole: its figure is mostly the disk's trips, and a too-quick cold
   open is the page cache keeping what it was told to drop, which `cache_left_kib` already declines.
3. The bands themselves are not moved; a re-take that takes in the quicker readings is still the
   answer when they recur (`tools/bands.sh`).
