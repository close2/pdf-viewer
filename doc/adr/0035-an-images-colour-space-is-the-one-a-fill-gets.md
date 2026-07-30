# ADR 0035 — An image's colour space is the one a fill gets

Status: accepted, 2026-07-30.

## Context

ADR 0034 read §8.6.5 as a family and left two of its requirements measured and open. Both live
in one function, `image::colour_space`, and both are the same mistake seen from two angles: an
image's `/ColorSpace` was not resolved the way every other colour space in the tree is.

**§8.6.5.5.** An `ICCBased` space was reduced to `DeviceGray`, `DeviceRGB` or `DeviceCMYK` by its
`/N` and unpacked as a device space, under a comment calling that "an approximation, and the
honest one: the alternative is refusing most real images". A *fill* in the same space went
through the profile. So the same four numbers produced two different colours depending on
whether they reached the page as an `scn` operand or as an image sample — which is exactly the
defect `crates/pdf-model/tests/colour_paths.rs` was written to prevent, one level up. Measured:
**31 corpus documents carry 1037 `ICCBased` images, and on 15 of them the profile's answer for a
mid grey differs from the device passthrough by 18 levels out of 255.**

**§8.6.5.6.** The clause requires the `/DefaultGray`, `/DefaultRGB`, `/DefaultCMYK` remapping of
"a colour space given as an entry in an image XObject, inline image, or shading dictionary", and
an image's space was parsed against an **empty** resource dictionary — defensible on the reading
that an image states its space in full, and wrong, because "in full" is about resolving a *name*
and this clause is about replacing a *device space*. One corpus document names a default at all,
and its images state their space directly, so no corpus page could ever have found this.

## Decision

### One function decides what a colour space is

`image::colour_space` refuses `Pattern` — Table 87 excludes it, and a `/Pattern` array *is* a
space the colour module reads, so the refusal has to come first — and then hands everything else
to `crate::colour::ColourSpace::parse` against the resources the image was drawn from.
`ColourSpace::reduced` picks the unpacker's byte-reading arm for the three device families and
its converting arm for everything else, which is a choice about speed and never about meaning:
`to_rgb` is the identity on a device space's components.

The resource dictionary reaches `image::decode` through a new `Dictionaries` struct, because
the three things every route reads — the document, the image's dictionary, the resources in
force at the `Do` — are never apart.

### The cost was the whole problem, and the corpus gate found it

Shipping the conversion on its own took **the corpus gate from 1.7 s to 11.4 s**, a 6.7×
regression across 974 documents. Callgrind on `issue19971.pdf`'s 2500×1364 `ICCBased`
photograph, interpreting page one:

| | instructions | share |
|---|---|---|
| `libm` | 1178 M | 36.3% |
| `icc::Curve::apply` | 612 M | 18.8% |
| `colour::xyz_d50_to_srgb` | 389 M | 12.0% |
| `icc::Profile::connection` | 307 M | 9.5% |

An ICC profile's tone curves are powers and the sRGB encode is a power; there are six of them
per pixel and a profile evaluation costs about 900 instructions. **This is a performance problem
before it is a correctness problem**, and the two things that fixed it are both exact.

**A per-image memo, keyed on the raw sample tuple.** Not an approximation and not an
interpolation: the key is the sample bytes themselves, so a hit returns what the conversion
would have returned for those very samples. It is direct-mapped and fixed-size rather than a hash
map, because an image's colours are *spatially* clustered — neighbouring pixels are usually the
same colour — and a collision costs one conversion, which is what the code did before. It is
sized from the image, a quarter of the pixel count clamped to 2^12..=2^18 entries, because both
ends cost: a fixed 2^18-entry table is 2.9 MB to allocate and zero, which a 16×16 icon should not
pay, and a fixed 2^12 collides constantly on a photograph. Measured both ways:

| table | the photograph | the corpus gate |
|---|---|---|
| none | 3249 M | 11.4 s |
| fixed 2^14 | 1681 M | 1.9 s |
| fixed 2^16 | 1363 M | 1.8 s |
| fixed 2^18 | 1051 M | 1.9 s |
| sized from the image | 1075 M | 1.8 s |

The one-component case never reaches it: `palette` already converts each of a space's at most
256 possible samples once, which is the same idea where an exact table fits.

**`channel` stopped calling `roundf`.** Converting a clamped 0.0..=1.0 float to an eight-bit
channel ran `.round()`, which is a library call: 205 M instructions on that one page, 10.7% of
it, 60 instructions per pixel to round three numbers. `+ 0.5` and a truncating cast is the
**same answer** on this domain — the value is non-negative, where round-half-away-from-zero and
round-half-up agree — and it is arithmetic. This is the "safety habits are expensive in a loop
that runs per pixel" pattern from the JPEG unpacker, in its smallest form.

### `DCTDecode` is converted too, because a fast path inherits no clauses

ADR 0034's lesson applied immediately: the JPEG route bypasses the unpacker, so it would have
inherited neither clause. `convert_channels` applies the space to the decoded channels, which is
the same arithmetic on the same domain — a JPEG component is an integer in 0 to 255 — and does
nothing at all where the space is the device one the decoder already produced, which is every
corpus JPEG but a handful.

## Consequences

The corpus gate is 1.8–1.9 s against 1.7 s before, about 8% for a correctness fix that removes a
silent inconsistency from every image in a non-device space. Neither gate's verdicts moved: 823
documents draw with nothing reported, 814 pages agree, 102 are contradicted. That is expected —
18 levels of mid grey on pages whose references already disagree does not cross a bound — and it
is the second session running where the honest result is *no movement in the numbers and one
fewer thing that is quietly wrong*.

**One page pays visibly and it is worth writing the number down.** Interpreting
`issue19971.pdf`'s first page went from 30 ms to 120 ms, because colour-managing 3.4 million
pixels is work that was not being done. The obvious next step is measured rather than assumed:
the loop is embarrassingly parallel apart from the memo, one cache per row band would keep it
exact, and this tree already has rayon. Nobody has tried it.

**What is still not enforced, and named rather than left implicit**: §8.6.5.6's rules about what
may *be* a default colour space — not `Lab`, not `Indexed`, not `Pattern`, and the same component
count as the space it replaces. A reader that honours a malformed default draws what the producer
asked for; one that refuses draws nothing. No corpus document tests the question.

§8.6.4 was read as the family beside it and is four rows: three of them say what the code already
does, and §8.6.4.4 is `partial` for the reason `CLAUDE.md` names as its standing example — the
clause defines no `DeviceCMYK` conversion at all, so the row records a silence in the standard
rather than a debt in the tree.
