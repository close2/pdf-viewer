# 2713 — What the PDF/A grey rule reaches, measured

Session 1949, a single archive round.
Status: accepted; the measurement behind ADR 2712.
Context: ADRs 2700, 2708, 2712; `doc/questions/A788`; `doc/pdfa-blocking-issues.md` section 8.

## The population

A file meets ADR 2712's rule when it states `pdfaid:part`, its intent is a `GTS_PDFA1` CMYK
profile, it states no `/DefaultGray`, and it paints `DeviceGray`. Counted over the decompressed
objects of every file (`qpdf --qdf --object-streams=disable`, the `/Metadata` packet, the ICC
header's colour space at bytes 16 to 20, and `g`, `G`, `cs`, image, shading and group spaces), in
a census the session worked in `~/scratch/agent/r1949/`:

- **The source corpora** (`doc/pdf.js/test/pdfs`, `doc/corpora`, 974 files): 16 state
  `pdfaid:part` and none has a CMYK intent, so the rule reaches **no source document**. No page of
  the corpus gates can move by it, which the gates confirm (below).
- **The converter's outputs** (round 1945's `outputs-shipped`, 5 105 files): 131 have a PDF/A CMYK
  intent, none states `/DefaultGray`, and **57 paint `DeviceGray`** — 13 at each part under
  `as-shown`, 16 and 15 under `as-if-printed`. Of the 57, 53 paint it by `g`/`G` outside any soft
  mask, 20 inside a mask group, and images, shadings and `Indexed` bases follow.

So the rule's reach in this tree's evidence is the converter's own archives, and every third-party
PDF/A file with a CMYK intent that a reader opens.

## Round 1947's six

Rendered by `quorra-transform render --dpi 72 --format ppm` on the processor, source beside output,
both by one binary, HEAD's against this round's:

| document | as-shown, HEAD | as-shown, rule | as-if-printed, HEAD | as-if-printed, rule |
|---|---|---|---|---|
| `bug1703683_page2_reduced` | 23 | 1 | 53 | 53 |
| `bug1755507` | 25 | 3 | 27 | 17 |
| `issue12798_page1_reduced` | 16 | 2 | 30 | 30 |
| `issue13520` | 9 | 3 | 16 | 14 |
| `issue18032` | 22 | 3 | 26 | 23 |
| `issue9017_reduced` | 25 | 1 | 31 | 4 |

The largest channel difference, the same at parts 2b and 4. Under `as-shown` the six now agree
with their sources within three levels; the residual is not traced. `as-if-printed` states the
shipped press, which moves the drawing by design (ADR 2691), so its columns are not a target.

## The corpus gates

`raster_golden`, `corpus` and the engine's `script_corpus` fail on this machine at HEAD; with the
rule they fail with the same report each, so no corpus page moved, as the population said.
