# 1687 — The offscreen lane's host pass is the medium's composite, paid on every frame, and no window pays it

Session 1422. Status: **accepted**; nothing is built. Answers ADR 1668 section 5's first lever and
says what it holds, which is more than frames 1 and 2. Supersedes nothing.
Context: ISO 32000-2 §11.4.7 (the page group "shall be treated as an isolated group, whose results
shall then be composited with a backdrop colour appropriate for the medium"); `CLAUDE.md` principle 2;
ADR 1668; trap 105.
Code: `crates/render-raster/src/lib.rs` (`Rasterizer::rasterize` for `QuorraRasterizer`, its
`premultiply` and `demultiply`), `crates/pdf-render/src/medium.rs` (`impose_within`,
`impose_on_medium`); the `host` and `caller` columns of `examples/first_frame` (`rasterize`'s own
clock less the stages it names, and the loop's wall less that clock).

## 1. What the pass is

`QuorraRasterizer::rasterize` gets straight-alpha RGBA back from the device. Wherever something
after the device needs premultiplied data — a four-, three- or one-component page group, a spot
separation, a crop, or a medium that marks anything — it premultiplies the whole raster, composites
§11.4.7's medium under it (`impose_within`, which for a uniform medium is `impose_on_medium`), and
demultiplies. `new_headless` and `with_options` start from `Medium::PAGE_ONLY`, a white page that
marks, so every offscreen frame of every gate pays the three passes. On ISO 32000-2's page 7 that
is 8.02 M pixels at 4×.

## 2. When it is paid

Page 7, `examples/first_frame`, the 890M through RADV, pinned to CPUs 0–3 and 12–15, four runs and one
under `strace`, milliseconds:

| target | `host`, frame 1 | frame 2 | frame 10 |
|---|---|---|---|
| 596 × 842 | 1.31–1.36 | 1.22–1.25 | 1.22 |
| 2382 × 3368 | 17.14–17.77 (one run 24.67) | 17.25–17.30 (one run 23.72) | 16.99–17.13 |

So the pass is paid **on every frame**: 17 ms at 4× and 1.2 at 1×. ADR 1668 section 1 named a
further 7.6 ms on frames 1 and 2, two-valued; that is here too, in one run of the five (24.67 and
23.72 against 17.13), and is the smaller part. Over the four plain runs, frame 1 less frame 10 at 4×
is 6.6 to 10.2 ms of `device` and 0.4 to 5.1 of readback, against 0.1 to 0.5 of the pass in the three
runs without that excess. The `caller` column — dropping the
32 MB raster — is 0.08 to 0.19 ms on frame 1 and nothing after.

## 3. Whether a window owes any of it

None. `QuorraWindowRenderer::render`, the surface path every window draws through, and
`QuorraRasterizer::rasterize_frame`, the offscreen copy of a window's frame, both put the medium at
the bottom of the scene, so the device draws it and the read-back is opaque: straight and
premultiplied are the same bytes and nothing is imposed after the device. And nothing outside
`render-raster`'s own tests, examples and the corpus gates calls `QuorraRasterizer::rasterize`:
`viewer-ui`'s two binaries use `QuorraWindowRenderer`, and the GTK, Qt, FFI and confined hosts draw
through `render-cpu`.

## 4. Decision

Not built. A pass no person waits for is a gate's wall clock, not a frame's, and principle 2's
latency argument does not reach it. The construction that would make it cheaper without moving a
byte is named so a later round need not find it again: with no page group to resolve and a uniform
opaque medium, the three passes compose pixel by pixel into one — a pixel at alpha 255 is left alone,
any other is premultiplied and composited, and the result is opaque, so the demultiply after it
changes nothing — which is one pass over the raster where there are three. It is owed when a gate's
time is the question, and measured then.
