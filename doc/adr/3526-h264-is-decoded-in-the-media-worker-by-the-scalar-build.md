# 3526 — H.264 is decoded in the media worker by `rusty_h264-decoder`'s scalar build

Session 2206. Status: accepted. Context: RFC 0015 section 6 (round 3), RFC 0012 sections 2.2 and
3.2, ADRs 2428, 3369, 3504.

## Context

RFC 0012 audited `rusty_h264-decoder` 0.16.1. The pin is now 0.17.3 (`rusty_h264-common` 0.17.3),
and RFC 0012's checks were run again on it:

- **Licence** BSD-2-Clause, both crates. `cargo deny check` over the workspace with the pin: exit 0,
  advisories, bans, licences and sources ok; `wide` is now a duplicate (0.7.33 beside 1.7.1).
- **Graph** with `default-features = false, features = ["std"]`: the decoder, `rusty_h264-common`,
  `bytemuck`, `once_cell`, `wide` (`portable-atomic` only on targets without 64-bit atomics). The
  allocator (`rusty_alloc-api`, behind `global-alloc`) and the SIMD kernels (`rusty_h264-accel`,
  behind `asm`) are not built. **What RFC 0012 did not count**: `wide` is unconditional and brings
  `safe_arch` 0.7.4 (new to the lockfile with `wide` 0.7.33): 312 and 859 lines with `unsafe`, the
  processor's SIMD intrinsics wrapped. So "keeps the SIMD crate out" holds of `rusty_h264-accel`
  and not of SIMD altogether; `doc/questions/Q1027` asks the owner, and the tree keeps the build
  inside the confined worker meanwhile.
- **`#[global_allocator]`**: `rusty_h264-common/src/lib.rs` lines 97-100, under
  `cfg(all(feature = "global-alloc", not(feature = "profile")))`, and a counting one under
  `profile`; neither feature is on.
- **`unsafe`** (grep): the decoder's 5 lines are comments and lint text under
  `#![forbid(unsafe_code)]`; the common crate's 20 lines are all under `profile` (its counting
  allocator, `_rdtsc`), with `#![cfg_attr(not(feature = "profile"), forbid(unsafe_code))]`.
- **C**: none — no `extern "C"`, no `#[link]`, no `cc`; the common crate's manifest names a
  `build.rs` the published package does not ship.

## Decision

1. `pdf_media_worker::video::Session` holds one decoder. An MP4 sample's length-prefixed NAL units
   (ISO/IEC 14496-15) are rewritten with Annex B start codes; `avcC`'s parameter sets go first.
   `mp4::movie_video_track` reads the first enabled `vide` track, an `avc1` or `avc3` entry and its
   `avcC` record (version 1, lengths of 1, 2 or 4 bytes, at most `MAX_PARAMETER_SET_BYTES`).
2. **Output order**: pictures are held until more than sixteen are held — Annex A's largest
   decoded picture buffer — and the lowest picture order count goes out; an IDR or the stream's end
   outputs every held picture in order. That is the crate's own per-sequence reordering bounded to
   the buffer the standard allows.
3. **Interlaced H.264** — the decoder's `Unsupported("interlace / field coding …")`, at the sequence
   for MBAFF and at the slice for a field — is `VideoError::Interlaced`, said by name and handed to
   this machine's own player (`MediaError::Interlaced`, `FRAME_INTERLACED` at the movie). Every
   other refusal keeps the decoder's words.
4. A picture past 1920 × 1088 luma samples is refused by its size (ADR 3528).

## Consequences

- The stream is tested with pictures built by hand from H.264's own syntax — `I_PCM` macroblocks,
  whose decoded samples are the bytes written, and all-skipped P pictures that copy them — so the
  expected values come from the standard, not from another decoder (`tests/video.rs`).
- A stream whose memory management operation 5 resets the order count inside a sequence is
  reordered by the counts as they are; the conformance walk (ADR 3527) is where that shows.

## Amended 2026-11-24 by A1027

The owner answered `doc/questions/Q1027` on 2026-11-24 with "option 1" (`doc/questions/A1027`):
`wide` and `safe_arch` stand in `pdf-media-worker`'s scalar `rusty_h264-decoder` build, inside the
confined worker (`Profile::Media`), as the part of the graph RFC 0012 did not count. No upstream
feature request is filed and `rusty_h264` is not forked. The Context's "the tree keeps the build
inside the confined worker meanwhile" is therefore the standing arrangement, not an interim one.
