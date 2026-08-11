# ADR 0270 — A silence that was half spaces, and fourteen faces that carry only Latin

Date: 2026-08-11 (session 434)
Status: accepted

## Context

ADR 0269 surveyed 65 944 web documents and printed, beside the report populations, one number
with no report behind it: **51 272 codes reaching no glyph in silence, over 635 documents**.
`doc/todo/21` §3 is the standing question that number belongs to. ADR 0152 measured the
alternative to the silence in the pdf.js corpus's terms — reporting every uncovered code named 13
documents that mostly draw fine, and each report costs the oracle a judged page (trap 11) — and
chose the silence deliberately; `doc/todo/21` then narrowed the corpus's own population to 24
codes over 8 documents, on which that trade is obviously right.

At 51 272 over 635 it may not be, and a code that reaches no glyph is a character the reader does
not see. So the question this round asks is what those 51 272 *are*, before it asks whether to
report them.

## Decision 1 — the count was two populations, and only one of them is a mark missed

The branch that counts a missing glyph is reached when `LoadedFont::outline` answers `None`, and
that answer collapses two different things. §9.6.5.4's and §9.7.4.2's routes either **reached a
glyph the program contains** — whose description has no contours, which is how every sfnt stores
a space and how a subset stores a character it was asked to carry and had nothing to draw for —
or they **reached nothing, or `.notdef`**, which is the program saying it has no glyph at all:
§9.6.5.2 substitutes `.notdef` where "an encoding maps to a character name that does not exist in
the Type 1 font program", and §9.7.6.3 substitutes "the glyph for CID 0 (which shall be present)"
where "no glyph exists for that CID".

`Interpretation::codes_reaching_a_blank_glyph` is the first of the two and
`codes_without_a_glyph` is now only the second. The split is `pdf_font::NOTDEF_GLYPH` against
`LoadedFont::glyph_index`, which `doc/todo/21` §3 had already identified as the distinction that
exists and is not used — "`outline` collapses the two and `glyph_index` does not".

**Over the 65 944 documents, before anything was fixed:**

| | codes | documents |
|---|---|---|
| reached a glyph the font describes as empty — **not a mark missed** | **28 837** | 359 |
| reached no glyph, or `.notdef` — a mark the reader loses | **22 435** | 277 |
| ADR 0269's total | 51 272 | 635 |

The two sum to 51 272 exactly and the document counts to 636, one document being in both, which
is what says the split is a partition of the same branch rather than a new measurement.

**Over the 974 pdf.js documents it is 62 over 10 → 5 over 2 and 57 over 9**, and the largest
contributor is the one `doc/todo/21` had already diagnosed *by hand*: `pr12564.pdf`'s 26 codes are
code 35 through `/TT3`, whose glyph 1 the program contains and describes with no contours, and
whose `/ToUnicode` reads it back as `#` — the document's own space. What was a paragraph of
argument in a todo file is now what the code computes. The whitespace exemption in front of the
count could never see it, and the web's largest single contributor is the same shape:
`0300276.pdf` shows one `Identity-H` code 118 times whose `/ToUnicode` maps it to U+0007.

`issue14821.pdf`, the corpus's other witness, splits the way its two ledger rows say it should:
five of its eight are `Identity-H` CIDs whose `loca` entries are empty by the glyph table's own
statement, and three are ASCII codes whose `(3, 1)` `cmap` maps them to glyph 0.

## Decision 2 — the mechanism behind the other half: §9.6.2.2's fourteen carry only Latin

**Five documents are 4912 of the remaining 22 435, and all five are Cyrillic.** Every one names a
standard-14 font — `TimesNewRomanPSMT`, which folds to `Times-Roman` — embeds nothing, and states
an `/Encoding` whose `/Differences` name `afii10017` and its neighbours, the Adobe Glyph List's
names for Cyrillic.

`substitute::find` answers a `/BaseFont` naming one of the fourteen **from the binary**, which is
ADR 0133's decision and the reason a machine with no fonts installed draws text at all. Ten of the
fourteen compiled-in faces are Foxit's bare CFF programs, and their charsets hold the standard
Latin character set and nothing else. So those codes reached no glyph — and because the *Latin*
codes of the same font drew, the "this font drew nothing" report never fired. The page lost its
Russian without a word.

