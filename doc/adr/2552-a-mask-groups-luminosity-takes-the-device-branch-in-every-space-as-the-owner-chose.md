# 2552 — A mask group's luminosity takes the device branch in every space, as the owner chose

Status: implemented. Session 1859. Q725 is not asked.
Context: ISO 32000-2 §11.5.3, §11.6.5.1, §11.3.4, §11.4.7, §8.6.3, §8.6.5.1; the owner's answer A23
(2026-09-06) to `doc/questions/Q23`; ADRs 0217, 0796, 0797, 0851, 0856, 0857, 1342, 1764; principle 5.
Code: `crates/pdf-model/src/soft_mask.rs` (`luminosity`, `luminosity_departure`),
`crates/pdf-render/src/soft_mask.rs` (`Luminance::device_rgb`),
`crates/pdf-model/src/content/transparency/soft_mask.rs`, and three doc comments in `crates/pdf-colour`.
Documents: §11.5.3's ledger row, `doc/todo/65`, `crates/pdf-model/tests/oracle.rs`.

## 1. What was owed

§11.5.3 converts a mask group's composited colour "to luminosity in one of the following ways,
depending on the group's colour space": for CIE-based spaces the `Y` of the colour in CIE 1931 XYZ,
for device spaces a grey "by implementation-defined means [...] with no compensation for gamma or
other colour calibration". ICCBased, CalGray and CalRGB are CIE-based (§8.6.3). ADRs 0796, 0797,
0851 and 0857 built the colorimetric branch for one, three and four components, and Q23 asked the
owner whether to keep it, since no other reader computes it. A23 answered: do what every other
viewer does, document it in an ADR, and perhaps one day offer a flag that chooses the clause. That
answer stood unbuilt; the oracle's page furthest from its nearest reference, `issue21346.pdf` at
11.17 bounds, was its one corpus witness, held as the references' departure.

**This is the ADR A23 owes.** A23's header reads "Owes: the ADR for the switch to the common
behaviour"; this record is that ADR and the code it names is the switch, so the debt is paid here.
The A file is the owner's and is not edited; §11.5.3's ledger row names ADR 2552 as the ADR A23 owes.

## 2. The decision

**Every mask group's luminosity is §11.5.3's device branch, on the components the group was
composited in.** The compositing is untouched: §11.6.5.1 still makes the `/CS` "the colour space in
which the compositing computation is to be performed", and the group is still painted in its own
one, three or four components by the routes those ADRs built. Only the weighing changes, to the
device family of the same component count: one component is the grey itself; three take EXAMPLE 2's
`0.30 R + 0.59 G + 0.11 B`, carried as `Luminance::device_rgb`'s linear curves, which every lane and
the confined protocol already read; four take EXAMPLE 2's `1 − min(1, 0.3 C + 0.59 M + 0.11 Y + K)`,
`Luminance::device_ink`, which a blending `DeviceCMYK` group already used (ADR 1342). A four-component
press with no conversion in by its profile is still refused as a blending space, as before.

This is the construction Q23 described — "the plain device grey with no calibration" — and it is the
clause's own other branch, so every value a test asserts is written from EXAMPLE 2, not from a reader.

## 3. The evidence, which is evidence and not a target

A white page with a black fill through a mask group in each space, the centre pixel at 72 dpi (the
last row, the red channel of the rectangle `issue21346.pdf` draws through its mask):

| group `/CS`, content | poppler | mupdf | ghostscript | before | now |
|---|---|---|---|---|---|
| ICC gray (gs `default_gray.icc`), 0.5 | 127 | 128 | 128 | 200 | 127 |
| ICC CMYK (gs `default_cmyk.icc`), magenta | 169 | 161 | 160 | 199 | 150 |
| ICC CMYK, black | 223 | 255 | 255 | 246 | 255 |
| ICC RGB (gs `a98.icc`), green | 105 | 95 | 95 | 95 | 105 |
| `issue21346.pdf`'s sRGB group, the rectangle | 206 | 207 | 206 | 245 | 206 |

The three readers agree with each other where the space is grey or sRGB, and there this tree now
agrees with them. Where they part — a CMYK press, a wide-gamut RGB — they are not "one way", and this
tree does not choose between them by fitting: it applies the device branch's rule, which equals
`poppler` on the a98 green and `mupdf` and `ghostscript` on the black ink. The disagreement left on
those two shapes is theirs to have.

## 4. What it costs, and what is kept

- **§11.5.3's ledger row is `departed`**: the colorimetric sentence is the one requirement not
  executed, by the owner's choice. Principle 5 is the owner's, and so is this exception to it.
- **The colorimetric `Y` stays in the tree with no drawing caller**: `RgbRoute::luminance`,
  `Press::luminance`'s profile grid and `ColourSpace::cie_luminance`. They are the clause's branch,
  kept for the choice A23 leaves open; a flag offering it is the owner's to ask for, and it attaches
  at the two `match` arms of `luminosity` that now name the device weights.
- **The tests that asserted the clause's `Y`** now assert EXAMPLE 2's values, each written from the
  formula: the CalGray, CalRGB, table-profile and four-component groups in `tests/transparency_groups.rs`
  and the family-black backdrop of ADR 1764 in `soft_mask.rs`, whose black ink now masks nothing.
- **The report for a group not composited in its own components** says what it departs from now, the
  compositing space, rather than the `Y` it no longer owes.
