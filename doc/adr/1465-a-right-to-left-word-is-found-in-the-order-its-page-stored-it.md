# 1465 — A right-to-left word is found in the order its page stored it, and a presentation form as its letters

Session 1315. Status: **accepted**.
Context: `crates/viewer-core/src/select.rs` (`find`, `spellings`, `Spelling::admitted`, `Order`,
`compares`), `crates/viewer-core/src/readback.rs`, `crates/viewer-core/src/search.rs`,
`crates/pdf-font/src/shaping/fold.rs` (`fold`, `right_to_left`), `crates/pdf-font/build.rs`
(`PRESENTATION_DECOMPOSITIONS`). Amends ADR 0119's search, which compared character for character.
Clauses: ISO 32000-2 §9.10.2, §9.10.3, §14.8.2.5.1, §14.8.2.5.3.

## 1. What was wrong

ADR 1453 left it: on `ArabicCIDTrueType.pdf` the find bar of all three windows answered "not in
this document" for the phrase every line of the page shows. The readback is
`ﺔﻴﺑﺮﻌﻟا طﻮﻄﳋا عاﻮﻧا` — presentation forms, in display order — and a person types
`انواع الخطوط العربية`. Two separate questions, and the standard answers neither.

## 2. What the standard says

- §9.10.2: "A PDF processor can use these methods, in the priority given, to map a character code
  to a Unicode value." The first is the `/ToUnicode` CMap, and §9.10.3 makes it map codes "to
  Unicode character sequences expressed in UTF-16BE encoding" — any sequence, presentation forms
  included. So the readback holding U+FExx is the file's statement, correctly read, and it stays.
- §14.8.2.5.1: "Page content order shall be defined by the sequencing of graphics objects within a
  page's content stream." The readback is in that order. Logical content order is the structure
  tree's, so an untagged page states none.
- §14.8.2.5.3 NOTE 1 says why a right-to-left line is often stored backwards: showing it with
  left-to-right glyph metrics "requires either positioning each glyph individually (which is
  tedious and costly) or representing text with show strings", and those strings are the ones
  "whose character codes are given in reverse order". `ReversedChars` is the one marker the
  standard gives for the second; a page without it says nothing about order in words.
- Annex D is Latin, PDFDocEncoding, Symbol and ZapfDingbats; it says nothing about Arabic forms.

## 3. The decisions, each a choice

- **Fold for matching, never rewrite.** Both sides of a comparison read a presentation form as the
  characters `UnicodeData.txt` decomposes it to — Alphabetic Presentation Forms and Arabic
  Presentation Forms-A and -B, under any tag, expanded through the table — and then lower-case.
  The readback, the selection and the copy are what §9.10.2 produced. A form decomposing to a space
  and a mark (the isolated harakat) is not folded: a space is a word break to the search.
- **The needle is spelled the way a page could store it** (Unicode Standard Annex #9's rule L2
  applied to the *query*, not the page): as typed, and as displayed under a left-to-right and a
  right-to-left paragraph — both, because the line's direction is not in an untagged page. Digits
  and Latin inside an Arabic word keep reading left to right, which is L2's result and not a rule
  of ours. Reordering the readback instead was rejected: it needs a paragraph direction too, and
  it would make the match's range something other than a contiguous run of stored characters.
- **Each spelling is admitted only where the glyphs say the page stored that order.** A run of
  right-to-left glyphs on one line votes pair by pair on whether each lands right of the one before
  (display order) or left (reading order, or `ReversedChars` read back). *Right* is a quarter turn
  from the glyph box's ascent side in the display list's space, which `base_transform` keeps
  right-handed under every `/Rotate`. Without this a two-letter word and its reversal — often both
  words — would each find the other. A run with no vote admits every spelling; a glyph a text
  matrix *mirrors* would vote the wrong way, and no document has asked for it.
- **A ligature is read in its run's stored order**: a lam-alef in a display-order run compares
  alef first.
- **What a person sees selected** decided between the alternatives: a match is a contiguous range
  of the readback, so `quads_for` draws it over exactly the word's glyphs, on the left of each line
  in the fixture, where the page draws it.

## 4. Consequences

The order is computed once per interpreted page from the text layer and kept beside the readback
in the search cache (counted in its budget); a readback with no byte at or above 0xD6 — no
character at or above U+0590 — skips the walk after one pass over its bytes. **The decomposition
table holds no pointer**: a first version was `[(char, &str)]`, whose nine hundred `&str`s are
relocations the dynamic loader applies before `main`, and `launch_path`'s instruction count of an
open moved from inside its band to 1826.25 thousand against a ceiling of 1820 on
`xfa_filled_imm1344e.pdf`; as a pool of letters indexed by `(u16, u8)` it is 1818.73, and that
is the whole difference, since the page walk's prefilter did not move the figure. Left: a line whose
paragraph direction puts a Latin word outside the Arabic run is found under both directions only
because both are tried; marks are compared as stored, so a word typed without its vowels does not
find one printed with them — that is diacritic folding, a decision about a language.
