# Carry an image *and its sampling intent* to the backends

Status: **the vocabulary is built and two of its three claimants are settled** — the mask
(session 370, ADR 0210) and JPEG 2000's resolution level (session 486, ADR 0321). What is left
is one backend's gap on sampled shadings, and a fourth claimant no document has asked for.
Priority: 24
Corpus: none — `issue19517.pdf` was the last corpus witness and it draws now
Clauses: §8.7.4.5.3, §8.9.6.3
Code: `crates/pdf-render/src/paint.rs` (`ImageSource`, `Grid`, `ImageAtDeviceScale`),
`crates/pdf-model/src/image.rs`, `crates/pdf-sandbox/src/decode.rs`, all three backends

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

## Closed: JPEG 2000 at a reduced resolution level

`issue19517.pdf`, 12608×16806 in four channels, 847 million samples, refused for wanting
gigabytes until session 486. §7.4.9 NOTE 3 addresses the answer to this program by name —
"[v]iewing and printing applications can gain performance benefits by using the resolution
progression" — and the four edits session 396 verified against a `[patch]` build are committed
(ADR 0321): `pdf-sandbox`'s `jpx` steps down resolution levels until `MAX_SAMPLES` is met, the
raster's grid travels on `SamplesOnGrid` and the `Image` is built at it, §7.4.9's "Width and
Height shall match" check reads the codestream's *stated* grid carried beside the raster —
`Raster::stated_width`, the honest fix rather than the `<=` relaxation — and the mask routing is
asked about the raster that exists, which sends the 3152×4202 base under its 12608×16806 `/SMask`
to §10.7.4's device-scale path. The page draws the receipt `poppler` draws.

One deliberate residue of that round, named in `eligible_for_the_device_scale`: a soft mask
*behind an image codec* still does not decode at a chosen grid — the reduction answers a memory
budget once, where this route would decode per raster request — and no corpus document states
one.

## Still owed

### A sampled shading on `render-gpu`

**This entry said "on the GPU backends" and was wrong about the one that ships**. `render-quorra`
draws a sampled grid — `sampled_fill` uploads it as a raster and clips it to the path — and
`render-cpu` evaluates it; only the Vello backend refuses, through `brush_for`'s
`UnsupportedPaint`, because a grid is not a brush any gradient can express. So this is one
backend's gap rather than a clause's, and no page on the quorra corpus gate is refused for it.

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
