# 628 — The launch abort was a configure nobody checked, and the acquire after it

Date: 2026-08-21.
ADR: [0462](../adr/0462-the-first-configure-is-the-one-that-can-kill-you.md).
Feedback to quorra: `doc/QUORRA_FEEDBACK.md` section 35.

**Finding.** The project owner's viewer core-dumped on launch. `Surface::configure` failed, this
program printed the note its uncaptured-error handler was told and presented anyway, and `wgpu`
answered the acquire with a panic rather than a status — because it chooses between panicking and
refusing by a field that is set **only by a configure that succeeded**. So the fatal branch exists
on the *first* configure of a process and nowhere else, and the first configure is the one this
program raced by design: ADR 0391 dispatches a render on another thread and presents on this one,
microseconds apart, and a submission inside the configure's wait is the documented cause of the
failure.

Touched: `crates/render-quorra/src/{lib,present,uncaptured}.rs`,
`crates/viewer-ui/src/bin/pdf-viewer/{renderer,surface,timing}.rs`,
`crates/pdf-model/src/requirements.rs`, `doc/conformance/ledger.toml`,
`doc/QUORRA_FEEDBACK.md`, `doc/traps/pixels-and-rasterisers.md`, `doc/HANDOVER.md`,
`doc/running-the-viewer.md`.

## Reproduced here, which `doc/environment.md` says is possible and this round needed

`Xvfb` + `lavapipe` give a real window, a real event loop and a real swapchain, and under them the
unfixed binary produced the owner's output byte for byte — the same note, the same panic, the same
`wgpu_core.rs:4036:26`. It is rare and it is real.

`race.sh` launches a binary N times on `doc/PDF20_AN001-BPC.pdf` with `--proxy-pages 12`, greps
for `In Surface::configure` and for `get_current_texture_view`, and keeps the output of any launch
that shows either. Debug binaries, both built from this worktree, the machine carrying three other
rounds' gates at the time:

| | launches | configure failed | **aborted** |
|---|---:|---:|---:|
| before | 150 | 5 | **5** |
| after | 60 | 0 | **0** |

**Every configure failure in the "before" column was the crash** — the same five run numbers appear
in both columns — which is what the branch in wgpu says it must be, and is worth having watched
rather than deduced. Two earlier samples of the unfixed binary, 80 and 60 launches, gave 8 and 2 by
the same script and are not added to the table: the 80 was taken before the script kept the failing
output, so it cannot say which branch each took.

**The "after" row is 60 rather than 150 because that is the run that finished**, on a machine
carrying three other rounds' gates; a 150-launch run of the same binary was still going when this
round closed and its number is not written here for the reason ADR 0281 gives — a later round runs
`race.sh` rather than reading a figure. The two rows are therefore not a matched pair, and the
weight is not meant to be on the arithmetic anyway: what makes the "after" row zero is that after
`ground` the surface has been configured, and wgpu's fatal branch requires one that has not.

## What the launch cost, since a configure moved onto it

`--trace=launch` on the same document and display, five launches apiece, process start to first
present, in milliseconds:

| | samples | median |
|---|---|---:|
| before | 321.3 437.2 341.7 365.8 462.1 | 365.8 |
| after | 397.2 407.6 372.4 351.8 368.1 | 372.4 |

Unchanged inside a spread that is wider than the difference. Grounding itself cost 17.1, 35.9,
19.7, 18.7 and 45.3 ms on those five runs and came off the first page's own present, which is what
the ADR argues it would: the swapchain's creation and the window's first acquire are paid once
either way. **These are debug binaries under `lavapipe`**, so they are a comparison and not a
figure: `tools/state.sh` and the release binaries are what a launch number comes from.

One thing the runs turned up that is nobody's fault here: with two presents on one swapchain, the
Vulkan validation layer reports `VUID-vkAcquireNextImageKHR-fence-10066` once. `wgpu-hal` passes
the swapchain's own fence to every `acquire_next_image` and waits on and resets it **only under
`#[cfg(target_os = "windows")]` (`wgpu-hal-30.0.0/src/vulkan/swapchain/native.rs:494-506`)**. The
before-binary shows it too as soon as it reaches a second acquire; an idle window under `Xvfb`
presents exactly once, which is why the first comparison appeared to show a regression. Not
reported upstream by this round — it is one grep away from being confirmed on a real driver and
this round had no session to confirm it in.

## The spec-driven half

§12.11.2's row warns that `requirements::Kind::unmet`'s answers are "a claim about this tree rather
than about the standard, so it decays exactly as a ledger row does" and records two decays already.
Reading it against the source found the third: `Kind::Transitions` said "the animation between two
pages is not drawn", and it has been drawn since session 393 — **§12.4.4's own ledger row has said
so for two hundred and thirty-five sessions, beside a source sentence saying the opposite.** A row
has a gate and a reason in `requirements.rs` does not, which is where the sweep has to point. The
arm now names what is missing: five of Table 164's twelve styles state no quantity a frame could be
shaped from and are reported by name.

## Gates

Run in this worktree, on a machine carrying three other rounds. `cargo fmt --all --check` and
`RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` silent; `cargo nextest run
--workspace` and `cargo test --workspace --doc` green; and the full §2 sequence, because
`pdf-model` changed. Their own summary lines are what they are — `tools/state.sh` prints them and
this file does not repeat them (ADR 0281).
