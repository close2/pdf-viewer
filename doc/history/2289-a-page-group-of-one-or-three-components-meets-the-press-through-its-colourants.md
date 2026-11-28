# Round 2289 — a page group of one or three components meets the press through its colourants

An audit-lane round in `pdf-viewer-audit`: ADR 3805's reading built, and the stale band rows.

**ADR 3857** (supersedes 3805 and ADR 1329 section 2's first bullet). §11.7.4.2 converts a group's
result "to the device colour space" first, and §10.5 puts the press's functions there.
`Press::ink_cube` samples `ColourSpace::to_cmyk` onto the press (side 17, or 256 samples for one
component); `pdf_render::PressConversion` carries it with the press's `BlendingSpace`
(`set_press_conversion`, `own_onto_press`, the confined codec). A backend splits the composited
raster into the press's pair (`press_conversion::split`) in place of the curve, cube and matte, and
draws a press page; every such page is separated under the simulation. CPU oracle and
`render_raster::finish` draw it, `render-gpu` refuses it by name, the glimpse skips it. A grid
regular in linear light measured worse (`CalRGB` neutral ramp 17 levels against 3).

**Cost** (1224 by 1584, 256 patches): CPU frame 21.6 to 36.7 ms, RX 580 frame 12.6 to 45.6 ms
(`DeviceRGB`); the cube 1.6 to 3.4 ms per interpretation; nothing off the simulation. **Pages**:
`raster_golden` draws none under it (held 974, moved 0). Of 100 pdf.js first pages carried onto the
press, against the screen: 24 within 2 levels, 30 past 40, mean 1.7; the largest, `issue18694.pdf`
(mean 36.0), is pure red printed as the press's `237 28 36`, as a page with no group drew it. Kept.

Tests: five in `press_transfers` on the oracle, `rasterize` and the window (RX 580), `render-gpu`
refusing; `press_conversion` units; the codec's round trip. `spot_press`'s two RGB-group cases and
`separation`'s RGB page now expect the press's yellow: (70, 139, 0) by hand, (69, 139, 0) drawn.

**Ledger.** 10.5, 10.8.3 and 11.7.4.2 `implemented` before and after; ADR 3805's departure is gone
from 10.5's note, the other two say what is built, and the tests are added.

**Gates.** Rebased on 995e2eaee. Over the brief's set and `viewer-confined`: fmt 0, clippy 0,
`pdf-sandbox` 0, nextest 0 (6244 passed; before the expectations were updated, 4 failed, all this
round's), so none against the empty manifest; rustdoc 0, fuzz 0, conformance 0 (485);
`raster_golden` 0 (held 974), strips 0; turn 0 (33 judged, 0 outside), launch 0 (36 banded, 0 outside).

**Bands.** From this tree's release build, 2 runs each, only the two rows copied. llvmpipe:
`personwithdog.pdf` turn 103.4 .. 168.5 ms (was 38.2 .. 62.5), the Type 3 page 2543.7 .. 3718.0 ms
(was 23.9 .. 35.1). OpenGL: 78.6 .. 112.3 ms (was 27.0 .. 47.3) and 1888.7 .. 2753.5 ms (was 20.4 ..
32.7). The RX 580 Vulkan row for the Type 3 page is already 1925.2 .. 2967.6 (A1073).

**Habit.** `bands.sh retake --gates turn` takes the runs and writes nothing, because a band
directory is written whole; render the turn file from the logs with `tools/bands.py`'s `render`.
