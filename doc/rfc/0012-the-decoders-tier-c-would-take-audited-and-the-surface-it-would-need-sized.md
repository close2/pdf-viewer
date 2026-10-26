# RFC 0012 — The decoders Tier C would take, audited, and the surface it would need, sized

Status: **proposed** — ready for the owner. Nothing in this document changes a ledger status, a
`Cargo.toml` or a line of code that ships. The round that wrote it fetched four crates and two
published conformance suites into scratch, and built nothing into the tree. Its one edit to
`CLAUDE.md` is the Tier D trigger sentence A632 dictated (ADR 2400).
Round: 1783 — the round the owner set in `doc/questions/A632` (2026-10-26): *"Q632 as recommended
please"*. The recommendation it adopted: *"an RFC first, nothing built — one round that audits the
decoders (their conformance claims run here against published streams, fuzzed in the worker,
`deny.toml`'s four checks), sizes the presentation surface, and asks again."*
Builds on: RFC 0009 section 3 (clause 13's four tiers, the sandbox argument, the architecture by
tier), review 1768 section 1.3 (the four crates, found on crates.io), Q632, ADR 0014 (C in a
confined worker), ADR 1946 (the ledger's reading of the amended exclusion), `deny.toml`,
`doc/stack.md`.
Asks: **Q649** (section 10).

`§N` in this document is a clause of ISO 32000-2 and nothing else. *H.264* is ITU-T H.264 and its
conformance specification ITU-T H.264.1, whose bitstreams are published in the JVT's directory at
`itu.int` (`jvt-site/draft_conformance/`). *AAC* is ISO/IEC 14496-3, and its conformance part ISO/IEC
14496-26, whose bitstreams ISO publishes among its Publicly Available Standards. This round held
neither standard's text, so it does not quote them. What it ran is the published streams and their
reference reconstructions. A crate's README is the crate's claim, and this document tests it rather
than repeats it.

---

## 0. In one paragraph

The trigger's decoder half is answered, and the answer is a pair of names with figures beside
them. **`rusty_h264-decoder` 0.16.1** reproduces 45 of ITU-T H.264.1's 66 published streams bit
for bit (Baseline 23 of 27, Main 16 of 25, High 6 of 14), refuses every interlaced stream by name,
and ran 35 257 fuzz inputs in ten minutes without a fault; **`rusty_aac` 2.0.1** reproduces 11 of the
14 ISO/IEC 14496-26 LC and Main streams this round could judge within 1 LSB at 16 bits, two of the
other three being noise-substitution streams no sample comparison can judge, applies SBR and PS on
all seven HE-AAC streams, and ran 565 802 fuzz inputs without a fault. Both pass `deny.toml`'s four
checks and both have a feature set with no `unsafe`, no allocator and no SIMD under which the verdicts
are the same. The other two candidates do not reach the bar: `rust_h264` is bit-exact on 31 of 66
and wrong from the first frame on nearly every High-profile stream; `oxideav-aac` fails six LC
conformance streams at frame 0 and decodes slower than real time. **No candidate decodes interlaced
H.264.** The surface is the larger build: a frame sink is 60–120 lines per window by analogy to the
§12.4.4 transition path each already has, but **no window links any audio library**, A/V
synchronisation needs a media clock the tree does not have, and the confined wire carries a raster by
copy in one reply per request, so video needs a streaming arm or shared memory. Section 10 asks
whether that is bought; nothing is built until it is.

## 1. What the trigger says, and what this round was asked to find out

`CLAUDE.md`'s clause 13 entry keeps "decoding or presenting audio and video" excluded "(an H.264
and AAC decoder admissible under `deny.toml`, or C in a confined media worker accepted in writing
under ADR 0014)". Review 1768 found that the first half was met *by licence* on the day the owner
ratified it. Four crates on crates.io decode H.264 or AAC, and each one's licence is on `deny.toml`'s
list. A632 does not reopen Tier C on that finding: "a crate existing is evidence, not an argument".
It asks for three readings before the question comes back:

1. **Each decoder's conformance claim, run here against published streams** — section 3.
2. **Each one fuzzed in the worker** — section 4.
3. **`deny.toml`'s four checks**: licences, bans, advisories and sources — section 2.

A632 also asks for **the presentation surface across the three window toolkits** — what a frame
sink, an audio sink and A/V synchronisation would cost in each (section 5).

"Admissible under `deny.toml`" is the trigger's word. `deny.toml`'s four checks are a floor, and
this RFC also reads what they do not see: `unsafe`, a global allocator, C, and SIMD (section 2.2).
Principle 3 puts a decoder of untrusted media in a confined worker whatever it is written in, and
those readings decide what that worker would hold.

## 2. The four crates, and `deny.toml`

### 2.1 The four checks

Each candidate went into a scratch package of its own (its own `[workspace]`, one dependency at an
exact version, default features), with a copy of the tree's `deny.toml` beside it. `cargo deny
check licenses bans advisories sources` (cargo-deny 0.20.2) ran there on 2026-10-26. To show that
the instrument fires, the same run was made over a planted package that adds `symphonia` (MPL-2.0)
and `openssl`: exit 6, `bans FAILED, licenses FAILED`, with 11 licence rejections and both banned
packages named.

| crate | version | licence | packages in its graph (lockfile) | `cargo deny` (four checks) |
|---|---|---|---|---|
| `rusty_h264-decoder` | 0.16.1 | BSD-2-Clause | 23 (incl. `rusty_h264-common`, `-accel`, `rusty_alloc`, `rusty_alloc-api`, `wide`, `safe_arch`, `bytemuck`, `libc`, the `windows-*` set) | exit 0 — advisories ok, bans ok, licenses ok, sources ok |
| `rust_h264` | 0.4.0 | MIT OR Apache-2.0 | 2 (itself, no dependency) | exit 0 — all four ok |
| `rusty_aac` | 2.0.1 | Apache-2.0 | 2 (itself, no dependency) | exit 0 — all four ok |
| `oxideav-aac` | 0.1.7 | MIT | 17 (`oxideav-core` 0.1.42, `serde`, `serde_json`, `thiserror`, `bytemuck`, …) | exit 0 — all four ok |

**All four pass all four checks.** On `deny.toml`'s own terms the trigger's first half is met, and
not only by licence.

### 2.2 What the four checks do not see

Counted with `grep -rw unsafe --include=*.rs` over each published package's source in the registry
(habit: a dependency's `unsafe` is counted with grep). The figures are evidence about the crates
and are no judgement of them:

| package | Rust lines | `unsafe` occurrences | of note |
|---|---|---|---|
| `rusty_h264-decoder` 0.16.1 | 20 478 | 1 | `forbid(unsafe_code)` in its own source |
| `rusty_h264-common` 0.16.1 | 14 745 | 5 | **installs `rusty_alloc` as the process's `#[global_allocator]` under the default feature `global-alloc`** (`src/lib.rs`, line 99), and has a build script |
| `rusty_h264-accel` 0.16.1 | 8 217 | 245 | the SIMD kernels behind the default feature `asm`, 4 `extern "C"` |
| `rusty_alloc` 2.2.7 + `-api` 2.2.6 | 22 025 | 697 | an allocator, 13 `extern "C"` |
| `rust_h264` 0.4.0 | 20 915 | 6 | no dependency at all |
| `rusty_aac` 2.0.1 | 24 423 | 41 | SIMD FFT and quantiser under the default feature `simd`, each block inventoried in its `UNSAFE.md`. `--no-default-features` is the scalar build |
| `oxideav-aac` 0.1.7 + `oxideav-core` 0.1.42 | 121 571 | 0 + 11 | |

So `rusty_h264-decoder`'s "`forbid(unsafe_code)`" is true of the crate and not of what its default
features link. A confined worker would take it with `default-features = false, features =
["std"]`. That choice keeps out the allocator, which would replace the worker's own, and keeps out
the SIMD crate. Section 3 says whether the verdicts survive the choice. No candidate links C: no
`-sys` package and no `cc` build in any graph.

## 3. Conformance, run here

### 3.1 The streams

- **H.264**: 66 bitstreams from ITU-T H.264.1's published set, each with the reference
  reconstruction shipped beside it in its zip. There are three groups: **baseline** (27 streams:
  the Baseline-profile set, CAVLC, including FMO/ASO and multiple-slice streams); **main** (25:
  CABAC, weighted prediction, B-slices, and the interlaced CAMA/CVPA/CAFI streams); **high** (14:
  FRExt's 8-bit 4:2:0 High-profile streams, two of them field and MBAFF). The verdict is **EXACT**
  when every reference frame is reproduced byte for byte in output order and no frame is extra.
  Decoding is deterministic, so that is the only pass there is.
- **AAC**: 26 streams from ISO/IEC 14496-26's `compressedMp4/`. Reference waveforms
  (`referencesWav/`, from the suite's second disc) exist as a single file for 15 of them: the LC and
  Main streams al00–al05, al00sf, al14, al17–al19, am00, am01, and al04/al05 at 44.1 kHz. The
  HE-AAC streams (`al_sbr_*`) and al06–al08 and al16 have no single reference wav in the published
  directories, so for those this round reports whether the stream decodes and at what rate and
  channel count. The MP4 container was turned into ADTS by `ffmpeg -c copy`, which copies the
  bytes and decodes nothing. The comparison puts both sides on a 16-bit scale, aligns them by the
  lag in ±4 096 samples that minimises the RMS difference (the lag is printed), and reports the
  maximum difference in 16-bit LSB. **WITHIN1** means at most 1 LSB. This round did not read
  14496-26's own criterion (the text is not held), so WITHIN1 is this round's bar and not the
  standard's.

The instruments (two Python scripts, one for YUV and one for PCM) were the same for every crate.

### 3.2 H.264

Both decoders took every stream as one Annex B byte stream through the crate's whole-stream
entry point (`rusty_h264_decoder::Decoder::decode_stream`; `rust_h264::decoder::OrderedDecoder::
decode_nal` per NAL unit and `flush`), default features. No stream took longer than 2 s to decode.

| group (streams) | `rusty_h264-decoder` 0.16.1 | `rust_h264` 0.4.0 |
|---|---|---|
| baseline (27) | **23 EXACT**, 3 differ, 1 error | **20 EXACT**, 6 differ, 1 error |
| main (25) | **16 EXACT**, 5 differ, 4 errors | **10 EXACT**, 12 differ, 3 errors |
| high (14) | **6 EXACT**, 2 differ, 6 errors | **1 EXACT**, 9 differ, 4 errors |
| all (66) | **45 EXACT** | **31 EXACT** |

What the non-EXACT streams are:

- **Interlace.** Every error but two is a refusal of field or MBAFF coding, said in the crate's own
  words — `Unsupported("interlace / field coding")` (`rusty_h264-decoder`, 9 streams: CABREF3,
  CAFI1, CAMA1, CVPA1, FREXT01, FREXT02, FRExt2, FRExt4, HCAFF1, HCAMFF1) and `Unsupported("field
  pictures not yet supported")` (`rust_h264`, 6 streams). Neither decodes interlaced H.264. A
  refusal that names itself is the loud failure principle 1 asks for.
- **`rusty_h264-decoder`'s other two errors**: MR1_BT_A ends `Truncated` (a multiple-reference-frame
  Baseline stream), and the three Baseline streams that decode but differ — NL2_Sony_H (first
  mismatch at frame 46 of 300), SVA_CL1_E and SVA_NL2_E (frame 2 of 50 and of 17) — are
  reference-list and slice-structure streams. Its README claims "35 of 35 clean streams from
  openh264's conformance corpus"; against the JVT's Baseline set the figure is 23 of 27. (CVSE2_Sony_B ships two
  references, with and without the repeated pictures its SEI messages ask for; the decoder returns
  the 278 un-repeated frames and is EXACT against that one, so it is counted EXACT, and the other
  decoder returns the 347 and is EXACT against the other.)
- **`rust_h264`** differs on four of the five quantiser-matrix streams (BAMQ1/2, NLMQ1/2, from the
  first frame), refuses MPS_MW_A (`invalid sub_mb_type`) and FREXT02 (`CAVLC coefficient position
  underflow`), and is wrong from frame 0 or 1 on nearly every High-profile stream — the 8×8
  transform path — and on most CABAC streams. Its README's "Baseline, Main, and High profiles" is
  not what the suite finds.
- **Without the SIMD and allocator features** (`--no-default-features --features std`),
  `rusty_h264-decoder`'s Baseline verdicts are the same 23/3/1, stream for stream. The confined
  worker's build loses nothing of the correctness measured here.

So **one H.264 decoder reaches two thirds of the published suite bit-exactly** (45 of 66) and the other under
half; **neither decodes interlaced video**, and each refuses it by name.

### 3.3 AAC

Both decoders took ADTS frames (`rusty_aac::AacDecoder::decode` per frame, default features;
`oxideav_aac::codec_decoder::make_decoder` over `oxideav_core::Packet`s, its PCM converted from
its output format to f32). One stream, al00sf_48, uses 960-sample frames, which ADTS cannot carry,
so 14 referenced streams were judged, not 15.

| | `rusty_aac` 2.0.1 | `oxideav-aac` 0.1.7 |
|---|---|---|
| referenced LC and Main streams (14) | **11 WITHIN1**, 3 OFF | **6 WITHIN1**, 2 OFF, 6 errors |
| unreferenced LC streams (al06, al07, al08, al16) | 3 decode (3, 6 and 3 channels); al08, a 48-channel stream, errors at frame 2 | 2 decode, 2 error (al06 at frame 0, al08 at frame 2) |
| HE-AAC v1 and v2 (`al_sbr_*`, 7) | all 7 decode, **stereo at the doubled rate** (48 kHz, 44.1 kHz; the PS streams at 32 kHz) — SBR and PS applied from the stream's own signalling | all 7 decode at the same rates, **one channel where `rusty_aac` gives two** — the parametric-stereo streams come out mono |
| wall time, al18_48 (60 s of 48 kHz mono) | 0.048 s | 81.9 s |

What the OFF verdicts are:

- **al18 and al19 are perceptual-noise-substitution streams** (the suite's own
  `readme_al_ap_informative.txt` names "PNS and LTP content" in the al18 family). PNS replaces a
  band's spectrum with generated noise, so no two decoders are sample-exact there, and the two
  candidates' RMS differences against the reference are the same to three figures (808 and 807
  LSB on al18; 933 and 934 on al19). That agreement is about the instrument, not the decoders: a
  sample-for-sample comparison cannot judge a PNS stream (trap 9's lesson, read the other way),
  and 14496-26's own criterion for these streams was not held by this round. **Not judged.**
- **al03_48** is OFF for `rusty_aac` (maximum 2 800 LSB, RMS 88 LSB, where the other eleven are
  within 0.01 LSB RMS) and errors in `oxideav-aac`. This round did not read what al03 exercises;
  it is the one LC stream `rusty_aac` gets wrong and the RFC says so.
- `rusty_aac`'s scalar build (`--no-default-features`) gives the same fourteen verdicts, so the
  worker's `unsafe`-free build loses nothing.
- **`oxideav-aac`'s six errors are at frame 0** of ordinary LC streams (`extension_payload
  extension_type 0x5 is reserved`), on bytes `rusty_aac` decodes within 1 LSB; and its decode is
  **slower than real time** on al18 (82 s for 60 s of audio, where `rusty_aac` takes 0.05 s).

So **one AAC decoder reproduces every LC and Main conformance stream this round could judge
within 1 LSB except one**, applies SBR and PS, and does so in a hundredth of real time; the other
does not reach the bar.

## 4. Fuzzed in the worker

Each decoder was built as a libFuzzer target of its own (unsanitised, as this tree's campaigns run
under a memory bound; `doc/verify.md`), fed the input the way a confined worker would be — the whole
input as an Annex B stream, or as ADTS frames, through the same entry point the conformance run
used — and run for ten minutes under the lock's long lane, under the confined decoder worker's
own address-space ceiling (`RLIMIT_AS` 1 GiB, `lockdown_linux::ADDRESS_SPACE_LIMIT`), libFuzzer's
`-timeout=10` and `-rss_limit_mb=1024`, seeded with 12 to 15 conformance streams cut to 32 or
64 KiB. The property is no panic, no hang, no abort. The H.264 target is the crate's `std`-only
build; the AAC target is `rusty_aac`'s scalar build.

| target | runs in 600 s | exec/s | coverage, seeds → end (`cov`) | features, seeds → end (`ft`) | peak RSS | ended by |
|---|---|---|---|---|---|---|
| `rusty_h264-decoder` | 35 257 | 58 | 4 179 → 5 167 | 10 744 → 30 269 | 139 MB | time; exit 0, no artifact |
| `rust_h264` | 4 704 (426 s) | 11 | 4 026 → 5 028 | 8 882 → 27 522 | 93 MB | **a 10 s timeout** on a 39 KiB input; exit 70 |
| `rusty_aac` | 565 802 | 941 | 956 → 2 211 | 2 009 → 8 592 | 57 MB | time; exit 0, no artifact |
| `oxideav-aac` | 101 (97 s) | 1 | 1 951 → 2 024 | 4 546 → 7 491 | 33 MB | **a 10 s timeout** on a 32 KiB input; exit 70 |

- **No candidate panicked, aborted or exceeded the worker's memory ceiling in ten minutes.** Ten
  minutes is a smoke test and not a campaign: ADR 1024's and ADR 0742's campaigns run for hours,
  and a decoder taken into the tree would get a seeded target under `fuzz/` and the campaign
  paragraph's hours, as `jbig2` and `jpx` did.
- **Both timeout units terminate.** Run once more alone, `rust_h264`'s artifact decodes in 5.0 s
  and `oxideav-aac`'s in 4.3 s (the fuzz runs shared a machine at a load average near 30). So each
  is a pathological slow path and not a hang; a worker under a wall-clock budget would refuse it,
  and the crates' throughput is the figure behind it — 11 executions a second on 64 KiB inputs
  against 58 for the other H.264 decoder, and **1 a second** on 32 KiB of ADTS against 941 for the
  other AAC decoder, which is the same ratio section 3.3's wall times show.
- `INITED → DONE` is what the ten minutes bought over the seeds (ADR 0747): the H.264 targets grew
  their feature count by about 2.8× and 3.1×, `rusty_aac`'s by 4.3×, so each run was finding new
  paths to its end and none had saturated; `oxideav-aac`'s 101 runs grew it by 1.6× before the
  timeout, which is not a campaign at all.

## 5. The presentation surface, across the three toolkits

Read in the tree on 2026-10-26. Line figures are `wc -l` of whole files, and estimates are by
analogy to the nearest existing path, not measured.

**What every window already has.** §12.4.4's transitions are a timed frame sequence drawn over
the page, so each window already holds the nearest analogue of a video frame sink. **What none
has is audio**: no audio library is in the workspace's lockfile (`grep -ciE
'alsa|pulse|pipewire|gstreamer|cpal|audio' Cargo.lock`: 0), and the Qt build links Core, Gui and
Widgets only (`crates/viewer-qt/build.rs`).

| | `quorra` (winit + wgpu) | GTK 4 | Qt 6 Widgets |
|---|---|---|---|
| a page reaches the screen as | `PresentFrame`'s display lists. Pixels are `Command::Image` over an RGBA8 `Raster` (`render-raster/src/present.rs`) | a `gdk::MemoryTexture` (RGBA8) in a `gtk4::Picture` (`viewer-gtk/src/page.rs`) | a `QImage` (RGBA8888), copied once per frame across the `cxx` bridge, drawn in `paintEvent` (`cpp/window.cpp`) |
| a second raster over the page today | the window-pixel overlay lists — selection, focus ring, caret, popups, sidebar, cards (`bin/quorra/overlays.rs`, 720 lines) | a Cairo `DrawingArea` in an `Overlay`, and stand-in and transition textures in the same `Picture` (`src/host/stand_in.rs`, 241 lines) | `ChromeOverlay::paintEvent`, and stand-ins as further `QImage`s drawn in order |
| the timed sequence that exists (§12.4.4) | `transition_frame()` builds a display list per frame, paced by `Cadence` at the monitor's refresh (`cadence.rs`, 455 lines) through `ControlFlow::WaitUntil` | `viewer_host::presentation_pump` and a one-shot `glib::timeout_add_local_once` re-armed each turn; each frame a full-viewport `MemoryTexture` | a `QTimer` calling `presentation_tick()`, its interval in whole milliseconds from the bridge |
| **frame sink**, estimated | 80–120 lines: one `Command::Image` overlay list per region, plus a tick. A YUV frame needs a conversion (on the processor, or a shader in `render-raster`, not sized) | 60–90 lines: a second `Picture`, a texture build like `page::samples()`, a timer. One `glib::Bytes` copy and a GSK upload per frame | 60–100 lines of C++ and about 30 of bridge: a `QImage` and its rectangle per frame, `update(rect)`, one new `cxx` function |
| **audio sink** | none linked. `cpal` takes PCM, with C linkage to `libasound` (RFC 0009 section 3.4) — the window process's first audio edge | none linked. `gtk4::MediaFile` hands a *file or stream* to GTK's GStreamer module, which is a system framework decoding untrusted bytes in the host (RFC 0009 section 3.6 rules that out). PCM needs `cpal` here too, or a `MediaStream` subclass | none linked. `Qt6::Multimedia` brings `QAudioSink` (PCM) and `QVideoSink` (frames), plus a runtime media backend that depends on the Qt build |
| **A/V sync**: the clock the loop offers | wake times through `WaitUntil`, no presentation timestamp back | `glib` timeouts only; no `GdkFrameClock` tick callback is used anywhere in the crate | `QTimer` at millisecond grain |

**A/V sync is the same problem in all three.** The audio device's sample clock has to be the master,
and video frames are shown or dropped against it. The tree has no media clock: `viewer_host::Clock`
is §12.4.4's `/D` and `/Dur` clock. The clock is one shared piece in `viewer-host` and three
adaptations, and no window gives it for free.

(`hosts/windows/`, the fourth window, is WinUI 3 over the C ABI. Its pages are `WriteableBitmap`s in
an `Image`, its timing is a `DispatcherQueueTimer` and `CompositionTarget.Rendered`, and WinUI's
`MediaPlayer` takes a file or stream at no package cost. Read in outline only.)

### 5.1 The wire

The confined worker sends to the host over a pipe on its standard output, as one request and one
reply (`Host::exchange`). A raster crosses **by copy** as RGBA8, the only `RasterFormat`
(`viewer-confined/src/protocol/answer.rs`). There is no shared memory: `grep memfd|mmap` over
`confined-transport` finds nothing. The message bound is `MAX_MESSAGE = 2 << 30` bytes
(`confined-transport/src/frame.rs`).

| stream | bytes per frame | bytes per second |
|---|---|---|
| 1920×1080 4:2:0 at 30 fps | 3 110 400 | 93 312 000 |
| the same as RGBA8 | 8 294 400 | 248 832 000 |
| 48 kHz stereo f32 PCM | — | 384 000 |

The bound admits a frame many times over: one RGBA8 1080p frame is 0.39 % of it. What does not fit
is the *shape*. Video is a stream, and the wire is one reply per request. A media worker would need
a streaming arm (frames pushed while the host plays) and, at a quarter of a gigabyte per second of
RGBA, either YUV across the wire with the conversion in the host or a shared-memory ring handed
over once as a descriptor. `SCM_RIGHTS` already rides the host-to-worker socket. Either choice is a
change to `confined-transport`, which RFC 0008 section 6.2 also wants shared.

## 6. What Tier C would be, priced with these numbers

What Tier C would be, with this round's figures where RFC 0009 section 3.5 had words:

| piece | what the figures say | size |
|---|---|---|
| the decoders | `rusty_h264-decoder` (`std` only) and `rusty_aac` (scalar), in the media worker; two packages, no C, no new licence | taken, not written; a seeded fuzz target each under `fuzz/` and a conformance gate over the streams this round fetched (5.5 GB of H.264, 7 MB of AAC — a held list of a few dozen, not the suite) |
| what they do not decode | interlaced H.264 (15 of the 66 streams); 48-channel AAC; xHE-AAC | the clause's report and the hand-off, as today; never a silent region |
| the demuxer | `mp4` (MIT, RFC 0009 section 3.4), unaudited by this round | the same audit as section 3 before it is taken |
| the worker | `pdf-media-worker` on `confined-transport`, `RLIMIT_AS` sized for a frame buffer: a 1080p RGBA frame is 8.3 MB, so a 1 GiB ceiling holds a decoder's reference frames many times over | the lockdown is `pdf-sandbox`'s as it stands |
| the wire | a streaming arm (frames pushed) and either YUV across it with the conversion in the host or a shared-memory ring passed as a descriptor; 93 MB/s of 4:2:0 or 249 MB/s of RGBA at 1080p30, against 0.4 MB/s of PCM | a change to `confined-transport`'s shape, shared with RFC 0008 section 6.2 |
| the frame sink | per window, by analogy to the transition path | 60–120 lines each, three windows (four with `hosts/windows/`) |
| the audio sink | none exists; `cpal` (C to `libasound`) for `quorra` and GTK, `Qt6::Multimedia` for Qt, WinUI's player for the fourth | a new dependency in every window process, two of them C |
| A/V sync | the audio device's clock as master; no media clock in the tree | one piece in `viewer-host`, three adaptations |
| the clause's own player behaviour | §13.2.6's play and screen parameters, Table 307's floating window, §13.2.7.5's monitor specifier, §13.2.2's viability already computed | not priced here; RFC 0009 section 3.5's "a media player's worth of work" stands |

Against this stands the hand-off: every viable clip already reaches the reader's own player under
the `ask` level, on A631 (ADR 2384), with the standard's own first position behind it — §13.2.1's
"variety of player software". Tier B — §13.3's PCM through the audio sink — is the smallest step
that buys the piece everything else needs, and RFC 0009 recommended it "when a document asks for
it".

## 7. The current restrictions, each with its rationale, and the unconstrained design

- **`CLAUDE.md`'s clause 13 exclusion** — "a media engine, not a rendering question". The
  unconstrained design is RFC 0009 section 3.5's Tier C: a `pdf-media-worker` on
  `confined-transport`, with a demuxer and the decoders inside and frames and samples out. This RFC
  supplies the decoders' figures.
- **Principle 3's sandbox** — a decoder of untrusted bytes runs confined, whatever it is written in.
  It binds Tier C unchanged: the candidates are pure Rust, and they still go in the worker.
- **Principle 2's launch path** — nothing of Tier C runs before a `Rendition` action the reader's
  level lets run. A media worker is spawned then, never at open.
- **Principle 5** — a decoder's output is judged against the conformance suite's reference
  reconstruction, the streams' own specification, and never against another player.

## 8. Easy and difficult, against this tree

Easy: the decoders exist and pass `deny.toml`; the windows each have a timed overlay path to grow a
frame sink from; the confined worker's lockdown is reusable as it stands. Difficult: audio output
is a new library in every window process (and C in two of the four); the wire's request/reply shape
and copy cost; a media clock and synchronisation that the tree does not have; the MP4 demuxer
(`mp4`, MIT) is a further parser of untrusted input that this round did not audit; and §13.2.6's
play and screen parameters, Table 307's floating window and §13.2.7.5's monitor specifier are
player behaviour that this RFC does not price.

## 9. The ledger

No row moves. The Tier C rows (§13.2.6.2, §13.2.6.3, §13.2.7.5) gain a pointer to this RFC, and the
Tier D rows keep the restated trigger (ADR 2400). Opening Tier C would move those rows and §13.7's
sound and video configurations to `partial` at best. RFC 0009 section 3.7 has the count and the
warning against arguing this on coverage.

## 10. Questions for the owner

The convention (`doc/questions/README.md`) puts every open question in a `Q` file; this round's is
`Q649`, which carries the figures above in one place and asks these by number.

1. **Is Tier C reopened on these figures?** The decoder half of the trigger is met with figures:
   one H.264 and one AAC decoder admissible under `deny.toml`, each bit-exact or within 1 LSB on
   most of a published suite, each fuzzed for ten minutes without a fault, each buildable without
   `unsafe`. The two gaps are named: no candidate decodes interlaced H.264, and the surface — the
   audio sink above all — is a new dependency in every window and the larger build.
   *Recommendation*: **the exclusion stands until the owner decides to build the player**, and its
   trigger is restated to say so — "an H.264 and AAC decoder admissible under `deny.toml` (RFC 0012
   names two), **and** the owner's decision to build the presentation surface" — because the decoder
   question is answered and the question left is whether this program becomes a player, which A632
   says the owner buys on an argument about the player and not on a crate existing. The crates'
   figures are the argument's first half; the surface's cost is the second, and it is the owner's
   to weigh against the hand-off that already works.
2. **If it is reopened, which decoders, under which build?** *Recommendation*: `rusty_h264-decoder`
   with `default-features = false, features = ["std"]` and `rusty_aac` with `default-features =
   false`, in the confined media worker only; `rust_h264` and `oxideav-aac` declined on sections
   3.2 and 3.3; interlaced video and every other codec stay on the hand-off, reported by name.
3. **If it is reopened, in which order?** *Recommendation*: Tier B first (§13.3's PCM through one
   audio output per window — the sink every later step needs and no window has), then the wire's
   streaming arm, then the media worker with the two decoders and the `mp4` demuxer audited as
   section 3 did, then the frame sink per window, then §13.2.6's parameters; one round and one ADR
   each, and a conformance gate over a held list of the fetched streams from the first decoder round.

## 11. Sources, read or run on 2026-10-26

- crates.io's API and `static.crates.io` packages for the four crates, and `u3d`.
- ITU-T H.264.1's bitstreams: `www.itu.int/wftp3/av-arch/jvt-site/draft_conformance/AVCv1/` and
  `…/FRExt/`.
- ISO/IEC 14496-26's bitstreams: `standards.iso.org/ittf/PubliclyAvailableStandards/
  ISO_IEC_14496-26_2010_Bitstreams/` (`DVD1/…/compressedMp4/`, `DVD2/…/referencesWav/`, and
  `DVD1/…/doc/readme_al_ap_informative.txt`, which names PNS content in al18).
- In the tree: `deny.toml`, `doc/stack.md`, RFC 0009, review 1768, `crates/confined-transport`,
  `crates/viewer-confined`, `crates/viewer-ui`, `crates/viewer-gtk`, `crates/viewer-qt`,
  `hosts/windows/`.
