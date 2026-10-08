# 1713 — A JPX CIE Lab is drawn under the white point its illuminant names

Session 1440. Status: **accepted** and **built**. Builds on ADRs 1383 and 1712; amends ADR 1383
section 3's "14 under another illuminant" fallback, which is now drawn. Context: ISO 32000-2 §7.4.9
and §8.6.5.4's EXAMPLE; ITU-T T.801 M.11.7.4.1 and Table M.29 (held, `doc/md/T.801.md`); ITU-T T.4
Annex E sections E.6.4 and E.6.7 (held, `doc/md/T.4.md`); the CIE's open-access datasets, CC BY-SA
4.0, vendored under `data/cie/` with their provenance; CIE S 017:2020 (the ILV) entry 17-23-067, read
on the CIE's site on 2026-10-08 and cited, not held; the 2019 SI's exact `h`, `c` and `k`. Code:
`crates/pdf-model/src/jpeg2000.rs` (`ColourSpecification::cielab_illuminant`, `Illuminant`),
`crates/pdf-model/src/image.rs` (`jpx_drawn`, `JpxSpace::Lab`), `crates/pdf-colour/src/planckian.rs`,
one hunk of `crates/pdf-transform/src/redact.rs`, `crates/pdf-model/examples/jpx_colour_census.rs`'s
label. Row: §7.4.9, which stays `partial`.

## 1. What the codec does and does not read

`hayro-jpeg2000` applies T.801 Equation M-18 with the box's ranges and offsets, or M.11.7.4.1's
defaults, and reads no `IL`. So the samples arrive as L\*a\*b\* and the white they are relative to is
this tree's to supply — which, with ADR 1712, is now a field of `Lab`.

## 2. Each white point, and where it comes from

T.801 Table M.29 and T.4 section E.6.7 code eight standard illuminants and a colour temperature.

| `IL` | white point (`Y` = 1) | source |
|---|---|---|
| D50 | 0.96422, 0.82521 | T.4 section E.6.4's own X 96.422, Y 100, Z 82.521 |
| D65 | 0.9505, 1.0890 | §8.6.5.4's EXAMPLE, the CCIR XA/11 value |
| D75, A, C, F2, F7, F11 | five places | the tristimulus sum of the CIE's distribution against its 1931 2° functions |
| `CT` + kelvin | computed | the Planckian radiator at that temperature (section 3) |

The six sums are compiled in and `every_white_point_is_the_cie_datas_own_sum` recomputes each from
`data/cie/` at every wavelength both tables state; the CIE's D65 table is kept to show the EXAMPLE's
value agrees with it within 2 × 10⁻⁴. Where the held standard prints a value, the held standard's
value is used, which is principle 5's order.

## 3. A colour temperature is a Planckian radiator's

ADR 1383 and the row's note said the `CT` code names no family — a black body or a daylight phase —
and would stay the fallback. That was a claim about a term, and the CIE's vocabulary answers it: a
*colour temperature* is the temperature of the Planckian radiator whose radiation has the
stimulus's chromaticity, and a source off the Planckian locus has a *correlated* colour temperature,
a different term neither T.4 nor T.801 uses. So `CT` names one spectrum. `pdf_colour::planckian`
sums Planck's law with `c₂ = hc/k` from the 2019 SI against the 1931 functions, compiled in from the
same CSV and read on the first temperature asked for. Its radiator at 2856 K is illuminant A's white
within 2 × 10⁻⁴ — illuminant A being a Planckian radiator at that temperature, by the formula the
CIE's dataset record names (ISO/CIE 11664-2 equation 1, not held, so the residue is not explained
here but bounded). The cost, stated: a producer meaning a daylight phase by `CT 6500` gets the radiator's
white, X 0.969 and Z 1.122 where D65's are 0.9505 and 1.0890; the file said colour temperature, and
D65 has its own code.

## 4. What is left, and the corpus

Four bytes neither table codes name no white point, so §7.4.9's own fallback stands for them; that
is the file's silence, not an unheld text. The row stays `partial` on e-sRGB and e-sYCC (PIMA 7667)
and CIE Jab (CIE 131) alone. The census's two witnesses (`PDFBOX-3599-0.pdf`,
`poppler-LINK-613-0.pdf`, the Altona Technical 2 page) state D65 in their `colr` boxes and a
`/ColorSpace [/Lab << /WhitePoint [.964203 1 .824905] >>]` over them, so §7.4.9 sets the boxes aside
and they draw as before — rendered and looked at; the Bradford step from that white to the tree's
D50 moves no level. A redaction re-encoding a JPX CIE Lab image now writes the illuminant's white
into the `Lab` array it states, where it wrote D50.
