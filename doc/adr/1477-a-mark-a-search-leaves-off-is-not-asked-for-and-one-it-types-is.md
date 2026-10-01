# 1477 — A mark a search leaves off is not asked for, and one it types is

Session 1321. Status: **accepted**.
Context: `crates/viewer-core/src/select.rs` (`find`, `spellings`, `matches_at`, `Cursor`,
`compared_as`, `axes`), `crates/pdf-font/src/shaping/fold.rs` (`decompose`, `mark_class`),
`crates/pdf-font/build.rs` (`CANONICAL_DECOMPOSITIONS`, `COMBINING_MARKS`). Amends ADR 1465, which
left both "diacritic folding" and "a glyph a text matrix mirrored" open.
Clauses: ISO 32000-2 §9.10.1, §9.10.2, §9.4.4.

## 1. What the three windows did

All three hand the needle to `viewer_core::select::find`, so they did one thing: compare lowered
characters one for one, presentation forms folded (ADR 1465). "cafe" did not find "café"; "café"
typed precomposed did not find `e` + U+0301 on a page; "كتب" did not find "كَتَبَ", nor "שלום"
"שָׁלוֹם". A Latin search for "é" did not find "e" — the stricter direction already held, by accident.

## 2. What the standard says, and where it stops

§9.10.1 places searching under text extraction: processors "need to determine the information
content of text -that is, its meaning according to some standard character identification as
opposed to its rendered appearance. This need arises during operations such as searching, indexing,
and exporting of text to other file formats", and names Unicode as "a suitable scheme for
representing the information content of text", referring to the Unicode Standard. The Unicode
Standard (section 3.7) makes canonically equivalent sequences the same text. So comparing both sides
by canonical decomposition is the clause's own handoff carried through, not a choice.

Whether a mark the reader did not type may be skipped is not in either standard. It is a choice,
recorded as one.

## 3. The rule, one for every script

- **Canonical decomposition on both sides**: `UnicodeData.txt`'s untagged field 5 expanded to its
  fixed point, plus the Hangul arithmetic of Unicode section 3.12; tagged (compatibility)
  decompositions are not used, because they change what the text says (only the presentation forms
  ADR 1465 folds are an exception).
- **A mark** is general category `Mn` with a canonical combining class other than zero: Arabic
  harakat U+064B–U+0652, U+0670 and the Quranic signs, Hebrew points U+05B0–U+05C7, Latin, Greek
  and Cyrillic combining accents. Every Arabic and Hebrew `Mn` in that range has a non-zero class.
- **Where either side has a run of marks**, both runs are sorted by class (canonical ordering) and
  the needle's must appear in order inside the page's. A mark the needle leaves off is not asked
  for; a mark it states must be printed. "كتب" finds "كَتَبَ", "cafe" finds "café"; "كَتَبَ" does not
  find "كتب", "café" does not find "cafe", and "كُتب" does not find "كَتَبَ".
- A match never starts on a mark the needle does not state, and it takes in the marks after its
  last letter, so the highlight covers what is printed.

**Why the class as well as the category.** `Mn` of class zero is a Devanagari or Thai vowel sign
(U+0941 DEVANAGARI VOWEL SIGN U, U+0E34 THAI CHARACTER SARA I): it spells the syllable, it is
never left off in writing, and skipping it would make "कल" find "कुल". The class is what Unicode
uses to say a mark attaches rather than spells, and it is the same field canonical ordering needs.
**The cost, written down:** Thai tone marks (class 107) and the Devanagari nukta and virama (7, 9)
are skippable under this rule; a reader who types them gets an exact match.

**What is not done.** The Unicode Collation Algorithm's tailorings, compatibility folding
(full-width forms, ligatures outside the presentation blocks), and any language's own equivalences
(Arabic alef variants, final sigma) are decisions about a language, not about a page.

## 4. A mirrored glyph reads its own way

`Order` decides a right-to-left run's stored order by whether each glyph lands *right* of the one
before. ADR 1465 took *right* as a fixed quarter turn from the glyph's ascent side. §9.4.4's text
rendering matrix may have a negative determinant (`-1 0 0 1 x y Tm`, a negative `Tz`, a mirroring
`cm`); the glyph's box then extends the other way from its origin and the producer's +x points the
other way across the display. `select::axes` now takes *right* as the quarter turn of *up* on the
side the box's own base points to — the determinant's sign read off the box — which is also
independent of the display space's handedness. A box of no width is read unmirrored.

Under a mirroring `Tm` (not `cm`) the readback has a second defect that is not this crate's:
`pdf-model`'s `separate_text` measures the gap to the next glyph along user-space x, so a `TJ` in
reading order reads back with a space between glyphs and the word is not found there. Left to
`pdf-model` (`doc/todo/27`).

## 5. Evidence

`select.rs`: four tests (marks both ways, Latin and canonical equivalence; canonical ordering and the
start rule; a class-zero vowel sign; a mirrored line), the mirror test failing under the old turn.
`fold.rs`: `decompose` and `mark_class` against `UnicodeData.txt`'s own lines. `headless.rs`: a
Type 3 page with fathas of no width (bare found on both lines, vowelled on one, dammas on none) and
1315's three-ways page under `-1 0 0 1 200 0 cm` (3 found; 0 under the old turn). Driven:
`25-find-vowelled` and `25-find-other-mark` in all three windows. `launch_path` unchanged: the two
tables are pointer-free `static`s nothing on the launch path reads.
