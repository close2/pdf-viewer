# 2428 — Tier C stays closed until the owner decides to build the player, and a reopening takes two decoders in five steps

Session 1797, a "dependency" round of batch one hundred and nineteen, 2026-10-27.
Status: accepted — the owner's answer to Q649 of 2026-10-27, given in chat, adopted all three of
Q649's recommendations; `CLAUDE.md`'s clause 13 entry carries the first, and RFC 0012 and
`doc/stack.md` carry the second and third. Context: Q649, RFC 0012, RFC 0009 section 3.5, ADR 2400,
ADR 1946, ADR 0014. Moves no ledger row. Builds nothing.

## 1. The answer

The owner's words, verbatim: "Q649: Agreed with all three recommendations (1: Tier C stays excluded
until I decide to build the player, and restate its trigger in CLAUDE.md accordingly; 2 & 3: record
rusty_h264-decoder and rusty_aac with default-features = false and the Tier B -> wire -> worker ->
sink -> parameters order for when/if it is reopened)." No `A649` file exists. This ADR is where the
answer is written down, and it cites the answer as the owner's answer to Q649 of 2026-10-27, given
in chat.

## 2. The trigger, restated

`CLAUDE.md`'s clause 13 exclusion named its audio and video trigger as "an H.264 and AAC decoder
admissible under `deny.toml`, or C in a confined media worker accepted in writing under ADR 0014".
RFC 0012 met the first half with figures: two pure-Rust decoders pass `deny.toml`'s four checks and
most of a published conformance suite, and each survived ten minutes of fuzzing in the worker. What
is left is the presentation surface: an audio sink in every window, a streaming arm on the wire, a
media clock. RFC 0012 section 6 prices it as the larger build. The trigger now names the
owner's decision to build the player as what reopens the exclusion. Admissible decoders, or C
accepted under ADR 0014, are what such a player is built on. So a decoder alone, however well it
scores, reopens nothing. This is the shape ADR 2400 gave the 3D trigger, which ends on the owner's
decision to write a reader. The edit is that one sentence of `CLAUDE.md`, and its ratification
line under the entry (Q254, A254) is unchanged.

## 3. Recorded for a reopening, decided now

- **The decoders and their build**: `rusty_h264-decoder` with `default-features = false, features
  = ["std"]` and `rusty_aac` with `default-features = false`, in the confined media worker under its
  own address-space ceiling and never in the host. The default features are out because they link
  what `deny.toml` does not see (RFC 0012 section 2.2): `rusty_h264-decoder`'s default features
  install an allocator as the process's global allocator and link a 245-`unsafe` SIMD crate, and
  `rusty_aac`'s default `simd` feature holds its 41 `unsafe`. RFC 0012 section 3 found the verdicts
  unchanged without them. `rust_h264` and `oxideav-aac` are declined on RFC 0012 sections 3.2 and
  3.3. Interlaced H.264, which neither candidate decodes, and every other codec stay on the
  hand-off and are reported by name, never drawn as a silent region.
- **The order**: Tier B first (§13.3's PCM through one audio output per window, the sink every
  later step needs), then the wire's streaming arm, then the media worker with the two decoders
  and the `mp4` demuxer audited as RFC 0012 section 3 audited the decoders, then the frame sink per
  window, then §13.2.6's parameters. Each is one round and one ADR, and a conformance gate over a
  held list of the published streams comes with the first decoder round.

A round that reopens Tier C on the owner's word starts from this list rather than re-auditing.
A version newer than RFC 0012's (0.16.1 and 2.0.1) is re-run against RFC 0012 section 3's streams
before it is taken, because the figures belong to those versions.

## 4. What stays as it is

The Tier C rows (§13.2.6.2, §13.2.6.3, §13.2.7.5) stay `out-of-scope`, and their notes now say the
question was answered. Every viable clip is still handed to the reader's own player under the `ask`
level (A631). No `Cargo.toml` names any of the four crates.
