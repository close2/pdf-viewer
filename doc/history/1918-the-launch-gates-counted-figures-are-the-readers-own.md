# 1918 — The launch gate's counted figures are the reader's own

Session 1918, a single round (instruments) on the server VM. ADRs 2666 and 2667; Q777 unused. Worked
in the rounds worktree and its `scratchpad/r1918/`.

**Why.** `t2-launch_path` was red here at every run since round 1913: `open_kinstructions` 315 to 571
thousand past five rows' bands, `peak_anon_mib` 0.8 to 6.9 MiB past four, `bring_up_anon_mib` 8.9
against 4.8 .. 7.5 (ADR 2660 §3). A gate that is always red detects nothing.

**The count (ADR 2666).** Callgrind by object: of the smallest row's 1484.9 thousand, 821.7 are the
dynamic loader, 389.9 the C library, 272.3 the test binary. The notebook linked with the default `cc`;
this machine's user cargo configuration names `mold`, which writes no `DT_RELACOUNT`, so relocation
costs 320 thousand more on every row — the binary relinked by GNU `ld` read exactly that much less.
Three rows then sat inside the notebook's bands, the other three over by about a tenth of the C
library's share of each open. Not the processor: valgrind gives every program its own Intel CPUID
model. The figure is now the binary's own object over every thread, the same linked either way; its
bands are this file's rule over four children of HEAD.

**The memory (ADR 2667).** 5.4 of the bring-up child's 9.3 MiB are pages the loader wrote relocating
Debian's `libLLVM.so.19.1` for RADV. Each row's first page less the bring-up figure reads where the
notebook's did less its own (12.2 against 12.1 .. 12.8 MiB on the five-page row; five rows agree to
tenths). Rows are judged beyond the device's allocation, against the notebook's bands less its
bring-up, 5.309 MiB; `bring_up_anon_mib` is `none`, the driver's. Rayon's width moved neither.

**The warm open's floor** (ADR 2667 §5). Three runs in ten put one warm open 4 to 9 % under its floor
with its bytes and calls inside and its count unchanged. One no further under than 0.85 of its floor,
whose three counted figures held in the run, is now printed and counted, not failed.

**Gates.** `t2-launch_path` on RADV, five `--clock` holds in order: exit 101 (an object first named
in a `cob=` line went uncounted; fixed, and a test plants it); exit 0, 0 outside; exit 101, one warm
open 1.508 against 1.63, before the floor rule; then with it exit 0 twice running, 58 banded, 0
outside, one warm open 9.4 % under printed in the first. `t2-turn_path` exit 101: 30 judged, 3
outside, clocks 1 to 4 % under floors in `doc/checks/machines/`, untouched here. `viewer-ui`: fmt
and `clippy -D warnings` exit 0, nextest on lavapipe 139 passed (a suite on the card was refused).
`cargo test -p conformance` exit 0. No amdgpu reset after any run.

**Unfinished.** `bring_up_anon_mib` has no band; a per-machine one needs `tools/bands.py` to read it.
The clock floors were taken under another account's load and a quiet minute crosses them; the turn
gate's three are a re-take's. No notebook profile by object exists to compare the count with.
