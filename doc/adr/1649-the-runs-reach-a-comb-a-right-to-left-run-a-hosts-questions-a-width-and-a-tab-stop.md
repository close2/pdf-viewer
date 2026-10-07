# 1649 — The runs reach a comb, a right-to-left run, a host's questions, a width and a tab stop

Session 1406. Status: **accepted**. Builds what ADR 1634 section 6 left to the next build and three of
the properties its section 1 reported; amends ADR 1634 (sections 1 and 6) and the scope of
`Owed::RichTextOneStyle`; none is edited.
Context: `crates/pdf-model/src/rich_text/layout.rs` (`Shaping`, `encode_items`, `Line`, `comb_line`,
`advances`, `tab_advance`, `Alignment`, `answer`), `crates/pdf-model/src/rich_text/style.rs`
(`STRETCHES`, `TabAlign`, `tab_stops`), `crates/pdf-model/src/rich_text/markup.rs` (`Piece::Tab`),
`crates/pdf-model/src/variable_text.rs` (`Order::from_levels`), `crates/pdf-model/src/appearance.rs`
(`rich_laid_out`); `crates/pdf-model/tests/rich_text.rs`. ISO 32000-2 §12.7.4.3, §12.7.5.3 (Table 231
bits 25 and 26), Table 120; UAX #9; XFA 3.3 section 27 (*Font*, page 1201; *Tab Stops*, pages 1205 to
1207) and chapter 2's *Tab Stops* and *Tab Leader Pattern* (pages 61 to 64), cited and never quoted.

## 1. One placement, read by the stream and by every question

A placed line now carries what each reader needs of it: its glyphs in display order, the order itself
(`variable_text::Order`, so a line of several styles is reordered by the arithmetic a line of one is),
the boundaries between its glyphs (`edges`), its ascent and descent, and the bytes of the string's
characters it holds. The stream writes each group at its edge, and a host's caret, point, range and
glyph questions are read off the same edges — so they are answered from the layout that was drawn,
where the one-style answer they had stood a caret beside a 24-point run at a 12-point distance.
The conventions are the plain layout's (ADRs 0211, 0225, 1501): a caret on the first line whose end the
offset reaches, between the line's descent and ascent; a point nearest a line, then a boundary; a range
one box per run of the line it covers; a comb's answers in cells.

**Offsets are the value's.** A host edits `/V` (or `/Contents`), and a rich text string states the
same words with its white space compressed (chapter 27, page 1194). `Alignment` maps one onto the other:
the characters that are not white space correspond in order, and a run of white space in one is the run
between the same two characters in the other. Where the two are one text — a value being typed, which
is drawn from `plain_text` — it is the identity.

## 2. A comb, run by run

Table 231 bit 25 divides the box into `/MaxLen` positions and lays the text into them; the cells are
the plain comb's (`variable_text`'s `comb`), filled left to right in display order, each glyph centred
in its cell at its own run's size, face and colour. Bit 24's question is asked in cells, as the plain
layout asks it; auto-sizing asks the plain layout's question, the whole advance against the box.

## 3. UAX #9 across runs, and joining across them

The levels are the whole string's — a run's direction depends on its neighbours — so `Levels` resolves
`RichText::text` once and each line is reordered by rule L2 over its glyphs' levels after rule L1 at
its ends, mirroring by rule L4 as the plain layout does (ADR 1413). Cursive joining is a property of the
character sequence, not of its styling, so the string is shaped once and a word set half in one colour
joins across the boundary; a form a face lacks is drawn as its letter and said
(`Owed::FormsNotInFont`, the plain layout's report, ADR 1414).

## 4. A width is a face, where `/DR` holds one

ADR 1634 section 1 reported `font-stretch` because §9.6.2.2 and §9.6.3 name no face by width. That
was right of those two clauses and not of the standard: Table 120's `/FontStretch` names chapter 27's
nine widths in the same order, so a run asking for one is set in the `/DR` face whose descriptor
states it, matched with family, weight and posture as ADR 1634 section 2 matches. Where none does,
the run is drawn in its family's normal width and the width is named in the report.

## 5. Tab stops

Chapter 27's *Tab Stops* (pages 1205 to 1207) and the grammar chapter 2 gives `xfa-tab-stops`
(pages 61 to 63): `tab-interval` sets default stops at every multiple of it beyond the stated ones;
`tab-stops` and `xfa-tab-stops` state stops with an alignment each; `xfa-tab-count` advances by that
many stops past the cursor. The text a stop aligns runs to the next tab or the line's end: its left
edge at a left stop, its middle at a centred one, its right edge at a right one, and at a decimal one
its first radix — or its right edge where it has none. **Three choices, each the chapter's to leave**:
the radix is the full stop (the chapter makes the locale's radix implementation-defined); a stop the
text would have to start behind the cursor to meet is met as near as the cursor allows; and the content
of an `xfa-tab-count` element, which the chapter lets a processor discard, is kept after the advance.
Stops are paragraph properties, as every example states them, measured from the left margin, and a
`br` restarts them. **Two cases are reported rather than drawn**: a leader other than the blank one
(chapter 2's *Tab Leader Pattern*), whose stop is still used; and a tab in a line read right to left,
which advances nothing, because its stops would be measured from the line's other side.

## 6. What stays

`kerning-mode:pair`, following an `a href`, `xfa:embed` and the algorithmic and Kana, Hangul and Iroha
list types are still named by `Owed::RichTextUnapplied`. `Owed::RichTextOneStyle` keeps one of its four
reasons: a character none of the runs' faces draws, in a face this program chose, takes the one-style
layout, which reaches a machine face (ADR 1414) — the runs' faces do not search the machine. That
includes a bidi control such as U+202E beside a run in a face this program chose, because no face of
§9.6.2.2's fourteen states a code for it.
