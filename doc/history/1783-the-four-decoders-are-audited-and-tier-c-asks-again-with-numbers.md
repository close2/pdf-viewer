# 1783 — The four decoders are audited and Tier C asks again with numbers

Session 1783, a dependency round in batch 118; 2026-10-26. RFC 0012; ADR 2400; ADR 2401 unused; Q649.

**Contract.** Slot 5: A632 — the four decoder crates review 1768 found, each run against published
conformance streams, fuzzed as a worker would run them, `deny.toml`'s four checks run; Tier C's
presentation surface sized across the three toolkits; RFC 0012 asking again; Tier D's trigger restated
in `CLAUDE.md` in A632's words. Nothing built into the tree: no `Cargo.toml` names any of the four.

**Hypothesis.** *One H.264 and one AAC crate pass a baseline-profile subset and survive the fuzz; the
surface is the larger cost* — **held**. `rusty_h264-decoder` 0.16.1 is bit-exact on 45 of 66 ITU-T
H.264.1 streams (Baseline 23/27, Main 16/25, High 6/14) and ran 35 257 fuzz inputs in 600 s clean;
`rusty_aac` 2.0.1 is within 1 LSB on 11 of the 14 ISO/IEC 14496-26 LC and Main streams judged, and
565 802 inputs clean. `rust_h264` 0.4.0: 31 of 66; `oxideav-aac` 0.1.7: 6 of 14, six errors at frame 0,
slower than real time; each stopped the fuzz on a 10 s timeout whose input terminates alone in ~5 s.
All four pass all four `deny` checks (the instrument fires on a planted MPL and `openssl`). No candidate
decodes interlaced H.264. No window links any audio library; a frame sink is 60–120 lines per window.

**Premises.** *Published conformance streams are fetchable* — held (JVT and ISO answer 200); the
960-frame AAC stream cannot be carried in ADTS and the HE-AAC streams have no single reference wav, so
14 AAC streams were judged and SBR by rate and channels only. *Two decoders agreeing is evidence* —
read the other way: both AAC decoders are OFF on al18/al19 by the same RMS, because they are PNS
streams no sample comparison can judge (trap 9). CVSE2 ships two references; EXACT against the un-repeated.

**Found, not built.** `rusty_h264-decoder`'s default features install `rusty_alloc` as the process's
global allocator and link a 245-`unsafe` SIMD crate; its `std`-only build gives the same verdicts. The
Tier C rows (§13.2.6.2, §13.2.6.3, §13.2.7.5) point at RFC 0012; no status moves; the recommendation is
that the exclusion stands until the owner decides to build the player, the decoder half being met.

**Files.** New: `doc/rfc/0012-the-decoders-tier-c-would-take-audited-and-the-surface-it-would-need-sized.md`,
ADR 2400, `doc/questions/Q649-is-tier-c-reopened-on-rfc-0012s-figures-and-with-which-decoders.md`,
this record. Edited: `CLAUDE.md` (the one Tier D trigger sentence), `doc/rfc/README.md` (one row),
`doc/conformance/ledger.toml` (three notes, `-U0`). I worked in `scratchpad/r1783/` — the harness and
fuzz packages, scripts, result tables and fuzz logs are there; the 5.5 GB of H.264 streams are deleted.

**Gates.** `cargo deny check` over each candidate's scratch package: exit 0 ×4, the planted control exit
6. Fuzz runs (600 s, long lane, `RLIMIT_AS` 1 GiB): `rusty_h264-decoder` exit 0, `rusty_aac` exit 0,
`rust_h264` exit 70 (timeout at 426 s), `oxideav-aac` exit 70 (timeout at 97 s). `cargo test -p
conformance --no-fail-fast`: exit 101, 2 red, none mine (record 1781's 46 lines and its Gates paragraph), every other target 0 failed. No crate touched: no tier 1, no `docs`. Duration
3054 s.
