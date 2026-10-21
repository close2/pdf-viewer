# 2150 — The read-back page is put on its medium by the device

Session 1657, slot 3 of batch one hundred and two. Status: **accepted**. Context: ADR 2124 section
3 ("2 ms outside both of raster's stages, which on this page is where `QuorraRasterizer::rasterize`
puts the read-back page on its medium … on the reading of the code rather than a stage measured")
and section 4's last lever ("Put the medium on the device … left as that: a candidate with its
figure"); habit 59; `CLAUDE.md` principle 2 and its rule for an optimisation; ISO 32000-2 §11.4.7
(the page group composited with the medium's backdrop) and §14.11.2.1 (the crop), which this moves
and does not reread. It moves no ledger row: the clauses' statements in `pdf_render::medium` are
unchanged, and so is every byte.

## 1. The profile, first

`QuorraRasterizer::rasterize` on `doc/PDF20_AN001-BPC.pdf`'s two pages at the toolkit windows'
620 × 876, release, RADV, the device warm (five frames a page; a probe of the round's own, settled
in the round): the page's time outside the scene and the device call was **2.16 to 2.28 ms of a
3.2 to 3.4 ms page**, and stamps around the three passes put **1.17 to 1.20 ms in the premultiply,
0.79 to 0.92 in `impose_within` and 0.17 to 0.18 in the demultiply** — the whole of it. ADR 2124's
reading of the code holds, now measured. The premultiply is the dearest because it rounds three
channels of every transparent pixel, which is most of a page drawn onto transparency.

## 2. What is built

- **`raster_gpu::Target::ReadbackOnto(Backdrop)`**: the frame `Readback` would hand back, put onto
  one opaque colour on the device before the host maps it. The target is copied into a storage
  buffer, `onto.wgsl` composites it there in integers, and the buffer is copied on into the one the
  host maps; every pixel leaves opaque, so the host takes the row padding off and converts nothing.
  `Backdrop::keep` is a rectangle of whole pixels outside which the frame shows the colour alone.
  The pipeline compiles on the first frame that asks for it and is kept, or its refusal is (ADRs
  0080, 1962 — the compute lane's rule), and that frame names the compile in its phases as
  `pipeline compile (first use)`. `Device::backdrop_fits` says before a frame whether its copy can
  be bound as storage, and a frame past it is refused as `FrameBudgetExceeded`.
- **`render-raster` asks for it where the three passes are all a page owes**: a uniform, opaque
  medium (`Medium::PAGE_ONLY` and every other one-colour page), no pass over the page's own
  components (§11.4.7's four, one or three, §10.8.3's planes), a target drawn in one frame. The crop
  becomes `pdf_render::crop_pixels`, and the colour `pdf_render::medium_bytes` — each stated beside
  the function whose rule it is, so that no backend restates either.
- **Stays on the processor**: a window's two colours, a translucent or absent medium, the
  component passes, a tiled target; and §11.7.5.2's transfer functions run after, on the bytes
  either path gives.

## 3. Why the bytes cannot move

Three steps, each exact in eight bits, and each held by a test:

1. **The round trip is a clamp.** raster's demultiply (`raster/doc/adr/0005`'s rounding) followed
   by `render_raster`'s premultiply, `(s·a + 127) / 255`, gives `min(c, a)` for all 65 536 pairs —
   a valid premultiplied channel back unchanged, and one a unorm blend left a hair above its alpha
   held to it. `readback.rs`'s `the_round_trip_through_straight_alpha_is_a_clamp_to_alpha` runs
   every pair.
2. **The crop is a rectangle.** `crop_to_page`'s rule keeps one run of columns and one of rows, so
   its set is their product; `the_cropped_pixels_are_the_rectangle_crop_pixels_states` holds the
   two to the pixel on fractional, overhanging, empty and not-a-number areas.
3. **Over an opaque colour leaves every alpha at 255**: `c + (b·(255 − a) + 127) / 255` cannot pass
   255 because `c ≤ a`, so the demultiply after it was the identity.

`raster-gpu`'s `a_backdrop_goes_under_on_the_device` holds the device's bytes to a transcription of
the host's three passes on a translucent, rotated scene under three backdrops and crops (a planted
truncation fails it), and `render-raster`'s `medium_on_the_device` holds `rasterize` on white to
the medium-less raster put through the passes, on four real pages at three scales (a planted
colour one level off fails it).

## 4. What it costs and buys

HEAD's export against this round's, interleaved in one `--clock` hold at a one-minute load of 3.2
to 3.9, every window pinned to the eight fast cores on `doc/PDF20_AN001-BPC.pdf`:

| ms, medians of 14 launches | GTK HEAD | GTK now | Qt HEAD | Qt now |
|---|---|---|---|---|
| page 2, a warm device: in all | 4.0 | 2.8 | 4.1 | 2.8 |
| — outside the scene and the device call | 1.6 | 0.0 | 1.6 | 0.0 |
| — read back | 0.4 | 0.8 | 0.4 | 0.9 |
| page 1, a cold device: in all | 8.9 | 7.9 | 8.3 | 7.4 |
| — outside both | 1.2 | 0.0 | 1.3 | 0.0 |
| — read back, the pipeline's first compile in it | 0.9 | 1.6 | 0.9 | 1.9 |

- **The page costs the window about 1.2 ms less warm and 0.9 ms less cold**, the toolkit gate's
  `device_page_one_ms` minima going from 8.8 and 8.5 to 7.6 and 7.8 (GTK) and from 8.1 and 8.1 to
  7.1 and 7.0 (Qt), and every one of its fourteen figures a run inside its band on both builds; the
  processor's lane did not move (page one 2.0 to 2.2 ms throughout).
- **What the device pays back** is 0.4 to 0.5 ms more read back on every page — the second copy
  and the two buffers and the bind group each frame makes — and, on page one, about 1 ms of the
  compile. Keeping the staged buffer across frames of one size is the next lever and is not priced.
- **`quorra`'s turn draws through the same call** and is a third quicker: `launch_path`'s `turn_ms`
  went from 4.09, 4.48 and 4.66 to 2.51, 2.69 and 3.07 ms on its three documents, past the bottom
  of each band, which the file's own rule makes a failure until the bands are taken again. The
  render-raster `turn_path` gate held all thirty-three of its figures inside their bands on both
  builds, two runs each.
- **The brief's hypothesis held in its shape and not its last clause**: 4.0 to 2.8 ms warm, not to
  2.5, and still above the processor's 2.2 for this page; the 0.4 ms the read back gained is the
  difference.

## 5. Found, not taken

**A medium-less page has two demultiplies.** With `Medium::NONE`, a page nothing crops comes back
as raster's straight bytes — rounded, `(c·255 + a/2) / a` — and one §14.11.2.1 crops goes through
`render_raster`'s demultiply, which truncates, `c·255 / a`. No window shows either — every window
composites onto an opaque medium — and one rule for both would move the bytes of every
partly transparent pixel a caller asks for on transparency; it is its own decision with its own
ADR if a caller ever needs the page's own alpha exactly.
