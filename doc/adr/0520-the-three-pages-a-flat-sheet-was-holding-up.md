# ADR 0520 — The three pages a flat sheet was holding up, and the silence one of them kept

Status: accepted, 2026-08-23. Session 685. Diagnoses the six pages ADR 0513 left owed, and fixes
the one defect of this tree they uncovered. **One pixel population moves: none.** What moves is
one document onto `corpus.rs`'s incomplete list, and a measurement from 5 codes over 2 documents
to 129 over 7.

## What was owed

ADR 0513 stopped a reference whose raster is flat from voting against a reference that drew marks.
Six agreements were the price, and it recorded why they are the interesting six: on each one two
flat sheets outvoted a renderer that drew, **and our own raster was one of the flat ones**. Its
closing item:

> **What is owed** is the three pages of `NOT_COMPARABLE_A_MARK_ONE_REFERENCE_DRAWS` that are not
> `issue18042.pdf`, by `doc/todo/00`'s method — a page where two of five renderers place a mark
> and we do not is exactly what that method is for, and until this round the oracle called it an
> agreement.

All three are read here, `issue18042.pdf` included, because "the page is on `corpus.rs`'s list" is
a statement about where the report goes and not a reading of the clause. **Each of the three turns
on a different clause, and none of the three is a page where we draw the wrong thing.** One is a
page where the standard's own algorithm ends and the standard says a processor may choose; one is
a page where a renderer paints artwork no clause states; one is a page where the clause says the
artwork the other two draw has been replaced. What was genuinely wrong is in the third section
below, and it is not about any of them being drawn.

**The panel figures here are taken with `-alpha off`**, which is `doc/todo/00` step 5's rule.
`hayro`'s two figures therefore differ from ADR 0513's by exactly the factor of ¾ that averaging
an alpha channel into a three-channel greyscale costs — 0.3498 against a recorded 0.262, and
0.313593 against 0.2352. Neither renderer moved; the instrument did.

## `issue17333.pdf` page 1 — §9.6.5.4 runs out, and its last sentence says so

100 × 100 points. The whole content stream is `1 0 0 1 50 50 Tm /TT3 20 Tf (\000) Tj` — one
text-showing operator, one **character code 0**.

`/TT3` is a `TrueType` simple font: `/BaseFont /BQGUTS+SymbolMT`, `/Encoding /MacRomanEncoding`,
`/FirstChar 0`, `/LastChar 165`, an embedded `/FontFile2`, and a descriptor whose `/Flags 32` sets
Table 121's **Nonsymbolic** bit. Read table by table, the program is:

| | |
|---|---|
| `maxp` | `numGlyphs` **2** |
| `loca` | short, `0, 50, 89` — so glyph 0 is 100 bytes and glyph 1 is 78 |
| `glyf` glyph 0 | `numberOfContours` **2**, bbox (103, 0)–(1127, 1280) of 2048 units per em |
| `cmap` | **one** subtable, (1, 0), format 6, `firstCode` 165, `entryCount` 1 → glyph 1 |
| `post` | version 2.0, glyph 1 named `bullet` |
| `hmtx` | advances 1229 and 942 of 2048 — 600/1000 and 460/1000 |

So `.notdef` here is a real hollow box rather than an empty glyph, and code 165 — `bullet` in
MacRomanEncoding, `/Widths[165] = 460` — draws correctly through every rule the clause states.

Code 0 does not, and the clause is walked to its end rather than assumed. §9.6.5.4's
named-encoding branch applies, because the `/Encoding` entry is `MacRomanEncoding` *and* the
Nonsymbolic flag is set. The table it requires maps codes to glyph **names**; MacRomanEncoding
assigns code 0 none, and the `StandardEncoding` fill assigns it none either. Both cmap rules then
have nothing to carry — each begins "A character code shall be first mapped to a glyph name using
the table described above" — and the `post` fallback has no name to look up. What is left is the
subclause's closing sentence:

> If a character cannot be mapped in any of the ways described previously, a PDF processor may
> supply a mapping of its choosing.

