# 2691 — An output intent this conversion adds says whether this renderer's drawing moves

Session 1937, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/decision/colour_and_graphics.rs`:
`output_intent_sentence`, `OUTPUT_INTENT_STATES_THE_SHIPPED_PRESS`,
`OUTPUT_INTENT_STATES_THE_SUPPLIED_PRESS`; `archive/decision.rs`, the `Rewrite::OutputIntent`
arm). The question it leaves for the owner is `doc/questions/Q780`.
Context: ADR 0927 (the sentence is the condition of the rewrite), ADR 1153 (the shipped CMYK
press), ADR 0263 (the assumed inks), ADR 2114 section 4 and ADR 2518 (the shipped press as a
`DefaultCMYK` is a loss), ADR 2690 (round 1936's outputs).
Clauses: ISO 32000-2 §8.6.4.1, §14.11.5; ISO 19005-2 and ISO 19005-4 sections 6.2.3 and 6.2.4.3.

## 1. The diagnosis

Round 1936 freed six pdf.js documents. Every output of each, under both parts and all four
profiles (46 outputs), was drawn at 72 dpi beside its source. Each was then drawn again with
nothing changed but the catalog's `/OutputIntents` key renamed. With the key renamed, all 46 draw
pixel-identically to their sources. With it in place:

| document | intent | pixels that differ | largest channel difference |
|---|---|---|---|
| `colorspace_atan`, `issue17065` | sRGB2014; none (`CalRGB` needs none) | 0 | 0 |
| `colorspace_cos`, `colorspace_sin` | sRGB2014 | 266 of 76 176 | 1 |
| `issue9940` | GRACoL2006 | 5 172 of 484 704 | 7 |
| `postscript_type4_many_outputs` | GRACoL2006 | 40 000 of 40 000 | 76 |

So the output intent is the whole cause, as round 1936 guessed. No shading, function, Type 4
program, alternate space or inherited resource was rewritten in a way that changes the drawing.

**sRGB (`cos`, `sin`).** ISO 19005-2 section 6.2.4.3 admits `DeviceRGB` only through an RGB
output intent or a `DefaultRGB`, so a profile is unavoidable. Without one, this processor draws
`DeviceRGB` as its own sRGB (ADR 0009). The shipped profile's colorants are sRGB rounded to
s15Fixed16: its `rXYZ` X is 0.436066 where `cie::linear_srgb_to_xyz_d50` uses 0.436035, and the
three colorants sum to 0.96428 where D50's X is 0.9642. Its curves are 1 024 samples. A grey of
188 comes out at 189 in red alone. The target requires the difference, at one level; nothing is
changed.

**CMYK (`issue9940`, `postscript_type4_many_outputs`).** Both paint in `DeviceCMYK` with no
profile, so ADR 1153 gives each the GRACoL press this program ships. The source was drawn
through the assumed inks (ADR 0263), and no ICC profile states those: §8.6.4.1 has device values
"map directly (or by simple conversions)" to a device's colourants. The output therefore draws
through a different press. Full cyan in GRACoL lies outside sRGB, so the red channel clips to 0
at about 70 % of the ramp, and the edge where it does is the banding that is visible. The target
requires *a* CMYK characterisation, but not this one. No profile this program ships states the
inks the source was drawn with, and choosing a press is the owner's choice (ADR 1153), so the
choice goes back as `doc/questions/Q780`.

## 2. The defect, and the fix

Every stated output intent carried one sentence (ADR 0927's condition on the rewrite): "This
renderer already showed those colours that way". That is true of the sRGB profile, to within one
level, and of a profile the file already held. It is false of a CMYK press this conversion adds:
the shipped profile always, and the operator's under `--output-intent-profile`, since this
renderer drew the source through neither.

`output_intent_sentence` keeps the row's sentence unless the intent is CMYK and its source is
`Shipped` or `Supplied`. In those two cases the sentence names the press, says this renderer drew
the colours through its assumed inks, and says that they now draw differently. The shipped
variant also names `--output-intent-profile`. The rows reach it at both parts:
`cmyk_answer_under_part_two`, `cmyk_answer_under_part_four`, and the `Rewrite::OutputIntent` arm
that the grey, page-level and transparency rows go through. No byte of any output changes, since
an output intent writes nothing into `xmpMM:History`.

## 3. What is not done

The drawing itself is unchanged until the owner answers Q780. ADR 2518 already calls the
shipped press a loss when it is written as a `DefaultCMYK`. As an output intent it is still a
statement, and the two answers differ.

## 4. Evidence

- The table in section 1. Each row is reproduced by `quorra-transform render --dpi 72 --format ppm`
  over the source, the output, and the output after `qpdf --qdf` with the catalog's
  `/OutputIntents` key renamed and the file rewritten by `qpdf`, then comparing the PPM samples.
- `crates/pdf-transform/tests/archive/drawn_through_the_intent.rs`. Four tests: DeviceRGB ramps
  stay within one level; a shipped CMYK intent moves the drawing and the sentence says so, at
  both parts; the operator's press is named as theirs; a CMYK profile the file already held stays
  within one level and keeps the sentence. Planting `true ||` in `output_intent_sentence` fails
  the second and third.
