# veraPDF 1.30.3: four readings the archive census found plainly veraPDF's

Status: **filed upstream** (2026-11-24 on `veraPDF/veraPDF-library`: #1668, #1669, #1670, #1671; drafted ADR 3447). Each section below the line is one issue as it would go
to `veraPDF/veraPDF-library` (the first to `veraPDF/veraPDF-validation-profiles`); a person files
them. ISO 19005 is cited by clause and paraphrased, never quoted (`doc/third-party-data.md`);
ISO 32000-2 is quoted from `doc/md/`. Each was found by `tools/verapdf-census.py` on round 2181's
kept outputs (`doc/pdfa-blocking-issues.md` section 1 has the census and the triage of every
document it named). Departures the census found that are not drafted here, because the reading
rests on a specification this tree does not hold or on a damaged file, are listed in that section
with their reason.

---

## 1. Profiles: ISO 19005-2 6.2.11.3.1 test 1 compares the Supplement entries the other way round

**Filed**: https://github.com/veraPDF/veraPDF-library/issues/1668

**Profile**: `PDFA-2B.xml` (and the other part 2 and part 3 profiles), rule `6.2.11.3.1` test 1;
`PDFA-4.xml`, rule `6.2.10.3.1` test 1. Both end in `CIDFontSupplement <= CMapSupplement`.

**What the clause says** (ISO 19005-2:2011 6.2.11.3.1, ISO 19005-4:2020 6.2.10.3.1, paraphrased):
where the Type 0 font's Encoding is not Identity-H or Identity-V, the Registry and Ordering of the
CIDFont's and the CMap's CIDSystemInfo are identical, and the CIDFont's Supplement is *greater than
or equal to* the CMap's. The clause's note gives the reason: the font then holds a glyph for every
CID the CMap can reach.

**Reproducer**: a Type 0 font with a predefined CMap of Adobe-Japan1 Supplement 1 (`EUC-H`) over a
CIDFont of Adobe-Japan1 Supplement 2 — pdf.js's `noembed-eucjp.pdf` as converted to PDF/A-2b. The
validator reports "CIDSystemInfo Supplement = 2, CMap Supplement = 1" and fails the rule; the
clause is met. With the supplements the other way round (CIDFont 1, CMap 2) the rule passes where
the clause fails.

**Asked**: `CIDFontSupplement >= CMapSupplement`.

---

## 2. A `%` directly after an operator is read as part of the operator

**Filed**: https://github.com/veraPDF/veraPDF-library/issues/1669

**Rule**: ISO 19005-2 6.2.2 test 1 (and part 4's) fails "operator re% not defined".

ISO 32000-2 §7.2.3: "The delimiter characters (, ), <, >, [, ], /, and % are special (LEFT
PARENTHESIS (28h), RIGHT PARENTHESIS (29h), LESS-THAN SIGN (3Ch), GREATER-THAN SIGN (3Eh), LEFT
SQUARE BRACKET (5Bh), RIGHT SQUARE BRACKET (5Dh), SOLIDUS (2Fh) and PERCENT SIGN (25h),
respectively). They delimit syntactic entities such as arrays, names, and comments."
So `25 25 200 200 re% comment` is the operator `re` and a comment.

**Reproducer**: pdf.js's `calgray.pdf` and `calrgb.pdf` (every line of their page content ends in
an operator followed directly by `%`).

---

## 3. A rectangle whose numbers are indirect objects is read as zero-sized

**Filed**: https://github.com/veraPDF/veraPDF-library/issues/1670

**Rule**: ISO 19005-2 6.1.13 test 11 fails "One of the page boundaries is out of range (… height =
0, width = 0)" on `/MediaBox [7 0 R 8 0 R 9 0 R 10 0 R]`, where objects 7 to 10 are the numbers
0, 0, 250 and 50.

ISO 32000-2 §7.3.10: "Any object in a PDF file may be labelled as an indirect object. This gives
the object a unique object identifier by which other objects can refer to it (for example, as an
element of an array or as the value of a dictionary entry)." §7.9.5: "A rectangle shall be written
as an array of four numbers", and four references to numbers are that array's elements.

**Reproducer**: pdf.js's `issue7872.pdf`.

---

## 4. A CFF encoding's supplement is read as a glyph index rather than a string identifier

**Filed**: https://github.com/veraPDF/veraPDF-library/issues/1671

**Rule**: ISO 19005-2 6.2.11.5 test 1 (part 4 6.2.10.5, and part 4 6.2.10.7) on a `Type1C` font with
no `/Encoding`, so the program's built-in encoding selects the glyph (§9.6.5.2).

Adobe Technical Note #5176 section 12 (cited, not quoted): a supplement maps a code to a SID, and
the glyph is the one whose charset entry is that SID. In pdf.js's `issue3566.pdf` (`CMR9`), code
174 is supplemented to SID 266 (`ff`), glyph 79, whose charstring states 599.53 against `/Widths`
599. veraPDF reports a program width of 342.59 for code 174.

**Reproducer**: pdf.js's `issue3566.pdf` converted to PDF/A-2b.
