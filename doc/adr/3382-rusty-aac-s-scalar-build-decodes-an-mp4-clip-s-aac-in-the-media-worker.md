# 3382 — `rusty_aac`'s scalar build decodes an MP4 clip's AAC in the media worker

Session 2170. Status: accepted. Context: RFC 0012 sections 2 and 3.3 (the audit and the conformance
method), RFC 0015 section 5 (step 9), ADRs 2428 (the two decoders and their feature sets), 3297,
3369, 3381, `doc/questions/A946`; ISO/IEC 14496-3 and 14496-26, not held in `doc/`.

## Context

RFC 0012 audited `rusty_aac` 2.0.1 and chose it with `default-features = false`; `doc/stack.md`
asks that a newer version be run against that RFC's streams again before it is taken. The current
version is 2.1.1.

## Decision

1. **`rusty_aac = "=2.1.1"`, `default-features = false`**, a dependency of `pdf-media-worker` alone,
   reached only from the worker's half (`aac::Session`). RFC 0012's checks, re-run on 2.1.1:
   licence Apache-2.0; no dependency, `build = false`, no `-sys` package and no C; `cargo deny check`
   over the workspace exit 0 (`advisories ok, bans ok, licenses ok, sources ok`); 22 752 Rust lines
   in `src/`, 41 occurrences of `unsafe`, of which **three remain in the scalar build** — all in
   `src/dsp.rs`, the class its `UNSAFE.md` calls "filling fresh capacity" (`mdct_fast`'s output and
   `Pow2Dct4::rotate_in_uninit`'s scratch), indexed by loop counters and plan lengths, never by
   stream data. The SIMD kernels are not compiled.
2. **Raw access units, never ADTS.** The worker builds `rusty_aac::decode::Decoder` from the file's
   `AudioSpecificConfig` and feeds it each access unit the sample table names; the crate's
   stream-level `AacDecoder::decode` sniffs ADTS and would reconfigure itself from a unit that began
   like an ADTS header, so it is not used. SBR and PS are applied as the configuration or the
   stream signals them; the output's rate and channels are the first decoded unit's, and a stream
   that changes them later is refused (the host holds one format a clip).
3. **A unit the decoder refuses is a frame of silence**, as long as the last decoded one, counted
   and said ("N of the clip's M AAC access units could not be decoded and were played as silence");
   a clip none of whose units decodes is refused.
4. **Samples cross as the worker's `i32`s**: `rusty_aac`'s `f32` at full scale ±1 times 2³¹,
   saturated (`aac::top_aligned`).
5. **The conformance walk** (`tests/aac_conformance.rs`, ignored, `// not a gate:`) decodes the 22
   held ISO/IEC 14496-26 `compressedMp4` streams through the confined worker and compares each with
   the suite's `referencesWav` (24-bit; fetched, never committed). **The tolerance is not the
   standard's**: the suite's readme files state no criterion, and 14496-4 and -26 are not held; the
   verdict is RFC 0012's bar, the largest difference at most one 16-bit LSB, and every figure is
   printed. Lags are searched, not assumed.

## The walk's result (2026-11-22, `rusty_aac` 2.1.1, scalar)

All 22 streams decode, and each has a reference (the two HE-AAC streams through `add-opt/`
candidates): **WITHIN1** — al00_44, al00_48, **al00sf_48** (960-sample frames, which RFC 0012's ADTS
route could not judge), al01, al02, al04_44, al04_48, al05_44, al05_48, al06 (3 channels), **al08 (all
48 channels)**, al14, al16, al17, am00, am01, al_sbr_cm_48_2 against the high-quality SBR reference,
al_sbr_ps_01_new against both. Largest difference 0.033 LSB16. **OFF** — al03_48 (2 800 LSB16, as
RFC 0012 found it), al07_48 in four of its six channels (two match within 0.004; the four match no
decoded channel at any lag in ±4 096 — not investigated), and al18/al19, the perceptual noise
substitution streams no sample comparison judges. The low-power SBR reference differs, as a
different decoder mode's would. The references begin 2 048 samples into the decoded output for the
LC streams that carry no edit list, and at 0 for al00sf, al14, al16, am00, am01 and the HE-AAC
streams.

## Consequences

- al03 and al07 are the decoder's two open conformance findings, each a stream a person can hear
  differ; they are reported upstream when a round reads what the two exercise.
- A version bump re-runs the walk and this ADR's checks before it is taken.
