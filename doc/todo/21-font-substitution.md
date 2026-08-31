# What is left of font substitution

Status: reported at runtime; seven distinct gaps — the first still **empty of witnesses**, the second unchanged, the third **characterised, fixed and re-measured** (ADR 0270), the fourth **half taken — the width on Table 109's own sentence (ADR 0358), the cap height still declined** (ADR 0267), the fifth **refused on the clause's own words, counted, split by cause, one population of it closed out of Annex D, and the rest given a voice outside `pdf-model` rather than a report** (ADR 0311, ADR 0318, ADR 0422), the sixth **closed** — the compiled-in fourteen no longer disagree with themselves about contour direction (ADR 0396) — the seventh **taken, with its remainder priced below — the first of those remainders closed by an instrument rather than a decision, and the two that followed it settled by measuring this machine's font catalogue rather than by arguing about a feature** (ADRs 0763, 0764, 0765).
Priority: 21
Corpus: 40 documents. **The corpus gate's own four silence lines are where the counts are, and
this file does not repeat them** (ADR 0281): `codes reaching no glyph *in silence*` and `codes
reaching a glyph the font draws blank` are the split ADR 0270 drew, the first of the two being what
§3 below is about, `codes §9.10.2 could not name *in silence*` is §5's, added in the
four-hundred-and-seventy-sixth, and `codes drawn upright where §9.7.5.1 named a vertical form` is
§7's, added in the eight-hundred-and-thirty-seventh — the only one of the four that is a mark
**made**, in the wrong shape. The line's own worst-ten follows each. `doc/HANDOVER.md`'s
not-implemented table stated the first of them wrongly for long enough to be worth this sentence.
Clauses: §9.10.2, §9.7.4.2, §9.6.2.1, §9.6.2.2, §9.6.4 (Type 3, for §5's `/a192`), §9.6.5.3, §9.6.5.4, §9.7.5.1, §9.7.5.2, §9.8.1, §9.8.3
Code: `crates/pdf-font/src/substitute.rs`, `crates/pdf-font/src/substituted.rs`, `crates/pdf-font/src/vertical.rs`, `crates/pdf-font/src/metrics.rs`, `crates/pdf-model/src/content.rs`

## 1. A per-character fallback — **0 documents, and this section was wrong about its own two**

§9.10.2 gives a code a character and the face a family match found has no glyph for it. Since the
hundred-and-eighty-third session a substitute is chosen by **coverage** — the widest-repertoire
face on the machine that can draw a character of the collection's own script (ADR 0153) — and
eight of the ten blank pages that named this now draw.

**All ten do.** This file said the two left were `issue11555.pdf` and `issue2128r.pdf`, "whose
characters no single face on this machine has", and the two-hundred-and-fifty-sixth session opened
the pictures: `issue2128r.pdf` draws every one of its Chinese characters and `issue11555.pdf`
draws its whole vertical mixture of Latin and kana. Both report nothing, both are above the text
gate's floor, and neither has a code reaching no glyph. The claim was a *prediction* about ADR
0153's rule that nobody re-checked after the rule landed — the same shape as a ledger row whose
"what IS done" half is wrong, one directory over, and `doc/todo/01`'s sweeps do not look here.

**So the mechanism is owed with no witness at all.** It was built in that session and reverted in
the same one, and what the attempt is worth is the two reasons it was not kept:

- **A sample is a sample, so a chain chosen from one is the wrong shape.** The first design
  extended `installed_covering` to return several faces covering `script_sample`'s characters
  between them. It changed nothing on either document, because both were *already* covered by one
  face — the characters that could go missing are the ones the sample does not contain, which is
  the whole point. What the mechanism actually needs is a lookup at *draw* time, per character.
- **The draw-time version has no machine-independent gate.** It works: the machine's widest faces,
  ranked by `cmap` size, bounded at eight, asked in order when the primary face lacks a character.
  But every assertion about it is an assertion about which faces this machine has, and ADR 0133
  exists precisely because `substitute.rs` was the last machine-dependent code in the tree. A
  feature whose only test says "on this machine, in August 2026" is not one this project ships.

What would make it shippable is a witness — a document whose substituted composite font shows a
character its chosen face lacks — or a face this binary carries that is addressable by character.
§9.6.2.2's fourteen are not: they are name-keyed CFF, which §9.7.4.2 leaves unreachable for a
composite font.

**And `freetext_no_appearance.pdf` is not this section's witness either**, which `doc/todo/22`
filed here for a long time and session 513 read out (ADR 0348). A per-character chain would draw
that value's Arabic in isolated forms left-to-right even where it found every glyph — trap 1's
wrong-but-plausible page, worse than the refusal it would replace. What that document needs is a
glyph source (no compiled-in face has one Arabic glyph — measured; Liberation Sans's `cmap` maps
the whole Arabic range to glyph 0 and its `GSUB` has no `arab` script), Unicode's joining-form
selection and right-to-left ordering, **together or not at all**; the cost of each is ADR 0348's.
So this section's mechanism stays owed with no witness at all, exactly as its own heading says.

## 2. A substitute that cannot be addressed — 40 fonts

Composite fonts naming an `Identity` ordering, where the codes are indices into a font nobody
supplied and §9.10.2's third method has nothing to read. §9.7.4.2 leaves such a font reachable
only through `/ToUnicode`, which addresses by character; without one there is no question to ask.
Honest refusals. The `-UCS2` `CMap`s closed the rest of this population in session 156 (ADR 0140).

## 3. A font is reported as a whole — measured, and the silence was half spaces

`FontError` and the "drew nothing" tally are the only channels a font has, so a font that maps
*some* of its document's codes draws those and says nothing about the rest. ADR 0152 measured the
alternative — reporting every uncovered code named 13 documents that mostly draw fine, and each
report costs the oracle a judged page (trap 11) — and chose "drew none" deliberately.

`Interpretation::codes_without_a_glyph` is the measurement that question needs, and the
four-hundred-and-thirty-fourth session found it was counting two different things (ADR 0270).
`LoadedFont::outline` answers `None` both where the routes of §9.6.5.4 and §9.7.4.2 **reached a
glyph the program contains** and describes with no contours — which is how every sfnt stores a
space — and where they reached **nothing, or `.notdef`**. Only the second is a mark the reader
loses, and `codes_reaching_a_blank_glyph` now holds the first.

**The split, over ADR 0269's 65 944 web documents and over the 974:**

| | web codes | web documents | corpus |
|---|---|---|---|
| a glyph the font describes as empty — not a mark missed | **28 837** | 359 | 57 over 9 |
| no glyph, or `.notdef` — a mark lost, before the fix | **22 435** | 277 | 5 over 2 |
| ADR 0269's total | 51 272 | 635 | 62 over 10 |

**The second row's *definition* changed in the six-hundred-and-eighty-fifth session and the
figures above are the old one** (ADR 0520). It had a third exclusion its own doc comment did not
name: a code §9.10.2 could not **name** was left out as well, which is a question about the reader
and not about whether the program answered. Counting it — the corpus's second row goes from
**5 over 2 documents to 129 over 7** and the first row does not move at all — the split's second
row is what `tools/state.sh` prints today rather than what is written here, and the web columns
predate the correction and would have to be re-crawled to be comparable. The one *report* it added
is `issue17333.pdf`, whose font drew nothing at all: 68 documents drawing incompletely to 69. The
four documents it added to the silent measurement without reporting are the population this
section is about, and they are worth naming because none of them is a substitution —
`issue20489.pdf` shows code 10, `issue18059.pdf` and `standard_fonts.pdf` code 0,
`issue6721_reduced.pdf` code 224 reaching `.notdef`, and `issue11403_reduced.pdf` a UTF-8 no-break
space written byte by byte into a simple font.

`pr12564.pdf`'s 26 — the corpus's largest contributor, diagnosed by hand in the
two-hundred-and-forty-fifth session — are the first row, and so is the web's largest:
`0300276.pdf` shows one `Identity-H` code 118 times whose `/ToUnicode` maps it to U+0007 and whose
glyph the font contains and draws blank. The whitespace exemption in front of the count cannot see
a font that reads its own space back as something else; the glyph index can.

**And the second row had one mechanism in it.** Five documents were 4912 of the 22 435 and all
five are Cyrillic through a standard-14 name: ten of §9.6.2.2's fourteen compiled-in faces are
Foxit's bare CFF, whose charsets carry the standard Latin character set and nothing else, so a
`/Differences` naming `afii10017` reached no glyph while the Latin codes of the same font drew.
`substitute_face` now replaces a substituted face with one of the same family whose code table
over Table 109's declared codes is a strict superset, and the population goes **22 435 codes over
277 documents to 780 over 236** — with the web's reports 1144 → 1138, six of them leaving because
the font they named now draws — so ADR 0269's second-largest reported population, a font with no
outline for any code the page shows, is **261 → 255 of 65 944**.

**So ADR 0152's trade holds, and this is the number rather than the opinion.** Reporting every
uncovered code would name 236 documents of 65 944 and 2 of the 974 to account for 780 codes, 231
of those documents losing four codes or fewer. What is left is characterised rather than counted:
a content stream showing **character code 0**, which reaches `.notdef` and reads back as U+FFFD or
U+FFFF — the mode of the distribution, 109 documents losing exactly four codes and 98 losing two;
an **embedded** subset missing a code it is asked for (`3867739.pdf`'s `$` and `>`, 43 of them);
and `4728077.pdf`'s 54, one `Identity-H` code 0 reaching CID 0. Nothing left has more than two
documents behind it.

**`issue14821.pdf` is the corpus witness and it splits the way its ledger rows say.** Five of its
eight are `Identity-H` CIDs whose `loca` entries are empty by the glyph table's own statement —
the first row above — and three are ASCII codes in a nonsymbolic `TrueType` subset whose `(3,1)`
`cmap` maps all three to glyph 0 and whose `post` is version 3.0 with no glyph names at all. Every
route §9.6.5.4 and §9.7.4.2 state ends at nothing. The refusal is on the handover's
closed-by-decision list; `poppler` draws them from a face this machine has.

**What this section still owes** is §1's chain: the superset rule takes one face or none, so a
document whose encoding no single face of its family covers still loses whatever the face in hand
lacks.

## 4. A fourth gap, half closed — the substitute's cap height, and its width

Not a code reaching no glyph but a glyph of the wrong size, and it is the only part of substitution
this tree has a *number* for. The compiled-in Helvetica is Liberation Sans; the reference renderers
resolve `NimbusSans` through `fontconfig`. Drawn straight from the two files, the capital `I` is
**0.687500 em** against **0.729167 em**, in the regular and the bold alike, and the corpus rasters
reproduce both exactly — `issue6108.pdf` at 12 pt draws 66 device rows against 70, `issue7580.pdf`
at 18 pt draws 99 against 105. That is 5.7% shorter capitals and 1.0% to 7.7% of the page's ink on
the `CONTRADICTED_SUBSTITUTED_FONT` pages naming a Helvetica or Arial face; the serif faces have
no such gap, and the advances have none in either family.

**And the six-hundred-and-eightieth session priced the whole substitution rather than the metric**,
which is what the gap is worth in the units a verdict is made of: rewriting each of that group's
eight documents with `gs -sDEVICE=pdfwrite` so that the face the references resolve is *embedded* —
after which every renderer draws one program — takes **seven of the eight inside every bound they
were failing**, the references rendering the rewritten file byte-identically to the original on
seven of the eight. `issue6108.pdf` is the exception, and the face owns 82% of its excess. So this
section's gap is not a residue beside a larger unknown; on these pages it is the whole of it. ADR
0510.

**It is left open on purpose** (ADR 0267): §9.5 NOTE 5 puts substitution beyond the standard,
§9.8.1 says a descriptor's metrics exist so that a processor may synthesise or select a substitute
and states no `shall` about it, and closing the gap by scaling to 0.729167 would be scaling to where
another program's font sits. **What would open it is a document** — a `/FontDescriptor` stating a
usable `/CapHeight` for a non-embedded face, which no corpus page has yet been shown to do.
`/CapHeight` is on §9.8.1's ledger row's list of Table 120 entries this tree does not read.

### The witness arrived, and it is about *width* rather than cap height

**`bug1671312_ArialNarrow.pdf`, found in the five-hundred-and-eighteenth session** at the head of
the ambiguous bucket's new ratio ranking — the pages where we sit further from every voting
reference than the closest two sit from each other. It is 1913 bytes, one line of text at 20 pt in
a non-embedded `/ArialNarrow`, and it states the whole of Table 120:

```text
/StemV 66  /StemH 66  /AvgWidth 362  /MaxWidth 833  /CapHeight 922  /Ascent 922
/XHeight 461  /Descent -210  /ItalicAngle 0  /Flags 32
/FontBBox [-250 -210 1000 1054]  /MissingWidth 238   and 224 /Widths
```

**Its `/CapHeight` is not usable and that half of item 4 stays shut**: 922 is also its `/Ascent`,
and Arial's cap height is 716 — the producer wrote the ascent into both entries, so a processor
scaling to it would draw capitals a fifth too tall. A witness has to state a *usable* number, and
this is the sharper form of item 4's condition.

**The width half is open, and this is the page that opens it.** Four measurements agree and
the picture says it in one look — our letters collide where the other four have clean gaps:

- the ink's bounding box is x[10, 149] y[15, 34] in ours against x[10, 147] y[15, 34] in
  `poppler`'s and `mupdf`'s, so §9.2.4's advances and the extent are already honoured;
- inside that box we mark **983 pixels against 844, 825, 812 and 702**, and our page ink is
  **18.45 of 255 against 15.52, 15.32, 14.97 and 12.71** — 19% to 45% heavier than four
  renderers within 2.9 of each other;
- at 576 dpi the modal dark run across the x-height band is **14 device pixels in ours and 12 in
  `poppler`'s**, where the `/StemV 66` the file states is **10.56** at that scale;
- **`hayro` is with the other three**, 10.41 of 255 from us, and it shares `skrifa` with this
  tree and nothing else — so this is the choice of face and not the rasteriser.

### The width half is closed, and the clause it rests on is Table 109's (ADR 0358)

**Taken in the five-hundred-and-twenty-third session.** The clause was read first and it requires
nothing of a substituted face's shape — not §9.5's NOTE 5, not §9.6.2.2's "[t]hese fonts, or their
font metrics and suitable substitution fonts, shall be available to the PDF processor", not §9.8.1,
not one row of Table 120. (**And §9.6.4 is not where a reader should look for that**, which this
file's own clause list above implies and which cost a round ten minutes to establish: §9.6.4 is
*Type 3 fonts*. The substitution `shall` is §9.6.2.2's and it is about *having* a face.)

What decided it instead is a sentence about the *file*, in Table 109's `/Widths` row: "[t]hese
widths shall be consistent with the actual widths given in the font program." That binds the array
to the program the document meant, so it states how wide the absent font's shapes *were* and not
only where the next glyph starts. `metrics::substitute_stretch` takes the median over the declared
codes of the stated width over the chosen face's own advance and applies it to the outline's **x**
alone; it condenses and never expands, because §9.2.4 makes a width a displacement, which bounds
ink from above and not from below. The witness's marked pixels go 983 → 861 against the four
references' 844/825/812/702, its page ink 18.45 → 15.28 against their 15.52 to 12.71, and its modal
stem at 576 dpi 14 px → 12 px against the `/StemV 66` the file states as 10.56 — a number nothing
in the derivation touched, which is what makes it the check rather than the target.

**And the pull request the owner supplied is answered by the standard rather than adopted.**
`mozilla/pdf.js#12725` lets a document override the built-in widths of a standard font and
justifies itself by experiment against Acrobat, which principle 5 refuses. Table 109's
`/FontDescriptor` row states it outright — "specifying them enables a standard font to be
overridden" — and §9.6.2.1's closing paragraph is the PDF 2.0 half of the same question, where
quoting ISO 32000-1 would have been the error. This tree had implemented both since
`simple_widths` was written.

**What is left of item 4 is the cap height and one population.** ADR 0267's condition is unchanged
and this file is still no witness for it (`/CapHeight 922` is its `/Ascent 922`; Arial's is 716).
And a substituted **composite** font is deliberately outside the new rule: §9.7.4.2 leaves it
reachable only by character through `/ToUnicode`, so no code has a `/W` entry and a face advance
that are two statements about one glyph. **The witness that would open it** is a document with a
`/W` array, a `/ToUnicode` and a non-embedded descendant whose collection a face on this machine
covers. There is also a known failure of the estimator with its own witness — `issue20489.pdf`,
whose `/Widths` is a third filler, so the median lands below the cluster its own letters are in —
and what would settle *that* is a file whose array says which of its codes it means; ADR 0358 has
both, with the mode that was measured and not taken.

## 5. A page that draws its text and can name none of it — counted, split, and one part closed

**The refusal is settled and the reading is in ADR 0311.** `french_diacritics.pdf` was the
sharpest named refusal in the text band, and §9.10.2 answers it in the clause's own words rather
than by a judgement of ours: all four of its routes are exhausted and the closing sentence says
"there is no way to determine what the character code represents in which case a PDF processor may
choose a character code of their choosing". A licence to invent is not a route, so no `aNNN` rule
is written and the printable-ASCII bound on §9.10.2's permission is not widened past 0x7E —
Annex D is what forbids it, because code 192 is `Agrave` in `WinAnsiEncoding`, `questiondown` in
`MacRomanEncoding` and unencoded in `StandardEncoding`. The sharpest fact in the reading is that
`a192` **is** a published Adobe glyph name: Annex D.6 puts it at code 218 of `ZapfDingbats`, so
following the one list that holds it would read `À` as an ornament, and §9.10.2 sends a reader to
that list nowhere.

**What was owed was not the answer but the voice.** A page whose fonts §9.10.2 cannot name and a
page with no text on it were the same `Interpretation` — an empty readback and `unsupported: []` —
so `Interpretation::codes_without_a_character` counts the codes a page showed and no method could
name, whatever was drawn for them. `tests/corpus.rs` prints it as the third silence line beside
the two ADR 0270 split, `examples/readback` prints it beside the glyph count, and `silent_fonts.rs`
holds `french_diacritics.pdf` at 29 glyphs, 28 unnamed and a readback of `1`.

**A count and not a report**, on ADR 0152's own trade: this is a shortfall in the readback and not
in the picture, and a report on each of the documents the third silence line names would cost the
oracle that many judged pages.

### The shape, and what is left of it

**Counted, and then split by cause** (ADR 0318). `pdf_font::NamingGap` says which of §9.10.2's
methods was the highest-priority one a font could have answered a code with — the clause ranks its
own methods, so that is the question worth asking — `Interpretation::codes_without_a_character` is
a counter per cause, and the census is a command rather than a sentence:

```sh
cargo run --profile gates -p pdf-model --example unnamed_code_census -- doc/pdf.js/test/pdfs/*.pdf
```

`PDFVIEWER_TRACE_UNNAMED_CODE=1` names each code, its font and its glyph name on stderr, which is
how the `UnlistedName` population was read.

**One population was the standard's to answer and is closed**: `ZapfDingbats`, whose glyph names
the Adobe Glyph List does not hold and whose **characters ISO 32000-2 prints itself**, in Table
D.6's `CHAR` column, under Annex D.1's "shall be predefined in any PDF processor". The refusal ADR
0311 recorded is untouched by it, because the route is keyed by code within *that font's own*
encoding rather than by name: `french_diacritics.pdf`'s Type 3 `/a192` is still counted, and the
corpus says so rather than the argument. It also closed a defect no count could see — a dingbat at
code 0x21 read back as `!`, §9.10.2's printable-ASCII permission applied to a font whose encoding
is not Latin.

**The rest is the clause's own answer and stays counted**, which is what the corpus gate's third
line and the census now say together, largest first: an `Identity` ordering with no `/ToUnicode`
(§9.7.4.2 leaves no question to ask), a glyph name no published list §9.10.2 names holds (pdfTeX's
`/aNNN`, dvips' `/GNN`, TeX's `circlecopyrt` — one code each in fourteen documents), a producer's
`/ToUnicode` that omits a code it shows, and a code selecting a glyph by code where the program
names nothing (the control range, and code 0 reaching `.notdef`).

**What would still change a number here** is a *file*, not a reading: a document whose §9.10.2 gap
is one of the shapes above and whose own tables answer anyway. The instrument to find it exists
now, and the honest expectation is that most of what is left does not move — a count that stops
falling where the clause says "there is no way" is the answer rather than a backlog.

### The band has a voice outside `pdf-model`, and that is what was owed rather than a smaller count

**Closed in the five-hundred-and-eighty-seventh session** (ADR 0422). The question this section had
left open was not the reading — it was what the program *says* — and it was asked in the wrong two
terms: report it, or leave it as a number in a struct nobody outside the crate could reach. The
reading was re-derived from the clause first and did not move: all three methods are implemented,
each falls through to the next **per code**, the third honours both of its disjuncts (an
`Identity-H` font whose descendant states a registered collection *is* inside it), and the second
and third cannot be tried out of order because a simple font and a composite one are what they are
conditioned on. `spec-errata emit` over clause 9 lands nothing on §9.10.2.

So there was no route unwalked, and reporting would cost the oracle 41 judged pages to repeat what
the standard already says. The third option is what was built: `Interpretation::shortfall` carries
this count and ADR 0270's two as one value, and three consumers read it —
`viewer_core::Query::Readback` beside `Query::Reports`, `viewer-accessibility`'s status group, and
`pdf-retrieve`'s `readback` object beside the text. The sentences are worded apart from a refusal's
on purpose: a code §9.10.2 ends at is the clause's own answer about a page that drew correctly, and
calling it a drawing fault would tell a person the picture is wrong when it is not.

**What is still owed here is a file and not a decision.** Nothing above changes the counts, and the
census remains the instrument for the day one of the four shapes turns out to have a document whose
own tables answer.

## 6. Two substitutes for one Type 1 family, wound in opposite directions — **closed in the five-hundred-and-sixty-first** (ADR 0396)

Found in the five-hundred-and-fifty-eighth by taking `doc/corpora/pdf-differences`
(`doc/todo/03` §14), and fixed in the five-hundred-and-sixty-first. **What the section is kept for
is the reading and the ordering it got wrong**, both of which outlive the three lines of code.

`OverlappingGlyphClipping.pdf` was the head of that corpus's ink ranking by two orders of
magnitude, and the difference was structural rather than a matter of glyph weight: where a glyph of
one substituted face overlapped a glyph of another inside §9.3.6's text clip, this tree left a
**hole** and every reference filled it.

**Three clauses decide it and the order matters.** §9.3.6 combines a text object's accumulated
outlines "into a single path … applying the non-zero winding number rule", and its NOTE 2 says that
"the direction of the paths comprising each glyph can cause different output for overlapping
glyphs" — so the standard has considered the case and calls it a difference rather than an error.
§9.5's NOTE 5 makes the *choice of face* ours. And §9.6.2.2 names its fourteen as "14 Type 1
fonts" — **one set, of one kind of program** — so a document may draw two of them into one path and
the two it names do not disagree about direction. This tree answered Helvetica with an `sfnt` and
Times with a bare CFF, which manufactured a disagreement the thing it stands in for does not have.

**So the rule is about the set rather than about the page**, and it is checkable in-tree with no
reference at all: *every compiled-in substitute for one of the standard 14 winds its contours the
same way.* `crates/pdf-font/src/standard.rs::every_compiled_in_face_winds_its_contours_the_same_way`
asserts exactly that, over all fourteen.

**What was taken is the third of the three options this section listed** — reverse at load, with
one amendment: the direction is **measured** (`Path::signed_area`) rather than keyed on the
program's format, because an OpenType face is an `sfnt` wrapper around CFF charstrings and is wound
the CFF way. So a machine-installed substitute is normalised too, which is broader than this
section's rule and costs nothing. The choice of *which* direction — counter-clockwise — is
documented as a choice, on two reasons that are not derivations: ten of the fourteen already carry
it, and §9.6.2.2 calls them Type 1 fonts.

**And this section owed "a population first", which was the wrong order.** The reasoning was that
outside the text-clip construction the defect costs nothing, so the pages it reaches should be
counted before anything is written. What that pricing missed is that the *fix* was three lines and
a measurement while the count was a corpus sweep — and that the property needed no population at
all, because it is a statement about `data/standard-fonts/` rather than about any document. A
count would have priced the symptom. **The general form is worth keeping**: when the thing that is
wrong is a property of this program's own data, a corpus census measures how often it has been
noticed, not whether it is wrong.

What the fix cost and what moved is ADR 0396: the witness at −8.989 → −1.116 of 255, 162 of 974
first-page display lists changed with every oracle metric line and every ink-sweep line
byte-identical, and 0.19% to 0.36% of interpretation on two substituted-text pages.

## 7. The form the producer chose — **taken in the eight-hundred-and-thirty-sixth** (ADR 0763)

A substitute is reached by character (§9.7.4.2) and §9.10.2's `-UCS2` table gives a character, so
the **form** a vertical `CMap`'s CID named was thrown away one step before the face was asked.
§9.7.5.1's NOTE is the sentence that makes that a loss — "in some cases, different shapes are used
when writing horizontally and vertically" — and `doc/corpora/pdf-differences`'s `VerticalText.pdf`
is the witness: `/Identity-V` over a non-embedded `Adobe-Japan1` `CIDFontType0` whose producer wrote
CIDs 7887, 7888, 7891, 7911–7916, drawn with horizontal brackets and centred punctuation on columns
`/DW2` already placed correctly.

`pdf_font::vertical` is the route, in two halves that are each a published table read for what it
says: the collection's own Unicode `CMap` pair says which CID is a vertical form
(`predefined::is_vertical_form`), and the chosen face's OpenType `vert`/`vrt2` feature says which
glyph that form is. ADR 0763 has the argument, the two designs that were declined and the
calibration.

**What is left, priced:**

- **A face with no `vert` drew the horizontal shape and nothing counted it — closed in the
  eight-hundred-and-thirty-seventh** (ADR 0764). The refusal to *report* stands and is ADR 0152's;
  what did not stand is "exactly as ADR 0270 left its neighbours", because ADR 0270 left its
  neighbours **counted** and this was counted by nothing at all. So the field was built:
  `Interpretation::codes_without_a_vertical_form`, `Shortfall::without_a_vertical_form`, and every
  consumer ADR 0422 gave the other three — `Query::Readback`, the confined pipe, `pdfv_readback_count`
  and `pdf-retrieve`'s `readback` object. The two silences are disjoint by construction: this
  question is asked of a glyph the face *reached*, so a character it cannot draw at all is
  `uncovered_character`'s and never gets here.

  The corpus gate prints it as a **fourth silence line**, and the population question has a command
  of its own — the clause's population out of the files' own dictionaries beside this machine's
  losses, because those are two different censuses (trap 13):

  ```sh
  cargo run --release -p pdf-model --example vertical_form_census -- --crawl   # or --pdfjs, or curated
  ```

  Its first crawl run found the witness `doc/pdf.js` does not have, and the shortfall turned out to
  be **per glyph rather than per face**: a face that supplies `VerticalText.pdf`'s brackets states
  no form for Adobe-Japan1's small kana. `PDFVIEWER_TRACE_VERTICAL_FORM=1` names each code with its
  character.

  **What that face actually carries was measured in the eight-hundred-and-thirty-eighth, and it
  closed the question the witness opened** (ADR 0765). `7311602.pdf`'s 33 codes are eight distinct
  small kana — っ ょ ァ ィ ャ ョ ッ ヶ, traced with the variable above — shown through non-embedded
  `HGMaruGothicMPRO` and `MS-Mincho` descendants stating Adobe-Japan1, and the face
  `installed_covering` picks for all four of the document's fonts is the same one: Droid Sans
  Fallback, the widest `cmap` on this machine that can draw あ.

  ```sh
  cargo run --release -p pdf-font --example vertical_feature_census
  ```

  It prints the collection's population beside the machine's, the way the census above prints the
  clause's beside the program's. Adobe-Japan1's own `CMap` pair states **251** characters with a
  distinct vertical form; of this machine's 2652 face files **5** can draw あ, **2** state any
  vertical feature at all, and both state `vert` alone — 86 single substitutions, supplying **46 of
  the 251**, none of them a small kana. **Not one face here states `valt`, `vhal`, `vkna`, `vpal`
  or `vrt2`.**

  **So consulting a second registered feature is not warranted, and the reason is a measurement
  rather than a reading.** There is no second feature on this machine to consult: the code path
  would be dead on every face the chooser can pick, which is §1's own objection — a feature whose
  only test says "on this machine, in August 2026" — with the sign reversed, because here the
  machine says *nothing at all*. The price of the gap is the other half of the same run: 205 of the
  251 forms are missing from the chosen face, so the eight small kana are 3.2% of a face-shaped
  hole rather than a feature this tree fails to read. What would open the question is a *face*, not
  a clause — one that states `vkna` or `vrt2` where `vert` is silent — and the command above is what
  says whether one has arrived.
- **Two collections have no pair to ask.** Table 116 publishes `UniAKR-UTF16-H` and no vertical
  counterpart for Adobe-KR — one of the four §9.7.5.2 requires — and Adobe-Japan2 is deprecated.
  Nothing can be derived for them from Table 116, and the only other route anybody has is a
  convention, which principle 5 forbids. **Closed unless Adobe publishes a pair**, rather than owed.
- **Only `GSUB` lookup type 1 is read.** A `vert` feature is one glyph for one glyph by
  construction, and a contextual or chained rule under that tag would be a statement about a
  sequence — which is shaping, and `doc/stack.md`'s standing refusal. "No face on this machine
  states one" was written as a sentence and is a command since the eight-hundred-and-thirty-eighth:
  `vertical_feature_census` prints the lookup shapes it found under `vert` and `vrt2`, and on this
  machine they are `single` and nothing else. A round that sees another shape in that line has a
  *measurement* to make before it has a decision.
- **The half-width vertical variants are not consulted.** `UniJIS-UCS2-HW-V` and its siblings state
  a second set of forms for the same collection, and the row here names only the proportional pair.
  A document whose producer chose a half-width vertical CID would be answered `false` and drawn
  upright. No corpus document does; the fix is one more row and the question is which pair wins when
  both name a CID, which wants a witness first.
