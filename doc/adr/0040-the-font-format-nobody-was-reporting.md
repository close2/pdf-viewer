# ADR 0040 — The font format nobody was reporting

Status: accepted, 2026-07-30.

## Context

The thirtieth session's first half (ADR 0039) left a page behind: `issue3566.pdf` stopped being
contradicted **without a pixel moving**, because the oracle picks a page's tolerance class from
`has_text`, and `has_text` asked whether we could read any text back. That is not the question.
A page of CJK from a subset with no `/ToUnicode` draws perfectly good glyphs that nothing can
name, and it was being judged by the bound measured on flat fills; `oracle.rs` had said so for
ten sessions and named the fix — "make `has_text` mean 'we drew glyphs' rather than 'we could
name what we drew'".

So this began as an instrument change, and the instrument immediately pointed at a feature.

## What the instrument found

`Interpretation` now carries `glyphs`, a count of glyphs that **marked the page** — filled,
stroked, or run as a Type 3 description, so rendering modes 3 and 7 and a hidden optional-content
layer contribute nothing. `has_text` is `glyphs > 0`.

It moves pages in both directions, and both are corrections:

- **A page of unnameable glyphs is a text page.** 25 pages left `ambiguous`, because a bound wide
  enough for glyph hinting is one two references can agree *inside*. 19 of them agree with us and
  6 do not. A page that was unjudgeable and is now contradicted is not a regression; it is a page
  that was already wrong and could not be said to be.
- **A page of invisible OCR text over a scan is an image page.** It reads back plenty of text and
  marks the page with no glyph at all. `issue11150_reduced.pdf` was one — and what the tighter
  bound then showed was that **we were drawing nothing at all** where four references draw three
  thetas.

That page's font is `/Symbol`, embedded as a `/FontFile`.

## The feature: §9.9's `/FontFile`

`/FontFile` is a bare Type 1 font program — Table 124's first row, the format for a `Type1` or
`MMType1` dictionary. It was on the "not implemented" list with a corpus count of **zero** beside
it and the note "No corpus page one reaches it".

**That zero was measuring reports, not documents.** An embedded program this crate cannot read
returns `UnsupportedProgram`, which falls through to substitution, and substitution only speaks
when the face it found can address *none* of the codes the document declares. So a page set in an
embedded Type 1 font drew in whatever installed face resembled it, plausibly and in silence, and
no count could see it. 57 corpus documents embed one on page one.

`read_fonts::ps::type1` parses the format — the PFB segments, the `eexec` decryption, the
charstring index, the Type 1 charstring interpreter — on exactly ADR 0006's argument for CFF: it
is the untrusted-input byte handling this project chose `skrifa` to avoid writing.

### One algorithm, two formats, and the clause says so

§9.6.2.1's NOTE 1 calls a CFF "an alternative, more compact but functionally equivalent
representation of a Type 1 font program", and §9.6.5.2 states one encoding algorithm for both.
So the shared shape is now a type: `name_keyed::NameKeyed` holds what a name-keyed program offers
— glyph by name, glyph by built-in code, and the name that built-in code selects — and `cff.rs`
and `type1.rs` each produce one. `simple_code_table` consumes it and cannot tell which reader
made it. ADR 0039's Table 112 finding applies to both without being written twice.

### A CIDFont's `/FontFile` is not a font program

Two corpus documents put a `/FontFile` in a **CIDFont's** descendant descriptor, which Table 124
does not allow: a Type 1 program is keyed by glyph *name* and a CIDFont selects by CID, and the
clause states no route between them. The first draft read it anyway and reported that the bytes
were not an sfnt, which named the wrong defect — the program is fine and its placement is what
the clause forbids. They now get what any CIDFont with no usable program gets, a substitute,
which is also what they got before this session.

### The parse is kept

This is the one place `type1.rs` differs from `cff.rs`, and it is a measured difference. A
`CffFontRef` borrows its bytes; a `Type1Font` decrypts the whole `eexec` section and indexes every
charstring, at 10–86 µs for each of `tracemonkey.pdf`'s twelve embedded programs. `build_outline`
runs once per *distinct* glyph, so re-parsing per glyph put `tracemonkey.pdf`'s interpretation at
**13.5 ms**; keeping the parsed program in `LoadedFont` takes it to **2.8 ms**, against 7.7 ms
before the feature existed at all — so reading the document's own fonts is now *faster* than
substituting for them.

## Consequences

| | before | after |
|---|---|---|
| corpus documents drawing with nothing reported | 823 | **843** |
| pages we claim to draw completely | 1620 | **1640** |
| agreeing with the reference consensus | 760 | **799** |
| contradicted by it | 99 | **88** |
| the references cannot agree among themselves | 751 | 743 |
| `CONTRADICTED_UNEXPLAINED` | 58 | **50** |

Seventeen pages left the unexplained list by being fixed and seven arrived from the instrument
change, and both halves are worth stating rather than netting off. The seven are pages this gate
could not judge before; one of them, `issue9915_reduced.pdf`, arrived with a diagnosis attached —
its `/OCRB` CIDFont writes `/W [32 [719] 0 180 719 181 [878] 182 65534 719]` and our letters sit
1.39× closer together than `poppler`'s and `mupdf`'s, which is exactly 1000/719, the ratio between
§9.7.4.3's `/DW` default and the width that array states.

`CONTRADICTED_GLYPHS_JUDGED_AS_VECTOR` is empty and stays as a comment, because what it recorded
was a defect in the instrument and the record of that is worth more than the list was.

§9.9 is reviewed as a family, which takes `9.9.1` off `REVIEW_OWED` — three clauses left there.
Its one honest `partial` is Table 125: `/Length1`, `/Length2` and `/Length3` are read by nobody,
because `read-fonts` finds the `eexec` boundary in the bytes, and the clause's only requirement on
a *processor* — that a `/Length3` of 0 means the 512 zeros and `cleartomark` "shall be added by
the PDF processor" — concerns a fixed-content portion holding no glyph description, so appending
it would change no outline.
