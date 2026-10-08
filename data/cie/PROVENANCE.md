# The CIE's illuminant and observer tables, and where these copies came from

Six tables of the CIE's open-access datasets: the CIE 1931 2° colour-matching functions and the
relative spectral distributions of the standard and typical illuminants ITU-T T.4 Annex E section
E.6.7 codes (T.801 Table M.29 takes the same codes for a JPX CIE Lab box's `IL` field). A white
point is the tristimulus sum of an illuminant's distribution against the three functions, scaled to
`Y` = 1, and `crates/pdf-model/src/jpeg2000.rs`'s `every_white_point_is_the_cie_datas_own_sum`
computes each of `Illuminant::white_point`'s six CIE-derived values from these files at every
wavelength both tables state. Nothing here is read at run time: the values are compiled in, and the
tables are what makes them checkable (ADR 1713).

D65 is drawn from ISO 32000-2 §8.6.5.4's EXAMPLE rather than from its table, and the table is kept
to show the two agree within 2 × 10⁻⁴ (X 0.95047 against 0.9505, Z 1.08883 against 1.0890, the CCIR
value). D50 is ITU-T T.4 section E.6.4's own statement and has no table here.

## Where they came from

Fetched on 2026-10-08 from `https://files.cie.co.at/Publications-datasets/`, each the file its
dataset page at `https://cie.co.at/datatable/…` links, kept byte for byte (four of them with CRLF
line ends, as published). Each dataset's metadata record carries the sha256 below, and each copy
matched it.

| file | dataset | DOI | source the CIE names | step |
|---|---|---|---|---|
| `CIE_xyz_1931_2deg.csv` | CIE 1931 colour-matching functions, 2° observer | 10.25039/CIE.DS.xvudnb9b | CIE 018:2019 Table 6 | 1 nm, 360–830 |
| `CIE_std_illum_A_1nm.csv` | CIE standard illuminant A | 10.25039/CIE.DS.8jsxjrsn | ISO/CIE 11664-2:2022 Table A.1 | 1 nm, 300–830 |
| `CIE_std_illum_D65.csv` | CIE standard illuminant D65 | 10.25039/CIE.DS.hjfjmt59 | ISO/CIE 11664-2:2022 Table B.1 | 1 nm, 300–830 |
| `CIE_illum_C.csv` | CIE illuminant C | 10.25039/CIE.DS.mjdd2enu | CIE 015:2018 Table 5 | 5 nm |
| `CIE_illum_D75.csv` | CIE illuminant D75 | 10.25039/CIE.DS.9fvcmrk4 | CIE 015:2018 Table 5 | 5 nm |
| `CIE_illum_FLs.csv` | illuminants representing typical fluorescent lamps, FL1–FL12 then FL3.1–FL3.15 | 10.25039/CIE.DS.ukaymjdn | CIE 015:2018 Tables 10.1–10.3 | 5 nm, 380–780 |

    sha256  fa663e3535a7e0763a745993a1f0a192eb0275ac46ad2d1befd7626841e713c1  CIE_xyz_1931_2deg.csv
    sha256  61ef23fe146b8b665c74706717ab28cec7db6c9022993490bdc71991f43cb59b  CIE_std_illum_A_1nm.csv
    sha256  e76f210bffff3d552ef7113025da5f325d5dfec200dd4b878b1a2f3a507032cb  CIE_std_illum_D65.csv
    sha256  51c53bf065345fc1fb9d50aff40fb0a2ce370bcf35121a3dc4bafc0888602f0b  CIE_illum_C.csv
    sha256  573a08831f2bcdf18a8e333490049e6d0fabaf1d1c0abbc2a49d6247a939b62d  CIE_illum_D75.csv
    sha256  24303adacbfee5e19b2123ea2c3208a993577a054abaf55ea7e19517f98e4d55  CIE_illum_FLs.csv

## Terms

Every one of the six records states its rights as Creative Commons Attribution-ShareAlike 4.0
International (CC BY-SA 4.0, `https://creativecommons.org/licenses/by-sa/4.0/`), creator and
publisher the International Commission on Illumination (CIE), Vienna. The attribution is this file;
the tables are unaltered, and nothing in the tree adapts them — the six white points compiled into
`jpeg2000.rs` are sums computed from them, and a copy of these files carries the licence with it.
