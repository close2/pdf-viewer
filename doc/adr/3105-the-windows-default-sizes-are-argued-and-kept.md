# 3105 — The windows' default sizes are argued, and each is kept

Session 2101. Status: **accepted.** It changes no size, moves no band in `doc/checks/launch-path.toml` and no ledger
row. Context: ADR 2926 §5, which found that "no ADR or comment argues" the hosts' window sizes; ADR 2944 (the counted
first frame states one scale).

## The sizes

| where | size | what it measures |
|---|---|---|
| `viewer-ui`'s `quorra/window.rs` and `quorra-confined.rs` | 800 × 1000 | the window's inner size, logical pixels |
| `viewer-gtk`'s `host/window.rs` (`set_default_size`) and `viewer-qt`'s `WINDOW` with `cpp/window.cpp`'s fallback | 1000 × 1100 | the whole window, its panels, tab strip and status line included |
| `viewer-confined`'s `worker.rs` `INITIAL_VIEWPORT` | 794 × 1123 | the worker's viewport before its host's first `Resize`, device pixels |
| `viewer-ui/tests/launch_path.rs` `VIEWPORT`, `render-raster/tests/support/frame_cost.rs` `WINDOW` (and the two examples that price a frame at it) | 1600 × 1000 | the viewport a gate's clock figures are taken at |

## The argument, size by size

**800 × 1000, `quorra`'s inner size.** The viewer opens at `Zoom::FitPage` (`viewer_core::open`'s `INITIAL_ZOOM`). A
portrait page fitted to 1000 logical pixels of height is 707 wide for A4 (595.276 × 841.89 pt) and 773 for US Letter
(612 × 792 pt), so both commonest page sizes are shown whole and as large as the height allows, with 12 % and 3 % of
the width left over — a wider window would add only margin at the fit the program opens at, and a narrower one would
make Letter the axis that decides the scale. The height with a title bar (about 30 pixels) stays inside the work area
of a 1920 × 1080 display at scale 1.0, which keeps the window's whole page on the screen it most often opens on. And
it is the figure the counted first frame is taken at (`launch_path.rs`'s `WINDOW`), which states the program's own
first `Resize`. **Kept.**

**1000 × 1100, the GTK and Qt windows.** These measure the whole window, and the 200 × 100 over 800 × 1000 is the
chrome: the side panel beside the page, and the tab strip and status line above and below it (`viewer-qt`'s
`place_window` test states its window as "200 wider and 100 taller than its 800 by 1000 viewport"). So the page
viewport of all three hosts is the same 800 × 1000, and `viewer-qt`'s own comment gives the reason that matters: "two
hosts whose windows are the same size produce launch numbers that can be read side by side". **Kept**, with its cost
written down: the outer window is taller than a 1080-line display. Mutter and KWin, the compositors these two hosts
are written for, constrain a new window to the work area, and the page is then fitted to the viewport it was given, so
`FitPage` still shows the whole page; a window manager that does not constrain shows the window's lower edge off the
screen. Sizing the window from the monitor instead would make the counted first frame's `Resize` a function of the
display the gate runs on, which is the property `launch_path.rs`'s `VIEWPORT` comment refuses for the clocks.

**794 × 1123, the confined worker's first viewport.** A page of A4 at 96 dpi (210 mm and 297 mm at 96 per 25.4 mm), in
force only until the first `Resize`; a host that resizes before it opens never sees it, and no figure is taken at it.
Its comment already states this. **Kept.**

**1600 × 1000, the gates' viewport.** A clock figure is a function of how many pixels are drawn, so it is taken at a
stated size rather than the machine's screen (`launch_path.rs`, `frame_cost.rs`). 1600 × 1000 is a 16:10 window inside
a 1920 × 1080 display, twice the pixels of the hosts' 800 × 1000, so a page-turn figure taken there bounds the smaller
default from above rather than flattering it; and the gate has stated it since its instrument was built (session 922),
so the bands in `doc/checks/launch-path.toml` were drawn at it and a new size would make them incomparable. **Kept.**

## Consequences

No size changes, so nothing moves: no gate figure, no golden, no `tools/drive-windows.sh` coordinate. The comments
beside `quorra`'s, `quorra-confined`'s and `viewer-gtk`'s sizes cite this ADR (`viewer-qt`'s already states its
reason), and `launch_path.rs`'s `WINDOW` no longer says that no ADR argues its size. A host added later states its
default as a page viewport of 800 × 1000 plus its own chrome.