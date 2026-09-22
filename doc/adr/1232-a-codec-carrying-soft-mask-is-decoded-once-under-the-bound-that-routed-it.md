# ADR 1232 — A codec-carrying soft mask is decoded once, under the same bound that routed it here

## Status

Accepted. Closes the first of §11.6.5.2's three residues, which session 1190 priced and
`doc/todo/41`'s last section states: "[w]hat is owed is a **bound** on that plane, and it is a
decision of its own". Amends ADR 0321, which built the device-scale route and refused every
codec on it; ADRs 0034, 0210, 0399, 1054 and 1218 are the clause's other decisions and are
untouched.

## Context

§11.6.5.2's soft mask supplies its image's alpha, and Table 143 makes the mask's dimensions
independent of the parent's: "both images shall be mapped to the unit square in user space …
regardless of whether the samples coincide individually". This tree has two constructions for
that. The ordinary one combines the two rasters on the refinement of their grids. Where that
refinement is too large to build — `issue16263.pdf`'s 2×2 image under a 34862×4332 mask asks for
604 MB of RGBA — the mask travels to the backend beside the image and the two are combined at
the device resolution §10.7.4 states.

The second route keeps the mask in the file's own packed bytes, so a raster of any grid is read
out of it by indexing. That is why it refused a mask behind an image codec: a `DCTDecode` or
`JPXDecode` codestream has a sample at no position until it is decoded, and decoding it *per
raster request* is exactly the cost the packed-bytes design exists to avoid.

Two things about that refusal had stopped being true of the tree it was written in.
`pdf_model::image::MaskCache` is keyed by the `/SMask`'s own `ObjectId` and holds the whole
`SoftMaskAtDeviceScale`, so a plane decoded once is held for the document rather than for the
request — the per-request cost the refusal names is not what a codec'd mask would actually pay.
And the constant that routed the pair here, `PREFER_DEVICE_SCALE_ABOVE`, already bounds the grey
plane the route *produces* (`SoftMaskAtDeviceScale::cells`), so a bound on the plane it would
*decode* is the same question already answered one line away. This is trap 40's shape: the
capability a refusal says is absent was forty lines above it.

## Decision

**A soft mask behind an image codec takes the device-scale route where the grid it states is
within `PREFER_DEVICE_SCALE_ABOVE`, and is decoded once into an eight-bit grey plane.**

**1. The bound is not a new number, and it says so at compile time.** A compile-time assertion
beside the constant names the corpus measurement it has to admit, so that a later round lowering it
for the grid it was measured on cannot take a mask out of this route in silence (trap 38).
`PREFER_DEVICE_SCALE_ABOVE` is 2^24 samples, and it is the
constant that already decides that a pair this large is better read at the device's grid and
that already caps the grid this route hands a backend. One byte per sample of plane, so the
bound is 16 MiB kept — against the 604 MB the eager combination asks for on the witness this
route was built for. `doc/todo/10` §6's rule is the one this has to satisfy: nothing arbitrary
replaced by something equally arbitrary. Nothing is: the number was measured once and this is
the third question it answers, all three about the same grid.

**2. Above the bound, nothing changes.** The mask is not eligible for this route, so it takes the
eager combination it took before, up to `combined_grid`'s `MAX_SAMPLES` ceiling and refused by
name past that. The refusal that existed is therefore narrowed rather than removed, and the
sentence that survives is about size rather than about filters.

**3. The plane carries Table 88's identity rather than the dictionary's `/Decode`.** The decode
this route makes is `apply_soft_mask`'s, on the same two terms — no resource dictionary, so
§8.6.5.6's defaults cannot remap a mask value (ADR 1054), and `Conversion::device()`, because a
mask is read for its one channel of opacity and not for colour — and it has already applied
§8.9.5.2's map. Storing the dictionary's map beside an already-mapped plane would apply it twice.

**4. The grid comes back from the decode, not from the dictionary.** A `JPXDecode` codestream
over the worker's budget is decoded at one of its own reduced resolution levels (§7.4.9 NOTE 3),
so the plane that exists is the level's. `SoftMaskAtDeviceScale` samples at §10.7.4's cell
centres, which is a question about the plane it holds.

## Consequences

What moves is a cost rather than a picture: a codec'd mask large enough to reach this route was
decoded on every eager combination and is now decoded once per document. What moves on a page is
the pair for which the eager combination was refused outright by `combined_grid` and which is now
drawn — a mask whose refinement with its image is past `MAX_SAMPLES` while its own grid is inside
2^24.

**The population is measured and it is outside every corpus this tree walks.** Of 1099 image
`/SMask`s over the 974 tracked documents, 16 stand behind an image codec, 27 189 055 samples
between them, the largest 6 522 400 in `22060_A1_01_Plans.pdf` — which is inside the bound. But
this route is reached only where `worth_combining` has already refused the finer grid, and none
of the curated documents reaches it: the population that does is 6 documents of the SafeDocs
crawl's 65 944. So the evidence here is a fixture, and `raster_golden` holding is the control
(trap 8).

**What is still owed on this clause is two residues rather than three**: a mask in a
one-component space that is not the `DeviceGray` Table 143 requires, and a `/Matte` whose parent
space is neither `DeviceGray` nor `DeviceRGB`.
