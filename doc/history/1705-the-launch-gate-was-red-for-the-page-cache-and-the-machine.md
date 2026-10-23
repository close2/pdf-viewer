# 1705 — The launch gate was red for the page cache and the machine, not for a commit

Session 1705, 2026-10-23. Batch 109, slot 2 (measure). ADRs 2244, 2245; no Q.

**Premise did not hold.** The brief's failing memory figure was `peak_anon_mib`. It was
`open_peak_mib`, the open child's whole `VmHWM`, and that is not an anonymous figure.
**Calibrated first** (habit 113, trap 13): seven parts in eight of that `VmHWM` are file-backed,
the test binary's and the C library's text (`smaps_rollup`). That share moved 6.9 to 7.5 MiB
by hand and 10 to 11 MiB in the gate's runs. The anonymous share stayed the same to the
kibibyte: 0.805 MiB on every child of `PDF20_AN001-BPC.pdf`.

**Hypothesis.** Half of it held. The copy and path sensitivity was the instrument counting mapped
pages. The other half failed: no commit allocates at open, because twelve children each of
`5ded4916` and `ee365e4f` give identical anonymous figures on all six rows. So the brief's
bisect had no oracle that moved, and it was not run. Today the base fails `open_peak_mib` on
every row too.

**Change (ADR 2244).** The row's figure is `open_anon_mib`: the anonymous high-water of one extra
small-pages child per row (ADR 1902's), banded by the file's allocation rule over HEAD's twelve
children. `VmHWM` stays printed and unbanded. A planted live megabyte read 1.812 MiB against
0.805 and fails the new 1.0 ceiling. The old band held that plant at 8.8 MiB. Owner's A32 is kept
on its own premise.

**Timing (ADR 2245).** No band moves. Base, HEAD and mine run the same instructions, and the
base fails the same warm-open mode. The merges' failing children had calibrations of 0.711 to
0.758 ms; this round's read 0.669 to 0.704.

**Gates.** Hold 1 (`--clock`, nine runs, base/HEAD/mine interleaved, load 2.9–4.7): base 3 of 3
exit 101, HEAD 3 of 3 exit 101 (`open_peak_mib` on every row), mine 3 of 3 exit 0. Hold 2
(`--clock --nice 0`): mine before a rebuild exit 101 (`opt_demo` warm 0.149), the canonical line
after rebuilding exit 0, then exit 0 twice (the last not judging, calibration 0.905 after idle).
Counted run of mine exit 0, 31 banded, 0 outside. `rustfmt --check` 0; `cargo fmt --check -p
viewer-ui` 0; clippy `-D warnings -p viewer-ui --all-targets` 0; `cargo nextest run -p viewer-ui`
137 passed, 1 skipped; `tools/state.sh --round 1705 launch` 0, 31 banded, 0 outside. `cargo test -p
conformance` in the worktree fails only on a sibling's §10.7.4 row naming ADR 2252, which has no
file yet. In an export of this round's six files it passes, apart from four tests that read git.
Those four pass in the worktree. Logs: `scratchpad/r1705/{cal,hold1,hold2}`.
