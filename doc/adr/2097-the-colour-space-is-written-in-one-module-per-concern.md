# 2097 — The colour space is written in one module per concern

Session 1630, slot 8 of batch ninety-eight, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 8, second half. Status: **accepted** and **built** in `crates/pdf-colour/src/colour.rs` and
`colour/`. Context: ADR 2096 (the same round's other half); ADRs 2004 to 2081. Clauses: none; no
behaviour moves.

## 1. What the census found

`colour.rs` was 7 613 lines, and the census's "45 methods in one impl block" was the smaller part of it:
one `impl ColourSpace` of 1 740 lines, and around it 4 000 lines of free types and functions that are
other clauses' — the compositing key, the spot planes, the grey and RGB routes, the presses and the ink
search, the CIE arithmetic — plus 1 460 lines of tests. The module boundary the file lacked was one per
clause for the free items and one per concern for the methods.

## 2. The decision

`colour.rs` stays the root: `ColourSpace`, `Tints`, `Separations`, `Reading`, `MAX_DEPTH`, the
re-exports of every public item and the tests. Fifteen modules:

| module | holds |
|---|---|
| `parsing` | `ColourSpace::parse*`, the families named, the `ICCBased`, `Cal*` and `Indexed` readers |
| `separation` | §8.6.6.4 and §8.6.6.5: `Separation` and `DeviceN`, process and spot components, §10.8.3's simulation |
| `queries` | what a parsed space states: components, ranges, identity, initial colour, ink |
| `to_rgb` | §8.6: components to RGB, and the CIE route |
| `to_cmyk` | §11.7.2: components into a four-component press |
| `compositing` | `Compositing` (what a colour is resolved for) and `Plane` |
| `spots` | §10.8.3: `DeviceSpots`, `SpotSet` and `separation_conversions` |
| `grey_route`, `rgb_route` | §11.3.4 and §11.6.6: the way into and out of a CIE-based blending space |
| `conversion` | `Conversion` and `InkScale` |
| `device_cmyk` | §10.4.2.5: the sixteen corners of the ink cube |
| `press` | §11.4.7 and §11.7.2: `Press`, `Presses`, `PressIdentity` and the process-wide caches |
| `ink_search` | §10.4.2.4 and the press's inverse: the grid, the ladder and the polish |
| `cie` | XYZ, `Lab`, Bradford adaptation and the sRGB transfer |
| `entries` | a colour space dictionary's entries: names, white point, ranges |

Every public item is re-exported from the root under its old name, so `pdf_colour::colour::X` and
`pdf_model::colour::X` did not change. **Visibility is what a sibling reads**: a private item,
method or field a sibling uses is `pub(super)`, and each `pub(super)` was removed one at a time and
kept only where the build failed (47 kept, 3 narrowed back).

## 3. The proof that nothing moved

The multiset of the old file's non-blank lines against the root and the fifteen modules: removed-only
is 37 lines — doc links re-qualified to the module that now holds their target (habit 126), the tests'
`super::cmyk(` and its kin re-pointed to `super::device_cmyk::cmyk(`, and four signatures `rustfmt`
wrapped because `pub(super)` is longer. Added-only is the module docs, the `use` and `mod` lines, the
five `impl ColourSpace {` wrappers and those same lines. `cargo test -p pdf-colour` passes unchanged and
the six corpus arms agree with HEAD's page by page.

## 4. What this costs

A reader looking for `to_cmyk` opens `to_cmyk.rs`, and for the ink search `ink_search.rs`. `colour.rs`
is still 1 900 lines, 1 460 of them tests. Forty-four ledger rows whose `code` named the file now name the
modules that cite their clause (habit 96).
