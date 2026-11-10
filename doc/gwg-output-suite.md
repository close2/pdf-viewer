# The Ghent PDF Output Suite 5.0 — this reader's results

Status: **standing**, kept true by the round that moves a verdict (ADR 2728). Where the suite comes from and why none
of it is in the tree: [`gwg-output-suite-provenance.md`](gwg-output-suite-provenance.md).

**How to reproduce.** `tools/fetch-gwg.sh` (pinned by SHA-256), then `tools/gwg-render.sh <dir outside the tree>`
draws every page on the CPU at 180 dpi in both views; `cargo nextest run -p pdf-model --test gwg_output_suite`
re-checks the patches a pixel comparison decides, and fails when one of their verdicts below moves.

**The two views.** *screen* is the default view; *press* is §10.8.3's simulation of separations, which a person asks
for (`ViewState::set_separation_simulation`). **n/a** marks a patch whose `X` exists to reveal ignored overprint:
§8.6.7 says the overprint parameter "shall be ignored" where the device does not support overprinting, which the
screen view honours, so the patch is judged in the press view. It marks as well a patch whose `X` reveals a colour
converted into the press's gamut, which a screen showing the colour by its own profile is wider than (17.2, ADR 2735).
A faint `X` is a pass by the suite's own rule; a clear
one is a failure. GWG's expected result is evidence about the standard, never the standard (principle 5).

## Patches

