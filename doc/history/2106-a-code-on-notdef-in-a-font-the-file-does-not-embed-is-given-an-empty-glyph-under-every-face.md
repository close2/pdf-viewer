# 2106 — A code on .notdef in a font the file does not embed is given an empty glyph under every face

Session 2106, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R03 and
R08 rows. ADRs 3125, 3126 and 3127; ledger rows 9.6.5.4, 9.10.2 and 7.3.9 (`implemented`, before
and after); no question.

**Found.** R08's "too large to be photographed" stood over `150277`, `210260`, `219789`, `392154`
and `455746`: an unembedded font shows, on most pages, a code `WinAnsiEncoding` names nothing for,
refused by `NOTDEF_BESIDE_A_DESCRIPTOR` or ADR 2458's `glyf` fence, so every such page was deferred
to the photograph and dozens passed its bounds. The remedy is not a larger bound.

**Built, ADR 3125.** Under a `glyf` face the font is made §9.6.5.4's symbolic font: Symbolic flag,
no `/Encoding`, ADR 2520's (1, 0) and (3, 0) `cmap` mapping every code to the glyph this reader
drew and each code on `.notdef` to an appended glyph with no outline at its width, a `/ToUnicode`
derived from the file's encoding (none for those codes), the Liberation face renamed (A550), proved
as written. On both routes; a null gained entry now removes its key.

**Built, ADR 3126.** The descriptor route writes the empty glyph too: the symbolic font under a
shipped or machine's `glyf` face, a named glyph and a restated encoding under a shipped CFF face;
Levels U and A refuse as the target's. `same_glyph` admits an empty glyph against a fitted face.

**Built, ADR 3127.** A font named in no UTF-8, the name its embedded program's own, has the pages
whose resources reach it (through `/DescendantFonts` and `/FontDescriptor`) photographed under
`as-if-printed`; `issue5010` converts at 2b.

**Census.** Re-taken by section 1's commands (without the pack, `issue15053` stops). Against 2102's,
`as-if-printed` converts 2 more pdf.js and 4 more corpora documents at 2b, 2 and 5 at part 4;
`as-shown` 1 and 3 more at each part. Refused or declined at every `as-if-printed` target: 35 to
28. R08 falls from 33 to 23 documents. veraPDF 1.30.3 passes the eight outputs of the freed
documents and the eight of four fixtures mirroring the tests; drawn against the source, the
differences are photographed pages and the named `machine-face` loss.

**Gates.** Over `-p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0 (after extracting
`held_to_the_source` for `too_many_lines`); nextest `--lib --bins --tests` 0, 1259 passed, none
expected (new: `descriptor_empty_glyphs.rs`, a GBK-named font in `photographed.rs`); rustdoc 0;
`fuzz` `cargo check --bins` 0 (`Described` gained `MadeSymbolic`); conformance 0.

**Left, in rank order.** Section 6.1.13's values (`150277`, `219789` at 2b) and `219789`'s deferred
font; R03's MediaBox with no area (`boundingBox_invalid`, drawn at `DEFAULT_MEDIA_BOX`); R08's pages
not drawn whole.
