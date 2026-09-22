# 1253 — The black point ISO 18619 states, and the eleven levels it closes

Status: accepted. Session 1208.
Context: ISO 32000-2 §8.6.5.9, §8.6.5.8, §8.6.5.3; ISO/CD 18619 (2013) clause 4; ICC White Paper
40; Adobe's 2006 paper; PDF 2.0 Application Note 001; ADRs 0012, 0187, 0510, 1208.
Code: `crates/pdf-colour/src/icc.rs`, `crates/pdf-colour/src/colour.rs`.
Documents: `doc/third-party-data.md`, §8.6.5.9's ledger row, `doc/md/ISO-CD-18619-2013.md`.

`§N` is ISO 32000-2 and nothing else. Nothing of ISO/CD 18619, of ICC White Paper 40 or of
Adobe's paper is quoted here or in the tree: the draft's notice permits reproduction only for
participants in ISO's own process and the white paper states no terms, which ADR 0187 answers
the same way. Cited by section and paraphrased.

## 1. The document is held now, and what it cost was one command

ADR 1208 established that no ICC text on this disk states the algorithm §8.6.5.9 defers to, that
the ICC publishes the committee draft free, and that fetching it was a job for a round that could
write where the fetch would survive. This round fetched all three texts named there —
340 916, 2 338 582 and 196 694 bytes, the sizes ADR 1208 recorded — converted them with
`tools/spec-md.py`, and put their rows in `doc/third-party-data.md`. `doc/` and `doc/md` are
gitignored, so nothing of them enters this history.

## 2. Which of ADR 1208's eight items held, and which did not

Verified against the draft's own clause 4, item by item, because a revisit note is an argument
and not an instruction.

**Held**: it is a source-to-destination pair operation taking two profiles and one intent
(section 4.1); the intent is constrained to relative colorimetric, perceptual or saturation and
must be the same on both sides (section 4.1), which is where §8.6.5.9's own `AbsColorimetric ⇒
OFF` rule falls out; the darkest colour comes from a stated vertex set, two for Gray and RGB and
four for CMYK (section 4.2.2.2); an output-capable CMYK **source** takes a different route
entirely (section 4.2.3); the black point is clamped in lightness and its chromaticity discarded
(sections 4.2.3, 4.2.6); the map is a single scalar scale with an offset towards the connection
space's white (section 4.2.6); and a validity test decides when to do nothing (section 4.2.5.3).

**Did not hold, and it is the ADR's own headline**: ADR 1208 called "no destination side" this
tree's single largest divergence and attributed ADR 0510's eleven levels to section 4.2.5's
estimation of a **LUT destination's** black. Both are wrong, for one reason. The destination of
every conversion in `pdf-colour` is the display, whose profile is sRGB's matrix and tone curves;
section 4.2.4 computes a *non-LUT* destination's black by the same vertex function, and sRGB
reaches XYZ (0, 0, 0) at device (0, 0, 0), so its `DestinationBlackPoint` is L\* = 0 exactly.
This tree's "destination black is zero" was therefore the draft's own answer arrived at without
it, and section 4.2.5 — the bulk of the draft — has nothing to run on here at all.

## 3. What actually caused the eleven levels

Section 4.2.3's branch for an **output-capable CMYK source**: a CMYK profile carrying any
transform from the connection space to its own (section 3.5) does not walk its device corners.
Its local black is the connection space's black taken through its *perceptual* "from CIE"
transform. That is the construction Little CMS implements, which is why `poppler`, `mupdf` and
`ghostscript` agree on it.

The evidence ADR 0510 already had, read the other way round: Artifex's 187 484-byte press profile
carries a `B2A` and is output-capable; `hayro`'s 8 464-byte CGATS profile carries none and is not.
The one where this tree disagreed by eleven levels is the one with the table, and the one where
it already agreed is the one without. Measured through `Profile::to_rgb` on
`0.82 0.7 0.54 0.67 k`:

| profile | before | now | the three renderers |
|---|---|---|---|
| Artifex `default_cmyk.icc`, output-capable | (36, 44, 53) | **(26, 35, 46)** | within a level of (25, 34, 45) |
| `CGATS001Compat-v2-micro.icc`, not | (25, 34, 44) | (25, 34, 45) | the same |

The vertex count is not the mechanism on either profile: over Artifex's four corners the darkest
is still full ink, at L\* 11.77 against the perceptual route's 16.49. Trap 9's sixth bullet holds
in the deep shadow now as well as on the sampled ramp.

**And an independent corroboration, on a profile neither ADR had looked at.**
`a_real_cmyk_profile_agrees_with_independent_evaluators` reads a real CMYK profile out of
`bug886717.pdf`; its darkest patch was up to eight levels from the readers it names and is
**one**, with every other patch exact. That profile is output-capable too. Its tolerance moved
from 8 to 1 with the measurement above it.

## 4. What was built

`Profile::source_black_point` is section 4.2.3, `Profile::local_black` its three branches,
`Profile::vertices` section 4.2.2.2's sets, `Profile::perceptual_from_pcs` the `B2A0` the CMYK
branch asks for by name, `compensation` and `pcs_luminance` section 4.2.6, and section 4.2.7's
`XYZ_DST = scale × XYZ_SRC + offset` sits in `to_xyz_with` with its exact inverse in `to_device`.
Four things are worth naming:

- **The black point is now one number, its L\***, because the draft discards chromaticity before
  mapping. The per-axis stretch this crate applied is gone with it. On a neutral black the two
  agree exactly, which is why most of the existing fixtures did not move.
- **The `usable` guard is gone and the clause's clamp replaces it.** ADR 0510's fifth failure — a
  profile whose black and white differ by a thousandth, divided by, and the page drawn as a
  negative — cannot happen under a black point capped at L\* = 50, whose scale factor is at most
  1.225 8. A guard arrived at from a defect has become a number the standard states.
- **It is computed on first use**, not at parse: finding it evaluates a table two to four times
  and, on the CMYK branch, parses a second one. `CLAUDE.md` principle 2 is why a document that
  converts no colour through a profile pays none of that.
- **`to_device` undoes the compensation of the route the colour was produced under**, not of the
  colorimetric one. Its own doc comment already claimed that; it was not true for a perceptual or
  saturation rendering, and now is.

## 5. What it cost, measured

A steeper map is a harder one to interpolate, and the press grid interpolates it.
`examples/press_census --sample` over the whole crawl — 287 presses, the same population ADR
0272 sampled — puts the worst gap between `PRESS_SIDE`'s grid and evaluating the profile
directly at **15.90 of 255** where it was 14.52, median 5.91 against 5.99. That is the price of
this reading and it is written above the constant. What it is measured against is unchanged and
is two orders of magnitude larger: compositing a page in somebody else's four components, which
ADR 0251 measured at 48 to 51 of 255.

## 6. What this does not decide

**Section 4.2.5.** It is not implemented, and the row says so plainly rather than calling it a
debt: it applies to a LUT-based *destination*, and this crate's destination is the display. A
round that gives this program a destination profile of its own — a calibrated display, or a
press as the target of a conversion — brings that section into scope with it.

**Table 63's `/BlackPoint` on a `CalRGB` or `CalGray` space stays read and not applied**, and now
for a second and stronger reason than ADR 0012's: section 4.1 takes two ICC profiles and a
rendering intent as the whole of its input, so a Cal dictionary's three numbers are not an input
to the operation `ON` names. What was a documented choice about an undefined stretch is now also
a reading of the referenced procedure.
