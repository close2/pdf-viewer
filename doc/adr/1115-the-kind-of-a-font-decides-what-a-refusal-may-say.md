# 1115 — The kind of a font decides what a refusal may say

Status: accepted. Session 1101.
Context: `crates/pdf-transform/src/archive/to_unicode.rs` (`derive_one`, `Naming`,
`NOT_A_SIMPLE_FONT`, `NO_GLYPH_NAME`), `crates/pdf-archive/src/table/fonts.rs`
(`type3_encoding`, now public), `crates/pdf-transform/tests/archive.rs`.
Builds: nothing new. `doc/rfc/0007` section 2's vocabulary is unchanged and no remedy is added.
Clauses: ISO 19005-2 section 6.2.11.7.2; ISO 32000-2 §9.6.4, §9.6.5.1, §9.7.4.2, §9.10.2, §9.10.3.

## 1. The question this round was given, and the answer the tree already held

The brief asked whether writing a `/ToUnicode` `CMap` the standard's own method derives is
`Mechanical`, `Authorised`, or an amendment for the owner. **It is `Mechanical`, it was decided in
session 951, and it is built**: `decision.rs` answers both Unicode requirements with
`Answer::Mechanical(Rewrite::ToUnicode)` and `to_unicode.rs` is §9.10.2's second method applied to
the codes a content stream showed. Nothing here re-opens that. The brief's proposed fixture — a
font with `/WinAnsiEncoding` and no `/ToUnicode` — would have proved nothing either: section
6.2.11.7.2's **first** exemption excuses exactly that font, so the requirement never binds it.

So the round measured instead. All eight PDF/A-2u refusals are the two Unicode rows (five
`fonts/to-unicode-present`, three `fonts/to-unicode-values-are-usable`), every one of them a
document whose codes no method of §9.10.2 answers for — which is the clause's own outcome, stated
in its closing sentence: where the three methods fail there is no way to determine what a code
represents, and what is left is a character of the processor's *choosing*. The converter declines
that permission (`doc/questions/A48`), so the refusals stand. **Two of them named the wrong
reason**, and that is what this ADR is about. ISO 19005-4's equivalents refuse nothing at 4f or 4e:
those targets refuse on embedded files, annotation appearances and a 3D stream.

## 2. A refusal about a construction the font does not have

`derive_one` decided a font was composite by looking at how wide a *code* came out —
`u8::try_from(code.value())`. A Type 0 font whose text shows small CIDs has codes that fit in a
byte, so it walked past that guard and failed later at `selected_glyph_name`, which returns `None`
for a composite font by design. The document was then told that one of its codes "selects its glyph
without a name", about a font that selects glyphs by CID and has no names to miss.
`PDF_A-2u/…6-2-11-7-2-t01-fail-d.pdf` is the witness: an `Identity-H` `CIDFontType2`.

The kind is now asked of the font — `/Subtype /Type0` — before any code is decoded, and the width
check stays behind it as defence with its unreachability written down. One reason, two detections,
because both are the same fact about the font.

## 3. A font this program cannot load is not a font the file failed to describe

`t01-fail-c.pdf` is a Type 3 font whose `/Differences` names `/square` and `/triangle`. It was
refused with "this font is one this program could not load or had to substitute for" — true of
`pdf_font`, which refuses a Type 3 outright because §9.6.4 makes its glyphs `/CharProcs` content
streams, and silent about the file.

**The clause has a name in the file, and the name is the point.** §9.6.4 requires a Type 3 font to
state an `/Encoding` dictionary whose `/Differences` array describes its whole encoding, and that
array *is* the glyph selection §9.10.2's second method asks about — which is why ISO 19005-2
section 6.2.11.7.2's second exemption names Type 3 beside Type 1. So `Naming` holds the two ways a
simple font reaches a glyph name, and the Type 3 arm reads that array through
`pdf_archive::type3_encoding`, made public for the reason a second reading would be a defect: the
validator judges the exemption by those names and the converter derives from them, and two readings
could exempt a font in one crate and refuse it in the other.

That corrects the sentence — `t01-fail-c` now refuses because `/square` is in neither list — and it
adds a conversion this converter could not do: a Type 3 font whose names *are* listed and whose
producer left a placeholder in its `/ToUnicode` derives its `CMap` like any other simple font. No
corpus document is of that shape, which is why the fixture is built from the clause.

## 4. What was not taken

**§9.10.2's closing permission.** `LoadedFont::text_from_program` takes it for *extraction* — the
`post` table and an inverted Unicode `cmap` — and the converter still does not, because the clause
calls it a choice rather than a method and a file that leaves this verb wears a claim. The five
symbolic-TrueType refusals are therefore refusals on purpose, and a reader who wants the text has
`-2b` or `-4`, where the rule is absent and a recommendation respectively.

**Nothing in the default run moved.** Eight documents refused before and eight after; what changed
is that each now names the absence in the file rather than one in this program, which is
`doc/rfc/0007` section 0's whole claim about what a refusal is for.
