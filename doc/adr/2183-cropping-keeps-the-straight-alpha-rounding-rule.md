# 2183 — Cropping keeps the straight-alpha rounding rule

Status: **accepted**. Session 1673, 2026-10-21.

## The contract

ISO 32000-2 §11.3.3 defines the result colour separately from its alpha. Section 11.3.2
permits an arbitrary colour at zero opacity and adopts zero for zero divided by zero.
These real-valued computations do not prescribe the integer rounding of an eight-bit
readback. Raster's documented boundary choice is nearest rounding (raster ADR 0005).
Section 14.11.2.1 clips contents to the crop box; it does not recolour surviving marks.

## Decision

Use `(255 * channel + alpha / 2) / alpha`, clamped to 255, in render-raster's
premultiplied-to-straight conversion, matching raster's readback. Previously the host
conversion truncated, so asking for a crop changed some surviving translucent pixels.
The opaque path is unchanged. At zero alpha this host helper preserves the zero pixel
its premultiply/crop inputs supply; raster's readback normalizes undefined colour to zero.

The `cropped_alpha` integration test compares surviving pixels of the same half-transparent
rectangle through the cropped and uncropped `Medium::NONE` paths. It asserts that the
crop route ran, that surviving pixels actually have partial alpha, and that excluded pixels
are transparent. The medium-on-device cross-check uses the same stated rounding choice.

This also changes final rounding after component conversion or a translucent medium.
Those call sites must retain their correctness gates; the ordinary opaque window is not
an adequate witness for an API that returns a page's own alpha.

Calibration: the unchanged `5ded4916` exported with only this test fails on a surviving
pixel; the candidate passes on RADV Radeon 890M. The negative control changes no production
source, so it tests the original crop route rather than a transcription of its arithmetic.
