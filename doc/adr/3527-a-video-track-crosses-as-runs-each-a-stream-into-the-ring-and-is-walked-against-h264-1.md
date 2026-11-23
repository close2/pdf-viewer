# 3527 — A video track crosses as runs, each a stream into the ring, and is walked against H.264.1

Session 2206. Status: accepted. Context: RFC 0015 sections 4.1, 4.3 and 6 (round 3), RFC 0012
section 3.2; ADRs 3381, 3501, 3513, 3514, 3526.

## Decision

1. **The movie, then runs.** The host walks the top boxes as for sound (ADR 3381) and sends the
   movie as `FRAME_VIDEO_MOVIE` (13). The worker answers `FRAME_VIDEO_TRACK` (14) — the declared
   size, the clock, every unit's offset, size and duration — or `FRAME_INTERLACED` (16) with the
   hand-off's sentence. The host checks every unit inside the file and at most
   `MAX_VIDEO_UNIT_BYTES` (4 MiB), and the declared picture at most 1920 × 1088.
2. **A run is a stream** (`FRAME_PICTURES`, 15, on ADR 3501's arm): at most `MAX_UNITS_PER_RUN`
   units and 4 MiB, a flag on the last. Each picture the session lets out is written into its slot
   by `emit_in_ring` (ADR 3513): version, width, height, then Y, U, V tight. The closing is a status
   byte — decoded, interlaced, refused — and its sentence. The worker serves its frames through
   `confined_transport::link::Link`, so the ring's descriptor arrives on its standard input.
3. **The ring** is attached once for the worker's life before its first video movie: four slots,
   each sized for a 1080-line picture (`RING_SLOTS`, 12.5 MB). One ring serves every later clip.
4. **The host trusts nothing it receives.** Every picture is read back by `decode_picture`: at most
   the track's declared size, planes exactly its size's. A picture that breaks that, or a closing no
   build writes, ends the worker.
5. **Budgets per run**: frames, the run's units plus the sixteen a session may hold plus one; bytes,
   that many pictures of the declared size; wall, `PROCESSOR_BUDGET` plus the run's own media time,
   so a decoder that falls behind real time by more than a second is stopped (RFC 0015 section
   4.1's deadline), said as `MediaError::Stalled`.
6. **Conformance** (`tests/h264_conformance.rs`, ignored, `// not a gate:`): ITU-T H.264.1's AVCv1
   set, 135 zips fetched by `tools/fetch-h264-conformance.sh`, each pinned by SHA-256, never
   committed. Through `video::Session`, each stream's output against its reference
   reconstruction byte for byte (`yuv`, `qcif`). 2026-11-23: **67 EXACT, 52 interlaced and handed
   off, 4 differ, 12 refused** (FMO 3, SP/SI 2, "B without references" 2, explicit B weighted
   prediction 1, "bitstream truncated" on 4 multiple-reference streams). The four that differ
   (CAPCM1_Sand_E, CAPM3_Sony_D, CVFC1_Sony_C, HCMP1_HHI_A) equal the crate's own whole-stream
   decode, so the session's ordering is not why. Bit-exactness is the criterion because H.264's
   decoding process is defined to the sample; neither H.264 nor H.264.1 is held, so no clause is
   quoted.

## Consequences

The admitted progressive subset is what the walk prints EXACT; the 16 others are the decoder's own,
reported upstream rather than patched here. No sink draws a picture yet: RFC 0015 section 6's
round 4 converts YUV on the device and in `render-cpu`.
