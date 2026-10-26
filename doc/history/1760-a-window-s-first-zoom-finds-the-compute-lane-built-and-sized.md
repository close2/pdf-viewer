# 1760 — A window's first zoom finds the compute lane built, and sized by its own count

Session 1760, 2026-10-26, a "measure" round of batch 116 (Slot 1). ADRs 2354, 2355; Q623 unused.

**Premise.** ADR 2339's two parts held: the four compute pipelines compiled inside the lane's first
frame, and the edges guess overflowed, so that frame ran twice (HEAD's first window reports the
re-run's 0.05–0.08 ms of residency, against 0.4–0.8 here).

**Built.** `Options::warm_compute_lane`, off by default: raster's warm-up thread compiles the lane
once the device has completed its first frame; a window's device asks (`render_raster::
window_options`: `QuorraWindowRenderer`'s constructors, the toolkits' read-back device, the launch
gate's first page; ADR 2354). The lane's first frame reads its own scanned total before the emit
and is sized by it (ADR 2355). The first build compiled the lane beside page one; the first hold's
launch arms measured it, and the cue replaced it.

**Measured** (two `--clock` holds, headless on the Radeon through RADV, no render-node holder of
this account). First window on ISO 32000-2 p. 20, HEAD → this tree: 17.3–27.0 → 10.6–13.7 ms at
the device's slow speed (loads 2.8–6.6), 8.3–12.8 → 5.2–7.6 at its fast (loads 1.5–2.3), second
window 8.4–10.9 and 4.0–5.6 in both; `tmp/Entwurf.pdf`: 175–244 → 142–172 slow, 124–132 → 97–100
fast, second window 44–46. Digests HEAD's. Launch, second hold: first page 25.8–31.0 → 25.4–30.5 ms,
each document within 1.5 ms, frames HEAD's; the toolkit gate judged and in band four of four.

**Hypotheses.** "The first window falls to the second's": held to 1–2 ms on p. 20; not on the
worst page, whose read waits for its first segment upload (26 ms fast). "Launch gates unmoved":
held, and by construction since the cue. "1.5–5 ms from the single run": measured with the other
lever only; the medians fell 7.6 ms slow and 4.7 fast on p. 20, 24 and 32 on the worst page.

**Found, not fixed.** `tests/capacity_growth.rs` fails clippy under `--features sabotage-capacity`
(three `unsafe` blocks with no safety comment), HEAD's. ADR 2355 §4 prices the read after staging.

**Where.** I worked in scratchpad/r1760/; the instrument lived only in its exports.

**Gates.** `cargo fmt --check` and `RUSTFLAGS="-D warnings" cargo clippy --all-targets` on
`raster-gpu`, `render-raster`, `viewer-device`, `viewer-ui`: exit 0 each; `cargo nextest run` on the
four: 1121 passed, exit 0; the growth road with its feature: 1 passed; two of the four new tests
fail with their defect planted (trap 13). Rustdoc `-D warnings`, three libraries and `raster-gpu`'s
private items: exit 0. Six arms against HEAD's six arms: 5 799 pages, 0 digests moved, exit 0 each.
`zoom-probe-wayland.py --wheel`/`--drift` at 1.6, both windows: HEAD's records byte for byte.
`t2-launch_path` A/B: exit 0 three runs, 101 one (a warm open); `t2-toolkit_launch`: exit 0 four of
four. `tools/state.sh --round 1760 docs` exit 0; `cargo test -p conformance --no-fail-fast` exit 0.
