# 1287 — The device filters an image premultiplied

Status: accepted and **built**.
Context: `raster/crates/raster-gpu/src/device/textures.rs` (`premultiplied`),
`raster/crates/raster-gpu/src/device/resident.rs` (`ensure_paint_textures`),
`raster/crates/raster-gpu/src/shaders/image.wgsl` (`fs_main`),
`crates/render-raster/tests/masked_image_edge.rs`.
Answers: the project owner's report from a Windows run — small icons that looked like JPEG
artifacts at some zoom levels and correct after a slight zoom in.
Builds on: ADR 0025 (area averaging, premultiplied), ADR 0706 / raster's ADR 0089 (the device
resolves the reduction per placement).

## What was wrong

The image lane uploaded straight-alpha RGBA and filtered it with the hardware sampler, then
premultiplied the *filtered* sample. A linear filter over straight alpha gives a transparent
texel's colour the same weight as an opaque one's, so whatever a soft-masked image holds under
its mask's zeros reaches the page along every edge the mask draws. Two sources of such colour:

- the encoder's own — a JPEG base under an `/SMask` keeps its ringing and its background there;
- the reduction's — `average_block` returns `[0, 0, 0, 0]` for a fully transparent cell, so a
  reduced variant is black under every transparent area, and a low-alpha cell's colour is a
  quotient of small integers.

The second is why the defect is scale-dependent: below two source samples per device pixel no
variant is drawn and the leak is the encoder's (often white, invisible on white paper); at two
or more the reduced variant is drawn and its black leaks as a grey fringe. `render-cpu`, the
oracle, premultiplies each sample into a `tiny-skia` pixmap before its bilinear filter reads it,
so the two backends disagreed by up to 65 levels on a masked icon at every non-integer reduction
and agreed at integer ones.

## Decision

Premultiply image textures at realisation, with `render-cpu`'s own rounding
(`(c × a + 127) / 255`), for the image's own samples and every reduced variant; the shader uses
the sample as premultiplied. Opaque images are borrowed unchanged, so nothing opaque moves.
Ramps and meshes are not image-lane textures and keep their straight form.

What it costs: one pass over an image's samples when it first becomes resident, and an
allocation where any sample is not opaque. Premultiplied 8-bit texels lose colour precision at
low alpha — the same loss the oracle takes, which is why the two now agree within two levels.

## Consequences

`masked_image_edge` holds a masked image whose colour is black under the mask at seven scales
spanning factors one to three: 59 levels apart before, within 4 after. `22060_A1_01_Plans.pdf` left
`render-raster`'s corpus list of pages that differ from the oracle: its four `DCTDecode` scans
under `/SMask` read 15–19% heavy wherever the device reduced them and 0.07% apart at 8×, where it
does not; `examples/zoom_ladder` now reads mean 0.0513 at 1× and ssim 0.99967. The owner's screenshots
were not reproduced from their own file; the fixture reproduces the mechanism, and the owner's
file is what would confirm it is the same one.
