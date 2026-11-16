# 2870 — A program present and unreadable is drawn from a substitute face

Session 2027, a font round in the audit worktree, on the owner's answer `doc/questions/A837`.
Status: accepted and built. In `pdf-font`: `LoadedFont::substitute_for_unreadable`,
`without_program`, `NO_CODE_REACHES_A_GLYPH`. In `pdf-model`: `content/font.rs`'s `load_font`
and `kept`.
Tests: `crates/pdf-model/tests/unreadable_program_substitute.rs` (all six),
`crates/pdf-model/tests/refused_font_metrics.rs` (its fixture now draws through the substitute;
the two arms about Table 120 read `LoadedFont::metrics_only` itself).
Context: ADR 0007 (a font naming no program is drawn from a substitute), ADR 1094 (a refused
program keeps the document's metrics), ADR 2619 (a program that stops short is not embedded), ADR
2636 (and one that is no program in its format), ADR 2830 (the archive followed the window's
nothing), ADR 2871 (the archive follows this).
Clauses: ISO 32000-2 §9.5, §9.4.4, §9.7.4.2, §9.7.5.2, §9.8.1, §9.9.

## Context

A font whose embedded program is present and unreadable — its data does not decode, stops short of
the extent §9.9's Table 125 states, fails its checksum, or is not a program in the format it is
filed under — was drawn from the dictionary's widths alone and marked nothing (ADR 1094), while a
font naming no program at all is drawn from a substitute face (ADR 0007). The same text was blank
when the producer embedded a broken program and readable when it embedded none: `issue11651`'s
"Hello world Test", the whole of govdocs1 `178360` but its frames, and fonts of `435321` and
`268346`.

§9.5 says a font dictionary carries "information that can be used to provide a substitute when the
font program is not available", and §9.8.1 that the descriptor's metrics "provide information that
enables a PDF processor to synthesise a substitute font or select a similar font when the font
program is unavailable". (Q837 and ADR 2830 cite the first sentence as §9.1's and quote the second
as "enable"; it is §9.5's, and "enables".) Both permit a substitute and neither requires one. Q837 asked the owner; A837 answered "agreed" to
drawing one.

## Decision

1. **The cause is `FontError::Malformed`**, the variant whose subject is a program the descriptor
   supplied and this reader read and rejected — the same reading `pdf_archive::program_unparsed`
   makes, so the window, the checker and the converter share one definition of an unreadable
   program. One sentence of that variant is excluded by name: a program that parsed and whose
   encoding reaches none of its glyphs (`NO_CODE_REACHES_A_GLYPH`) is readable, and what it draws
   is the producer's.
2. **The substitute is the font with no program.** `without_program` copies the descriptor — a Type
   0 font's descendant's — without Table 120's `/FontFile`, `/FontFile2` and `/FontFile3`, every
   other entry the producer's, and `LoadedFont::load` reads that copy exactly as it reads a font
   naming none: widths, flags and metrics, encoding and `/ToUnicode` decide the face and reach its
   glyphs. No second substitution route exists to disagree with the first.
3. **ADR 1094's "metrics only" is narrowed to the refusals no substitute recovers**: every refusal
   other than decision 1's, and a decision 1 font whose substitute is itself refused — a composite
   font with neither a `/ToUnicode` nor a character collection this reader carries, which §9.7.4.2
   ("CIDs shall not participate in glyph selection") leaves no route to a face. Its inner refusal,
   `collection_gap`'s §9.7.5.2 sentence, is discarded, because that sentence is about a file that
   embedded nothing and this file embedded something; the caller reports the program's refusal.
4. **§9.7.5.2's forbidden combination is unchanged**: `/Identity-H` over a descendant naming no
   program never reaches this path, and is refused and drawn as before.
5. **The refusal is still said.** The page notes `Unsupported::Font` with the program's refusal and
   "drawn from a substitute face instead", and stays incomplete: the producer's shapes are not what
   it shows (trap 5). The substitute is kept in the page's cache and never in the cache that
   outlives the page, whose hits report nothing — the same rule as for a metrics-only font.
6. **Text readback follows the marks.** ADR 1094's "no readback" was the sentence "this font marked
   no part of the page"; a substitute marks it, so its codes read back through §9.10.2 as any
   substituted font's do.

## Consequences

The witnesses draw their text in the window: every page of the four on which a substitute stands in
reads text back (`the_witnesses_draw_their_text`). A simple font that states no `/Widths` advances by
the substitute face's widths, as a font naming no program does (ADR 0007's route, not changed here),
where the metrics-only reading advanced by `/MissingWidth`. `raster_golden` and the corpus walk
move on the pages that show such a font; the round's record lists them.
