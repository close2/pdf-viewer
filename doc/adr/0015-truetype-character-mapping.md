# ADR 0015 — A character code reaches a `TrueType` glyph by §9.6.5.4, and by nothing else

Status: accepted, 2026-07-28.

## Context

An embedded `TrueType` or `OpenType` font maps codes to glyphs through its own `cmap` table,
and for four working sessions this crate did that in one line:

```rust
charmap.map(code).or_else(|| charmap.map(0xF000 + code)).or_else(|| u16::try_from(code).ok())
```

`skrifa`'s `Charmap` selects the most comprehensive *Unicode* subtable a font offers. That is
the right choice for laying out text, and the wrong one here, because ISO 32000-2 §9.6.5.4's
entire subject is that **a PDF character code is not a character**. Each `cmap` subtable is
indexed by something else — a Unicode scalar, a Mac OS Roman code, a two-byte symbol code —
and the subclause is a set of rules for turning a code into whichever of those the font
happens to carry. Handing the code straight to a Unicode subtable is right only by
coincidence, for ASCII, in a font that has one.

Two things hid the cost. The coincidence covers most Latin documents, so the common case
worked; and where it failed the last clause of that line drew glyph number `code` instead,
which is a *wrong glyph, confidently*, and reports nothing. That is trap 1 in the handover,
and both of its halves were live:

- `issue20504.pdf` sets six scripts in six embedded subsets. Every one of its `TrueType`
  fonts carries a single (1, 0) Macintosh subtable — which is exactly what §9.6.5.4's own
  guidelines tell a producer to emit — so `Charmap` found nothing, the fall-through asked for
  glyph 33 of a nine-glyph subset, and five of six lines drew nothing at all. The page
  reported `unsupported: []`.
- `issue5501.pdf` carries its byte-to-glyph mapping in a (0, 0) subtable. The fall-through
  drew glyph 87 for code 87, and the page read `v 0' ' W` where poppler reads
  `What's an interval?`. Also `unsupported: []`.

The oracle had 81 pages contradicted "with nothing on the page to explain it". Fifteen of
them were this.

## The decision

`truetype_code_table` implements §9.6.5.4 as written, resolving all 256 codes once when the
font is loaded, and `CodeMapping::Charmap` — the per-glyph route through `skrifa`'s
`Charmap` — no longer exists. Subtables are found by their platform and encoding IDs, which
is what the subclause selects on, rather than by a best-subtable heuristic.

The rules, in the subclause's own order:

1. **The font's own codes.** Symbolic flag set, or no `/Encoding` entry: a (3, 0) subtable is
   addressed by the code with the high byte of its range prepended, and failing that a (1, 0)
   subtable by the single byte.
2. **Through a glyph name.** Otherwise the code selects a name — base encoding, updated by
   `/Differences`, undefined entries filled from `StandardEncoding` — and the name reaches a
   (3, 1) subtable through the Adobe Glyph List, or a (1, 0) subtable through Mac OS Roman.
3. **The `post` table**, for a name neither of those could map.

Three decisions inside that are not the specification's, and each has a cost.

### Mac OS Roman is transcribed, because `MacRomanEncoding` is not it

Rule 2's second half needs a glyph name turned *back* into a code, and §9.6.5.4 is explicit
that the encoding for this is "the standard Roman encoding that is used on Mac OS", which
Table 113 defines as `MacRomanEncoding` plus fifteen mathematical and symbol glyphs, with
`currency` at code 219 replaced by `Euro`. Reusing PDF's table would reach no glyph for those
sixteen names. `encoding::mac_os_roman_code` is the addition, and a test asserts the
subclause's own arithmetic about itself: of Table 113's sixteen rows, exactly one may land on
a code `MacRomanEncoding` already uses, and it must be 219.

### Two mappings "of this processor's choosing", both narrower than what they replace

§9.6.5.4 closes with "if a character cannot be mapped in any of the ways described
previously, a PDF processor may supply a mapping of its choosing". Two are supplied.

The first offers the code to every subtable the font lists, in the font's order. It earns its
place twice: a symbolic font carrying only a (3, 1) subtable — common, and contrary to the
guidelines — reaches no rule above and its codes really are ASCII; and `issue5501.pdf`'s
(0, 0) subtable is named by no rule and is that font's only correct answer.

