# 1963 — A slow section is not a hung card: the kernel decides the hang marker

Session 1963, a single round, PDF/A lane's worktree. ADRs 2740, 2741.

**The premise.** ADR 2665's marker stopped every lane twice: 2026-11-05 08:51 (`frame`, 12 566 ms)
and 2026-11-06 07:40 (`turn-round`, 9 407 ms, round 1958). `journalctl -k`: the first window holds
08:51:22.83 `amdgpu: ring gfx timeout, but soft recovered`; the second holds no amdgpu line at all,
at a load of 28 to 31. The 07:40 section is `frame_cost::round`'s hold: three bring-ups and a dozen
frames under one lock. Slow, not hung.

**Decided (ADR 2740), option (c).** ADR 2665's bound is the ring timeout read backwards, so it is
necessary and not sufficient. A section past it now asks `journalctl -k -o short-unix` from a second
before it began, for up to five seconds, for amdgpu's timeout, reset, fault or illegal-register
line. Any process counts, because the soft-recovered line names none. A line found writes the
marker, with the line in it. A quiet log appends `<lock>.pdf-viewer-slow` and warns on stderr.
A log it cannot read writes the marker on the bound alone. Round 1951's `turn.log` stamps the 11-05
marker in second 1793868682 and the kernel line 0.83 s into it, so the line can follow the
section's end and the wait finds it. (a) alone is a call per section; (b) cannot tell a fence wait
from starvation. `journalctl -k` reads here (`systemd-journal`); `/dev/kmsg` does not.

**Tests** (`render-raster --test gpu_lock`, stub `PDF_VIEWER_GPU_KERNEL_LOG`): soft-recovered line →
marker, next section refused, allowed after removal; hour-old timeout plus a non-amdgpu line → no
marker, a warning, next section runs; no log → marker `kernel=unread`; `hang_line` on ADR 2665's
lines, the window edge and an NVMe timeout. Plants: the window ignored and a hang not marked → three
failures; the old clock-alone marker → the slow test fails, naming the marker.

**`bands.py` (ADR 2741).** It dropped load and clock declines and kept a thread-wait, first-pass,
carve-out, disk or cache decline. Its rule is bands over kept readings, and a kept reading is one
the gate would judge, so every `not judged:` reading is dropped now. On round 1958's logs this moves
`bring_up_ms` to 45.0 .. 64.8 (2 of 3 kept), and two first-page bands narrow (114.1 → 112.6,
108.2 → 91.3). The committed bands stand until the next quiet retake.

**Owed.** The 07:40 marker stood throughout: no `launch_path`, `turn_path`, `toolkit-launch`
retake or hardware nextest ran.

**Gates.** `cargo fmt --check` 0 and clippy `-D warnings --all-targets` 0 over render-raster,
raster-gpu, viewer-device, viewer-ui, viewer-host and render-gpu; `render-raster --test gpu_lock` 0
(7 passed, 1 ignored); nextest of render-raster, raster-gpu and viewer-device on lavapipe 0 (1002
passed, 940 bring-ups on `Cpu`, no hold); `cargo test -p conformance` 0 (471 passed) after `git add
-N`; `tools/state.sh --round 1963 docs` 0.