**A permission, and `doc/todo/00`'s third shape: the clause puts the answer beyond itself and
names the sentence that does it.** `mupdf` and `hayro` supply the mapping code → `.notdef` and
draw the hollow box, agreeing with each other to **0.004 of 255** — 0.346 and 0.3498. `poppler`,
`ghostscript` and this tree supply nothing and return white. Both answers conform; the clause
ranks neither.

This tree's two supplied mappings are ADR 0015's and are unchanged by this round: offer the code
to every subtable the font lists, and — only for a font with **no readable `cmap` at all** —
treat the code as a glyph index. Neither reaches code 0 here, and the second one's restriction is
why: this font has a `cmap`, and it does not cover the code. ADR 0015 recorded that reinstating
the per-code fall-through "contradicts `issue17333.pdf` immediately", which was measured against
an oracle that let two flat sheets vote. **Re-derived under ADR 0513's rule the observation is
weaker than it was and points the same way**: with the box drawn, `poppler` and `ghostscript`
still return flat sheets that a drawing reference disagrees with, so they still abstain and the
page still has one reading. The verdict is `not comparable` whichever mapping is supplied — which
is the useful part, because it means the choice cannot be made by the instrument and has to be
made from the clause.

**The mild evidence about the producer's intent is recorded and not acted on.** `/Widths[0]` is
600, which is glyph 0's own advance to the thousandth, so whoever wrote the file took code 0's
metric from `.notdef`. That is a fact about a *producer* and §9.6.5.4 states no rule deriving a
glyph from a width; it is written here so the next round does not have to find it again.

## `issue18042.pdf` pages 1 to 4 — a `DCTDecode` stream that is four bytes of ASCII

1247 bytes. Four pages share one content stream, `100 0 0 100 146 152 cm /Im1 Do`, and `Im1` is

```
<< /BitsPerComponent 8 /ColorSpace /DeviceRGB /Filter /DCTDecode
   /Height 7600 /Subtype /Image /Width 7300 /Length 4 >>
stream
1234
```

§7.4.1 says "[a] PDF reader shall invoke the corresponding decoding filter or filters to convert
the information back to its original form", and §7.4.8 says what that form is: data "encoded in
the JPEG baseline format in accordance with ISO/IEC 10918 (all parts)". Four bytes of digits are
not a JPEG datastream, there is no original form to convert back to, and **no clause of ISO
32000-2 states any artwork to put in an undecodable image's place.** This tree invokes the filter,
the filter refuses by name — `Im1: malformed image: JPEG headers: Error parsing image. Illegal
start bytes:3132` — and the page draws without it and says so. That is §7.4.1's sentence and trap
5's rule, and it is the whole of what the standard determines here.

`mupdf` paints the image's rectangle solid black. The closed form confirms it exactly rather than
approximately: §8.9.5.1 puts every image in the unit square, the `cm` maps that square onto
100 × 100 of a 400 × 400 page, and a black fill over one sixteenth of a white sheet is
`255 ÷ 16` = **15.9375 of 255** — which is what the panel measures, to the digit. `poppler`,
`ghostscript`, `hayro` and this tree return white.

So this is `doc/todo/00`'s second shape with one renderer on the far side of it: the clause
determines what the page states, and one renderer substitutes a mark for it. The page is on
`corpus.rs`'s incomplete list and stays there.

## `text_field_own_canvas_calc.pdf` page 3 — §12.7.4.3's regeneration is a splice

612 × 792, one widget. The field is `/T (Mirror)`, `/FT /Tx`, `/Ff 1`, **no `/V`**, under an
`/AcroForm` carrying `/NeedAppearances true`. Its stored normal appearance is one line:

```
/Tx BMC q 0.85 0.85 0.85 rg 0 0 200 20 re f Q EMC
```

