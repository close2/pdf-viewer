# ADR 0210 — Two rasters, and the grid only a device knows

Status: accepted, 2026-08-06 (session 370).

## Context

ISO 32000-2 puts two of its rasters on the same unit square without putting them on the same
grid. §8.9.6.3, of an explicit mask:

> The base image and the image mask need not have the same resolution ( Width and Height
> values), but since all images shall be defined on the unit square in user space, their
> boundaries on the page will coincide; that is, they will overlay each other.

Table 143 says it of a soft-mask image's `/Width` in different words — "independent of it. Both
images shall be mapped to the unit square in user space (as are all images), regardless of
whether the samples coincide individually" — and neither clause says how to sample one against
the other, because §10.7.4 already has: a device pixel's centre is mapped back into source space,
and each raster answers for that point independently.

This tree held **one raster per image**, decoded at the grid the file states, so combining an
image with a mask meant choosing a grid at interpretation time. Since the fifteenth session that
choice has been the finer of the two in each axis (`combine_on_the_finer_grid`), which discards
nothing either raster carries and is therefore equivalent to a device-resolution composite up to
half a sample of position. What it costs is the *product* of the two larger dimensions, and that
product is a number a document controls: `issue16263.pdf` gives a **2 × 2** image a **34862 ×
4332** `/SMask`, which is 151 022 184 samples and **604 MB** of RGBA for two distinct colours.
`MAX_MASK_GRID` refused it, the image was drawn opaque, and the page — a sheet of vector
arithmetic, `OA + OB + OC = u` forty times over — came out with black bars across it.

The same missing sentence appears twice more, which is what made this a vocabulary question
rather than a bug fix (`doc/todo/24`):

- **JPEG 2000 at a reduced resolution level.** `issue19517.pdf` is 212 megapixels. The format
  decodes at a chosen level natively and §7.4.9's NOTE 2 says so; the decoder is never told one,
  because nothing between `interpret` and `pdf-sandbox` knew a target scale.
- **Sampled (type 0) shadings on the graphics backends**, where a grid stands in for a function
  and wants the same "here is a raster, here is the intent" vocabulary.

**And the constraint is asserted by a test.** `zooming_rasterises_again_without_interpreting_again`
is what makes a display list re-rasterisable at any zoom, so the interpreter must not learn the
device scale. Resolving the grid during interpretation would freeze every mask at whatever
magnification the first frame happened to use.

## Decision

### The display list names a raster it does not hold

`pdf_render::Command::Image` carries an `ImageSource` rather than an `Image`:

```rust
pub enum ImageSource {
    Decoded(Image),
    AtDeviceScale(DeferredImage),
}
```

`DeferredImage` wraps an `Arc<dyn ImageAtDeviceScale>`, whose whole interface is one method:
`samples(Grid) -> Image`, answering with a raster **no finer than** the grid it was asked for.
`ImageSource::at(placement)` is what a backend calls; it borrows for the ordinary variant and
produces for the other.

Three things about the shape, each of which was the alternative that lost:

- **The grid is derived in `pdf-render` and nowhere else.** `Grid::for_placement` is the device
  pixels the unit square covers, from the same `geom::length` of the placement's columns that
  `Image::is_smoothed` and `Image::area_averaged` already measure. It is the fourth device
  decision to live in that crate for trap 2's reason: the CPU backend is the oracle for the other
  two, and a resolution decision made three times is a decision three backends can disagree
  about.
- **The producer is a trait rather than a closure.** A closure would have been shorter and would
  have said nothing about what it owes; a named trait carries the contract — "no finer than
  `grid`" — in the place a second implementer will read it, and JPEG 2000 is expected to be that
  implementer.
- **`samples` is infallible.** Everything an interpreter can check without decoding is already
  checked and reported by it — `unapplied_soft_mask` is asked of the dictionary alone precisely
  so that the report and the behaviour cannot drift. What is left is a decode that fails at draw
  time, and the answer to that is the one `apply_soft_mask` has always given: an image visibly
  present and opaque beats one dropped entirely.

### What travels, and why it is not the document

`Document` caches what it parses behind `RefCell`, so it is **not `Sync`**, and a display list is
drawn on every core. A deferred raster therefore cannot hold a document, a stream or a lifetime:
it holds the mask's bytes with every non-image filter already applied, its grid, its bit depth
and §8.9.5.2's `/Decode` as one table — everything the read needs, settled before a sample is
touched. For `issue16263.pdf` that is the 18.9 MB its `FlateDecode` stream inflates to, against
604 MB for the raster it would otherwise have become.

Three restrictions decide whether a mask can go this way, and each is the clause's own:

- **no image codec on the stream** — a `DCTDecode` or `JPXDecode` sample has no position until the
  whole codestream is decoded, which is the cost being avoided; that is the JPEG 2000 item, still
  owed;
