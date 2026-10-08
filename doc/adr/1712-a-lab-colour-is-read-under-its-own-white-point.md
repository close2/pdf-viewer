# 1712 — A `Lab` colour is read under its own white point

Session 1440. Status: **accepted** and **built**. Amends ADR 0012 in one respect: `Lab` now takes
the adaptation stage ADR 0012 gave `CalGray` and `CalRGB`, where it took none. Context: ISO 32000-2
§8.6.5.4 and Table 64; §8.6.5.3 for what a white point is; ICC.1:2022 for the connection space's
illuminant. Code: `crates/pdf-colour/src/colour.rs` (`ColourSpace::Lab`, the `Lab` arm of
`parse_at`, `lab`, `lab_xyz`, `cie_xyz_at`, `white_point`), one hunk of
`crates/pdf-model/src/content/transparency.rs` (`space_identity`). Row: §8.6.5.4.

## 1. What the tree did, and the premise that held

`ColourSpace::Lab` carried a `Range` and nothing else. The parser read no `/WhitePoint` and no
`/BlackPoint`, and `lab()` computed every colour's XYZ as `D50 × g(·)` under a comment calling D50
"PDF's default white point for Lab". Table 64 states no default: `/WhitePoint` is **(Required)**,
and its row sends the reader to §8.6.5.3 for what the entry means. The ledger row said
"`/WhitePoint`, `/BlackPoint` and `/Range` are read", and two of the three were not. The brief's
premise held in the text, the code and the note; the one inaccuracy beside it was a test comment in
`pdf-model/src/soft_mask.rs` repeating the claim, corrected with the code.

## 2. The reading

§8.6.5.4's second transformation stage is `X = X_W × g(L)`, `Y = Y_W × g(M)`, `Z = Z_W × g(N)`
(the markdown conversion drops the formulae; `pdftotext -layout` over the held PDF gives them). So
a colour's XYZ is a multiple of the space's own white, exactly as a `CalGray`'s is under §8.6.5.2,
and §10.3.1's ICC-based conversion maps that white onto the connection space's D50. That is ADR
0012's stage 2, Bradford, and `Lab` now takes it through `cie_to_srgb(xyz, white)` and, for a
conversion into another CIE-based space, `adapt(xyz, white, D50)` — one route, as ADR 0012 requires.
A consequence worth stating because it is easy to mistake for an error: the space's own white is
the display's white and its neutrals stay neutral, but a chromatic colour is a *different colour*
under D65 than the same three numbers under D50 — 4 to 6 levels of 255 on the fixture's colours —
because Bradford scales cone responses, not X, Y and Z.

## 3. What a dictionary without the entry is read as

D50, as for the Cal spaces: the connection space's own white, so the adaptation vanishes and the
`Range` the file did state does what it says. It is the same substitute `white_point` already
supplied for Tables 62 and 63, for the same reason, and a file omitting a required entry is not one
whose intent can be recovered. `/BlackPoint` is read, defaulting to Table 64's `[0 0 0]`, and is not
applied — ADR 0012's argument, unchanged.

## 4. Evidence

`a_lab_space_is_read_under_its_own_white_point` runs §8.6.5.4's EXAMPLE — `[/Lab << /WhitePoint
[0.9505 1.00 1.0890] /Range [-128 127 -128 127] >>]` — through the parser and the conversion: the
white, the default black and the range are carried; three neutrals are neutral and equal to their
D50 reading; two chromatic colours differ from theirs by more than a level; the XYZ handed to
another space is the adapted one. Planting the old D50-only arithmetic fails it.
`cargo run --release -p pdf-colour --example lab_white_point_census` names the pdf.js corpus
documents that state a `Lab` space, opening an encrypted one with the password the corpus gates use
and naming any it cannot open. Its first run skipped encrypted files in silence and found no
non-D50 white; the CPU arm then moved one page the census had not named, `bug1782186.pdf`, encrypted,
whose `Separation` strokes through a `Lab` alternate under the EXAMPLE's own D65 white — trap 13 with
the corpus as the plant. That stroke, `[26.87 2.07 -4.39]`, is (64, 63, 70) under both readings, so
the page moves below a level at the stroke's anti-aliased edge and is drawn as §8.6.5.4 states;
rendered at 4x and looked at. Two space identities in a transparency group compare their white
points as well as their ranges, since two `Lab` spaces differing only in white are two spaces.
