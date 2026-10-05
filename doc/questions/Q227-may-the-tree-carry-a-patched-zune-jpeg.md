# Q227 — May the tree carry a patched `zune-jpeg`, as it carries a fork of `hayro`?

Source: round 1330, the `jpeg_bands` fuzz target (ADR 1495).
Status: **open** — answered when `A227-may-the-tree-carry-a-patched-zune-jpeg.md` exists beside this file.

## What was found

`zune-jpeg` 0.5.15, the `DCTDecode` decoder `pdf-model` runs in process, dequantises a block's DC
coefficient as `*dc_prediction * qt_table[0]` (`src/bitstream.rs`, line 400), while the prediction
itself is updated with `wrapping_add` four lines above it in `decode_dc`. A frame whose DC
differences accumulate far enough under a 16-bit quantiser — 847 bytes, written out in
`doc/patches/zune-jpeg-dc-prediction-overflow.patch` — overflows that multiply. In the release
profile (no overflow checks) it wraps and the frame decodes to wrong samples, as any damaged frame
may; in every profile with overflow checks — `dev`, every `cargo test`, `fuzz` — it **panics**,
so a test or a fuzz run over such a document aborts instead of reporting.

## Why it needs the owner

The fix is in the dependency, and the tree has no fork of it: `Cargo.toml` takes `zune-jpeg` from
crates.io. The `hayro` crates are patched by the owner's `close2/hayro` fork, and adding a second
fork — or a `[patch.crates-io]` of a vendored copy — is a decision about the dependency set
(`doc/stack.md`), which is the owner's. Nothing else in the tree waits on it: the release build is
unaffected beyond the wrong samples, and the round's other findings are fixed.

## Recommendation

Report it upstream with the reproduction, and until a release carries the fix, carry the one-line
patch the way the `hayro` patches are carried — applied to a fork the owner controls and pinned by
`rev`. The patch makes the multiply wrap as the addition beside it already does, which is the
release build's behaviour in every profile; it changes no sample any release build produces. Once
it is in, the reproduction becomes a regression test in `crates/pdf-model/tests/banded_decodes.rs`
(the whole decode of it returns rather than panics).

## A second patch, 2026-10-05

The same fork would carry `doc/patches/zune-jpeg-scan-complete-without-eoi.patch` (ADR 1520). A
baseline scan whose data holds every MCU and ends with no `EOI` — complete by ITU-T T.81 section
E.2.3's MCU count — is decoded whole except its last MCU row, which `zune-jpeg` 0.5.15 fills with
128: its lookahead reaches the end of the data while that row's bits are still in the buffer, and
the MCU loop stops at the next row on having reached it. The patch stops a row only once the
decoder has consumed past the data. Unlike the first patch this one changes samples a release build
produces, on such frames only: every frame with an `EOI` decodes as before, and no truncation
decodes fewer correct rows. `crates/pdf-model/tests/banded_decodes.rs` holds the current grey row
by name and fails the day the fork takes it. The recommendation above covers both.
