# 2737 — An image, a stencil mask or a shading overprints by the colourants its own colour space names

Status: accepted. Session 1962. Context: ISO 32000-2 §8.6.7, §8.9.6.2, §8.7.4.2, §11.7.4.3; ADR 1311 (the special
blend mode on spot planes), ADR 2736; deficit 6 of `doc/gwg-output-suite.md`.

## The reading

§8.6.7: "Any painting operation marks some specific set of device colourants, depending on the colour space in which
the painting takes place. In a Separation or DeviceN colour space, the colourants to be marked shall be specified
explicitly". For a sampled image the space is the image's `/ColorSpace`, for an `sh` the shading's, and for a stencil
mask the current non-stroking colour's (§8.9.6.2). The nonzero overprint mode "shall not, however, apply to the
painting of images or shadings". Only fills, strokes and glyphs asked the overprint question
(`Interpreter::overprint_blend`), so an image or a shading under `op` erased every plate on the simulated device.

## The construction

`Interpreter::overprint_blend_of` parses the object's space and asks `DeviceSpots::overprint` with no tints and the
mode treated as 0 (`overprint_beside` on a page compositing in one or three components); `stencil_overprint_blend`
asks the same of the current fill space. An answer other than `state.blend` under a blend mode other than Normal takes
§11.7.4.3's implicit group, as a path does. Both return before parsing anything unless `op` is true on a simulated
device, so the screen view is unchanged. A shading pattern fill and a tiling cell are not yet asked: a patterned fill
still returns `state.blend` (`special_overprint`), and the suite exercises neither under `op`.

## What moved

In the press view 2.0's image, mask and shading squares, 3.1, 8.01, 8.1, 8.2, the image halves of 19.0–19.2 and 1.0's
DeviceN shadings pass. The screen view is unchanged on every patch.
