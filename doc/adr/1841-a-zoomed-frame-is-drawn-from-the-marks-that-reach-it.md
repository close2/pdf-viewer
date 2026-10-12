# 1841 — A zoomed frame is drawn from the marks that reach it, and its strips are its tiles

Session 1502. Status: **accepted** and **built**. ADR 1817's candidate A, `doc/todo/69` §3, pulled
forward on the project owner's word of 2026-10-10 — "filtering only the elements, which are
relevant for a frame during zooming". Context: ADRs 0137, 0139, 0219, 0702, 1742, 1817, 1825,
1829, 1835. Code: `crates/pdf-render/src/reach.rs` (`MarkBounds`, `DisplayList::mark_bounds`,
`DisplayList::reaching`), `render-cpu/src/lib.rs`'s `rasterize`, `render-cpu/src/glimpse.rs`'s
`encode_page`, `render-raster/src/present.rs`'s `culls` and `build`.
Tests: `pdf-render`'s `reach` module (seven), `render-cpu/tests/culled_window.rs`. Instrument:
`render-raster/examples/tile_price.rs` section 5. Clauses: §10.7.4. No ledger row moves.

## 1. What the tree did at a zoom

A window at a zoom is one target the window's size whose transform places the page
(`viewer_ui::software::compose_pages`, `render_raster::PresentFrame`). Three things paid for the
whole page there. The processor walked all 4 408 405 of the witness's commands in every strip and
measured each one's `Command::device_bounds` again in each (`misses_surface`, ADR 0137), and culled
by rows only, so a mark beside the window was built and clipped. The strip planner counted every
mark's rows, so a mark beside the window forbade cuts across rows it never touches: the 4× window
drew on 5 strips. The device encoded every command into the scene.

## 2. Decision

**A bound per top-level command, measured once per magnification and kept with the list.**
`MarkBounds` is `device_bounds` under one transform, measured on every core. It serves every
transform with the same linear part: a scroll moves every bound by one translation. The rounding of
that move is bounded by 2⁻¹⁶ of the largest coordinate and added to the margin. `DisplayList`
keeps the last magnification's, invisible to its equality and its `Debug` (`raster_golden` digests
that rendering), and forgets them in the three methods that change its commands.

**A target that shows part of a page is drawn from the commands that reach it.**
`DisplayList::reaching` keeps a command unless its bound, grown by the pixel `misses_surface`
already takes, misses every pixel of the target. §10.7.4 paints only a pixel a shape's area meets,
so a command left out marked no pixel there. It declines a list carrying §11.4.7's second raster,
§11.7.5.2's channel or §10.8.3's planes, which are drawn from lists of their own. `CpuRasterizer`
draws every target from it, so the planner sees only the window's marks. Each strip's test reads
the kept bounds.

**The device takes the culled list in a window's scene**: a baked scene always, since it is of
its view already. A page-space scene that read no view survives every zoom and scroll (ADR 0702); a
culled one would not. So it culls only where the scene it replaces was of the same pages and read
the view — the witness, whose lines are its view's (ADR 1835) — and then says the scene read the
view. **The offscreen path keeps the whole list.** Its targets are pages, where the cull gains
nothing, and on one corpus page it moved the device's frame (section 3).

**The tiles are the window's strips.** `culled_window.rs` found that a mark of partial coverage cut
by a target's edge converts a level apart from the same mark inside a wider target, culled or not.
It is ADR 1817's seam, 588 and 778 pixels. Rows of one target are ADR 0139's exact cuts and columns
are not, so a window is not cut into columns. Its strips run on the rayon pool, and each lands
through ADR 1825's glimpse as before. No second path was needed in `render-cpu`.

## 3. Evidence

- **Exactness.** The witness's window drawn in one strip has the same digest as HEAD's at the fit
  and at 2×, 4× and 8× (`tile_price` §5, both A/B rounds). The window in its shipped division
  differs from that uncut frame by 554, 88, 67 and 106 pixels; HEAD's division by 554, 92, 69 and 106.
  Those are ADR 0219's shifted-origin residual at the cuts each plan grants. `culled_window.rs` holds
  a culled window to the window of only its reaching marks, byte for byte, and its planted check
  (a margin of −½) fails it. **The six corpus arms**, by page against HEAD's: every digest held on
  the processor and the compute lane at 1× and 4×. On the gpu lane one page moved at both scales,
  `bug946506.pdf`, 254 of whose 462 commands lie off its page: oracle error 0.1101 → 0.1141 at 1×
  and 0.0324 → 0.0351 at 4×, inside the gate. So a device frame is not independent of marks that
  draw nothing in it; that is raster's question and is not settled here. It is why the offscreen
  path takes no cull, which leaves those arms on HEAD's code. Neither page was looked at.
- **The witness**, `tile_price` §5, HEAD's export against HEAD plus this, alternating, two rounds
  each in one clock hold, pinned to the eight fast cores, four rayon threads, load 4.2–4.9. The
  quickest of three draws:

  | window | HEAD | this | commands | strips | the cull |
  |---|---|---|---|---|---|
  | fit | 961.9 / 1 026.4 ms | 843.2 / 771.1 ms | 4 408 405 | 7 → 7 | 16.6 / 13.3 ms |
  | 2× fit | 756.3 / 778.1 ms | **512.3 / 432.6 ms** | 1 443 630 | 5 → 5 | 113.3 / 84.4 ms |
  | 4× fit | 727.3 / 759.6 ms | **255.7 / 204.1 ms** | 514 527 | 5 → 8 | 43.0 / 35.9 ms |
  | 8× fit | 992.5 / 928.6 ms | **338.2 / 250.5 ms** | 273 973 | 4 → 8 | 26.7 / 20.7 ms |

  The fit is the same draw: nothing is culled, and its digest is HEAD's. It pays the test of every
  command, 13–17 ms, and no longer measures every bound in every strip. An earlier hold at load
  6.7–18 read the same direction: 4× 1 432.7 / 919.3 → 263.2 / 304.1 ms. **The device**, the 4×
  window as a zoom step from the list's own fit frame: 935.0 / 895.5 → **338.5 / 307.1 ms**, and 0
  pixels differ from the window drawn from `culled_to`'s list, which holds the same commands.
- **The gesture**, `quorra`, first frame and ten `plus` notches 300 ms apart, two runs per arm in
  one clock hold. Seconds from the last notch to the real frame on the owner's display:
  0.983, 0.824 → **0.207, 0.169**. Under Xvfb: 1.420, 1.673 → **0.173, 0.240**. Real frames shown
  during the gesture: none → 2 to 5. Every frame took the processor (`quorra`'s choice; under
  Xvfb the device is `llvmpipe`): its zoomed frames drew in 166–298 ms on the owner's display and
  154–286 ms under Xvfb, where HEAD's two draws a run took 806–962 ms and 1 147–1 356 ms. First frame, owner's display: 4.26, 3.44 against
  4.28, 4.16 s, unmoved within the run's spread.

## 4. What it leaves

The cull is linear in the list: 13–113 ms a frame on the witness, and at 2× most of it is the
copy of the 1.4 M commands kept (the clip table is 87 entries). A bin index would make the test
proportional to the view, and a list that names its commands by index would not copy them. `pdf_render::command_extents`, the planner's, still
measures its own extents and could read the kept bounds. The toolkit windows ask
`viewer_core` for the whole page at a zoom, never the window, so the cull does not reach them. A
device frame of a list the scene has not yet drawn is not culled. The device's culled window has not
been compared pixel for pixel with HEAD's whole-list window, which `bug946506.pdf` says is owed.
