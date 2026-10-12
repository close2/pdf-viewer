# 1517 — A page turn is drawn while it is read, `quorra --cpu` draws page one so, and the bring-up band's excess is the desktop's carve-out

Slot 3 of batch eighty-two, 2026-10-12, a loop round on the owner's performance target. ADRs 1871,
1872; no question; no ledger row moved (the partial-row word yields to the target).
**Premise** held: ADR 1855 §5's three paths read and drew as before, and a turn's read ran inside
`Viewer::handle` on the toolkit thread. **Hypothesis** held for `quorra` once the core split the turn
(`handle_unread`, `read_the_rest`), though no look can show during a turn's read — the toolkit thread
is the reader; for the confined window it held for a turn, not the launch (page one is read inside
`Open`, before any window). **Built**: the viewer.rs hunk named for slot 1 (an `unread` field,
`arrange`'s guard, the two methods, one test); `App::turn` for every `GoTo`; `--cpu` takes page one's
drawing, shows its looks, and draws under `compose_pages_watched`; the worker's `Turning` (Parts and
Underway, no `viewer-host`; a named thread was a `prctl` SIGSYS `tests/confined.rs` caught).

**Figures** (one clock hold, HEAD against HEAD plus these hunks, s, Xvfb ×3 / owner's display ×2):
`--cpu` first look never → **0.536–0.538** on both; first frame 2.52–3.17 → 2.27–2.72, owner's 3.26,
2.57 → **2.17, 2.16**. Turn key → frame, `quorra` 2.42–2.96 → 2.10–2.35, owner's 3.17, 3.53 → 2.94,
3.03; `--cpu` 2.40–2.55 → 2.06–3.11, owner's 2.63, 2.73 → 2.15, 2.34; the read's end to the frame
0.53–0.62 → 0.06–0.10. The confined turn end to end is inside its read's spread (3.14–4.08 both
arms); a probe stamped the worker's stages: draw after the read 504–657 → 53–83 ms, read unslowed.

**Bring-up** (ADR 1872): nothing installed or on the path changed since the band; `request_device`
moved 1.0–1.9 → 7.0–10.9 ms because `wgpu`'s first 128 MiB device-local block lands in GTT (fdinfo
197 476 KiB asked of VRAM, 868 placed) while the session holds 409–420 of the 512 MiB carve-out. Band
kept with the cause; `MemoryHints::MemoryUsage` reads 1.80–4.01 ms (device 15.2–24.3 against
20.3–31.1 ms), `raster-gpu`'s lever. **Unfinished**: the confined launch (ADR 1871 §4); the toolkit
windows' turns can take `handle_unread`. **Habit proposed**: stamp the stage a change moves before
calling it lost on end-to-end runs — the worker's drawer was withdrawn once and reinstated on the split.

**Files**: `viewer-core/src/viewer.rs`; `viewer-ui/src/bin/quorra.rs`, `quorra/{app,composer,dispatch,
surface,window}.rs`, `viewer-ui/src/software.rs`; `viewer-confined/{src/worker.rs,tests/confined.rs}`;
`render-raster/examples/bring_up.rs`; `doc/checks/launch-path.toml`; ADRs 1871–1872; state-of-play; todo/69.

**Gates.** `rustfmt --check --edition 2024` exit 0; `RUSTFLAGS="-D warnings" cargo clippy
--all-targets` for viewer-core, viewer-ui, render-raster and viewer-confined exit 0; `cargo nextest
run` viewer-core 376 passed, viewer-ui 182, viewer-confined 104, render-raster 102 (exit 137 first at
`--tree 6`, 6.82 GiB); `cargo test -p conformance` 432 passed; `awkward_classes` exit 0, 0 killed of 225. One `--clock --tree 12` hold, queued
157 s, held 1 221 s, exit 0: the arms, the owner's loop (exit 0), `launch_path` on the export exit
101 (`bring_up_ms` 25.6 and four `first_page_ms` outside, ADR 1872's cause; counted figures inside),
step 80 4 works 0 wrong, the drive of `quorra` and `quorra-confined` 105 works 0 wrong; three more
holds of 27–30 s for the confined diagnosis, against ADR 1863's one. Duration 11 600 s.
