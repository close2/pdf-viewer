# Carry an image *and its sampling intent* to the backends

Status: **the vocabulary is built and one of the three consumers is on it** (session 370, ADR
0210). Two remain, and neither is blocked by the display list any more.
Priority: 24
Corpus: 1 document on the corpus gate (`issue19517.pdf`); the rest is one backend's
Clauses: §7.4.9, §8.7.4.5.3, §8.9.6.3, §10.7.4
Code: `crates/pdf-render/src/paint.rs` (`ImageSource`, `Grid`, `ImageAtDeviceScale`),
`crates/pdf-model/src/image.rs`, all three backends

## What the vocabulary is

`Command::Image` carries a `pdf_render::ImageSource`, which is either `Decoded(Image)` — the
raster on the grid the file states, which is every image but one on this corpus — or
`AtDeviceScale(DeferredImage)`, a raster the display list *names* and a backend produces once it
knows the scale. `ImageSource::at(placement)` is what a backend calls; `Grid::for_placement` is
where the device grid is decided, once, so that the three backends cannot ask for different ones.
An implementation owes one thing: `samples(Grid) -> Image`, no finer than the grid it was asked
for.

The interpreter still does not know the device scale, which is what
`zooming_rasterises_again_without_interpreting_again` asserts and what makes a display list
re-rasterisable at any zoom. A deferred raster holds no document and no lifetime — `Document`
caches behind `RefCell` and is not `Sync` — so it carries whatever it needs to answer, which for a
soft mask is the file's own packed bytes.

## Closed: a mask on a grid the bound refuses

`issue16263.pdf`, a 2×2 image with a 34862×4332 `/SMask` — 604 MB of RGBA on the finer of the two
grids, drawn as black bars until the display list could carry the two rasters apart. It now
combines at device resolution (§10.7.4's centre rule), draws in 49 MB, and **agrees with the
reference consensus**. The corpus's incomplete list went 73 → 72 and no corpus document reports an
`/SMask` at all.

## Still owed

- **JPEG 2000 at a reduced resolution level** — `issue19517.pdf`, 212 megapixels, refused for
  wanting gigabytes. §7.4.9's NOTE 2 makes the resolution progression the format's own answer and
  the decoder can be asked for a level. What was missing on *this* side of the sandbox is built:
  `Grid::for_placement` is the number to hand across, and `ImageAtDeviceScale` is where a
  `JPXDecode` stream would sit. What is missing now is an API on `hayro-jpeg2000` — a decode that
  can be told where to stop — which is `doc/JPEG2000_FEEDBACK.md`'s to ask for.
- **A sampled shading on `render-gpu`**, and **this entry said "on the GPU backends" and was
  wrong about the one that ships**. `render-quorra` draws a sampled grid — `sampled_fill` uploads
  it as a raster and clips it to the path — and `render-cpu` evaluates it; only the Vello backend
  refuses, through `brush_for`'s `UnsupportedPaint`, because a grid is not a brush any gradient
  can express. So this is one backend's gap rather than a clause's, and no page on the quorra
  corpus gate is refused for it.

  What is left of the item is the *shape*: §8.7.4.5.3's type 1 shading reduces to a grid standing
  in for a function of two variables, and `pdf-render` fixes that grid at a
  resolution — `FUNCTION_GRID`, 128 — before any device has been asked. That is the same
  interpret-time resolution decision `ImageSource` removed for a mask, one command over, on
  `Paint::Shading` rather than on `Command::Image`. Nothing in ADR 0210 prejudges what it looks
  like; what it settles is that "a raster plus the intent to resolve it at the device" is
  expressible without the interpreter learning the scale.

And a fourth claimant this file did not have: **§8.9.6.3's explicit mask**, whose own ledger row
carries the same "the true answer is a composite at device resolution" sentence the soft mask's
did. It can be moved onto `ImageSource` whenever a document asks for it, and none on this corpus
does — every stencil here refines to a grid that fits.