| patch | category | screen | press | reason (press unless said) | clause |
|---|---|---|---|---|---|
| 1.0 CMYK overprint (010) | CMYK | n/a | pass | all ten squares; font and vector before ADR 2736, the DeviceN shadings before ADR 2737 | §8.6.7, §10.8.3 |
| 1.1 overprint mode (011) | CMYK | n/a | pass | both squares; OPM 1 showed the `X` before ADR 2736 | §8.6.7 |
| 5.0 font substitution (050) | CMYK | pass | pass | the embedded program's check mark | §9.6.2 |
| 5.1, 5.2 font subsets (051, 052) | CMYK | pass | pass | both subsets draw their own glyphs | §9.9.2 |
| 6.0, 6.1 shadings (060, 061) | CMYK | pass | pass | all eight match their reference images | §8.7.4.5 |
| 8.2 DeviceN 4c (082) | CMYK | n/a | pass | check mark in both images (ADRs 2736, 2737) | §8.6.6.5, §8.6.7 |
| 9.0 font types (090) | CMYK | pass | pass | seven lines match their bitmaps | §9.6, §9.7 |
| 9.1 OpenType (091) | CMYK | pass | pass | both lines match | §9.9 |
| 15.0–15.2 optional content (150–152) | CMYK | pass | pass | default view, no crossed marks | §8.11.4 |
| 16.0, 16.1, 16.2 blend modes, DeviceCMYK (160–162) | CMYK | pass | pass | no clear `X`; faint ones in 16.1 and 16.2 | §11.3.5 |
| 16.6 image soft mask (166) | CMYK | pass | pass | soft edge, no contour | §11.6.5.2 |
| 16.8, 16.9 vector soft masks (168, 169) | CMYK | pass | pass | glows, shadows and feathers match their references; too sharp before ADR 2734 | §11.5.3, §10.4.2.3 |
| 16.10 text soft masks (1610) | CMYK | pass | pass | matches its references | §11.5.3 |
| 16.11 text soft masks (1611) | CMYK | pass | pass | basic, directional and gradient feather match; cut short before ADR 2734 | §11.5.3, §10.4.2.3 |
| 17.0 JPEG 2000 DeviceCMYK (170) | CMYK | pass | pass | no `X` (checked) | §7.4.9 |
| 17.3 JBIG2 (173) | CMYK | pass | pass | faint `X` only; needs the sandbox worker | §7.4.7 |
| 18.1 16-bit DeviceCMYK (181) | CMYK | pass | pass | photograph (checked); noise before ADR 2729 | §7.4.4.4 |
| 19.0–19.2 DeviceN overprint (190–192) | CMYK | n/a | pass | all four squares; a before ADR 2736, b (image) before ADR 2737 | §8.6.7, §10.8.3 |
| 2.0 CMYK and spot overprint (020) | SPOT | n/a | pass | all ten; image, mask and shading squares before ADR 2737 | §8.6.7 |
| 3.0 grey overprint (030) | SPOT | n/a | pass | no `X`; b and h (DeviceGray over spot) before ADR 2758 | §8.6.7, §10.3.2, §10.4.2.3 |
| 3.1 grey image overprint (031) | SPOT | n/a | pass | matches "Correct"; "Wrong"'s white rectangle before ADR 2737 | §8.6.7 |
| 4.0.1, 4.1 white overprint (040, 041) | SPOT | n/a | pass | no clear `X` | §8.6.7 |
| 8.01, 8.1 DeviceN with spots (080, 081) | SPOT | n/a | pass | check marks in both images and along the gradient (ADR 2737) | §8.6.6.5, §8.6.7 |
| 12.0 white overprint and knockout (120) | SPOT | n/a | pass | knockout half also passes on screen | §8.6.7 |
| 13.0 ICC source profile (130) | ICC-CMS | n/a | pass | all four squares within two levels; a green `X` of about 20 before ADR 2757. On screen the RGB pair is shown by its own profile, wider than the press (as 17.2) | §8.6.5.8, §11.7.5.3 |
| 13.2, 13.3 ICC overprint (132, 133) | ICC-CMS | pass | pass | no check mark | §8.6.7 |
| 16.1 blend modes, ICCBased RGB (161) | ICC-CMS | pass | pass | faint `X` at most (Difference, Hue, Luminosity); clear in about twelve of sixteen before ADR 2757 (`/Saturation`) | §11.3.4, §11.7.5.3 |
| 16.4 blend modes, ICCBased CMYK (164) | ICC-CMS | pass | pass | faint `X` in Luminosity, ten levels against the README's faint example's eleven: §10.3.1's conversion between the two profiles under §8.6.5.9's `Default` compensation, the blend exact (ADR 2761) | §11.3.5.3, §8.6.5.9 |
| 16.7 image soft mask, ICC RGB (167) | ICC-CMS | pass | pass | soft edge | §11.6.5.2 |
| 17.2 JPEG 2000, ICCBased RGB (172) | ICC-CMS | n/a | pass | the image converted into the press (ADR 2736); on screen the dictionary's Adobe RGB is beyond the press's gamut (ADR 2735) | §7.4.9, §10.8.3 |
| 18.0, 18.2, 18.3, 18.4 16-bit images (180, 182–184) | ICC-CMS | pass | pass | photograph (checked); noise before ADR 2729 | §7.4.4.4 |
| 20.5, 20.6 ICC v4 images (205, 206) | ICC-CMS | pass | pass | images, not crosses | §8.6.5.5 |
| 22.0 conversion indicator (220) | ICC-CMS | pass | pass | no `X` | §11.3.5.2 |
| 22.1 output intent indicator (221) | ICC-CMS | pass | pass | no `X`; a light one before ADR 2757 (its `X` is drawn `/Perceptual`) | §11.3.5.2, §11.7.5.3 |
| 23.0 four greys (230) | ICC-CMS | pass | pass | all four alike in both views (checked). Press: a grey is the press's black ink (ADR 2758) and `Black` its colourant. Screen: the grey is previewed as the intent's black ink (A810, ADR 2765), and both `[/Black]` spaces follow it through their `DeviceGray` alternates | §10.3.2, §8.6.6.4, §14.11.5 |

Counts by patch file (51): **CMYK** (27) screen 21 pass, 6 n/a; press 27 pass. **SPOT** (8) screen 8 n/a; press 8
pass. **ICC-CMS** (16) screen 14 pass, 2 n/a; press 16 pass. In all: screen 35 pass, 16 n/a; press 51 pass.

**The test pages** (`*_CMYK_X4`, `*_SPOT_X4`, `*_ICC-CMS_X4`, `*_ALL_X4`, `*_ALL_REFERENCE`) draw without error in both
views. They place each patch in an isolated transparency group; in the CMYK page's press view 1.0 and 1.1 show only a
faint `X` and 19.0–19.2 c none, which the harness checks (deficit 8 below).

## Deficits, ranked by what a reader sees and by clause

1. **Fixed — sixteen-bit TIFF-predicted images drew as noise** (18.0–18.4, both views). `pdf_syntax::filter`
   handed every depth but eight back unpredicted; §7.4.4.4 predicts components "taking into account the number of
   bits per component". ADR 2729.
