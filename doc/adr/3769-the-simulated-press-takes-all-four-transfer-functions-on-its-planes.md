# 3769 — §10.8.3's simulated press takes all four transfer functions, on its planes

Session 2267. Status: accepted. Context: ISO 32000-2 §10.5, §10.6.5.6, §10.8.3, §11.7.5.2; review
2248's B03 and D01; ADRs 1125 (the channel), 1317 and 1329 (the separations), 3701 (the window's
passes).

## The finding

`Transfer::read` kept the first three of Table 57's four functions — "this device is RGB, so the
fourth is read and never asked" — and every backend mapped the finished sRGB pixel through them.
That is right for the screen and wrong for §10.8.3's simulated device, whose step a) is a device
"that supports subtractive process colourants and possibly spot colours". §10.5: "a CMYK device
shall use all four", "Transfer functions shall always operate in the native colour space of the
output device", and a subtractive component "shall be converted to additive form by subtracting it
from 1.0 before it is passed to the transfer function". `/TR [identity identity identity zero]`
over `0 0 0 0 k` is full black ink on that device and stayed white.

## The decision

- **A transfer holds what both devices take.** `pdf_colour::Transfer` keeps the screen's red, green
  and blue, the press's cyan, magenta, yellow and black (Table 57's four in order, "or their
  complements"), and a function for the press's spot colourants. A single function fills all of
  them ("it shall apply to all components"); an array of four leaves the spot colourants' empty,
  because a halftone dictionary "is the only way to set transfer functions for nonprimary colour
  components". This departs from the finding's suggested "gray function" for a spot plane: the
  clause gives the gray function to gray and black, and names the halftone for the rest.
- **A Type 5 halftone is read by each device's names**: §10.6.5.6's "Red , Green , and Blue for
  DeviceRGB ; Cyan , Magenta , Yellow , and Black for DeviceCMYK". A spot colourant takes
  `/Default`'s; its own named entry is not read yet.
- **`pdf_render::TransferMap` carries every table**, and the channel says which device it is applied
  on (`TransferDevice`). `pdf_model` states `SimulatedPress` where `draw_separated` hands over a
  page drawn as the press's process pair (with or without spot planes), and nowhere else.
- **On the press the functions go on the planes**, before `blending::resolve` or
  `separation::resolve` compose them (`resolve_press_transfers`): the chromatic plane's three
  channels through cyan, magenta and yellow, the black plane through black, every spot plane
  through the spot function. The planes already hold the additive complements, premultiplied, so
  the value is the stored one over the pixel's alpha and nothing is subtracted. Which function a
  pixel takes is §11.7.5.2's topmost-object walk, shared with `resolve_transfers` (`choose`), since
  §11.7.3 keeps one shape and opacity for every plane.
- **The lanes**: the CPU oracle (`render-cpu`) and `render-raster`'s `finish`, which serves its
  `rasterize` and the window's present (ADR 3701). `render-gpu` refuses every list with a channel,
  on either device, and the frame goes to the CPU oracle.

## What it costs

Nothing on a page without a channel, and nothing on the screen beyond one comparison: the screen's
walk is unchanged. A map that is the identity on the screen and not on the press now keeps its
channel, so such a page pays the shape walk on the screen for no change there.

## Left

- A page whose group composites in one or three components is separated with no process plates
  (ADR 1329) and keeps the screen's application on the finished pixel.
- A Type 5 halftone's entry for a spot colourant by its own name.
