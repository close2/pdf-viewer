# 1246 — A spot ink passes through a group of one or three components

Date: 2026-09-26. Branch: `batch-1246`, sequential mode.
ADR: [1329](../adr/1329-a-spot-colour-passes-through-a-group-of-one-or-three-components.md).

## §10.8.3's residue

§11.7.3: a spot colour "shall not be subject to conversion to or from the colour space of the
enclosing transparency group or page", so a `/DeviceRGB`, `/DeviceGray` or `ICCBased` page is
separated: its process plane is the page in its own space, spot marks painting it white
(`Compositing::paint_beside`, `Interpreter::spots_beside`), the spot planes beside it
(`DisplayList::set_spot_planes`), matte before the curve or cube, multiply after. Process names
revert on every plane of such a page (`DeviceSpots::outside_native`); spot planes are held by
`shape_digest`; the simulated press is the space in force on a no-`/CS` page; a press that cannot
hold a group gives way to the device's components; an unparseable colourant has no plane; a
separation given up is on the report (`Unsupported::SeparationGivenUp`). Codec shapes 5–7.

## Fixtures (hand-worked, `tests/spot_press.rs`)

RGB group: LogoGreen 0.5 (134, 198, 183); over RGB yellow under OP (64, 146, 0); erased without OP
(255, 255, 0). Linear-sRGB `ICCBased`: grey 0.5 times LogoGreen (21, 108, 97). `/DeviceGray`: grey
0.5 times LogoGreen 0.5 (64, 98, 90). LogoGreen at half opacity in an RGB group, Multiply onto a
CMYK page's yellow: (145, 187, 0). Planting `paint_beside` fails four; planting `Own` fails four.

## Crawl and cost

`spot_depth --separate`, once: 8 517 spot pages, 8 506 separated, 11 given up (2 press, 9
unparseable `/Identity` tint transforms); both causes then built and the four files checked by hand.
callgrind: interpret p101 +0.019%, CMYK page +0.062% (12 instructions per colour operator);
rasterise +0.027% / −0.001%.

## Rows

§10.8.3 `partial` → `implemented`; frontier bucket 1 bullet removed. Gates: `raster_golden` held
974 moved 0; both corpus walks and `headless_gpu` green.
