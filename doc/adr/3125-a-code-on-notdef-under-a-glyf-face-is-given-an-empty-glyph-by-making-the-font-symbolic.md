# 3125 — A code on `.notdef` under a `glyf` face is given an empty glyph by making the font symbolic

Session 2106, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R08
documents. Status: accepted and built. In `pdf-transform`: `archive/fonts/made_symbolic.rs`
(`restated`, `flags`, `to_unicode`, `proved`), `archive/fonts/described.rs` (`with_empty_glyphs`,
`describe`), `archive/fonts.rs` (the descriptor route, `gain`), `archive/rewrite/values.rs`
(`describe_font` removes a key given a null value). Tests:
`crates/pdf-transform/tests/archive/descriptor_empty_glyphs.rs`. Context: ADR 2458 (the empty
glyph and its refusal "on the fence's side" under a `glyf` face, item 3), ADR 2520 (the symbolic
restatement of an embedded TrueType program, whose `cmap` writer and proof this reuses), ADR 2769
and A550 (a modified Liberation face is renamed), `doc/questions/A48`, A694 item 3.
Clauses: ISO 32000-2 §9.6.5.4, §9.6.6 Table 121, §9.10.2, §9.10.3, §7.3.9; ISO 19005-2 sections
6.2.11.4.1, 6.2.11.6, 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1, 6.2.10.6, 6.2.10.9.

## Context

Round 2102's census ranked R08 third: under `as-if-printed`, five government documents
(`150277`, `210260`, `219789`, `392154`, `455746`) stopped at "a page that would be photographed is
too large": a font they do not embed shows, on most pages, a code `WinAnsiEncoding` names nothing
for (a tab, a carriage return), so every such page was deferred to the photograph, and dozens of
pages pass its bounds (ADR 2517). The code reaches `.notdef` in any program under the encoding, so
no face resolves it, and ADR 2458 refused it under a `glyf` face because a non-symbolic `TrueType`
font reaches a glyph only by a name the Adobe Glyph List gives a character, which A48 forbids.

## Decision

1. **The font is made the symbolic font §9.6.5.4 describes.** Its second route — "When the font has
   no Encoding entry, or the font descriptor's Symbolic flag is set" — reaches a glyph through a
   (3, 0) subtable at 0xF000 plus the code, and states no character. The descriptor states the
   Symbolic flag and not the Nonsymbolic one (Table 121), the dictionary states no `/Encoding`
   (section 6.2.11.6), and the face carries ADR 2520's (1, 0) and (3, 0) subtables: each code the
   face reached under the file's encoding on that glyph, and each code on `.notdef` on a glyph
   appended with no outline at the width the dictionary states (`notdef_glyph::NewOutline::Nothing`,
   one per width). The modified Liberation face is renamed (A550).
2. **The characters the encoding named are kept.** Where the file states no `/ToUnicode`, one is
   derived from its encoding by §9.10.2's glyph-name method, exactly as `archive::to_unicode` does
   for Level U; a code on `.notdef` has no name and no entry, so the empty glyph states no
   character — A48's condition, met in the CMap as ADR 2458 met it by a name outside the list.
   This reader's extraction answers such a code with the Private Use value U+F000 plus the code,
   read off the (3, 0) subtable as it does for every symbolic font ADR 2520 writes: a value
   Unicode assigns no meaning, which is what the tests hold it to.
3. **Proved before it is written**: the font is loaded from the dictionary, descriptor and program
   as written and every one of the 256 codes must reach the glyph it reached or was given, none by
   a reader-chosen mapping; every shown code keeps its advance, every given glyph draws nothing,
   and the font is drawn from its own program.
4. **A key removed is a null value** among a font's gained entries, which §7.3.9 makes the same as
   no entry; the two places that write gained entries remove the key.
5. Not offered at Levels U and A (ADR 2458 item 4), and on the descriptor route the refusal there
   is now the target's (`NotThisTarget`), naming 2b and 4.

## Consequences

Three of the five convert at 2b and four at part 4 under `as-if-printed` (ADR 3126 holds the other
half); `150277` and `219789` stop at 2b at section 6.1.13's values, and `219789` keeps a deferred
font. veraPDF 1.30.3 passes all seven outputs. Drawn at 50 dpi against the source, `392154` is
identical but for 21 pixels one level apart; the other differences are pages photographed and the
`machine-face` loss `as-if-printed` authorises and the report names.
