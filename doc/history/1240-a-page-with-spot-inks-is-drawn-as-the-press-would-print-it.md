# 1240 — A page with spot inks is drawn as the press would print it

Date: 2026-09-23. Branch: `batch-1239-1244`, worktree shared with five sibling rounds.
ADR: [1317](../adr/1317-a-page-with-spot-inks-is-drawn-as-the-press-would-print-it.md).

## §10.8.3 stages three and four

The separated page is the page drawn: the display list is the chromatic process plane carrying the
black plane and the spot planes (`DisplayList::set_separated`, `pdf_render::SpotSeparation`), each
colourant with step b)'s flat XYZ at 256 tints. `pdf_render::resolve_separation` is steps b) to d)
per pixel — each plane over the white matte in its own components, to flat XYZ, multiplied,
converted — called by `render-cpu` and by `render-raster`, which draws the planes the way it draws
the pair (the brief said refuse; trap 40). `render-gpu` refuses by name. §11.7.4.2's Normal is
substituted on spot planes where the planes are made. `Debug` prints the field only when present.
The wire carries it as a fourth blending shape. A colourant past the bound is on the page's report.

## Fixtures

`tests/spot_press.rs`, worked by hand through the assumed inks and the sRGB/D50 matrices:
LogoGreen at 0.5 (134, 198, 183); over yellow (69, 139, 0); over black (2, 13, 11); cyan over yellow
(0, 166, 80); a process mark erasing the spot without overprint (255, 242, 0) and keeping it under
OPM 1. Planting the multiply out failed four of six.

## Measurement

callgrind, one sitting, separate target directories: interpret p101 +0.004%, a 3000-mark CMYK page
−0.05%; rasterise p101 +0.09% (inside sitting noise), the CMYK page −0.08%. The first build paid
+4.9% on the CMYK page: `BlendingSpace::convert` stopped being inlined once it had two callers;
`#[inline(always)]` with the number. `raster_golden` moved the same nine rows with and without this
round's hunks, so none are mine.

## Rows

§10.8.3 stays `partial`: of the crawl's 8 517 spot pages 951 are not separated, 875 because the page
group is `/DeviceRGB` or `ICCBased` (`examples/spot_depth --separate`). Table 275's answer is met.
