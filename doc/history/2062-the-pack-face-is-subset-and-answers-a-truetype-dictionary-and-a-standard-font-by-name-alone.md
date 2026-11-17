# 2062 — The pack face is subset, and answers a TrueType dictionary and a standard font by name alone

Session 2062, a PDF/A round in the audit worktree. What round 2060 left of the Latin-Greek-Cyrillic pack (ADR 2934):
rows 9.9.2, 9.6.3 and 9.9; ADRs 2938 and 2939.

**The slice.** The brief's three items in order: subset the pack's face, a `TrueType` dictionary only the pack draws,
and the descriptor-less route of the standard fourteen.

**The premise.** All three unbuilt (ADR 2934 section 3, record 2060's "Left"). One part had moved: the
`Adobe-Identity` route already subset the pack face (`issue5244`'s `DVHNUQ+NotoSerif-Bold`, 13 213 bytes); only the
simple-font descriptor route wrote it whole, because `pdf_font::embed` subset CFF only as CID-keyed programs.

**Built.** `pdf_font::embed::compact_subset` (`embed/type1c.rs`) cuts a bare name-keyed CFF to the shown glyphs and
`.notdef` with a format-0 charset naming each, sharing the CID subsetter's closure, strings and Private DICT writer;
a non-Standard built-in encoding is written whole. `described::pack_face_subset` keeps the glyphs the font as
written reaches, names the cut by §9.9.2 in the program, `/BaseFont` and `/FontName`, and holds it to the whole face
(same copy, same outline, 256 advances). `described::pack_restated` re-files a `TrueType` dictionary as `Type1`
(Table 124 admits a bare CFF under no key of it; §9.6.3's same entries). `described::from_pack` gives one of the
fourteen stated by name alone the pack face, a descriptor measured off it and the laid-out widths. The pack proof now
reads back the twin codes' encoding and laid-out widths with the font. `doc/lgc-pack.md`, state-of-play.

**Conversions.** ADR 2934's fixture under `as-if-printed`: 748 202 / 748 189 bytes (2b / 4) before, 13 070 / 13 057
after, four glyphs a face; `issue5244` 13 213 / 13 200 as before. The fixture as `TrueType` (`TimesNewRoman`,
`CourierNewPS-BoldMT`) and a descriptor-less `Times-Roman` and `Courier-Bold` showing Latin Extended-A, at HEAD a
refusal and a photograph, are set as text; all drawn and looked at. `as-shown` refuses as before (`machine-face`).
veraPDF 1.30.3 passes all twelve outputs. **Plants**: the charset given each glyph its neighbour's name fails both
pack tests; the `TrueType` arm given back Table 124's refusal fails its test; the new route off fails its own.

**Gates.** Over `pdf-font` and `pdf-transform`: fmt exit 0; clippy `-D warnings --all-targets` exit 101 on a
quotation's backticks and an addition, then on `too_many_lines`, then exit 0; nextest 1629 of 1629 passed, and the
four ignored pack tests passed against the built pack; rustdoc `-D warnings --document-private-items` exit 0.
`cd fuzz && cargo check --bins` not owed: no public type or signature changed (two items added). Conformance after
`git add -N`: exit 0.

**Left.** `/Delta` (Q888); an italic fixed-pitch request; a font outside the fourteen with no descriptor, and the
fourteen the conversion defines for a name with no resource (ADR 2522); a code the standard metrics give no width
keeps the window's zero advance.