This is session 405's finding one turn further round: ten of fourteen compiled-in faces were bare
CFF and unreadable by `skrifa`'s sfnt reader, which had made every serif substitution's widths
zero. The same ten faces, a different consequence.

**Trap 1, and it cut both ways.** `0423102.pdf` is 1336 of the missing codes and its page looks
*identical* before and after — 345 pixels of 1.1 million — because it is a scan with its text
layer under the image. `1407993.pdf` and `1284895.pdf` are the same. `0792341.pdf` is not: before,
a Serbian school's table of prizewinners is a grid of blank cells with the numbers floating in it;
after, it reads. `0300722.pdf` is a title page that was a coat of arms and a date and is now a
letterhead. A count is not a picture, and the pictures say both that the defect is real and that
its size is not the count.

## Decision 3 — a substitute is replaced only by a face that draws everything it drew and more

`substitute_face` builds the code table §9.6.5's encoding implies for the face `find` chose, and —
where some code the document *declares* is unanswered — asks
`substitute::installed_wider` for a face of the same family whose table is a **strict superset**
over that range. A page can gain marks and cannot lose one, which is what makes the rule safe to
apply to every substituted simple font rather than to a population somebody picked.

§9.5's NOTE 5 puts substitution outside the standard — "some details of font naming, font
substitution, and glyph selection are implementation-dependent" — so this is a documented choice
in a place the standard leaves open. Three parts of it were chosen against measured alternatives:

- **A comparison of tables, not ADR 0153's coverage of a character set.** That rule was tried
  first and is weaker here: `0546109.pdf` states a Greek encoding whose `/Differences` also name
  `controlSTX` and its thirty neighbours, no face on any machine has the C0 controls in a `cmap`,
  and a coverage test refuses every candidate over glyphs the page could never show. Its 17 Greek
  codes draw under the superset rule and not under the coverage one.
- **`.notdef` is not an answer**, and the same document decides it: eleven of its `/Differences`
  entries are `/.notdef`, which a name-keyed CFF has a glyph for and an sfnt reached through the
  Adobe Glyph List has no character for. Counting it made every sfnt candidate look worse than the
  face in hand. The rule is `pdf_font::NOTDEF_GLYPH`, the same constant decision 1 counts by.
- **A dictionary stating neither `/FirstChar` nor `/LastChar` has said nothing**, and nothing is
  decided from it. `declared_codes` widens that silence to all 256 codes, which is right for "does
  this face draw *any* of them" and wrong for this comparison — `franz.pdf` is `/Helvetica-Bold`
  with no `/FirstChar` and a `/Differences` naming `ff`, `ffi` and `ffl`, and it was the **one**
  page of the 974 whose raster moved: it had traded its typeface for three ligatures it never
  shows. ADR 0133 compiled the fourteen in so that a rendered page would stop being a property of
  the machine, and spending that on a glyph no page asked for is the wrong side of the trade.

**Only the family's own preference list is searched**, not the whole catalogue.
`installed_covering` walks the catalogue for a composite font because a Latin face is *no* answer
for a Chinese collection; here the face in hand is already the right shape and the search is for
the same shape with a wider repertoire, so leaving the family would trade a page's typeface for a
glyph. `installed` grew an `accept` predicate to make that walk possible — before this round it
returned the first family match and stopped, so a `Serif` request got `NimbusRoman`, which has no
Cyrillic, and never reached `LiberationSerif`, which has.

**What it costs.** One code table per candidate — 256 `cmap` lookups over a face already read for
the catalogue — and only for a font with a declared code the first face cannot answer. Every Latin
document pays one table build that `substitute_code_table` would have paid anyway. The five
witnesses together went from 3.9 s to **0.7 s** when the preference walk replaced the
catalogue-wide coverage search, which is the same measurement from the other side.

## Decision 4 — the silence stays, and now the numbers say so rather than an opinion

ADR 0152's trade was made on the corpus and re-measured here on the web:

