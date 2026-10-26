# Q632 — The triggers that reopen clause 13's two exclusions have been met by letter: is either reopened?

Asked by round 1768, which re-read the ledger's `out-of-scope` rows for scope
(`doc/reviews/1768-the-inapplicable-and-out-of-scope-rows-re-read-for-scope.md`, section 1.3 and
finding 5).

## The rows, and the reason they record

- **Tier C, audio and video presented by this program**: §13.2.6.2, §13.2.6.3 and §13.2.7.5, and
  §13.7's rows for sound and video configurations. Excluded "until an H.264 and AAC decoder
  admissible under `deny.toml`, or C in a confined media worker accepted in writing under ADR 0014".
- **Tier D, 3D artwork rendered**: §13.6's 27 rows, §12.6.4.16, and §13.7's 3D configuration.
  Excluded until "a U3D reader in Rust".

## What has changed

Read on crates.io on 2026-10-26 (`curl https://crates.io/api/v1/crates/<name>`, `unsafe` counted by
`grep -rw` over the published `.crate`):

- `rusty_h264-decoder` 0.16.1 (BSD-2-Clause, `forbid(unsafe_code)`, first published 2026-06-27) and
  `rust_h264` 0.4.0 (MIT OR Apache-2.0, 2026-04-05) decode H.264; `rusty_aac` 2.0.1 (Apache-2.0,
  2026-07-29) and `oxideav-aac` 0.1.7 (MIT, 2026-04-17) decode AAC. All four licences are on
  `deny.toml`'s list. RFC 0009 section 3.4 found none, though all four predate it.
- `u3d` 0.1.0 (MIT, no `unsafe`, 2 153 lines, published 2026-10-09) reads ECMA-363; its README
  leaves the progressive-mesh continuation, fuzzing and Adobe's mesh resource unticked. `prc` 0.2.0
  (MIT, 50 `unsafe`) reads PRC.

Licence is one of `deny.toml`'s four checks; bans, advisories and sources were not run. A crate
existing is evidence, not an argument: `CLAUDE.md` reopens an exclusion by argument.

## What reopening would take

- **Tier C**: RFC 0009 section 3.5's second confined worker on `confined-transport`, a demuxer (`mp4`
  is MIT), the decoders, frames and samples across the wire, a clock and synchronisation, Table
  307's floating window and §13.2.6's screen parameters, and a presentation surface in each of three
  windows. A stream of several batches. The hand-off already gives every viable clip to the reader's
  own player.
- **Tier D**: a U3D reader that reads the whole of ECMA-363's mesh set and is fuzzed (in-tree or this
  crate grown), §13.6.4's views, projections, lighting and render modes, and a mesh renderer under
  `wgpu`; Table 310's instantiation on page open runs into RFC 0008's ordering. A stream. Neither
  pdf.js nor poppler renders 3D, so the oracle has nothing to compare against.

## The question

1. Is Tier C reopened, and if so, does it start with an RFC that prices it against these crates?
2. Is Tier D reopened, or is its trigger restated?

## Recommendation

**1: an RFC first, nothing built** — one round that audits the decoders (their conformance claims run
here against published streams, fuzzed in the worker, `deny.toml`'s four checks), sizes the
presentation surface, and asks again. **2: keep Tier D excluded and restate its trigger** as "a U3D
reader in Rust that reads ECMA-363's fourth edition whole and is fuzzed, or the owner's decision to
write one in-tree", since ECMA-363 is freely published and the present crate is a start, not a reader.

**What the tree does meanwhile**: the annotations' appearances are drawn, clips are handed over, and
the rows stay `out-of-scope`.
