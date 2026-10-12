# 1514 — The toolkit windows' pictures between frames

Slot 6 of batch eighty-one, 2026-10-12, a host round on the owner's performance target; the
partial-row word yields to it. ADRs 1865, 1866; no Q. No ledger row moves.

**Premises.** (1) Held: HEAD's `quorra-gtk` fails step 83 along the page's edge. (2) Held, in the
code: a page with no frame shows the ground in `quorra-qt` and the other page in `quorra-gtk`.
(3) **Did not hold**: no window has a view rotation (`Command` has none, `Viewing` is page, zoom
and scroll, no key turns a page), so a turn has nothing to stand in for; nothing built. **Hypotheses**: GSK's filtering at a
fractional offset held for half the seam; the other half was the face's medium under the raster's
rounded-up last row. A retained page drawn on the drawing thread was not needed: a toolkit frame is
the whole page, so the picture is a reduction of it.

**Built.** `stand_in::whole_pixel`: own-size pixels at the nearest whole device pixel in both
windows; `clock::face_as_shown`: a toolkit face is the page's raster there over `SURROUND`;
`stand_in::Retained`: a 512-pixel reduction of a page's first frame at an ink, eight kept, shown
(`its retained`) where no frame is; `--proxy-pages N` in both binaries; step 82's `retained_page`.

**Figures.** Step 83, pictures of neither page, Xvfb `quorra-gtk`: HEAD 6 and 48 (2 970–3 468 px)
→ whole pixel alone 48 × 990 px on one row → **0**; owner's display (2880 × 1800, one clock hold):
`quorra-gtk` 10 → **0**, `quorra-qt` 15 (3 598–3 899 px, its own latent seam) → **0**. Step 82's
`retained_page`, Xvfb: HEAD wrong in both (the control) → **works** in both. Reduction 1.1–4.0 ms a
page (Xvfb), 1.2–6.8 ms (owner's display), once per page per ink. Not judged: the turn photographed
0.15 s after `Left` — keys undelivered on the owner's display and in a manager-less repro.
**Unfinished**: a page never drawn that a notch out brings on screen; the drive whole (below).

**Files.** `crates/viewer-host/src/{stand_in,clock,lib}.rs`, `crates/viewer-gtk/src/{host.rs,bin/quorra-gtk.rs}`,
`crates/viewer-qt/src/{host.rs,bin/quorra-qt.rs}`, `tools/drive-windows.sh` (steps 82, 83),
`doc/todo/30`, `doc/state-of-play.md`, `doc/crate-map.md`, ADRs 1865 and 1866, this record.

**Gates.** `rustfmt --check --edition 2024` on the seven Rust files, `bash -n`: exit 0. `RUSTFLAGS="-D
warnings" cargo clippy -p viewer-gtk -p viewer-qt --all-targets`: exit 0; `-p viewer-host --lib
--tests`: exit 0 (its `--all-targets` fails on slot 1's new `examples/drawn_while_read.rs`, not
this round's). `cargo nextest run` on the three: 309 passed. `cargo test -p conformance`: exit 0 but `records`, 1512's Gates unfilled.
Steps 82, 82-retained, 83 (`--tree 6`, queued 2 060 s): 6 works of 6; HEAD's binaries: 3 wrong.
`launch_path` (`--clock`, queued 349 s, held 741 s): exit 101, 31 banded, 1 outside —
`opt_demo.pdf` `open_kinstructions` 1 372.831 against ≤ 1 372, an open's count, which nothing this
round changed runs; the tree holds siblings' `pdf-model` reader edits. Drive whole (`--tree 12`):
**not run** — queued 6 041 s behind three clock holds, stopped at the budget (`lane=-`). Owner's
display: one clock hold, queued about 6 500 s, held 85 s. Duration 14 098 s.