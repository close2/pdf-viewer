# 1975 — 16.4's Luminosity `X` is the conversion between two presses, and deficit 8 is pinned

Session 1975, the fifth round of the GWG lane. ADR 2761 and `Q810`. ADR 2762 was not needed. No renderer code changed.

**Deficit 10, diagnosed and refuted.** The hypothesis was that §11.3.5.3's CMYK rule ("the K component of Cs for the
Luminosity blend mode") was wrong somewhere. It is not. 16.4's `X` is `1 0.49 0 0` in FOGRA39 over a `0 1 0 0 k`
backdrop, with `Lum` 0.4109 against 0.41. §11.7.2 converts the `X` into the `DeviceCMYK` group through the intent's
ISO Coated v2 300% `B2A1`, which gives `0.982 0.478 0 0`. A throw-away probe evaluated the two embedded profiles
through `pdf_colour::icc`. Its `Lum` is 0.4234, so `SetLum` and `ClipColor` give `0 0.977 0 0`: (230, 10, 127) on
(229, 0, 125). With `/UseBlackPtComp /OFF` patched in, it draws six levels. So six levels come from the two profiles
and four from §8.6.5.9's `Default` compensation, which is this processor's choice (ADR 1054). The README's own faint
example measures eleven, so 16.4 passes. Compensation is not switched off for a GWG square, because that would be
principle 5's curve-fitting.

**Deficit 8, pinned.** The HEAD press render of the CMYK test page shows no `X` on 19.0, 19.1 or 19.2 c. The harness
now checks all three.

**Deficit 7, screen half, asked.** §10.3.2's CMYK sentence needs a CMYK-native device. Its first sentence, and
§14.11.5's `AToB` "may optionally be used … for screen preview", leave the screen to the processor. `Q810` recommends
reading `DeviceGray` as any CMYK intent's black ink, as A788 did for PDF/A. Both `[/Black]` spaces revert to their
`DeviceGray` alternate on a screen (§8.6.6.4), so that one answer decides all three failing halves. The tree keeps the
neutral grey meanwhile.

**Tests and plants.** `gwg_output_suite.rs` gains five checks: 16.4 Luminosity in the screen and press views, and
19.0, 19.1 and 19.2 c on the CMYK test page's press view. Two plants, each applied as a patch and reversed:
- `Named::Direct` reading the nonzero mode into a process-named `DeviceN` fails the three test-page checks (gaps 201,
  165 and 255);
- `Luminosity` computed as `Color`'s formula fails both 16.4 checks (gap 229).

No fix, so no constructed fixture is owed; `transparency_groups.rs`'s `clause_blend_cmyk` already pins the CMYK rule.

**Verdicts.** 16.4 now passes in both views. Screen: 34 pass, 1 fail (23.0, `Q810`), 16 n/a. Press: 51 of 51 pass.
Against HEAD, nothing renders differently, because only a test and documents changed.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-model`): exit 0. nextest `pdf-model`: 2385 of 2385.
GWG harness: exit 0. `cargo test -p conformance` after `git add -N`: exit 0. `state.sh docs`: exit 0. Tier 2 is not
owed, since no renderer crate changed.

**Unfinished.** Deficit 7's screen half waits on `Q810`.