- **`DeviceGray`**, which Table 143 requires outright and which makes a sample's opacity a lookup
  rather than a colour conversion (the eager route tolerates any one-component space and pays for
  it);
- **a depth Table 87 names**, the same five `unpack` admits.

A `/Matte` never meets this route: Table 143 makes the mask's grid the parent's wherever one is
present, so the pair can never be far enough apart to need it — checked rather than assumed.

### The sampling is §10.7.4's, which is a departure from ADR 0025's departure

`SoftMaskAtDeviceScale::raster` point-samples at the centre: output cell `i` of `n` reads source
sample `⌊(2i + 1) × samples ÷ 2n⌋`. That is the clause word for word — "the point whose
coordinate values have fractional parts of one-half … There shall not be averaging over the pixel
area" — and it is deliberately *not* what `Image::area_averaged` does one function away.

ADR 0025 departs from that sentence for a reduced image, on a measured witness whose thin
features vanish otherwise. Extending the departure here would mean decoding every one of a mask's
samples to average them, which is the entire cost this route exists to avoid; and the clause's
own closing sentence states the price — "if the resolution of the source image is higher than
that of device space, some source samples might not be used". The witness's mask is a field of
two values and loses nothing to it. If a document ever shows the loss, the honest fix is a
producer that averages while it unpacks, not a grid chosen larger than the device.

### The combination itself does not move

Once the mask has been read at a grid the device can use, the two rasters are an ordinary pair,
and `combine_on_the_finer_grid` combines them — the same function, the same rounding, the same
`α = shape × opacity`. One rule for combining a pair, with the grid as its only new degree of
freedom. This is why the whole change adds no second reading of §11.6.5.2.

### A mask is read once per object, not once per `Do`

The first working version cost **750 MB** on a 960 × 540 page: `issue16263.pdf` runs `Do` on that
image **55 times** and each drew its own copy of the 18.9 MB inflate. `image::MaskCache` is the
memo, keyed by the `/SMask`'s own object number — sound here in a way `shading::Cache`'s key was
not, because every input is the mask object's own and none comes from the resource dictionary in
force. A mask written directly into the image dictionary has no object number and is read each
time, which is exact rather than approximately right.

## Consequences

### What it costs the backends

Each of the three now resolves before drawing, which for `ImageSource::Decoded` — every image in
the corpus but one — is a `Cow::Borrowed` and a branch. For a deferred source it is a fresh raster
**per draw**, and on the quorra backend a fresh **upload**: the image cache is keyed by the
display list's own raster, and a raster produced for one placement may not be reused for another,
so a deferred image is pushed to the transient list beside a reduced one. That is the honest cost
of the vocabulary and it is bounded by the same `MAX_MASK_GRID`, which halves both axes until the
product fits — a magnification the user chooses can otherwise ask for any grid at all.

Measured on `issue16263.pdf`, page one, `examples/render_at`, peak RSS and wall clock:

| | before | after |
|---|---|---|
| 0.25× | — | 49.0 MB, 26 ms |
| 1× (960 × 540) | 15.6 MB, 12 ms — **and wrong** | **49.0 MB, 33 ms** |
| 4× | — | 74.0 MB, 93 ms |
| 16× (15360 × 8640) | — | 692 MB, 806 ms — of which 531 MB is the target raster |

The 604 MB the eager combination would have needed is never allocated at any scale.

### What moved on the gates

- **corpus 73 → 72 incomplete**, and the report that left is `issue16263.pdf`'s.
- **oracle 1685 → 1686 complete, 856 → 857 agree**, contradicted 68 and ambiguous 750 unchanged:
  the page that entered the judged set **agrees with the reference consensus**, which is the
  strongest thing a round of this kind can produce. Ink at 72 dpi: ours 7.0499, `poppler` 7.9193,
  `mupdf` 7.9413 — the residue is a SymbolMT overline glyph and not the image, which is now
  invisible in all three as its mask asks.
- quorra 914 / 42 / 1 / 17, text 99.2%, dates, XMP and JPEG 2000 all unmoved.
- `doc/todo/00`'s step 7 unchanged at its head over all 786 ambiguous pages.

### What is still owed

Two of `doc/todo/24`'s three claimants. **JPEG 2000 at a reduced level** now lacks only a decoder
that can be asked to stop early — the vocabulary on this side of the sandbox is built, and
`Grid::for_placement` is the number to hand it. **Sampled shadings on the graphics backends** want
the same shape one command over, on `Paint::Shading` rather than on `Command::Image`; nothing here
prejudges what that looks like. §8.9.6.3's explicit mask is a fourth, cheaper claimant: it can be
moved onto `ImageSource` whenever a document asks, and none on this corpus does.