2. **Fixed — a grey in a `/DeviceCMYK` luminosity group under a CMYK intent masked darker** (16.8, 16.9, 16.11, both
   views). The luminosity was already §11.5.3's device branch; the grey reached the intent's press through its `B2A`
   separation, four inks EXAMPLE 2 weighs darker than black alone. §11.5.3's "no compensation for gamma or other colour
   calibration" and §10.4.2.3 now make it the press's black ink alone. ADR 2734.
3. **Not a defect — 17.2's JPEG 2000 image is drawn in its dictionary's space.** The `X` is (0, 247, 0), the stated
   Adobe RGB entry on an sRGB screen, not the codestream's sRGB palette; the screen shows a green the press cannot
   print, so the screen view is n/a. The press view draws no simulation, which is deficit 5. ADR 2735.
4. **Fixed — a CIE-based colour entered the press through `B2A1` whatever its intent** (13.0, 16.1 ICC, 22.1, both
   views where the page composites in the press). §11.7.5.3's intent "influences the conversion from a CIE-based colour
   space to a target colour space, taking into account the target space's colour gamut", and is the object's; the
   gamut mapping is the destination's `B2A0`, `B2A1` or `B2A2`. Planted on 13.0's own profiles: the source's `A2B0` was
   already honoured, and the CMYK pair's `X` had the RGB pair's cause, the press's table. ADR 2757.
5. **Fixed — the press view did not simulate a page with no spot colourant** (1.0, 1.1, 8.2, 19.0–19.2 a, 17.2).
   `content::separate` answered `NotSeparated::Nothing` without a spot colourant, but §10.8.3's simulated device
   "supports subtractive process colourants and possibly spot colours": a process-only page is now separated into the
   press's process pair, where §8.6.7 and §8.6.6.4's process colourant names act. ADR 2736.
6. **Fixed — images, stencil masks and `sh` shadings did not overprint** (2.0 c–e, h–j; 3.1; 8.01, 8.1, 8.2's
   images; 19.0–19.2 b; 1.0 e, j). §8.6.7's colourants are named by the space "in which the painting takes place",
   the object's own; the nonzero mode does not apply to them. A shading *pattern* fill still paints under the state's
   blend mode, which no patch exercises under `op`. ADR 2737.
7. **Fixed — grey and black in other families** (23.0, both views). On the simulated press a grey is the press's
   black ink (§10.3.2 on §10.8.3's CMYK device, ADR 2758), and it stays `DeviceGray` to §8.6.7. On screen §10.3.2's
   sentence does not reach, and §14.11.5's `AToB` "may optionally be used" for a screen preview; the owner chose that a
   grey is previewed as the intent's black ink whenever the selected intent is a CMYK profile, whatever its subtype
   (`doc/questions/A810`, ADR 2765). Both `[/Black]` spaces revert to their `DeviceGray` alternates on a screen
   (§8.6.6.4) and follow. The harness checks six halves on screen and five on the press.
   ADR 2765's census: of 773 corpus files under a CMYK intent, page one moves on 43, all PDF/X, by a mean peak of 32
   levels.
8. **Fixed, and now pinned — in an isolated group, a `DeviceN` zero tint read as overprint mode 1** (test pages'
   19.x c). Since ADR 2736, a `DeviceN` naming process colourants paints them directly on the simulated press, and
   §8.6.7 confines OPM to `DeviceCMYK`. The harness checks 19.0, 19.1 and 19.2 c on the CMYK test page's press view.
   A plant that reads the nonzero mode into a process-named `DeviceN` fails all three, by 165 to 255 levels.
9. **Fixed — 22.1's Color Burn indicator** showed a light `X` (both views): its `X` is drawn `/Perceptual`, deficit 4's
   cause. ADR 2757.
10. **Not a defect — 16.4's Luminosity `X` is the conversion between two presses** (both views). §11.3.5.3's CMYK rule
   is exact. The `X`, `1 0.49 0 0` in FOGRA39, enters the `DeviceCMYK` group through ISO Coated v2 300%'s `B2A1`
   (§11.7.2) as `0.982 0.478 0 0`, so its luminosity exceeds the magenta backdrop's by 0.013. That is ten levels:
   six come from the profiles and four from §8.6.5.9's `Default` compensation, measured with `/UseBlackPtComp /OFF`.
   The README's own faint example measures eleven. The harness checks the patch in both views. ADR 2761.
