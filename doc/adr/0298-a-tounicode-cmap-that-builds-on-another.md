# ADR 0298 — A `/ToUnicode` `CMap` that builds on another, and the whole shortfall behind the text gate

Status: accepted, 2026-08-13. Session 463. Corrects §9.10.3's ledger row, which claimed a reader
this tree did not have; amends §9.7.5.3's and §9.7.5.4's rows, which read one clause as a licence
to refuse. Does not amend ADR 0140 or ADR 0259, whose readings of §9.10.2 this extends by one
route rather than changes.

## The question the round was given

The text gate is two gates and sits a little under a hundred percent, and the brief asked which
documents make up the shortfall and whether each fails for a reason the standard settles. The
gate names only the documents below its 0.90 floor, so the band between the floor and 100% had
never been listed at all.

## What the population is

One line added to the pdf.js gate — print every document scoring under 1.0 rather than under the
floor — and the answer is smaller than the phrase "a population nobody has characterised"
suggests. **974 documents, 62 not gated because this tree reports something on them, 24 skipped;
of the 888 that are gated, 27 fall short of 100% and they account for 184 words of 24 191.**
Twenty-three of the twenty-seven are the named list the gate already carries. The four that are
not had never been looked at:

| document | short by | what it is |
|---|---|---|
| `bug1997343.pdf` | 8 | four §14.9.4 `/ActualText`, four §14.8.2.3 soft hyphens — both sides right, see below |
| `issue918.pdf` | 7 | §9.10.2 exhausted: a dvips Type 3 whose glyph names are `/aNN` and whose codes are outside printable ASCII; the reference answers U+001C and U+001E, which are not characters either |
| `issue20489.pdf` | 1 | one reference "word" — `Date>SCALE` — is two labels `pdftotext`'s column analysis ran together and this tree draws forty lines apart |
| `issue1350.pdf` | 1 | the reference reads `beginnerÕs`; this tree reads `beginner’s`, which is what MacRomanEncoding 0xD5 is |

So three of the four new ones are the *reference* being wrong or doing layout analysis, one is a
route the standard closes itself, and the whole band is 17 words.

**And the same instrument found the one defect in the round**, which was not in that band at all
— it was on the named list, in a document reading back the empty string.

## The defect: `issue5010.pdf`

A Korean `Identity-H` font. Its `/ToUnicode` stream states five mappings, all for codes `<46FB>`
to `<4704>`, and its page shows none of them; what the page shows is twelve codes answered by one
line the tree was not reading:

```
/Adobe-Korea1-UCS2 usecmap
```

The stream dictionary carries only `/Length` and `/Filter` — no `/UseCMap`. Every other route
§9.10.2 states is closed for this font, and closed correctly: the second method does not apply to
a composite font, and the third is constructed from the descendant's `/CIDSystemInfo`, whose
registry is `Unidocs`, so the name to fetch would be `Unidocs-Korea1-UCS2` and nobody publishes
one. The page read back the empty string, reported nothing, and sat on the gate's named list.

### What the clauses say

§9.10.3's first bullet is the one that was missing:

> The only pertinent entry in the CMap stream dictionary (see "Table 118 -Additional entries in a
> CMap stream dictionary") is UseCMap , which may be used if the CMap is based on another
> ToUnicode CMap.

Table 118 says what a base means: the referencing `CMap` "shall specify only the character
mappings that differ from the referenced CMap".

`to_unicode` read the stream and nothing else. **§9.10.3's ledger row said "Table 118's `/UseCMap`
is read", and that sentence was true of the wrong reader** — `read_cmap`, which handles the
`/Encoding` `CMap`. This is the ledger failure shape the sweeps hunt: a row describing what the
code ought to do, checked against a different function that does it.

That alone would not have read `issue5010.pdf`, because it states no `/UseCMap`. §9.7.5.4 a) is
what settles the rest:

> If the embedded CMap file contains a usecmap reference, the CMap indicated there shall also be
> identified by the UseCMap entry in the CMap stream dictionary.

**The clause makes the dictionary entry and the in-file operator agree by requirement.** So
reading the operator where the dictionary is silent cannot contradict a conforming file — on one,
the two say the same thing — and on a file that omitted the entry it is the only statement there
is. That is a different act from guessing: the name is in the file, the data behind it is Adobe's
own published `Adobe-Korea1-UCS2`, which §9.10.2 step d) names as the source and which this binary
already carries for the third method, and the twelve characters come out of that file's own
`bfrange` lines. Derived, not matched — and the derivation was checked against the file by hand
before any code was written.

### Why the `/Encoding` reader was changed too

`read_cmap` had the opposite of a gap: it *refused* a `CMap` whose `usecmap` reference was not
named by `/UseCMap`, "because what it inherits cannot be found". That reason had outlived itself
— the branch two lines above resolves a `/UseCMap` **name** through `predefined::cmap`, so a name
this binary carries can plainly be found. One rule now covers both readers: follow the file's own
statement where the dictionary is silent, and keep the refusal for the case the reason actually
describes, a name nobody publishes. No corpus `/Encoding` `CMap` references another, so this half
stays synthetic, which is trap 8's advice rather than an accident.

### What it deliberately does not do

**A named base this binary does not carry answers nothing rather than reporting.** A `FontError`
would make the page incomplete and take it out of the oracle's judged set (trap 11), and §9.10.2
provides later methods and then an explicit permission for exactly this outcome. It is the same
stance §9.10.3's row already took for a damaged `CMap`: extraction degrades to missing characters,
which shows, rather than to a font refused whole.

**A `usecmap` naming a CID `CMap`** — `90ms-RKSJ-H`, say — is not read as a `/ToUnicode` base by
accident: those files state `begincidrange` and no `bfchar` or `bfrange`, so `ToUnicode::parse`
finds nothing in them and `unicode_cmap` answers `None` on an empty map. §9.10.3's bullet asks for
"another ToUnicode CMap" and that is what the parser's own shape enforces.

## The gate's hyphen rule, corrected by the same clause it already cited

`without_hyphens` removes U+002D from both sides of the comparison, on the argument that "nothing
in the content stream says which hyphens those are" and that §14.8.2.3 gives a tagged producer a
way to say so. `bug1997343.pdf` is a producer that **used** it: six line-broken words come back as
`in\u{ad}cluding`, `fol\u{ad}low`, `mathemat\u{ad}ics` and three more, and the gate was scoring
them as missing.

The readback is right and stays as it is — §14.8.2.3's ledger row already says the reader's half
is to deliver U+00AD unchanged and leave the rejoining to a consumer. What was wrong is the
instrument: the character the function exists to remove is the one a line break introduced, and
this is the case where the file *stated* it instead of leaving it to be inferred. Removing it is
the same rule with a stronger warrant, and it is symmetric across both sides of the comparison.

## What it cost and what it bought

One gate moved and nothing else did. The pdf.js comparison went **24 007 → 24 012 of 24 191
words** — one from `issue5010.pdf`, which was 0 of 1, and four from `bug1997343.pdf`'s hyphens —
and the named list **23 → 22**; the `PDFBox` gate has no document of either shape and is unmoved
at 14 257 of 14 281. Every other gate in `doc/todo/02` §2 is line-for-line identical,
which is what says a new route into §9.10.2's first method did not disturb the three below it.

No pixel moves: `/ToUnicode` is read for extraction and, for a *substituted* composite font, for
the character a code stands for — and `issue5010.pdf` embeds its font, so nothing about the page's
ink depends on this. The ink sweep is therefore not owed and the corpus, oracle and quorra runs
say so.