| | codes | documents |
|---|---|---|
| session 433's silence | 51 272 | 635 |
| of which never a mark missed (decision 1) | 28 837 | 359 |
| a mark missed, before decision 3 | 22 435 | 277 |
| **a mark missed, after decision 3** | **780** | **236** |

**96.5% of the marks the web was losing in silence were losing them to one mechanism**, and the
answer to "should this be reported" is that it should be *fixed*. What is left is ones and twos:
231 of the 236 documents lose four codes or fewer, three more lose five, and the residue has
exactly two documents above that. Reporting every one of them would cost 236 judged pages of a
65 944-document population, and two of the 974 in the corpus's terms, to name 780 codes — the
trade ADR 0152 made is not close, and it is further from close than when it was made.

The residue is characterised rather than left as a number. `PDFVIEWER_TRACE_MISSING_GLYPH=1` now
prints the glyph index and the classification beside the readback, and over the survivors it names
three groups: a content stream showing **character code 0**, which reaches `.notdef` and reads back
as U+FFFD or U+FFFF (the mode of the distribution — 109 documents lose exactly four codes and 98
lose two, and they are these); an **embedded** subset missing a code it is asked for
(`3867739.pdf`'s `$` and `>`, 43 of them); and `4728077.pdf`'s 54, which are one `Identity-H` code
0 reaching CID 0. Nothing in the residue is a mechanism with more than two documents behind it.

## The gates

**Not one of the 974 page-one rasters changes, byte for byte**, which was checked by hashing all
974 before and after rather than inferred from unchanged verdicts. So `doc/todo/00`'s step 7 is
not owed, and the oracle's 1794 verdicts (905 / 68 / 786), quorra's 957 (911 / 35 / 11 / 17), the
text gates' 99.2% and 99.8%, the corpus's 68 incomplete and the dates, XMP and JPEG 2000 lines are
what `doc/HANDOVER.md` already says.

**On the web population the reports move by six, and every one of them leaves.** 1144 → **1138**
incomplete: `0300722.pdf`, `0546444.pdf`, `1653053.pdf`, `3006756.pdf`, `6327027.pdf` and
`7557933.pdf` each carried "font /X's program has no outline for any of the N code(s) the page
shows through it", and each of them now draws that font's text. A report leaving is the thing trap
5 does not cover, so it is named document by document and one of the six is in the pictures above.
ADR 0269's ranking moves with them: "a font with no outline for any code the page shows" was the
second-largest population this tree reports on real files at **261 of 65 944**, and it is
**255**. The whole pass is **65 944 documents in 1172.1 s, 0 failures of any kind**, with ADR
0269's 173 unopenable, 45 locked, 23 encrypted beyond us and 52 pageless reproduced exactly. (Its "2 slow" is
not reproduced and is not comparable: these runs shared the machine with the round's builds.)

`cargo nextest run --workspace` reports **1584** where session 433 printed 1580: two in
`silent_fonts.rs` for the two halves of decision 1's branch, and two in `pdf-font` for the
invariant decision 3 rests on and for the range a dictionary declines to state. A fifth test
changed its name and its assertion rather than being added: `variable_text`'s
`a_stand_in_that_cannot_draw_the_value_declines` is now
`a_stand_in_draws_the_whole_value_or_none_of_it`, because a form field whose `/DA` names a font
`/DR` does not define, and whose value is Arabic, now finds a face for the `/Differences` this tree
invents for it — on a machine that has one. The assertion is the equivalence, which is what holds
on both kinds of machine.

## Consequences

- **`substitute.rs` is machine-dependent again for a bounded set of documents**, and that is the
  cost ADR 0133 priced in the other direction. It is bounded by the rule: only a font whose
  compiled-in face cannot answer a code the document declares can leave the binary, and on the 974
  no page does. A machine with no fonts installed still draws exactly what it drew.
- **`doc/todo/21` §3's question is answered and its §1 is not.** A per-character fallback — a
  *chain* of faces, so that a document whose encoding no single face covers draws what it can —
  is still owed and still has no witness this round produced: the superset rule takes one face or
  none.
- The trace is now the instrument the characterisation used, so it prints `blank`/`absent`, the
  glyph index and the readback. The survey prints both counts and every document behind the first
  of them rather than a top ten, which is what a 65 944-document population needs.
