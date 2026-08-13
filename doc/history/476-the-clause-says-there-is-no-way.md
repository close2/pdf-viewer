# 476 — The clause says there is no way, and the refusal had no voice

**Finding.** The round was asked whether §9.10.2 has a **legitimate** route to a character for
`french_diacritics.pdf`'s `/a192`, `/a224` … — the sharpest named refusal in the text band — and
the answer is the clause's own: **there is none, and the standard says so in the sentence that
ends the list.** All four routes are exhausted (no `/ToUnicode`; a name neither the Adobe Glyph
List nor the Adobe Glyph List for New Fonts holds; no composite route, a Type 3 font being simple;
and the code itself declined because 0xC0 is outside the 0x21–0x7E band where §9.6.5's encodings
agree), and what follows is a **licence** — "a PDF processor may choose a character code of their
choosing" — rather than a fourth method. The sharpest fact in the reading is that **`a192` is a
published Adobe glyph name for something else**: Annex D.6 puts it at code 218 of `ZapfDingbats`,
as it does eight more of the file's twenty-nine names (`a49`, `a194`, `a196`, `a199`–`a203`), so
following the one list that holds the name would read `À` as an ornament — which the eighth session already paid for on the drawing
side, when a Type 3 font was substituted and the substitute drew dingbats. **So the refusal stands
and what was wrong was the silence.** The page draws all 29 glyphs, agrees with every reference,
reports nothing and reads back `1`; a page whose fonts §9.10.2 cannot name and a page with no text
on it were the same `Interpretation`. `Interpretation::codes_without_a_character` is that channel
— a count, never a report, on ADR 0152's own trade — and over the corpus it is **1342 codes over
45 documents**, against 5 over 2 reaching no glyph and 57 over 9 reaching a blank one. **The
reading band is two orders of magnitude wider than the drawing band and had never been counted.**
The round then took session 464's second item, the vacuous whitespace test, and the population it
asked for is the same one: `Readback` is now a three-state answer from the *font* rather than an
inspection of the *buffer*, which leaves both existing tallies byte-identical, argues the rule
they were following by accident, and fixes a second blindness nobody had named — inside
§14.8.2.5.3's reversal no code's text reaches that buffer at all, so every code there was being
called a space. **Every gate is identical**, which is what a round that adds an instrument and
corrects a reason should expect.

**Date.** 2026-08-13.
**ADR.** [0311](../adr/0311-the-clause-says-there-is-no-way-and-the-refusal-had-no-voice.md).
**Touched.** `crates/pdf-model/src/content.rs` (`Readback` and its two methods, `read_back`'s
return, `show_text`'s classification branch, `Interpretation::codes_without_a_character` and the
interpreter field behind it), `crates/pdf-model/tests/corpus.rs` (a third silence line and the
`silence` helper the three now share), `crates/pdf-model/tests/silent_fonts.rs` (two tests),
`crates/pdf-model/tests/accessibility.rs` (one test), `crates/pdf-model/examples/readback.rs`
(one line), `doc/conformance/ledger.toml` (§9.10.2, §9.6.4, §9.6.5.3), `doc/todo/21` (§5),
`doc/adr/0311-*`, this file.

## The numbers

Every gate ran before the first edit and after the last, and not one moved:

| gate | before | after |
|---|---|---|
| pdf.js text | 99.3% (24010/24189 words), 22 below floor, 62 not gated | identical |
| PDFBox frozen text | 99.8% (14257/14281) in both orders, 4 below floor | identical |
| corpus | 65 incomplete; 5 codes with no glyph over 2 documents; 57 blank over 9 | identical, plus the new line |
| oracle | 906 agrees, 67 contradicted, 786 ambiguous, 19 no render | identical |
| quorra | 918 agree, 37 differ, 1 refused, 18 not comparable | identical |

`fmt`, `clippy --workspace --all-targets`, `nextest run --workspace`, the doctests and
`cargo test -p conformance` are all clean; the last was re-run *after* the ledger edit, which is
the only order in which it means anything.

The new line is `codes §9.10.2 could not name *in silence*: 1342 over 45 documents`, and the ten
largest are `complex_ttf_font.pdf` 616, `issue5874.pdf` 130, `bug911034.pdf` 72,
`font_ascent_descent.pdf` 70, `issue15716.pdf` 64, `issue20489.pdf` 61, `ZapfDingbats.pdf` 50,
`issue11131_reduced.pdf` 37, `bug1146106.pdf` 34, `standard_fonts.pdf` 32.

**A flat text gate is the expected result and not a disappointment**, for the reason session 464
recorded: both gates strip whitespace from the comparison, and this round changed no readback at
all. It changed what the tree can *say* about the readback it already had.

## The two drafts that were wrong, and how the corpus said so

Worth writing down, because both looked right and one of them looked right for a whole gate run.

**Draft one counted every code with no readback as a mark missed.** That is the literal reading of
session 464's finding — an empty slice is not whitespace — and it took the corpus from 5 codes over
2 documents to **129 over 7**, the blank tally from 57 to 163, and produced **two new
`Unsupported::Font` reports**, which is two oracle pages lost. `PDFVIEWER_TRACE_MISSING_GLYPH=1`
named them in ten seconds: `issue7769.pdf` is one code through `/T1_1`, glyph 1, which the program
*contains* and draws blank, reading back nothing — and "this font drew nothing the document asked
for" is a claim about a page nothing in the file supports.

**Draft two moved `coverage.empty` into the no-glyph-at-all arm**, on the reasoning that ADR 0270
calls a blank glyph "not a mark missed", so it should not feed the report either. That is wrong,
and `silent_fonts.rs`'s oldest test caught it in the same minute: `issue13316_reduced.pdf`,
`recursiveCompositGlyf.pdf` and `issue20232.pdf` all lost their reports, and all three are **blank
pages** whose codes read back real characters — 开, `h` — through glyphs the program contains and
describes as empty. ADR 0270's split is about which of two things happened at the end of §9.6.5.4's
route, not about whether the page came out right. A gate that holds a *report* by name is what
makes a draft like that survivable.

What both drafts have in common is that they were changes to a *rule* justified by a reading of
another ADR, taken before the population they applied to had been looked at. The third draft
changed the rule's **reason** and left its behaviour where it was, which is what the round had been
asked to do in the first place: count it before doing anything to it.

## Why the count is not a report, stated once more

Forty-five documents, most of which draw perfectly. Trap 11's arithmetic is unchanged from ADR
0152's: each report takes a page out of the oracle's judged set, and this is a shortfall in the
readback rather than in the picture. What a count buys that silence did not is that
`doc/todo/21` §5 can now ask the next question with a number in front of it — what the 1342 are
made of — and `examples/readback` prints it beside the glyph count, so the instrument that shows
what a page reads back also shows what it did not.
