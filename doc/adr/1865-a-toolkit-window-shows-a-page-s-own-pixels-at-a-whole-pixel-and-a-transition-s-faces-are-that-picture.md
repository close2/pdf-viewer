# 1865 — A toolkit window shows a page's own pixels at a whole pixel, and a transition's faces are that picture

Session 1514. Status: **accepted** and **built** in `quorra-gtk` and `quorra-qt`. Found by round
1508 (ADR 1853 §3): `quorra-gtk` failed drive step 83 on a seam of about 2 970 pixels along the
page's own edge in mid-transition frames. Context: ADRs 0064, 1303, 1843, 1853. Code:
`crates/viewer-host/src/stand_in.rs` (`whole_pixel`, `Placed::shown_at`),
`crates/viewer-host/src/clock.rs` (`face_as_shown`), `viewer-gtk`'s `place` and `face_arrived`,
`viewer-qt`'s `Host::frame` and `face_arrived`. Test: `clock.rs`'s
`a_face_as_shown_is_the_pages_raster_at_a_whole_pixel`. Drive: step 83 `transition_frames`.

## 1. What was found

Every picture of the drive's slide show photographed with `xwd`, under Xvfb, HEAD's binaries
(`scratchpad/r1514/repro/`). `quorra-gtk`'s viewport is 990 × 1090, so a 640 × 480 slide is drawn
742.5 pixels high and centred at y = 173.75; `quorra-qt`'s is 1000 × 1100, which gives 750 and 175
exactly — Qt passed by its geometry, not by its code. Two differences, one under the other:

1. **GSK filters a texture drawn at a fractional offset.** `GtkFixed::move` took the viewer's
   origin as it came, so the settled page's top edge row held (159, 32, 32) — the red slide blended
   half and half with the quarter-grey ground — and every row below it was a blend of two. The
   brief's hypothesis; it held.
2. **A face is not drawn the way the window shows the page.** `viewer_host::face` draws the page
   over `Medium::WINDOW`, so the half of the raster's last row that lies outside the page carries
   the surround: (208, 80, 80). The viewer's raster is the page's extent rounded up (ADR 0064) and
   drawn over `Medium::PAGE_ONLY`, so the same half carries the page's white: (255, 127, 127). With
   the first difference gone this was what remained — 990 pixels on one row, in 48 pictures.

## 2. Decision

1. **A page's own pixels are shown at the whole device pixel nearest its origin**
   (`stand_in::whole_pixel`, `Placed::shown_at`): GTK moves the picture there, Qt is handed it. The
   raster is drawn on its own grid, and rounding is the one placement that shows it unresampled. A
   stand-in keeps its fractional origin, because the toolkit filters it as it scales (ADR 1843).
2. **A toolkit window's transition faces are the picture it settles on** (`clock::face_as_shown`):
   the page drawn into the target the core asked for — byte for byte the raster `Query::Frame`
   holds — composited at that whole pixel over `SURROUND`. At Table 164's `/D` the effect is over
   and the screen is the page moved to (ADR 1853), and the page a person then sees is this one.
3. **`viewer_host::face` stays `quorra`'s.** That window draws its whole arrangement itself over
   the window's medium, so there the window face is the one that meets the page — step 83 passes it
   on the device and under `--cpu`. Each host's face is the picture that host settles on, which is
   what ADR 1303's single function was protecting; two shapes of settled picture need two.

What the half row past the page's edge ought to hold — the page's white or the surround — is a
rounding choice (ADR 0064), not a clause, and is not changed here.

## 3. Measured

Step 83, pictures of neither page between two settled pages. **Xvfb**, `quorra-gtk`: HEAD 6 in the
drive (2 970–3 468 pixels each) and 48 in a second (3 392–3 468); whole pixel alone 48 (990 pixels
each, one row); both, **0** (25 + 21 moving pictures). `quorra-qt` 0 → 0.
**Owner's display** (XWayland, full screen at 2880 × 1800, one clock hold, load 4–8): `quorra-gtk`
10 pictures of neither page (133 898–200 488 pixels) → **0**; `quorra-qt` 15 (3 598–3 899 pixels,
the same edge seam at a geometry that is fractional there) → **0**. A face is one page
rasterisation and one viewport copy where it was one viewport rasterisation: 24–126 ms for the
two at 2880 × 1800 on the owner's display, against 28–78 ms at HEAD, at that load.
