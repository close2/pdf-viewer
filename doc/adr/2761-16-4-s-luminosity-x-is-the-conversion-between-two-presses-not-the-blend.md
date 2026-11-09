# 2761 — 16.4's Luminosity `X` is the conversion between two presses, not the blend

Status: accepted. Session 1975. Context: ISO 32000-2 §11.3.5.3, §11.3.4, §11.7.2, §11.7.5.3, §8.6.5.9, §10.3.1; ADRs
1054 (the `Default` this processor compensates), 2757 (the object's intent selects the `B2A` table); the Ghent PDF
Output Suite's patch 16.4 (ADR 2728), deficit 10 of `doc/gwg-output-suite.md`.

## The suspicion

Rounds 1957 to 1973 judged 16.4 failed in both views: its Luminosity square shows an `X` and its other fifteen do
not. The file states no rendering intent, so ADR 2757's cause was ruled out, and §11.3.5.3's luminosity of a CMYK
colour was the next reading: its CMYK rule complements C, M and Y into R, G and B, applies the RGB formulae, and
takes "the K component of Cs for the Luminosity blend mode".

## What the file states, and what is drawn

- **The backdrop** is `0 1 0 0 k`, `DeviceCMYK`, in a page group whose `/CS` is `/DeviceCMYK`, under a PDF/X-4 intent
  whose profile is ISO Coated v2 300% (ECI).
- **The source** is the `X`, `1 0.49 0 0` in `[/ICCBased <Coated FOGRA39>]`, painted `/BM /Luminosity`. Its
  luminosity by §11.3.5.3's `Lum` on the complements is 0.4109; the backdrop's is 0.41. Had the source reached the
  group unchanged, the clause's arithmetic would leave the magenta within a tenth of a percent.
- **It does not reach it unchanged, and §11.7.2 is why**: a colour whose space is not the group's "shall be
  converted to the group's colour space". The two profiles differ, so this is §10.3.1's conversion between two
  CIE-based spaces, into the intent's press through its `B2A1` (ADR 2757). It gives `0.982 0.478 0 0` under black
  point compensation and `0.996 0.480 0 0` without, evaluated through `pdf_colour::icc::Profile` on the two embedded
  profiles.
- **The clause then does exactly what it says.** `Lum` of the converted source is 0.4234; `SetLum` lifts the magenta
  by the difference and `ClipColor` brings it back to `0 0.977 0 0`, K the source's zero. That is the `X` drawn,
  (230, 10, 127) on (229, 0, 125): ten levels, in green.

**The suspicion is refuted.** The blend is §11.3.5.3's, term for term; `transparency_groups.rs`'s
`clause_blend_cmyk` already pins the CMYK rule end to end, and a plant of `Color`'s formula under `Luminosity` moves
the 16.4 check by 229 levels.

## Where the ten levels come from

- **Six are the two profiles.** The same file with `/UseBlackPtComp /OFF` on the Luminosity state draws the `X` at
  six levels: FOGRA39's colorimetry of that blue is not exactly reproduced by ISO Coated v2 300%'s `B2A1`.
- **Four are black point compensation.** The file states no `/UseBlackPtComp`, so §8.6.5.9's "If the value is not
  given or set to Default , then the behaviour is left to the PDF processor to determine" applies, and this processor
  compensates (ADR 1054). Bringing FOGRA39's black point onto ISO Coated v2 300%'s lightens this blue by 1.4 %
  of cyan.

## The decision

16.4 is judged a pass, by the suite's own rule: its README puts a faint `X` down to differences between colour
engines and not to a failure, and its own faint example measures eleven levels against this one's ten. The harness
holds both views with a check (`gwg_output_suite.rs`, 16.4 Luminosity), which a plant of the formula fails.

**Not done, and why: the `Default` is not switched off to clear the patch.** §8.6.5.9 leaves the choice to the
processor and §11.7.5.3 states what compensation is for, detail in dark regions; turning it off for one GWG square is
fitting a constant to a corpus (principle 5), and would still leave the profiles' six levels. A later round that finds
a clause deciding compensation for a conversion *into a blending space* reopens this; GWG's verdict does not.