The second treats the code as a glyph index, and now applies **only to a font with no
readable `cmap` at all**. The restriction is the point. The old code fell through per *code*,
so a font with a perfectly good `cmap` that merely did not cover one drew glyph number `code`
— a wrong glyph in place of a blank. §9.9.1 requires a simple font's program to carry a
`cmap`, so a program without one is malformed, and a subset really is often ordered by code;
a program *with* one has already answered the question.

The oracle confirms the restriction is load-bearing rather than tidiness: reinstating the
per-code fall-through contradicts `issue17333.pdf` immediately.

### An unrecognised `/Differences` name is kept, not dropped

The glyph-name table used to hold `&'static str`, so a name with no static spelling was
discarded and the code kept whatever the base encoding had put there. A code the document had
*explicitly reassigned* therefore silently kept its old meaning. `issue20504.pdf` writes
`/Differences [33 /gid2436 …]`, a subsetter's convention for naming a glyph by index that
§9.6.5 does not define, and all four codes fell back to `StandardEncoding` and drew `!"#$`.

§9.6.5.4's "any *undefined* entries in the table shall be filled using `StandardEncoding`" is
about codes the encoding never assigned, not about codes it assigned to a name we do not
know. The table is now `Cow<'static, str>`: a recognised name still costs no allocation,
which is nearly every name a real document writes, and a novel one is owned and reaches the
`post` table or the CFF charset, where it may well be found.

## Two silences this uncovered, and closed

Neither is about `cmap` tables. Both were only visible once the algorithm above stopped
masking them.

**Type 3 fonts are refused.** A Type 3 font has no font program: §9.6.4 makes each glyph a
content stream in `/CharProcs`, which is the interpreter's work and not this crate's. All 24
corpus documents carrying one were reaching the *substitution* path, where the names in a
Type 3 `/Differences` array — `/a192`, `/g3`, names of procedures — were resolved against a
Latin system font. `issue918.pdf` drew 388 text operations of letter fragments at the wrong
places and reported nothing; poppler draws a page of readable text. The refusal is
`FontError::Type3`, and when Type 3 is implemented it will be implemented a layer up, in
`pdf-model`, which is where a content stream can be run.

**A substitute is judged on the codes the document declares.** The old test asked whether the
substitute reached *any* of the 256 codes, and a Latin face always does — so a font whose
whole `/FirstChar`..`/LastChar` range mapped to nothing still passed as usable.
`issue20504.pdf`'s Chinese line drew nothing in silence for exactly that reason.
`tracemonkey.pdf` is the smaller and more instructive case: a Type 1 `CMSY7` subset whose one
declared code is `/circlecopyrt`, so the © is missing from a page that otherwise draws
perfectly, and 19 documents now say so.

That rule is deliberately about the *font* rather than about each code, and it is therefore
incomplete: a font that maps some of its declared codes and not others is still silent about
the rest. Closing that needs a report where a glyph is *shown* rather than where a font is
loaded, which is a change to the interpreter and not to this crate.

## What it cost

- The corpus's incomplete count rose from 250 to 290: 24 documents for Type 3, 19 for the
  declared-codes rule, less one that now draws completely. Every one of those is a page that
  was drawing wrongly, or not at all, in silence.
- The oracle's contradicted count fell from 129 to 108, and 15 of the 81 "unexplained" pages
  left that list at once — the largest single fall it has had.
- `Cow` in the glyph-name table, in place of `&'static str`. One allocation per novel name
  per font, at load time.

## Alternatives rejected

**Keep `Charmap` and add the (1, 0) case.** It would have fixed `issue20504.pdf` and left the
shape of the defect intact: the code would still be a heuristic that happens to agree with
§9.6.5.4 on the fonts anyone tested. The subclause is short and its rules are distinguishable
by fixtures, so there is no reason to approximate it.

**Report the `/Encoding` a symbolic font names but this crate cannot read.** §9.6.5.4 says
that when the symbolic flag is set the `/Encoding` entry "is ignored", so `issue5701.pdf`'s
`/Encoding /Identity-H` on a simple `TrueType` font is not a font we cannot read — it is an
entry the specification tells us not to read. Refusing it lost a page of text that draws
correctly; a *report* would have been an accurate statement about a document, and a false one
about our reading of the clause.

**Test through the corpus alone.** Every real font carries several subtables, so a page
drawing correctly proves only that *some* route worked, and a page drawing wrongly does not
say which route was missing. The fixtures in `truetype_encoding_tests` carry exactly one
subtable each, so exactly one rule can apply, and a rule that stops working fails one test by
name. This is trap 8 — a corpus finds what documents contain, not what the specification
says — and it is why the corpus and the fixtures are both here.