Table 224 makes `/NeedAppearances` "[a] flag specifying whether to construct appearance streams
and appearance dictionaries for all widget annotations in the document (see 12.7.4.3, "Variable
text")", and §12.7.4.3's closing paragraph says what constructing one over an existing stream
*is*:

> The interactive PDF processor shall then replace the existing contents of the appearance stream
> from / Tx BMC to the matching EMC with the corresponding new contents

Every mark this widget states sits **inside** that pair, and a field with no value has no new
contents, so the splice replaces the lot with nothing. ADR 0032 is where that reading was made —
it is also where the clause's opposite answer is recorded, "[i]f the existing appearance stream
contains no marked-content with tag Tx , the new contents shall be appended to the end of the
original stream", which is why a fixture differing only in where its marks sit gets the opposite
result — and `crates/pdf-model/tests/variable_text.rs` pins both.

**The closed form says the other two renderers draw the pre-regeneration artwork and nothing
else.** `0.85` at eight bits is 217; the rectangle is 200 × 20 of 612 × 792; so the sheet's ink is
`(255 − 217) × 4000 ÷ 484 704` = **0.313591 of 255**. `ghostscript` and `hayro` each measure
**0.313593** with a minimum channel of exactly **217**. `poppler`, `mupdf` and this tree return
white. Nothing here is a question about colour, geometry or scan conversion: the disagreement is
entirely about whether §12.7.4.3's regeneration runs, and the clause says it does.

`doc/todo/00`'s first shape — the clause determines it and we can be checked against it, by
arithmetic out of the file with no renderer trusted.

## The defect, which is none of the three and is this tree's

`issue17333.pdf` drew **zero commands** and reported `unsupported: []`. `doc/todo/00` step 4 names
that shape and says it "has been a real defect every time", and this instance is worse than a
blank page: the page states one mark, the standard's own algorithm ended without an answer for it,
and every instrument in the tree that measures the picture read zero.

The cause is one condition. `Interpretation::codes_without_a_glyph`'s doc comment says it counts
"[c]odes this page showed that reached no glyph at all", excluding two cases that are not marks
missed — a code that reads back as whitespace, and a code §9.10.2 gave a character the substitute
face lacks. **The code had a third exclusion the sentence did not name**: the branch was entered
only for `read.is_some_and(Readback::names_a_mark)`, so a code §9.10.2 could not *name* was
excluded as well. `codes_without_a_glyph` stayed 0, `Coverage::empty` stayed 0, and the report
that fires on a font which drew nothing of what it was asked to show — `coverage.drawn == 0 &&
coverage.empty > 0` — could not fire, because its second operand was never raised.

### Why the readback was the wrong question

The exclusion was not arbitrary and its argument is quoted in `Readback::names_a_mark`: §9.10.2
ends "there is no way to determine what the character code represents", and a silence is not
evidence in either direction. That is right, **and it is a question about the reader.** Whether
the *program* answered a code is a different question with its own evidence: §9.6.5.4 and
§9.7.4.2 state the routes from a code to a glyph, and a route that ends at no glyph — or at
`.notdef`, which §9.6.5.2 and §9.7.6.3 both make the program's way of saying it has none — ended
without an answer whatever anybody can or cannot say about the character.

The tree already had that distinction and was using it one line too late: the `blank` split
inside the branch decides exactly this, and the branch's *entry* condition was overruling it.

So the arms are separated by what each is about:

- **the whitespace exemption stays and is the only readback condition on the way in.** A space is
  meant to have no outline, and ADR 0270's measurement of what counting one costs — the corpus's
  incomplete documents from 79 to 109 — is unchanged;
- **a code that reached a glyph the program contains and describes as empty** stays gated on a
  named character. The program has answered; the only thing left to lose is the reading, and
  §9.10.2's silence is not evidence that one was lost. `codes_reaching_a_blank_glyph` is
  unmoved by this round, by design and by measurement;
- **a code that reached no glyph, or `.notdef`,** is counted whatever the readback says, because
  the fact being recorded is the program's.

### What it matched, printed rather than trusted

Trap 11's rule. Before and after, on the same tree, with §5's worker built:

| | before | after |
|---|---|---|
| documents drawing incompletely | 68 | **69** |
| codes reaching no glyph *in silence* | 5 over 2 documents | **129 over 7** |
| codes reaching a glyph the font draws blank | 57 over 9 documents | 57 over 9 |
| codes §9.10.2 could not name *in silence* | 1226 over 41 documents | 1225 over 40 |

**The report fired exactly once**, and the diff of the incomplete list is one line:

```text
incomplete: issue17333.pdf: [Font { detail: "font /TT3's program has no outline for any of
the 1 code(s) the page shows through it, so the text it states is not drawn" }]
```

The last row moves by one document and one code for that reason and no other: the `in silence`
filter is `is_complete()`, and `issue17333.pdf` stopped being complete.

The second row is the measurement rather than the report, and its four new documents were read
rather than counted: `issue20489.pdf` shows code 10 through fonts that draw everything else,
`issue18059.pdf` and `standard_fonts.pdf` show code 0, `issue6721_reduced.pdf` shows code 224
reaching glyph 0, and `issue11403_reduced.pdf` shows 194 and 160 — a UTF-8 no-break space written
byte by byte into a simple font. None of them reports, because each of those fonts drew the rest
of its page; they are content this tree does not draw that nothing named before, and
`doc/todo/21` is where the question of what to do about them lives.

### Two tests asserted the opposite, and had been wrong since they were written

The widened count failed two of them, and both are the same fixture in two crates: a `/Helvetica`
whose `/Differences` names `/a192` and `/a224` — pdfTeX's private labels for `À` and `à`, which
ADR 0311 refuses to follow — on a page showing `(A\300a\340)`.

- `tools/pdf-retrieve/tests/retrieval.rs` asserted `without_a_glyph == 0` under *"the substitute
  draws something for both: this is a reading gap, not a drawing one"*;
- `crates/viewer-core/tests/headless.rs` asserted the same under *"a substitute drew all four"*,
  above a doc comment describing the page as one that "interprets whole and draws every glyph".

**Neither sentence is true and neither ever was.** A name no encoding defines addresses nothing in
the substitute face either, so the page states `AÀaà` and marks `Aa` — two characters lost *and*
two marks lost. The count could not contradict the prose, because it carried the very exclusion
this ADR removes: the same silence that let `issue17333.pdf` draw a blank sheet let a fixture's own
description of itself stay false in two crates.

Both assertions are now 2 and both comments say what the page does. **The decision each test is
about is untouched** — this crosses as a count and not as a report — and the reason it survives is
worth stating, because it is the report's condition doing its job: the font drew two of its four
codes, and the report fires only on a font that drew none.

## Consequences

- `content/text.rs`'s branch divides on the glyph rather than on the readback, with the
  whitespace exemption in front of it; `Readback::names_a_mark`,
  `Interpretation::codes_without_a_glyph` and `Interpretation::codes_reaching_a_blank_glyph` say
  which question each answers.
- `crates/pdf-model/tests/silent_fonts.rs` gains
  `a_code_with_no_character_and_no_glyph_is_a_mark_missed_and_is_reported`, against the real
  document, and its module comment records the exemption that was in the code and not in the
  prose. `tools/pdf-retrieve/tests/retrieval.rs` and `crates/viewer-core/tests/headless.rs` have
  their false sentences corrected beside their corrected expectations.
- **Trap 11 gains a fifth instance with its sign reversed** — a condition narrowed by an exemption
  written for something else — and the two rules that come out of it: an exemption is part of the
  condition and needs the same evidence, and a report built out of a count inherits every one of
  that count's exclusions.
- `NOT_COMPARABLE_A_MARK_ONE_REFERENCE_DRAWS`'s note carries the three readings, with the clause
  each rests on and the two closed forms. The group's membership does not change and no verdict
  moves: our rasters are byte-identical, because a report is not a mark.
- No ledger row changes status. §9.6.5.4, §7.4.1, §7.4.8 and §12.7.4.3 each gain the reading this
  round took, which is the retrofit rule rather than a status change.
- **What is not decided**: whether this tree should supply `.notdef` for a code §9.6.5.4 cannot
  map. The clause permits either and the oracle cannot rank them, so changing it would be a
  choice made for no stated reason — which is the shape principle 5 forbids. It stays a
  documented departure with the sentence that licenses it written down.
