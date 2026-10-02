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
