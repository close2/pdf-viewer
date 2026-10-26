# Q649 — Is Tier C reopened on RFC 0012's figures, and with which decoders?

Asked by round 1783, the round `A632` set: *"Q632 as recommended please"* — the recommendation
being an RFC first, nothing built, that audits the decoders, sizes the presentation surface and
asks again. RFC 0012 is that document; this file asks its section 10's questions by number.

## The figures, in one place

Read on 2026-10-26, each in RFC 0012's section named beside it.

- **`deny.toml`** (section 2): all four crates pass all four checks in a scratch package of their
  own; the instrument fires on a planted MPL-2.0 and a banned `openssl`. What the checks do not
  see: `rusty_h264-decoder`'s default features install a 22 000-line allocator as the process's
  global allocator and link a 245-`unsafe` SIMD crate; `rusty_aac`'s default `simd` feature holds
  its 41 `unsafe`; both have a feature set with neither, and the verdicts below are the same under it.
- **H.264 conformance** (section 3.2), 66 of ITU-T H.264.1's published streams, bit-exact against
  the shipped reconstructions: `rusty_h264-decoder` **45 of 66** (Baseline 23 of 27, Main 16 of 25,
  High 6 of 14); `rust_h264` **31 of 66**. Neither decodes interlaced video, and each refuses it by
  name.
- **AAC conformance** (section 3.3), ISO/IEC 14496-26's streams against its reference waveforms:
  `rusty_aac` **11 of the 14 judged LC and Main streams within 1 LSB** at 16 bits; two of the three
  others are PNS streams no sample comparison can judge; all seven HE-AAC streams decode with SBR
  and PS applied. `oxideav-aac` 6 of 14, six errors at frame 0, and slower than real time.
- **Fuzzed in the worker** (section 4): ten minutes each, unsanitised, under the decoder worker's 1 GiB
  address-space ceiling: `rusty_h264-decoder` 35 257 runs and `rusty_aac` 565 802 runs with no panic,
  hang, abort or over-memory; `rust_h264` and `oxideav-aac` each stopped on a 10 s timeout whose input
  terminates alone in about 5 s — slow paths, not hangs, at 11 and 1 executions a second against 58
  and 941.
- **The surface** (section 5): a frame sink is 60–120 lines per window by analogy to the §12.4.4
  transition path each already has; **no window links any audio library**, and an audio sink is a
  new dependency in each — `cpal` with C linkage to `libasound` in `quorra` and GTK, `Qt6::Multimedia`
  in Qt; A/V sync needs a media clock the tree does not have. The confined wire carries a raster by
  copy, one reply per request, so video needs a streaming arm or shared memory (section 5.1).

## The questions

1. **Is Tier C reopened?** The trigger's first half is met: an H.264 and an AAC decoder admissible
   under `deny.toml`, now with figures rather than crate names. The owner's A632 said a crate
   existing is evidence and not an argument; this is the argument, and it has two gaps the figures
   name: **interlaced H.264 is not decoded by any candidate**, and the presentation surface — audio
   above all — is the larger build. *Recommendation*: **the exclusion stands until the owner decides to build the
   player**, and its trigger is restated to say so — "an H.264 and AAC decoder admissible under
   `deny.toml` (RFC 0012 names two), and the owner's decision to build the presentation surface" —
   because the decoder question is now answered and the question left is whether this program
   becomes a player, which A632 says is bought on an argument about the player. RFC 0012 section 10
   is that argument's two halves: the crates' figures, and the surface's cost against the hand-off
   that already works.
2. **If it is reopened, which decoders, and under which build?** *Recommendation*:
   `rusty_h264-decoder` with `default-features = false, features = ["std"]` and `rusty_aac` with
   `default-features = false`, each in the confined media worker under its own address-space
   ceiling, never in the host; `rust_h264` and `oxideav-aac` declined on their figures. A decoder's
   refusal (interlace) reaches the reader as the clause's own report and the hand-off, never a
   silent black region.
3. **If it is reopened, in which order?** *Recommendation*: RFC 0009 section 3.5's Tier B first
   (§13.3's PCM through one audio output in each window — the audio sink is the piece every later
   step needs and the one no window has), then the wire's streaming arm, then the media worker with
   the two decoders and the `mp4` demuxer audited the same way, then the frame sink per window,
   then §13.2.6's parameters. Each is its own round and its own ADR.

## What the tree does meanwhile

The rows stay `out-of-scope`, with a pointer to RFC 0012 in the three Tier C rows' notes. The
annotations' appearances are drawn, §13.2.2's viability is computed and reported, and every viable
clip is handed to the reader's own player under the `ask` level (A631). Nothing of the four crates
is in any `Cargo.toml`; the scratch packages and the fetched streams were the round's and are not
in the tree.
