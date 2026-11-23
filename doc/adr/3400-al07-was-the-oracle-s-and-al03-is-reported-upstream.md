# 3400 — al07 was the oracle's, and al03 is reported upstream

Session 2174. Status: accepted. Context: ADR 3382 (rusty_aac decodes an MP4 clip's AAC), RFC
0012; ISO/IEC 14496-26's bitstreams and references. ISO/IEC 14496-3 is not held.

## Decision

1. **al07_48 decodes correctly; the conformance walk mis-paired it.** Its configuration is a PCE
   (front SCE and CPE, back CPE, LFE, one coupling channel), which `rusty_aac` implements. The
   decoded order is L, R, C, LFE, Ls, Rs; the references are named f01, f02, f00, l00, b00, b01.
   The walk chose a channel and a lag over the first 16 384 frames, which are digital silence in
   four of the six, so every candidate scored nothing and the first won. The walk now chooses both
   over 16 384 frames from where the reference first passes sixteen 16-bit LSB: all six are within
   0.004 LSB at a lag of 2 048, and no other stream's verdict moved.
2. **al03_48 differs in `rusty_aac`, not in our glue**: 23 of 386 blocks differ, each one whose
   `window_sequence` does not follow its predecessor in the usual order, and `synth.rs`'s
   `imdct_and_window` chooses the left window half from the previous sequence. The issue text is
   `doc/upstream/rusty-aac-al03-window-sequence.md`, drafted and not filed. Without 14496-3 the
   text asks rather than asserts which reading is the standard's.
3. al18 and al19 (PNS, random by construction) and the low-power SBR candidate stay as RFC 0012
   left them.
