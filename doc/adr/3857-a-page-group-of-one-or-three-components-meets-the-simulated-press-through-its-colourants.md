# 3857 — A page group of one or three components meets the simulated press through its colourants

Session 2289. Status: accepted. Supersedes ADR 3805; supersedes section 2's first bullet of ADR
1329. Context: ISO 32000-2 §10.5, §10.8.3, §11.7.4.2, §11.7.5.1, §11.7.5.3; ADRs 1317, 1329, 3769.

## The decision

ADR 3805 wrote the reading down and left it unbuilt: on §10.8.3's simulated press, a page whose group
composites in one or three components reaches the press's process colourants only "after the group's
results have been converted to the device colour space" (§11.7.4.2), and the press's functions
"shall always operate in the native colour space of the output device" (§10.5). It is now built, in
the order ADR 3805 gave.

- **The conversion.** `pdf_colour::Press::ink_cube` samples `ColourSpace::to_cmyk` — the conversion a
  colour in the group's space takes onto that press — on a grid of side 17 (three components) or
  256 samples (one), under the initial graphics state's rendering, with no black generation
  (§11.7.5.3 gives the graphics state's pair to a colour painted directly into a `DeviceCMYK` group,
  and a group's result is not one). `DeviceRGB` for a page compositing on the device's components,
  `DeviceGray` for one compositing in it, the stated space for the CIE-based ones.
  `pdf_render::PressConversion` carries the cube and the press's own `BlendingSpace`
  (`DisplayList::set_press_conversion`, set by `separate`'s `own_onto_press`), and the codec
  carries it to the confined renderer.
- **The pair.** A backend splits the composited raster — the list's components, before its curve
  or cube — into the press's chromatic and black planes (`pdf_render::press_conversion::split`),
  and from there the page is a four-component page: `resolve_press_transfers` puts cyan, magenta,
  yellow and black on the process planes and the spot function on the spot planes, and
  `resolve_blending` or `separation::resolve` composes them. The matte and the curve or cube of
  ADR 1329 are not applied to such a page.
- **Every painted pixel takes the round trip**, transfer or none, and every such page is separated
  under the simulation, spot colourants or none: what a page looks like on the simulated press is
  the press's picture of it. Before, a page of one or three components naming no spot colourant was
  not separated at all; a page stating no blending space was always carried onto the press, so the
  two kinds of page now agree on the press's gamut.
- **Lanes.** The CPU oracle and `render_raster::finish` (its `rasterize` and the window's present)
  draw it; `render-gpu` refuses a list carrying the conversion by name; the CPU glimpse does not
  glimpse it. Five tests in `render-raster`'s `press_transfers` hold every lane to the oracle:
  `DeviceRGB`, `DeviceGray` and `CalRGB` groups under `/TR [identity identity identity zero]`, a
  spot plane under a Type 5 `/Default`, and an in-gamut grey that comes back within 3 levels.

## The grid is regular in the components

A grid laid over the group's own conversion out's input curves (linear light, for `CalRGB`) was
tried and measured worse: a `CalRGB` group with sRGB's primaries and gamma 2.2 came back on its
neutral ramp within 3 levels of 255 through a grid regular in the components and within 17 through
one regular in linear light, because a press's conversion in is closer to linear in an encoded
component. `DeviceRGB`'s ramp comes back within 2, `DeviceGray`'s within 1.

## What it costs, measured

On a 612 by 792 point page of 256 colour patches at scale 2 (1224 by 1584 pixels), 32 threads at a
load of 17 to 40 (other rounds), medians of five:

| group | interpret screen / simulated | CPU frame screen / simulated | RX 580 frame screen / simulated |
|---|---|---|---|
| `DeviceRGB` | 0.89 / 5.16 ms | 21.6 / 36.7 ms | 12.6 / 45.6 ms |
| `DeviceGray` | 0.86 / 2.55 ms | 19.5 / 27.9 ms | 11.8 / 36.3 ms |
| `CalRGB` | 0.83 / 4.88 ms | 15.1 / 26.2 ms | 28.2 / 38.1 ms |

The window's frame pays the same pass on the processor, over the device's read-back. Over 1.94
million pixels each distinct — the worst case, no run of equal pixels — the split alone is 51 to 58 ms
(three components) or 12 ms (one), and the press's own `resolve_blending` 18 to 21 ms, which every
page compositing in the press already pays; a page's runs of one colour take the previous pixel's
answer, as `resolve_cube` does, which took the `DeviceRGB` page's frame from 78.6 ms to the 36.7 above.
Sampling the cube onto the assumed press is 1.6 to 3.4 ms per interpretation (`DeviceRGB`), 0.3 to
1.0 ms (`DeviceGray`). All of it is under a reader's request
for the simulation; a page drawn without it is unchanged.

## Which pages change, and by how much

Without the simulation, none: `raster_golden` draws no page under it. Under it, of the pdf.js
corpus's first pages that name a `/Group`, 158, 100 are carried onto the press; none of them states a
transfer and one names a spot colourant. Against the same page drawn for the screen — which is what
the simulation drew of 99 of them before — 24 move by at most 2 levels, 9 by at most 16, 37 by at
most 40 and 30 by more; the mean difference over a page is 1.7 levels, and the largest is
`issue18694.pdf`'s 36.0, a page of `1 0 0 rg` that the assumed press prints as its red, `237 28 36`.
That is the press's gamut and not an error: the same colour on a page stating no blending space was
already drawn so. Each figure is the first page drawn on the CPU oracle at scale 1 with and without the
simulation, the larger channel difference per pixel.

So the stop is kept: it is what the clause says, it makes the two kinds of page agree, it costs
nothing off the simulation and tens of milliseconds a frame on it.
