# 1724 — The device's bring-up is the loader and two device-select probes, and holds no pipeline

Session 1724, 2026-10-23. Batch 111, slot 5 (measure). ADRs 2282, 2283; no Q filed.

**The split.** The launch gate's own bring-up and first-page children, run one per process on the
owner's session in three environments interleaved, and both windows read off their own traces: two
loop runs, each in one `--clock` hold at one-minute loads of 1.8 to 7.4, no capture. Headless (no
display variables) the child reads 26.1 to 30.7 ms, instance creation 13.6 to 17.4 of it. With the
session's variables it reads 42.4 to 53.8: Mesa's device-selection layer asks the compositor on both
of the launch's enumerations, adapter check 12.4 to 22.3 and adapter 5.7 to 13.0, gone under
`NODEVICE_SELECT=1`. `quorra-qt` brings the device up in 59.0 to 87.0 ms at a quiet load, ISO 32000-2's
page 20 included, so round 1716's 135 to 227 ms was its load of 30 to 44 (trap 237); its device is up
before its window's first size in all fourteen launches. `quorra`'s first present: 71.3 to 79.1 ms
with the layer, 60.7 to 60.9 without.

**Hypotheses.** The first did not hold: no pipeline is in the bring-up — raster returns the device
before its warm set. The first frame compiles `LineOver` and the read-back copy itself, 0.9 to 2.0 ms
with RADV's on-disk cache and 57.5 to 60.3 ms without it. The second held as RADV's cache, already
on (first page 54.7 to 68.6 ms against 137.8 to 161.3). The third held as far as this session says:
the layer is 13 to 25 ms here, so the nested compositor's second is the nested compositor's.

**Levers** (ADR 2283): a cache of this program's own and a warmer warm set have ceilings of 2 ms;
enumerating once is worth 5 to 9 ms of `quorra`'s first present, 0 on the gate and 0 on the toolkit
windows' first frame, and is deferred until a gate can see it. Nothing built; the bands stand.

**The change.** `quorra-qt` and `quorra-gtk` print the device thread's five steps and the remainder
on their `graphics device up in` line (`viewer-device`'s `Steps`, ADR 2282), so a window's figure
is split where it is taken. On an idle Xvfb the layer costs 145 to 220 ms more (the display's).

**Gates.** In the export (habit 138; a sibling's `pdf-model` edit broke the worktree's build):
`cargo fmt --check -p viewer-device` exit 0; warnings-denied clippy `--all-targets` exit 0; `cargo
nextest run -p viewer-device` 92 passed, 1 skipped, exit 0. `t2-toolkit_launch` in one `--clock`
hold at a load of 4.5 to 5.7, the tree's binary and HEAD's: Qt exit 0 both (`device_up_ms` 44.7
against 45.0, first frame 91.9 against 92.2); GTK exit 101 both (`device_up_ms` 35.9 and 36.5 over
35.3). `t2-launch_path`, two interleaved pairs in one `--clock` hold at 5.5 to 10.9: HEAD exit
101 and 0, the tree 101 and 0 (101 again in the earlier hold), each 101 a sub-millisecond open;
bring-up 26.7 to 29.4 ms against HEAD's 27.1 to 27.6, not judged for the carve-out; `nm` finds no
`read_back` code in `quorra`. `cargo test -p conformance --no-fail-fast` exit 0, and `--test
evidence --test adr_status --test records` exit 0 (`records` failed once mid-batch and passed on
the rerun). Warnings-denied rustdoc on `viewer-device` exit 0; `tools/state.sh docs` withdrawn after 1294 s queued.
