# 1202 — A reverting space overprints, a sample keeps its depth, and a cube reads the state

## §11.7.4.3 — the defect round 1196 found (ADR 1241)

`content::colour::cmyk_tints` answered `Some` for a literal `ColourSpace::Cmyk` alone, so
§11.7.4.3's special blend mode never fired for a `Separation` or `DeviceN` reverting to a
`DeviceCMYK` alternate. §10.8.2's own example measured it: cyan over yellow under `/OP true /OPM 1`
in a `DeviceCMYK` page group came out green written in `k` and yellow through the two `Separation`
spaces the example names. ADR 1157's two reasons for setting NOTE 2 aside both fail. "Defined within the PDF file" is
§8.6.7 contrasting with *quantisation*, not with a tint transform; Table 146's `Separation` rows
are for a space still one when the blend function runs, which §11.7.3 makes unreachable inside a
group that states its own space. And §8.6.7's EXAMPLE settles it outright: it calls
`0.2 0.3 0.0 1.0 k` *equivalent* to `0.2 0.3 1.0 scn` in a reverting `DeviceN`, and two operators
a clause calls equivalent may not take different blend functions. Both examples are fixtures now;
planted back, each fails. The row's note said NOTE 2 "closes the route that might have escaped"
while the route stayed open; that sentence is gone.

## §8.9.6.4 `partial` → `implemented` (ADR 1242 section 1)

`pdf_sandbox::protocol::Raster` carries a `precision`, one byte a sample to eight bits and two
big-endian above, and `decode.rs` interleaves the components at the depth the codestream states.
§8.9.5.2's `/Decode` map is built at that depth, so the page's eight bits are reached after
§8.9.6.4's comparison instead of before it. Signed samples, components that disagree on a depth
and more than sixteen bits keep their refusals. The twelve-bit fixture decoded to saturated white
because its generating command wrote the raw samples little-endian where `opj_compress` reads them
big-endian; regenerated with a true 3000, which lets the test state three ranges — the sample, its
eight-bit stretch 187, and `[0 255]` — that separate the two readings in three directions.

## §11.7.5.3 `partial` → `departed` (ADR 1242 section 2)

The second bullet's reason for being owed was "a cube resolved per pixel in a backend where no
colour space exists". True of the resolving, false of the sampling: `into_parent_cube` runs in the
interpreter, forty lines from the graphics state, and `parent_channels` was already calling
`Compositing::paint` with `None` where the pair goes. It now travels from `outer`, the state at the
`Do`. No backend changed — what one receives is a `ColourCube` either way. Trap 40.

## Measured

`raster_golden` held 974 and moved 0: no page of `doc/pdf.js` witnesses any of the three.
