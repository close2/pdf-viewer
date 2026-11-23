# 3529 — A video picture is converted by its own VUI, on the device and on the processor as the oracle

Session 2207. Status: accepted. Context: RFC 0015 section 6 (round 4), RFC 0012 section 5; ADRs
3513, 3526, 3527; traps 5, 13, 251, 256.

## Decision

1. **The inputs are the stream's, read beside the decoder.** `rusty_h264-decoder` 0.17.3 stops its
   sequence parameter set at `vui_parameters_present_flag` and yields 8-bit 4:2:0 only, so
   `pdf-media-worker`'s `vui` module walks each set to its VUI (H.264 7.3.2.1.1, scaling lists
   passed, E.1.1) and each access unit's first slice to its picture and sequence sets. Each
   `video::Picture` carries a `Signal`: `video_full_range_flag`, `colour_primaries`,
   `transfer_characteristics`, `matrix_coefficients`, `chroma_sample_loc_type_top_field`, with
   Annex E's inference for each absent code (E.2.1: 0, 2, 2, 2, 0). A set cut inside its VUI is
   refused out loud. The picture item's header grows by those five bytes; the wire is version 2.
   H.264 is not held in `doc/`: everything here is cited, never quoted.
2. **One formula, stated once.** `pdf_render::video` resolves a `Signal` into a `Conversion`
   (Table E-5's `K_R`/`K_B` for codes 1, 4, 5, 6, 7, 9; the range's equations; Figure E-1's six
   places) and states the formula as an affine map from integers both backends hold exactly: the
   luma sample, and each chroma sample interpolated in sixteenths with weights in quarters.
3. **Choices, said as choices** (principle 5). An unspecified or reserved matrix takes BT.601's pair
   up to 576 lines and BT.709's above, and `Conversion::matrix_stated` is false for it. The filter
   between chroma samples is linear at the stream's siting, edges held: H.264 places the samples and
   leaves display upsampling open. Primaries and transfer are carried and not applied — the
   `R'G'B'` is the window's RGB, as a page's `DeviceRGB` is.
4. **Refused by name**: matrix 0 (4:4:4 only), 8, 10, 11, 12, 13, 14, whose formulas are not the
   pair, and a siting code above 5.
5. **The oracle** is `render_cpu::video::convert_picture`, `f64` rounded once, rows in parallel
   (1080 lines: 114 ms on one thread, 12.1 ms at the default pool, both on the VM's EPYC).
6. **The device** is `QuorraRasterizer::convert_picture`: three `R8Uint` plane textures, one
   compute pass of `video.wgsl` in `f32`, each level taken in integers and written as a packed word
   into a storage buffer, so no render target's float-to-level conversion stands in between (traps
   251, 256). Compiled on the first picture, never at bring-up; the size's resources are kept.
7. **The tolerance is checked, not granted**: a channel may differ from the oracle by one level only
   where the exact value lies within a thousandth of a level of a half. RX 580 (RADV): 13 450 536
   channels of hand-built planes, 67 at a tie; 788 pictures of 84 H.264.1 streams, 129 415 248
   channels, 201 at a tie; nothing else differs. A siting planted wrong in the shader fails it.
8. **Vello** (`render-gpu`) takes no video: a picture is not a display-list command. The frame sink
   (RFC 0015 section 6 rounds 5 to 8) hands the converted frame to the window, and where a frame
   becomes a page's `Command::Image`, every backend draws it as an ordinary image.

## Consequences

1080 lines on the RX 580: 5.5 ms to upload, convert and finish, 31.6 ms more to read back, so the
sink keeps the pixels on the device (the output's rows are 256-byte aligned for a texture copy).
None of the conformance streams' first pictures states a colour description, so the stated-matrix
path is held by the hand-built fixtures. Left: primaries and transfer onto the display, the
constant-luminance and `YCgCo` matrices, and the sink.
