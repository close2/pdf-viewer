# Carry an image *and its sampling intent* to the backends

Status: **one `pdf-render` change that unblocks three items.** Priced, not taken.
Priority: 24
Corpus: 3 documents
Clauses: §8.9.5.3, §11.6.5.2, §10.7.4
Code: `crates/pdf-render/src/paint.rs` (`Image`), both backends, `crates/pdf-model/src/image.rs`

Today an image reaches a backend as decoded RGBA8 samples at the grid the *file* states, with
`Image::is_smoothed` the only thing said about how to sample it. Three separate refusals are all
the same missing sentence — the display list cannot say *what resolution this image is wanted
at*, or that two rasters belong together:

- **A mask at a grid the bound refuses** — `issue16263.pdf`: a 2×2 image with a 34862×4332
  `/SMask`, which `combine_on_the_finer_grid` would resolve at 151 022 184 samples, 604 MB. The
  clause's answer is compositing at *device* resolution, which needs the display list to carry
  the image and its mask **separately** so the backend combines them where it knows the scale.
- **JPEG 2000 at reduced resolution** — `issue19517.pdf`, 212 megapixels. The format decodes at a
  chosen resolution level natively; the decoder is never told one, because nothing between
  `interpret` and `pdf-sandbox` knows the target scale.
- **Sampled shadings on the GPU** — 2 documents. Type 1 only on the GPU backends; the CPU backend
  draws them. A sampled shading is a grid standing in for a function, and it wants the same
  "here is a raster, here is the intent" vocabulary.

The interpreter deliberately does not know the device scale — that is what makes a display list
re-rasterisable at any zoom without re-interpreting, asserted by
`zooming_rasterises_again_without_interpreting_again`. So the change is *not* "resolve it during
interpretation": it is to carry enough for the backend to resolve it, and to keep the decode lazy
where the format allows one.
