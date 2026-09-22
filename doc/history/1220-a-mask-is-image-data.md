# 1220 — A mask is image data, and a codec an inline image may use is decoded

A §12.5.6.23 round of batch thirty-four: three of the row's five owed cases closed, two narrowed.

## What moved

§12.5.6.23 stays `partial`, its note narrowed; `doc/todo/64` and `doc/todo/65` bucket 4 say the
same. ADR 1277.

## Findings

**A codec-free picture's mask carried the removed shape.** `build_cleared_image` carried `/SMask` and
`/Mask` by reference, untouched, so the silhouette of what was removed survived in the mask. Not on
any refusal list. Each mask is now cleared as an image of its own, on its own grid, under the
picture's placement, and copied wherever the picture is.

**A codec picture's transparency was refused because the decode was asked the wrong thing.** Decoding
it with the masks multiplied in left an alpha the re-encode had no channel for. Decoding a copy of
the dictionary without them gives the picture's own samples; the masks go through the first finding.
Four fresh rasters (`RasterKind`), each admitted only where the decode's output fits it.

**`/SMaskInData` 1 and 2 name what to create.** Table 87 has the processor create a soft-mask image
from the channel, so the decode's alpha is written as one, cleared beside the samples.

**Inline**: `DCTDecode` and `CCITTFaxDecode`, the two codecs §8.9.7 leaves an inline image, are
decoded and re-expressed; a `/CS` resource is written back under its name. §8.9.7's EXAMPLE is now a
fixture (its data is elided, so the samples are the fixture's own, LZW then base-85 as it states).

**Still owed**: an outline holding an arc; a reduced-resolution JPX decode (the budget is principle
3's); over eight bits; a colour-key `/Mask` or a `/Matte` under a codec picture.

## Corpus

`/Redact` planted with pikepdf over the first masked image drawn on a page (placement from `mutool
trace`), `quorra-transform redact --report=json`, each under the lock: `webCapture.pdf` p2 (Flate
picture with `/SMask`: both cleared), `govdocs1 226387.pdf` p7 and `367594.pdf` p6 (CCITT image
masks, formerly refused, now one-bit stencils), `Orellana2022.pdf` p29. All four applied, one image
each. `mutool draw` before and after differs only inside each region, to within a pixel.
