# 1494 — A page that draws for seconds is shown as it is built, in the two toolkit windows

Slot 5 of batch seventy-eight, 2026-10-10, a host round on the owner's target (his drawing's first
frame; okular's "step by step … better than waiting; but not a requirement"). ADR 1825; no ADR 1826,
no question, no ledger row.

**Premise, held**: `drawing.rs` drew a page whole and crossed it back whole; the read is the toolkit
windows' first 2.3–3.6 s. **Hypothesis, held in its win and not in its unit**: no tile can be cut on
this page — ADR 0139 admits a strip boundary only where no curve or stroke crosses, and the sheet's
frame crosses every row — so the unit is painting order. `render_cpu::Glance` is asked, each strip
answers between top-level commands with its rows finished as the page is (medium, crop, straight
alpha), and a strip that ends sends its final rows: the tile shape wherever cuts exist.

**Built.** `CpuRasterizer::showing` (`glimpse.rs`; `lib.rs` routes the page's own strips through
`encode_page`). `viewer_host::Drawing` asks at `GLIMPSE_AFTER` 500 ms and every `GLIMPSE_EVERY`
250 ms, holds a `Partial` for the request in flight only, and `over_frames` paints it where its page
is placed at its size. GTK's and Qt's bars take `Partial::fraction` after ADR 1811's read; `first
frame on the screen` stays the finished page, a picture part-way says `shown as it stands`. Drive
step 81 `page_as_built` photographs its page red without its last green mark before the first frame,
then with it; `--step page_as_built_heavy` times the witness. Looked at (trap 1): both windows'
part-way shots, and GTK's witness at 2 % drawn — frame, rooms, hatching begun, the bar's sliver.

**Moved** (first page content, HEAD → this, two alternating rounds, s by the window's trace). Xvfb:
`quorra-gtk` 9.64, 9.10 → **3.58, 3.12**; `quorra-qt` 10.06, 8.20 → **3.97, 2.82**. Owner's display
(XWayland, 890M, one clock hold): `quorra-gtk` 8.58, 9.15 → **3.14, 3.01**; `quorra-qt` 10.34, 9.95
→ **3.97, 2.98**; re-taken at load 2, three rounds, Qt 9.46, 9.50, 9.47 → **2.88, 2.88, 2.98**, its
draw 7.06–7.12 → 7.14–7.16 s (+1 %; the earlier pairs ran at load 6–13 and wandered both ways). The
draw alone (`examples/page_as_built`, 1002x708): first picture 0.503 s, +0.46 %, rasters identical.

**Unfinished.** `quorra`'s processor fallback and `quorra-confined` do not ask; a zoom shows the
page part-way at its new size, not reprojected (ADR 1825 section 3).

**Gates.** `rustfmt --check --edition 2024` on the eight Rust files: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy -p render-cpu -p viewer-host -p viewer-gtk -p viewer-qt --all-targets`: exit 0; `cargo
nextest run` on them: 476 passed. The new tests are calibrated (trap 13): without the medium pass two
of four fail. `cargo test -p conformance --no-fail-fast`: 429 passed, 1 failed (round 1491's 41-line
record, a sibling's). `tools/state.sh --round 1494 launch` (`--clock`, queued 80.2 s, held 452.9 s):
exit 0, 31 banded, 0 outside. `tools/drive-windows.sh` whole (`--tree 12`, queued 3 400.5 s, held
904.8 s): exit 0, 270 works, 0 wrong, 7 not offered. Clock walks, queued/held: the draw 363.3/32.0 s,
Xvfb pair 7.5/78.4 s, owner's 212/95, 71/184, 1 144/95 s. Duration in the report.
