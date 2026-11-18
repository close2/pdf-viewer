# 3067 — A substituted composite font borrows, per character, the face that draws it

Session 2091. Status: **accepted.** Context: ADR 3051 §2 (the defect), ADR 1441 (style ranking), ADR 1154 (one
matcher), ADR 3066 (the coverage index), `crates/pdf-font/src/loading.rs` (`Borrowing`),
`crates/pdf-font/src/loading/substitution.rs`, `crates/pdf-font/tests/borrowed_face.rs`. Clauses: ISO 32000-2
§9.7.4.2, §9.7.5, §9.10.2.

## Context

A composite font whose program the document did not embed is reachable only by what its codes mean: §9.7.4.2
says CIDs "shall not participate in glyph selection", and §9.10.2 turns a code into a character by `/ToUnicode`
or by the collection's own table. `composite_substitute` chose the face for the character collection's script
sample — あ for `Adobe-Japan1` — and drew every character from it. `issue15053.pdf` shows "This Message can be
toggled." and `mixedfonts.pdf` the Latin alphabet and digits through `Adobe-Japan1` fonts and `UniJIS-UTF16-H`;
Droid Sans Fallback maps あ and no Basic Latin letter, so both pages drew none of their 29 and 74 characters while
DejaVu, Nimbus and Noto Mono, each of which maps them, were on the machine.

§9.10.2 makes the substitute a processor's choice and the characters the file's. A face that draws none of the
characters a page shows is not a substitute for them, so the choice by the script alone answers a question the
clause does not ask.

## Decision

**The script chooses the font's face, and a character that face does not map is asked for on its own.** Each such
character goes to `substitute::installed_covering_styled` with the font's own request and style and itself as the
only character wanted — the same walk, in the same process or across the same port (ADR 1154) — and the face that
answers draws it, as `Face::Borrowed`. A borrowed face reads §9.7.5.1's vertical forms from its own tables
(`Downward::for_face`). `uncovered_character` asks the same question, so a character is reported as uncovered only
where no face on the machine maps it.

Why not choose the font's face by the characters the page shows: a `LoadedFont` outlives a page and is shared by
every page that uses the font (ADR 0710), so the characters of the page that loaded it are not the font's, and a
face chosen from them would depend on which page was drawn first. A per-character answer is a function of the
character and the machine alone, so pages drawn in any order, on any thread, draw alike. Why not take the sample
and the shown characters together as one set: a set that no single face maps (kana and a Latin letter on a machine
whose CJK face lacks Latin) would then find nothing, which is the refusal this replaces.

The answers are memoised per character, and a face is held once whatever number of characters it answers; the
port's own memo keeps one copy of a program that answers several keys.

## Consequences

- On a machine whose CJK face lacks Latin, a Latin run in a CJK font is drawn from the family's Latin face. On this
  machine, with Noto CJK offered (ADR 3066), the two corpus pages' faces map their Latin themselves; the borrowing
  is what holds where they do not.
- Not taken: a font whose sample no face maps is still refused as a whole (`FontError::NoSubstitute`) rather than
  drawn from a face chosen by family with its characters borrowed. That widens what a Latin-only machine draws
  and is the next step on this reading.
