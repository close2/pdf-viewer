# ADR 0147 — Colour-managing a photograph on eight threads

Status: accepted, 2026-08-02. Session 171. A priced item taken, and taken in the wrong place
first.

## The item

`doc/HANDOVER.md` has listed this under "still open, priced" for thirty sessions:

> colour-managing an image in parallel (`issue19971.pdf`'s 3.4-megapixel photograph went 30 ms →
> 120 ms when `ICCBased` images began converting through their profile; the loop is
> embarrassingly parallel apart from its memo and rayon is already here, and nobody has tried
> it).

Two things about that sentence turned out to matter. The number is still true — the page costs
~110 ms of interpretation, and callgrind puts 27.6% in `image::convert_channels`, 12.8% in
`icc::Curve::apply`, 8.2% in `colour::xyz_d50_to_srgb` and 26.2% in `libm` under them. And **"the
loop" names the wrong loop**.

## The wrong loop, measured

`image::unpack` is the per-sample path: it walks rows of packed samples and converts each through
the colour space, with a `Conversion` memo. It is the obvious target, it was parallelised first,
and the A/B said nothing — 126–135 ms serial against 109–137 ms split, which is noise.

Because this page's photograph is a **JPEG**, and a JPEG does not go through `unpack` at all.
`zune-jpeg` writes eight-bit components straight into the raster and `convert_channels` converts
that raster in place afterwards (the route §8.6.5.5's `ICCBased` made necessary). Two loops, both
"the image conversion", and only one of them runs on the file the item names.

The `unpack` change was reverted, because a change that does not measure does not ship.

## The right loop

`convert_three` is `convert_channels`' three-component arm, and it splits into bands with
`par_chunks_mut`. `issue19971.pdf`, whole-page interpretation, A/B in one sitting on 24 cores:

| bands | median clock | instructions |
|---|---|---|
| serial | ~110 ms | 1 085 M |
| 4 | ~85 ms | 1 206 M |
| **8** | **~57 ms** | **1 365 M** |
| 24 (one per core) | ~55 ms | 1 605 M |

**Twice as fast on the clock, and 26% more processor time.** Both are true and both are quoted,
which is session 162's rule: a session reporting only the counter here would report a regression
on a change that makes the page appear twice as fast.

The cap of 8 is the interesting row. It is the whole of the clock and two thirds of the extra
processor time, and what the extra buys nothing is the memo: each band allocates and zeroes a
`Conversion` table, and one proportioned to a twenty-fourth of the image collides no less than
one proportioned to an eighth.

Below a quarter of a megapixel the split is refused outright, for the same reason — a small image
would pay a table per thread to save a few hundred conversions. The corpus gate is 2.4 s either
side of the threshold, which is the check that the threshold is not being paid for by the 974
documents whose images are mostly icons.

## Why the split is exact, and how that differs from the rasteriser's

`Conversion` memoises a **pure function of a sample tuple**. Two bands that meet the same tuple
convert it twice and agree, so a band boundary changes which conversions are *repeated* and never
which answer is given. `a_band_boundary_changes_no_pixel` is the guard: the same raster converted
at band sizes 1, 3, 64, 999 and whole is byte-identical, and band sizes 1 and 3 are the
interesting ones because they make the memo miss almost every time — which is where a wrong key
would show.

This is worth stating beside ADR 0138, which is the same idea failing. There, a curve clipped by
a strip's edge is **re-parameterised**, so the clipped curve is genuinely not the curve that was
split and the coverage differs by up to 64 of 255. The difference is not that one split was more
careful: it is that a colour conversion is a function of one pixel's samples and a rasterisation
is not a function of one row's geometry. **Ask what the parallel unit's answer depends on before
asking how to divide it.**

Every oracle verdict, corpus count and text percentage is unchanged, which is the same evidence
ADR 0139 offered for the strips.

## The cost written down

`rayon` becomes a dependency of `pdf-model` rather than a dev-dependency. It is already in the
tree for `render-cpu`'s strips, it pulls in no runtime `CLAUDE.md` forbids, and its global pool is
built on first use — which for this path is exactly when there is a megapixel photograph to pay
for it.
