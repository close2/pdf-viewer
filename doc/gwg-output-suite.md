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
| 1.0 CMYK overprint (010) | CMYK | n/a | **fail** | font, vector and DeviceN-shading overprint not simulated; image and mask rows right | §8.6.7, §10.8.3 |
| 1.1 overprint mode (011) | CMYK | n/a | **fail** | OPM 1 square shows the `X` | §8.6.7 |
| 5.0 font substitution (050) | CMYK | pass | pass | the embedded program's check mark | §9.6.2 |
| 5.1, 5.2 font subsets (051, 052) | CMYK | pass | pass | both subsets draw their own glyphs | §9.9.2 |
| 6.0, 6.1 shadings (060, 061) | CMYK | pass | pass | all eight match their reference images | §8.7.4.5 |
| 8.2 DeviceN 4c (082) | CMYK | n/a | **fail** | no check mark in either image | §8.6.6.5, §8.6.7 |
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
| 19.0–19.2 DeviceN overprint (190–192) | CMYK | n/a | **fail** | squares a and b show the `X`; c and d right | §8.6.7, §10.8.3 |
| 2.0 CMYK and spot overprint (020) | SPOT | n/a | **fail** | font and vector right; image and shading rows show the `X` | §8.6.7 |
| 3.0 grey overprint (030) | SPOT | n/a | **fail** | b and h (DeviceGray over spot) show a dark `X` | §8.6.7, §10.4.2.3 |
| 3.1 grey image overprint (031) | SPOT | n/a | **fail** | the white rectangle of "Wrong" | §8.6.7 |
| 4.0.1, 4.1 white overprint (040, 041) | SPOT | n/a | pass | no clear `X` | §8.6.7 |
| 8.01, 8.1 DeviceN with spots (080, 081) | SPOT | n/a | **fail** | no check marks in images or gradient | §8.6.6.5, §8.6.7 |
| 12.0 white overprint and knockout (120) | SPOT | n/a | pass | knockout half also passes on screen | §8.6.7 |
| 13.0 ICC source profile (130) | ICC-CMS | n/a | **fail** | green `X` on the RGB vector and image: "Perceptual" not honoured | §8.6.5.8 |
| 13.2, 13.3 ICC overprint (132, 133) | ICC-CMS | pass | pass | no check mark | §8.6.7 |
| 16.1 blend modes, ICCBased RGB (161) | ICC-CMS | **fail** | **fail** | clear `X` in about twelve of sixteen | §11.3.4, §8.6.5.8 |
| 16.4 blend modes, ICCBased CMYK (164) | ICC-CMS | **fail** | **fail** | Luminosity alone shows an `X` | §11.3.5.3 |
| 16.7 image soft mask, ICC RGB (167) | ICC-CMS | pass | pass | soft edge | §11.6.5.2 |
| 17.2 JPEG 2000, ICCBased RGB (172) | ICC-CMS | n/a | **fail** | drawn in the dictionary's Adobe RGB, beyond the press's gamut on screen (ADR 2735); no simulation on a process-only page | §7.4.9, §10.8.3 |
| 18.0, 18.2, 18.3, 18.4 16-bit images (180, 182–184) | ICC-CMS | pass | pass | photograph (checked); noise before ADR 2729 | §7.4.4.4 |
| 20.5, 20.6 ICC v4 images (205, 206) | ICC-CMS | pass | pass | images, not crosses | §8.6.5.5 |
| 22.0 conversion indicator (220) | ICC-CMS | pass | pass | no `X` | §11.3.5.2 |
| 22.1 output intent indicator (221) | ICC-CMS | **fail** | **fail** | a light `X`; the README calls this indicator unreliable | §11.3.5.2 |
| 23.0 four greys (230) | ICC-CMS | **fail** | **fail** | DeviceGray, Separation Black and DeviceN's image half differ from DeviceCMYK (checked) | §10.4.2, §8.6.6.4, §14.11.5 |

Counts by patch file (51): **CMYK** (27) screen 21 pass, 6 n/a; press 21 pass, 6 fail. **SPOT** (8) screen 8 n/a;
press 3 pass, 5 fail. **ICC-CMS** (16) screen 10 pass, 4 fail, 2 n/a; press 10 pass, 6 fail. In all: screen 31 pass,
4 fail, 16 n/a; press 34 pass, 17 fail.

**The test pages** (`*_CMYK_X4`, `*_SPOT_X4`, `*_ICC-CMS_X4`, `*_ALL_X4`, `*_ALL_REFERENCE`) draw without error in both
views. They place each patch in an isolated transparency group, and there the press view behaves differently: 1.1
passes, 1.0 shows only a faint `X`, and 19.0–19.2 show the `X` on squares b and c as well — deficit 8 below.

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
4. **`ICCBased` RGB into a `DeviceCMYK` blending space** (16.1 ICC, 13.0, 16.4's Luminosity). The clear `X` even
   under Darken says the converted source colour differs, before any blending; 13.0's README names the rendering
   intent. First check: whether §8.6.5.8's `/RI` (Perceptual, Saturation) reaches the RGB→press conversion. Two
   readings so far, neither yet planted. 13.0's press view draws exactly its screen view: a page with no spot colourant
   gets no simulation, so it is deficit 5 before it is this one. In 16.1 an object's colour enters the page group's
   press through `to_cmyk`'s `to_press`. The object's own intent reaches its source profile there, but the press's
   `B2A` table is the one sampled under the group's rendering (`transparency::press`).
5. **Press view: overprint on a page with no spot colourant is not simulated** (1.0, 1.1, 8.2, 19.0–19.2 a and b).
   `content::separate` answers `NotSeparated::Nothing` when `spot_colourants` is empty, so a process-only page is
   drawn composite and §8.6.7's "erasing actions shall not be performed" never applies to the process plates. The same
   answer leaves 17.2's Adobe RGB image unconverted into the press in the press view (ADR 2735).
6. **Press view: images and shadings do not overprint spot plates** (2.0 c, e, h, j; 3.1; 8.01 and 8.1's images).
   §8.6.7 applies to "any painting operation"; the separation path honours it for fills and text only.
7. **Grey and black in other families do not render as DeviceCMYK's black** (23.0 both views; 3.0 b and h). Whether
   `DeviceGray` beside a CMYK output intent means the intent's black channel (§10.4.2.3 and §14.11.5, the
   `GreyReading` the code names) and whether a `Separation` named `Black` is the press's process colourant under
   §10.8.3 (§8.6.6.4) are the two readings to take back to the text.
8. **Press view: in an isolated group, a `DeviceN` zero tint is treated as overprint mode 1** (test pages' 19.x c).
   §8.6.7 confines OPM to `DeviceCMYK`; a `DeviceN` colour paints every colourant it names, zero included.
9. **22.1's Color Burn indicator** shows a light `X` (both views). §11.3.5.2's Color Burn in a CMYK blending space,
   read on additive complements; low rank because the README itself calls the indicator unreliable.
