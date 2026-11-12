# 2828 — An `Identity` font's machine face goes in as drawn, `CFF ` faces included

Session 2005, a single archive round.
Status: accepted and built. In `pdf-transform`: `archive/fonts/machine_cid.rs` (`shown` reads
`Identity-H` and `Identity-V` by Table 116's rule; `drawn_face` admits a `CFF ` face; `compact`;
`Written` carries its Table 124 row).
Tests: `crates/pdf-transform/tests/archive/machine_cid.rs`
`an_identity_font_drawn_from_a_machines_cff_face_is_embedded_with_that_face`; `rekeyed.rs`'s
`an_identity_font_is_a_loss_nobody_authorised` no longer asserts `machine-face`.
Context: ADR 2807 (the route and its proof), ADR 2378 (re-keying, whose `CFF ` branch
`pdf_font::embed::renumbered` this shares), ADR 2829 (what `as-shown` admits of it), A832.
Clauses: ISO 32000-2 §9.7.4.2, §9.7.5.2 Table 116, Table 117, Table 124, Table 125, §9.9.1,
§9.9.2; ISO 19005-2 sections 6.2.11.4.1, 6.2.11.5, 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1,
6.2.10.5, 6.2.10.9.

## Context

ADR 2807 built the route for a composite font's machine face and named four `Adobe-Identity`
documents of the pdf.js corpus as waiting on Q832 alone. A probe showed the route took none of
them, for two reasons. **`shown` asked `pdf_font::predefined::cmap` for the font's `CMap`, which
carries Adobe's files and not `Identity-H`**: Table 116 states that one by a rule ("maps 2-byte
character codes ranging from 0 to 65,535 to the same 2-byte CID value"), so no `Identity` font
ever reached the proof. And **the window drew three of them from URW faces** (`NimbusSans-Regular`,
`NimbusSans-Bold`, `NimbusRoman-Regular`, `OS/2` `fsType` 0x0004, preview-and-print embedding,
subsetting not forbidden), which are OpenType with `CFF ` outlines, and the route took `glyf` alone.

## Decision

1. **`Identity-H` and `Identity-V` are read by Table 116's rule** (`CMap::identity`,
   `identity_vertical`), as the window reads them; every other name is still the predefined file.
2. **A `CFF ` face goes in as a bare CID-keyed subset under `/FontFile3` `/Subtype
   /CIDFontType0C`, the descendant restated as `CIDFontType0`** (Table 124, Table 117), written by
   `pdf_font::embed::renumbered`, whose charset names the glyph the window drew for each shown CID
   by that CID (§9.7.4.2); a glyph two CIDs show is written once under each. No `/CIDToGIDMap`:
   Table 117 gives it to a Type 2 `CIDFont` alone, so a descendant stating one keeps its old route.
3. **Advances restated where they disagree with `/W`** and proved (§9.7.4.3, ISO 19005-2 section
   6.2.11.5); a bare `CFF ` states no vertical metrics.
4. **A CID the window drew nothing for is not this branch's**: the charset could only send it to
   `.notdef`, which ISO 19005-2 section 6.2.11.8 forbids; ADR 2824's empty glyph is a `glyf` writer.
   A descendant stating no descriptor is not either: the descriptor is measured on an `sfnt`.
5. **The same proof**: the font as written, loaded by the window's reader, draws every shown code's
   outline once wound one way and its metrics bit for bit, or nothing is written.

## Consequences

`issue20065`, `issue2840` and `issue9084` take the route at both parts, losing
`glyphs-from-to-unicode` alone, 0 px from their sources at 72 dpi, and so does `issue5244`, whose
`/ToUnicode` names a character no shipped face draws and the machine's face does. `issue11651` does not: its
`/FontFile2` is damaged, so the window draws nothing for it from metrics alone (ADR 1094) and no
face was drawn to embed. `as-if-printed` and `state-every-loss` keep the shipped face (ADR 2774).
