# 3210 — A font name that is no UTF-8 is written as the name of the program embedded for it

Session 2127, an archive round in the `lane3` worktree. Status: **accepted** and built. Extends ADR
2846 (a composite font's name restated where the conversion embeds the program it names) to the
simple fonts the shipped-face and machine-face routes embed into, and narrows ADR 3127 (a page
reaching a font named in no UTF-8 is photographed under `as-if-printed`). Context: ADR 2520 and
ADR 2744 (R11's restated `cmap`, over the face the substitution embeds), ADR 2939 (a subset named in
both places §9.9.2 names); `crates/pdf-transform/src/archive/fonts/names.rs`
(`embedded_program_name`), `fonts.rs` (`embed_pass`), `fenced.rs` (`bound_name_pages`,
`substituted_parts`), `prepare.rs` (the `cmap` restatement's `refused_anyway`). Clauses: ISO 19005-2
section 6.1.8, ISO 19005-4 section 6.1.7; ISO 32000-2 §9.6.3, §9.8.1 Table 120.

## Context

With ADR 3209 built, `SimFang-variant` and `XiaoBiaoSong` stopped on two rows. Section 6.1.8 failed at
the font's `/BaseFont` (`/#ba#da#cc#e5`, GBK bytes), and the conversion left that name as the
producer wrote it although the program now embedded has a name of its own. And R11's restatement was
never prepared: `refused_anyway` saw a failed row the table answers by no remedy (section 6.1.8 is
the table's fence) and declined to copy any program. Under `as-if-printed` ADR 3127 photographed the
page instead, taking its text.

## Decision

1. **The name goes with the program.** §9.6.3 reads a TrueType font's `/BaseFont` off the program's
   `name` table, and Table 120 makes the descriptor's `/FontName` the same name. Where a simple font
   the file did not embed is given a program and its own `/BaseFont` is no valid UTF-8, both entries
   are written as the program's PostScript name, read from the program as embedded (the `name`
   table, or a bare CFF program's font name); a program stating none a PDF name can carry leaves the
   name, and the row, as they were. A valid name is never touched. The producer's name stays in the
   report and `xmpMM:History` as the font's `requested` name, as ADR 2846 has it.
2. **A restated name refuses nothing.** The `cmap` restatement's `refused_anyway` passes over
   section 6.1.8 where `Substitutes::font_names_restated` holds, so R11 is prepared for these fonts.
3. **Such a font is left to the substitution, not the photograph.** `bound_name_pages` drops from
   its places every simple font the pages render whose descriptor names no program, with its
   descriptor object: the substitution names it by its program and keeps the page's text, and a font
   the substitution refuses is still photographed around where the gate allows, which takes the
   name with it. A row left with no place holds no page, and `decide` answers it by the
   substitution's restatement. A font whose file embeds the program keeps ADR 3127's photograph.

## Consequences

Both documents convert at 2b and 4 under `as-shown` and `as-if-printed` with their text as text and
pass veraPDF 1.30.3 at both parts. A font the pages render, that embeds no program and whose
substitution embeds a program stating no usable PostScript name, is refused where ADR 3127 would
have photographed it; the census is the measure of whether any document is such a font.
