# 1913 — The clock bands are the server VM's

Session 1913, a single round (measure) on the server VM. ADRs 2660 and 2661; Q776 unused.

**What.** Every clock band, `machine` line, calibration band and disk probe in `launch-path.toml`,
`turn-path.toml` and `toolkit-launch.toml` is this machine's, by ADR 1969's rule over every run of
HEAD under the load ceiling (ADR 2660); `device_busy_percent` stays 20 (0.0 % idle, 95.8 % planted).
The turn gate declined every figure for want of a faster class of core (no `cpufreq` here) and now
judges unpinned (ADR 2661). CLAUDE.md's GPU bullet carries the owner's rule of 2026-11-03.

**The notebook's probes, as the files stated them** (for the factor question round 1914 asks):
launch calibration band 0.62 .. 0.74, good children 0.667 .. 0.715 ms; turn calibration band
0.62 .. 0.78, children 0.659 .. 0.802, median 0.735; `io_latency_ms` 0.073 .. 0.25 over 0.086 min,
0.098 median, 0.401 max; `io_ms` 2.0 .. 7.0, reading 2.8; `bring_up_ms` 17.0 .. 25.3 over 19.99,
21.01, 21.04 ms. Here: calibration 1.979 .. 2.010, `io_latency_ms` 0.556 min, 0.740 median, `io_ms`
12.3 min, 17.4 median (a rotating disk), bring-up 65.7 .. 71.5 ms; page one 98 .. 215 ms against 20 ..
37, three to eight times the notebook where the probe is three.

**Premise.** Held: the bands were the notebook's, and the calibration probe (2.0 ms against 0.74)
kept the launch gate from judging any clock. Not held: "re-taking leaves the counted ones green".
`read_kib`, `read_calls` and `open_anon_mib` hold on every row; `open_kinstructions` is past every
row's band but ISO 32000-2's by a near-constant 320 to 530 thousand (4497.5 against 3933 .. 4070 on
the five-page row), identical over every run; `peak_anon_mib` is 18 to 21 MiB over every row's band,
and `bring_up_anon_mib` reads 15.1 to 15.5 against 4.8 .. 7.5. Findings, not re-banded (ADR 2660).

**Quiet.** Not one load: the other account's work on CPUs 32-47 moved this account's figures. At a
one-minute load near 30 the turn gate's ISO page read 30 to 31 ms, near 7 it read 27; the five-page
warm open read 2.0 ms near 30 and 1.26 to 1.41 near 7 to 24. Bands from one hold at either load
failed the next run at the other, so each band is the rule over every run under the ceiling (five to
seven a figure, loads 5 to 44), not three of one hold. Two turn rows' `step_lowest_ms` stay the
notebook's: no run read at the RX 580's lowest level.

**Gates.** On the new bands, each run under its own `with-gpu-lock`: `t2-launch_path` exit 101,
every clock inside, 3 not judged for a child's probe, 12 outside, all the counted findings above.
`t2-turn_path` exit 0, 33 banded, 27 judged, 0 outside. Toolkit (`--no-fail-fast`, so Qt runs after
a GTK failure) exit 0 for both windows. `fixed_documents` exit 101: 0 of 77 rows on this machine,
the SafeDocs crawl never fetched here. `state.sh --round 1913 launch` exit 101, the same 12. `cargo
test -p conformance` exit 0, 471 passed, `bounded`'s self-test green under gawk. `render-raster`:
fmt and `clippy -D warnings` exit 0; nextest exit 100, 101 of 116: 15 device-pixel tests red on the
RX 580, outside this diff; not chased.
