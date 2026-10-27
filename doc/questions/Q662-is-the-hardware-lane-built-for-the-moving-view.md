# Q662 — Is the hardware lane built for the moving view, and is the resident image texture taken first?

Asked by round 1796, on RFC 0013 and the owner's words of 2026-10-26: "but why don't we send
everything to the GPU in a way, that it's GPU elements?" and "or are the PDF paint instructions
completely incompatible to GPU painting instructions?" Nothing of it is built; the probe is an
example binary (`crates/render-raster/examples/resident_triangles/`).

## What the probe found (RFC 0013 sections 4 and 5)

On the Radeon 890M, headless, in an 800 × 1600 window, one clock hold: a page's fills resident as
stencil-then-cover triangles with Loop–Blinn curves, drawn under 8× MSAA with the view a uniform,
costs **0.87 to 1.15 ms** a zoom step on ISO 32000-2 p. 101 (the compute lane: 1.2 to 2.6 ms),
**1.27 to 1.49 ms** on the two-page column (the compute lane cannot keep a column in page space; in
the window ADR 2408 measured 4.8 to 9.9 ms), and **1.30 to 2.38 ms** on `22060_A1_01_Plans` (the
compute lane: 5.6 to 39.7 ms, most of it 52 million image samples crossing again on each step).
Its price: coverage is a count of eight samples rather than §8.5.3.3's area (inside the corpus
gate's bounds on the text pages, outside them on the plan sheet's minified scans at 1×); a path
can lose §10.7.4's no-disappearance (glyphs are exempt by the clause); the bytes differ between
adapters by one sample's level; groups, soft masks and blend modes other than Normal need a layer
per view.

## The question

**Is the hardware lane built as the lane of the moving view — every step of a gesture drawn by it
at sampled coverage, the compute lane's exact frame drawn when the gesture ends — and is an image's
samples held resident on the device as a texture taken first, as a lever of its own?**

*Recommendation*: **the resident image texture first, yes; the hardware lane after it, yes, for the
moving view only.** The texture is the larger saving on the page that misses the refresh most (the
plan sheet's step is its images, not its paths), it changes no path's coverage, and it can keep the
oracle's bytes by doing the tree's averaging in a shader. The hardware lane then replaces the
blurry stand-in during a gesture with a sharp, sampled frame inside a 120 Hz refresh on every page
measured, falling back to the compute lane for any frame holding a mark it does not carry (a
non-default group, a soft mask, a blend mode, a zero-width stroke, a mark thin enough to vanish).
It is a departure from principle 5 for frames in motion — their bytes are not the oracle's — and
that departure, with a gate of its own for §10.7.4, is what answering yes would accept.

*Meanwhile*: nothing changes. The compute lane draws every moved view as it does today (ADRs 0698,
2315, 2355), the stand-in rules of ADR 2408 stand, and the probe stays an example that prices the
lane again when it is run.
