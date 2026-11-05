# 2700 — The assumed inks are written as an ICC output profile

Session 1943, a single archive round.
Status: accepted and built in `pdf-colour` (`colour/inks_profile.rs`:
`assumed_inks_profile`; `examples/write_assumed_inks_profile.rs`) and checked in as
`data/icc/quorra-assumed-inks.icc`.
Context: the owner's answer A780 (option c), ADR 0263 (the ink cube and its right inverse), ADR
2691 (the shipped press moved the drawing), ADR 1153 (the shipped press), ADR 0927 (the sentence).
Clauses: ISO 32000-2 §8.6.5.5, §10.3.2, §14.11.5; ICC.1:1998-09 sections 6.1, 6.3.3.2, 6.4.16,
6.5.6, Annex A; ISO 19005-2 sections 6.2.3, 6.2.4.2.

## 1. The premise, checked

- **This reader draws `DeviceCMYK` through a CMYK output intent's `A2B`, two ways.** A fill is
  parsed as `ICCBased` over the intent (`Conversion::under_output_intent`) and converted through
  the profile itself; a composite or a separation goes through the intent's press
  (`simulated_press`), the profile's `A2B` sampled at 17 points an ink (`PRESS_SIDE`, relative
  colorimetric, black-point compensation on) and interpolated in sRGB by the backend.
- **A sampled `A2B` reproduces the cube at its nodes** to its 16-bit quantisation (0.26 levels of
  255, the darkest nodes' XYZ), **and not between them**: a reader interpolates in the connection
  space, and the cube is multilinear in encoded sRGB. The premise that the renderer reads only
  nodes held for the press grid at side 17 and failed for fills.
- **Grid, measured** (largest channel difference against `device_cmyk::cmyk`, levels of 255):

| `A2B` side | four one-ink ramps of 256 | the 17⁴ press grid | its cell midpoints | profile |
|---|---|---|---|---|
| 9 | 4.59 | 4.47 | 4.01 | 0.2 MB |
| 17 | 1.16 | 0.26 | 1.30 | 0.7 MB |
| **21** | **0.88** | **0.79** | **0.78** | **1.6 MB** |
| 25 | 0.67 | 0.78 | 0.45 | 2.5 MB |
| 33 | 0.36 | 0.26 | 0.26 | 7.3 MB |

  Over a `Lab ` connection space side 2 is 48.8 levels on the ramps, 3 is 28.0, 5 is 12.9; storing
  each tristimulus value through the sRGB curve with decoding output tables did not help (side 17,
  midpoints 1.60). **21 is taken**, the smallest side under one level everywhere measured; GRACoL
  is 2.7 MB.

## 2. The construction

ICC.1:1998-09, version 2.2.0, class `prtr`, `CMYK` over an `XYZ ` connection space, the ten tags
its Table 29 requires; one `lut16Type` serves the three `A2B` intents and one the three `B2A`
(section 6.3.3.2 permits it). The `B2A` uses section 6.5.6's matrix, which that text allows where
the input is XYZ, to take the connection space to linear sRGB, and its input tables encode it, so
its 33³ grid stands on encoded sRGB and samples `rgb_to_ink` — the renderer's own conversion into
the press — where the cube is multilinear. Over L\*a\*b\* the same grid round-tripped the cube's
17⁴ colours to within 25.0 levels; over sRGB it is **3.62**, 3 740 of 83 521 past one level, all at
the gamut's edge or in the darkest quarter where the search's black changes rung (side 17: 6.56;
side 49: 3.50 at 1.6 MB more). `gamt` is zero where the inks reproduce the node within one level,
a choice: section 6.4.16 fixes only that zero is in gamut.

A version 4 `lutAtoBType` could carry the sRGB curve and matrix *after* its table (M curves and
matrix), which would make even a side-2 table exact; this reader refuses such a tag
(`parse_mab`), and teaching it to is the cheaper profile's price.

**Deterministic**: a fixed header date (2026-11-05), no clock, every `powf` in `f64`, and the `f32`
stages taken from the renderer are its matrices and its search, which use IEEE-exact operations.
**Checked in, not built**: `include_bytes!` costs nothing at launch, the bytes can be read and
validated as a file, and `the_checked_in_profile_is_what_the_generator_writes_every_time`
regenerates and compares. A build script would make `pdf-transform` build `pdf-colour` twice.

## 3. What it costs, and what it does not settle

**Transparency.** An output intent is also the page's default blending space. Of the 30 documents
whose `as-shown` outputs changed, 24 now draw within one level of their source at both parts; six
(`bug1703683_page2_reduced`, `bug1755507`, `issue12798_page1_reduced`, `issue13520`, `issue18032`,
`issue9017_reduced`, every one built of soft masks, blend modes and `DeviceCMYK` groups) differ by
up to 25 levels (under GRACoL, up to 53). With `/OutputIntents` renamed each draws identically, and
with the conversion into the press taken by `rgb_to_ink` rather than this profile's `B2A` it
differs by exactly as much, so the cause is the blending space the intent brings, not the profile.
The report's sentence says so. Whether a page that names no blending space should blend in the
intent's space at all is §11.4.7's question, and it is not this round's.


1.6 MB in the binary and, deflated, in every archive whose output intent it is. Inside this
renderer a page group composited in `DeviceCMYK` now converts into the press through the
profile's `B2A` (ADR 0796) rather than the search, so a composite may move by up to the measured
3.6 levels against its source.
