# Q1027 — May the H.264 decoder's scalar build keep `wide` and `safe_arch` in the media worker?

Asked by round 2206 (ADR 3526), for video's third round (RFC 0015 section 6): H.264 in the media
worker.

**The condition.** ADR 2428 and RFC 0012 section 2.2 chose `rusty_h264-decoder` with
`default-features = false, features = ["std"]` so that the crate's global allocator
(`rusty_alloc-api`) and its SIMD kernels (`rusty_h264-accel`, 245 `unsafe`) stay out of the worker.
They do stay out. But `rusty_h264-common` 0.17.3 depends on `wide` 0.7 **unconditionally**, and
`wide` on `safe_arch` 0.7.4: RFC 0012 listed both in the default graph and counted neither. Counted
here with `grep -rw unsafe`: `wide` 0.7.33 has 312 lines with `unsafe`, `safe_arch` 0.7.4 has 859
(they wrap the processor's SIMD intrinsics; licences Zlib OR Apache-2.0 OR MIT). No feature of the
decoder turns them off, so the decoder's arithmetic on a stream's samples passes through them. Both
crates run only inside `pdf-media-worker`, under `Profile::Media` (no filesystem, no network, no
executable memory, one descriptor, a 256 MiB ceiling), which is where principle 3 puts a decoder of
untrusted media.

**The question.** Which do you want?

1. **Keep it as built** — `wide` and `safe_arch` in the confined worker, recorded in ADR 3526 as the
   part of the scalar build RFC 0012 did not count.
2. **Ask upstream** for a feature that builds `rusty_h264-common` without `wide`, and keep option 1
   until it ships.
3. **Fork** the decoder under the owner's control (as A227 did for `zune-jpeg`) with `wide` replaced
   by scalar code.

**Recommendation**: 2 — the confinement holds whatever the crate does, and the precedent of the
JPEG encoder's SIMD dependencies (`pdf-sandbox`'s module notes) is the same arrangement; an upstream
feature removes the question without a fork to maintain. Nothing waits on the answer: the worker
decodes with the build as it is, and the conformance walk's figures (ADR 3527) are this build's.
