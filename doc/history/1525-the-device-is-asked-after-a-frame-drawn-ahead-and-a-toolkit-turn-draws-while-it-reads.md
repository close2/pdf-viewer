# 1525 — The device is asked after a frame drawn ahead, and a toolkit turn draws while it reads

Slot 4 of batch eighty-three, 2026-10-12, a loop round of the owner's performance target; the
partial-row word yields to it (brief §3). ADR 1886; no ADR 1887, no question. No ledger row moved.

**Premise.** Held in the code (`lane_for` `surface.rs:100`, `Costs` `renderer.rs:204`, no toolkit
caller of `handle_unread`), but `lane_for` picks a coverage lane on the device; the notch's lane is
`Costs`'. What kept notches one to five on the processor was ADR 1860's one ask waiting for a
*measured* processor price, which a frame finished ahead and a notch taken back never give.
**Hypothesis.** Half held: the device is the cheaper lane from the second notch on (0.32 s kept
against 0.53) and the prediction says so; ten notches under 0.25 s did not — the device's build at
the first notch is 0.75–0.84 s and kept frames at the low notches 0.26–0.36 s.

**Built.** `Measured::processor_drew`; the device priced built or kept (`FrameCost::scene_kept`,
`Costs::holds`); `device chosen: …` on every device frame; seeds re-taken in the window (187, 73 ns;
the processor's 120 kept); `quorra-gtk` and `quorra-qt` turn through `Host::turn` beside
`viewer_host::Ahead` (ADR 1871 §2.2's shape).

**Figures** (owner's display, four notch runs and three turns an arm, HEAD `56e42ea5` → this).
Notches with a frame of their own 2, 1, 2, 3 → 4, 5, 5, 5 of ten, each on the device in 0.187–0.308
s; last notch 0.250, 0.272, 0.196 → 0.223, 0.203, 0.187, 0.210 s. Turns: GTK 2.96, 2.45, 3.27 →
2.16, 2.14, 3.12 s, Qt 3.44, 3.41, 2.49 → 2.03, 2.09, 2.09 s, the draw after the read 509–563 →
49–113 ms. Xvfb, six an arm: GTK 2.94–3.62 → 2.19–3.43 s, Qt 2.44–3.41 → 2.17–3.24 s; the read beside
the drawer is up to a second slower there and hides the 0.49–0.79 → 0.05–0.18 s saving.

**Unfinished.** The device's scene built while nobody waits (ADR 1886 §4); the low notches' encode.

**Files.** `crates/viewer-ui/src/bin/quorra/{renderer,surface}.rs`, `crates/viewer-gtk/src/host.rs`,
`crates/viewer-qt/src/host.rs`; ADR 1886; `doc/todo/69` (section 4), `doc/state-of-play.md`; this.

**Gates.** In an export of HEAD plus these hunks, own target directory: `rustfmt --check --edition
2024` on the four files, exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p viewer-ui -p viewer-gtk -p
viewer-qt --all-targets` exit 0 (the worktree's failed in siblings' `pdf-model` and `raster-gpu`);
`cargo nextest run` on the three, 234 passed, 2 skipped, exit 0. `cargo test -p conformance` in the
worktree: 396 passed, 1 failed — `names`, a sibling's `raster-gpu/src/encode/instance.rs:250`. The
drive, all four windows (lane 1, held 1 044 s): 279 works, 1 wrong — Qt's `60-script-timer` at load
6–9, then works twice alone; the timer turns by script, not `GoTo`. Two clock holds, `--tree 12`: one
queued 150 s, held 413 s, exit 0 — `zoom_frame`, Xvfb turns, the owner's display (its toolkit turns
found no window under Wayland), `launch_path` 31 banded, 0 not judged, 0 outside; two queued 169 s,
held 288 s, exit 0 — Xvfb turns and the owner's display again under X11. Duration 5 960 s.
