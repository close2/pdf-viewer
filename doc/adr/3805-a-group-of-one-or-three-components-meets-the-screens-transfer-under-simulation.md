# 3805 — A page group of one or three components still meets the screen's transfer under simulation

Session 2276. Status: accepted, as a departure. Context: ISO 32000-2 §10.5, §10.8.3, §11.7.4.2,
§11.7.5.1; ADRs 1329 (such a page's separation), 3769 (the press's functions on its planes).

## The reading

§10.8.3 a) processes the page "as if separations were to be created for a simulated device that
supports subtractive process colourants and possibly spot colours", so the output device whose
§10.5 functions apply is that press, and §10.5 says where they operate:

> Transfer functions shall always operate in the native colour space of the output device,
> regardless of the colour space in which colours were originally specified.

§11.7.4.2 says what a group in another space does before that point: "If the group colour space is
different from the native colour space of the output device, its components are not the device's
actual process colourants; the blending computations shall affect the process colour ants only
after the group's results have been converted to the device colour space." And §11.7.5.1 applies
the transfer "only when the final colour at a given point on the page is known". So a page whose
group composites in one or three components meets the press's functions after its result is
converted to the press's cyan, magenta, yellow and black, and its spot planes meet the spot
colourants' functions: the press's transfer, not the screen's.

## What the tree does, and why it is not changed here

ADR 1329 makes the process separation of such a page the page in its own components, taken to
flat XYZ through its own curve or cube with no stop in the press's colourants, so there is no
plane on which the press's four functions could act. `draw_separated` therefore leaves the
channel on the screen, and every lane agrees: the CPU oracle and `render_raster::finish` (its
`rasterize` and the window's present) map the finished pixel through the screen's three after
`resolve_over_device`, and `render-gpu` refuses the page by name (§10.8.3).

Building the reading needs the group's result converted to the press's process colourants — a
cube from the group's components to the press's four, sampled in `pdf_model` from the same
conversion a colour in that space takes onto the press — the four functions there, and the press's
own conversion back to flat XYZ; for every pixel, not only the transferred ones, or the page would
change colour where a function is the identity. That changes every such page under simulation,
not only the ones stating a transfer, and is ADR 1329 reopened rather than a fix to the channel.
It is left in that order: the conversion first, then the functions on it.
