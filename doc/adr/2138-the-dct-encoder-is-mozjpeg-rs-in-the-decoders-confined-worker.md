# 2138 — The DCT encoder is `mozjpeg-rs`, in the decoders' confined worker

Session 1651, the archive slot of batch one hundred and one. Status: **accepted**, **built** in
`pdf-sandbox`. Context: `doc/questions/Q01` and `A01` (the owner, 2026-09-06: "Use mozjpeg-rs"),
ADR 0014 (why pure-Rust codecs still run confined), ADR 0842 section 5 (the refusal this replaces),
RFC 0002 sections 6.4, 6.5 and 13 question 2. Code: `crates/pdf-sandbox/src/encode.rs`,
`protocol.rs`'s `Request::Dct`, `lib.rs`'s `encode_dct`; ADR 2139 is the two features over it.

## 1. The premise that did not hold

The brief described `mozjpeg-rs` as a wrapper of the C library `mozjpeg`, so that principle 3's
C clause would decide where it runs. Read in its source (0.9.2, 2026-04-17, sha256
`c53b3245…275e8f`, checked by cargo against the index): it is imazen's **pure-Rust** port of
mozjpeg's trellis quantisation, `#![cfg_attr(not(feature = "mozjpeg-sys-config"),
forbid(unsafe_code))]` in its own `lib.rs`, BSD-3-Clause, no build script. C mozjpeg is reachable
only through the opt-in `mozjpeg-sys-config` feature and as a dev-dependency, neither of which
this tree compiles. So no C is built, no build-time tool is needed, and principle 3's written
justification for a C dependency is not owed.

## 2. Why it is confined anyway

ADR 0014's second decision, for the same three reasons it gave pure-Rust decoders: a panic in the
worker costs one image rather than the process (`panic = "abort"`), the worker's gigabyte bounds an
allocation no call site has to cooperate with, and the parent's deadline bounds the time. And one
reason of its own: with `default-features = false` the crate still brings `archmage` (with
`archmage-macros`), `safe_unaligned_simd`, `wide`, `safe_arch`, `bytemuck`, `enough` and
`winarm-cpufeatures` — `grep -rw unsafe` counts 218, 616, 443, 1157 and 6 in the five that carry
it — and the samples handed to it were decoded from an untrusted document. Principle 3's
"untrusted input never reaches unsafe code" is satisfied for this tree's own source by
`forbid(unsafe_code)`; the confinement is what keeps the dependency's `unsafe` inside a process
with no filesystem, no network and a system-call filter. The filter is the decoders', unchanged:
an encode asks the kernel for nothing a decode does not, which the confined tests show by passing
under it. `Isolation::InProcess` moves the encode as it moves the decodes, and the crate
documentation now says what that costs for the encoder.

## 3. The request

`Request::Dct { samples, parameters }` — interleaved 8-bit samples, width, height, one or three
components, a quality 1 to 100 — answered as `Decoded::Dct(jpeg)`. The parent checks the answer is
a JPEG from SOI to EOI before anything writes it into a file. The wire format's magic is not
bumped: an older worker answers the new kind with "request kind 5 is not defined", and the build
identity names a stale worker in any case (ADR 0458).

**The budget is the decoders'**, 2^26 samples, refused by name before allocation, with the
encoder's own `Limits::max_alloc_bytes` at half the gigabyte. Measured where it acts:
`tests/encode.rs` encodes a 4729×4729 RGB grid (19 141 samples under the bound) in the confined
worker inside its 1 GiB address space, and refuses a grid two pixels a row wider.

## 4. What §7.4.8 fixes, and the settings that follow

"The DCTDecode filter decodes grayscale or colour image data that has been encoded in the JPEG
baseline format", and NOTE 4: "There is no benefit to using progressive JPEG for stream data that
is embedded in a PDF file." So `Preset::BaselineBalanced` with `progressive(false)` and
`force_baseline(true)` — the second because without it the encoder writes a sixteen-bit
quantisation table at a low quality, which ISO/IEC 10918-1's baseline process does not admit
(tested at quality 1: `Pq` 0, SOF0). Three components go out as JFIF `YCbCr` with no APP14 marker,
which is Table 13's default `ColorTransform` 1. Four components are not encoded: the crate has no
CMYK path, and a `DeviceCMYK` image keeps its producer's bytes.

**The bytes do not follow the processor.** RFC 0002 section 9 makes a written file a function of
the plan; `encode.rs`'s test encodes one image through the CPU's own SIMD dispatch and through
`SimdOps::scalar()` and asserts the same bytes. The version is pinned `=0.9.2` for the same
reason, as `fuser` is.

## 5. What `cargo deny` says

BSD-3-Clause is already in `deny.toml`'s allow list (for `ed25519-dalek`), the rest are `MIT OR
Apache-2.0` or `Zlib OR Apache-2.0 OR MIT`, nothing is banned, no advisory names any of them, and
no exception is added. `cargo deny check` is in the record.

## 6. The shipped worker is built alone

The encoder brought `bytemuck` into the worker's graph, and the windows' graph (`color`, `peniko`,
`winit`) turns on five of its features — `aarch64_simd`, `bytemuck_derive`, `derive`,
`extern_crate_alloc`, `min_const_generics` — that no stanza of the worker asks for. `tools/batch.sh
install` and CI's release build built the worker in one Cargo run with every program, and `--bin`
with no `-p` selects every member, so feature unification put them into the confined process.
`worker_features.rs`, ADR 1714's guard read by ADR 1718 section 5, refused it.

Three answers were weighed. A `pdf-sandbox` feature that only the broker turns on does not exist
here: one binary answers decodes and encodes, and the worker must encode whoever asks. A git source
for `bytemuck`, ADR 1714's answer for `hayro-jbig2`, would make the worker's copy a different
package and leave the encoder's dependencies (`wide`, `safe_arch`) and every future addition to the
same exposure, one patch at a time. **The answer taken is the general one: the worker is built in a
run that selects `pdf-sandbox` alone** — `install_alone` in `tools/batch.sh`, left out of the
programs' invocation and built with `-p pdf-sandbox --bin pdf-sandbox-worker`, and the same two
lines in CI's release step. The corpus gates and the arms already built it so (`--build '-p
pdf-sandbox --bins'`). What it costs is one more link, sequential after the programs' (ADR 0222's
reason for one invocation), of the smallest binary installed.

`worker_features.rs` now holds the resolution that run makes (`-p pdf-sandbox`: nothing inherited),
holds the two builds to it as text, calibrated by removing CI's line and watching the check name
it, and prints what a whole-workspace build would add — `bytemuck`'s five, today — as the reason for
the separate run. `install`'s stand-in workspace gives each program built alone a package of its
own, so a build that forgot the `-p` fails there.

## Consequences

- `doc/stack.md` gains the encoder in its image-codec row; `doc/todo/57` §1's item is built.
- A host that runs the transform suite with the worker off runs the encoder's SIMD in itself, and
  the crate documentation says so beside the decoders' sentence.
- Revisit: a release past 0.9.2 is a re-pin with this section's checks re-run, since the output
  bytes are the version's.
