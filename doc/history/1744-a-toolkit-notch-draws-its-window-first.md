# 1744 — A toolkit notch draws its window first

Session 1744, 2026-10-24 and 25 (cut by the weekly quota between), a "measure" round finished
beside batch 114. ADRs 2322, 2323; no Q filed.

**Premise.** The brief's 1238 × 1994 window is Qt's held first Wayland size at scale 2 (ADR 2267);
at 1.6 the page area is 990 × 1554, 38, 49 and 31 % of what the three device notches draw. ADR
2070 gives the read-back lane `coverage_for` as the launch rule and argues it no further.

**The ceiling** (habit 164; ADR 2322 §2). Headless, one `--clock` hold at load 0.8, `zoom_frame`:
on `Cpu` a window saves little (the atlas rasterises glyph shapes, which a window keeps); only the
margin-less window on `Compute` put the owner's single-page notches under 8.33 ms, so it was built.

**Built** (ADR 2322). Where a page's view moved, the read-back lane draws the part the viewport
shows first on `lane_for`'s lane and hands it back as a window; the page follows as before, and
until it lands the host shows the window alone. At rest every frame is HEAD's. Against the page's
pixels the `Compute` window differs on 8.07 to 8.45 % of them, worst 14 to 16 of 255, on glyph
edges (looked at, trap 1); on `Cpu` by 0 to 4 pixels by 1.

**Measured** (ADR 2323). Headless, one hold, four interleaved passes at loads 2.1 to 2.9: settled
notch 2 20.6–23.4 → 9.0–11.8 ms, notch 3 22.2–29.3 → 6.6–10.8. On the owner's adapter, one loop
run in one hold (windows activated by pid and confirmed active, no capture), loads 1.3 to 2.4:
notch 2 16.4–17.8 → 9.9–11.2, notch 3 20.1–23.1 → 7.6–9.6, notch 1 22.7–24.3 → 15.5–19.7. What is
left is raster's: the compute lane's first use, and the device call's remainder per window.
Hypotheses: "a quarter to a third" held at notch 3 only; "9–12 ms on `Cpu`" was not built (its
ceiling 9.2 to 13.3); "under a frame on `Compute`" held for two notches of six on the owner's.

**Where.** I worked in scratchpad/r1744/ and the owner's tmp/r1744/; the scratch example
`window_against_page` was deleted in this round.

**Gates.** `cargo fmt --check` for `viewer-device`, `viewer-host`, `viewer-qt`, `viewer-gtk`: exit
0; `RUSTFLAGS="-D warnings" cargo clippy` on the four `--all-targets`: exit 0; `cargo nextest run`
on the four: 467 passed, exit 0. The drive of the two toolkit windows on the lever export: 251
works, 4 wrong; re-driven, 01-center-window and 73-list-arrows work and 88-archive-refused is
wrong on HEAD's export as well. `zoom-probe-wayland.py --wheel` and `--drift` at 1.6, both
windows: HEAD's records exactly. `t2-toolkit_launch` A/B in one hold: both trees outside the bands
by the same margins at load 2.75 to 3.3, exit 101 each. Rustdoc `-D warnings` on the four: exit 0.
The six arms are not owed: `render-raster/src` is untouched, 0 viewer crates in its `cargo tree`.
`cargo test -p conformance --test evidence --test adr_status --test records`: 6 passed, exit
101 on a sibling's record (1748) alone.
