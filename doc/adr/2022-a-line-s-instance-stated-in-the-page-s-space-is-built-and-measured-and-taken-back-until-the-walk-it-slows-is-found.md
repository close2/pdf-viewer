# 2022 — A line's instance stated in the page's space is built and measured, and taken back until the walk it slows is found

Session 1593, slot 2 of batch ninety-four, `doc/todo/75` item 2 on the project owner's answer
`doc/questions/A473` (2026-10-19, "I agree with both recommendations, please answer them"). Status:
**accepted as a measurement; the build is not in the tree.** Context: ADRs 0085, 0097, 1420, 1877 §5,
1884, 1992, 1993, 2008, 2030; raster's ADR 0082. Cites no clause in the tree and moves no ledger row.
What stays is the instrument: `render-raster`'s `examples/zoom_frame.rs` `ZOOM_FRAME_DUMP`, which writes
each frame's bytes under either backend so two builds, or a device frame and the processor's, are
compared byte by byte where a digest only says that they differ.

## 1. What was built

`line.wgsl` read its instance in the page's space — the endpoints after the line's own transform, the
pen's semi-axes there, Table 53's end packed with how the view resolves the width (by its stretch; by
its stretch or one device pixel under §10.7.5's adjustment; fixed for §8.4.3.2's zero), and the clip's
rectangular links intersected in the page's space — and mapped all three by a view `Globals` carried,
collapsing the quad of a line the view admits nothing of. The host decided the road in the page's
space, so the stream was a function of the scene; `LineRuns` named the commands it was written for, as
exact runs, and a retained encode at a new view adopted its predecessor's device buffer where the runs
were equal. Tests held the ladder to the set's exact area to one level through a stretched and a
turned view, and a zoom step to zero line bytes and to the bytes of a device that never saw the first
view; planted back (the view's offset dropped, the adoption refused), each failed. The patches are in
`/home/AI/r1593-line-lane/` with a README, outside the repository.

## 2. What it measured

One `--clock` hold (load 7.4 → 5.8, the Radeon 890M through RADV, headless), HEAD's export against the
change's, interleaved, the witness at the fit (0.594) then 1.25×, minimum of three rounds, two runs:

| the 1.25× step | HEAD `Cpu` | change `Cpu` | HEAD `Compute` | change `Compute` |
|---|---|---|---|---|
| frame | 490.1, 357.2 ms | 408.1, 408.6 ms | 397.2, 334.0 ms | 399.2, 428.2 ms |
| host encode | 191.3, 140.7 ms | 208.0, 212.8 ms | 145.6, 120.0 ms | 202.3, 240.3 ms |
| transfer | 46.4, 29.0 ms | 1.7, 1.2 ms | 47.1, 30.7 ms | 1.4, 1.6 ms |
| line bytes | 263 935 920 | 0 | 263 935 920 | 0 |

In the window (`ZOOM_FRAME_WINDOW=900x1100`, `Compute`, 2.4 then ×1.25 and ×1.5) the step went from
151.4 and 144.1 ms to 251.7 and 248.5: a line the view does not admit is an instance now, so the walk
writes all 4 398 932 every step where HEAD wrote 332 995 and 235 905, and the first frame sends 264 MB
where HEAD sent 26. The eleven turn rows were inside their bands in both runs of the change (33 of 33;
HEAD's second run had EC3's step outside, 2.41 ms against 2.10).

**The transfer is saved and the walk loses more than it.** `encode: recording` on the witness's fit is
84–108 ms for HEAD and 146–217 ms for the change in most processes and HEAD's in some (pinned to the
four full-clock cores, unpinned, and with address randomisation off, where the change is slow every
time). It is not instructions: callgrind counts the walk's thread at 933 M for HEAD and 924 M for the
change and the line makers at 955 M and 1 016 M, with the same simulated cache misses and branch
mispredictions; nor the makers (the walk made 1–5 of 1 077 blocks itself and waited 0.2–0.5 ms on them);
nor page faults (167 352 against 167 025); nor the per-line run (a build without it was as slow); nor the
instance's layout. A walk of 4 400 000 witness-shaped lines inside `raster-gpu`'s own tests is the same
for both, 79–100 ms. So the cost is real, reproducible and unattributed, and **by principle 2 a build
whose shipped path is slower is not merged**: the step it was for is no faster on the page and 100 ms
slower in the window.

## 3. What the pixels did

The six corpus arms' device four (gpu and compute at 1× and 4×) moved no page against
HEAD's arms — the corpus gate bakes the target into every command under an identity view,
where the shader's mapping is the host's to the bit. The page-space path is the window's, and there
`ZOOM_FRAME_DUMP` on the witness held the change to HEAD within 6 225 bytes at the fit (≤ 3 levels) and
10 889 at 1.25× (≤ 6 levels, 71 bytes past 3), mean 0.0011 and 0.0013 levels; against the processor's
frame the change's mean is 0.16427 where HEAD's is 0.16425 (`Cpu`) — the oracle bound unchanged.

## 4. What the next round owes

Find the walk's cost before the build comes back: a hardware profile (no `perf` on this machine) or
a build bisected between HEAD's `place_line` road and the change's. And keep the host's cull until item
3 builds the op list once: without it the window writes the page's lines every step.
