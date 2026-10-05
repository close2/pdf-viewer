# 1502 — A word gap is half a space, a choice read off the gaps the corpus states

Session 1333. Status: **accepted**.
Context: `pdf-model`'s `content/text.rs` — `WORD_GAP_SHARE`, `NOMINAL_SPACE_EM`,
`Interpreter::word_gap` — and every reader of `Interpretation::text`. Amends ADR 1490 section 1,
which set the two numbers at 0.6 of a space and a quarter em and left them untested.

## 1. What the standard states, and does not

§9.4.4 places each glyph and §9.3.3 adds `Tw` to the displacement of code 32; neither states a
quantity a reader compares a gap with, and nothing in clause 9 defines a word. §14.8.2.6.2 says
when the file itself states its words — a tagged producer puts the white space in, so that "the PDF
processor can determine word breaks without having to rely on heuristics based on information such
as glyph positioning on the page, font changes, or glyph sizes" — and §14.8.2.5 makes the readback's
order the content stream's on a page with no structure. So on an untagged page both numbers are
choices; the standard's silence is the reason to choose once, write the choice down, and choose by
the shape of the evidence rather than by matching another extractor.

## 2. The evidence: what gaps the corpus's pages state (one build, a scratch switch, removed)

Every step `separate_text` measures between show operations, over the first sixty pages of each
document under `doc/`, `doc/pdf.js/test/pdfs` and the `PDFBox` inputs — 2808 pages, 1 620 369 steps,
1 538 302 of them along a line; 5216 fonts, 885 stating no width at code 32.

- **Fonts stating a space** (1 233 476 steps), the step as a share of that space, per 0.05: 277 231
  and 56 622 below 0.1 (kerning), then 8327, 4694, 1428, 1241, **524, 1321, 532, 561, 684, 411, 762,
  560, 385** from 0.15 to 0.75, then 3119 at 0.75–0.8 and the word gaps above. A flat trough from 0.3
  to 0.75; a half is inside it, and so was 0.6.
- **Fonts stating none** (304 826 steps), in ems: kerning below 0.025 em, word gaps peaking at 0.25–
  0.3 em, and between them a trough from 0.065 to 0.185 em of 44 to 341 steps per 0.005 em (941 of
  the trough's steps are one document, the ISO 8601 working draft).
- **Steps within a quarter of the threshold either side**, the ambiguous band a threshold should
  sit in least: stated fonts 4634 (0.4), **4500 (0.5)**, 4620 (0.6), 7084 (0.7); fonts stating none
  1024 (0.10 em), 1136 (0.125), 1449 (0.15), 7261 (0.17).
- **Per font, the widest gap** (ratio of neighbouring steps between 0.02 and 1 em; 394 fonts with
  forty steps and a gap of 1.5 or more): the threshold falls inside it for 197 fonts at 0.4, 186 at
  0.5, 158 at 0.6, 110 at 0.7; of the 117 fonts stating no space, 86 at 0.125 em against 69 at 0.15.
  The middles of those gaps are not one number (quartiles 0.083–0.112 em for fonts stating none), so
  a per-font threshold would need a pass over the page before its first space is decided, which the
  readback's single pass does not have; a constant inside the trough is what is left.
- **The nominal space**: the spaces fonts do state have quartiles 0.226, 0.278 and 0.653 em (median
  0.278, Helvetica's). A quarter em lies between the first two; replacing it by 0.278 moved 8 pages in
  2 documents.

## 3. The choice

**A gap is a word break once it exceeds half the font's space** — nearer a space than no gap — and
a font stating none is read with a quarter em, so its threshold is an eighth of an em. Both sit in
the troughs above; the half is where the ambiguous band is smallest for fonts stating a space and
near it for fonts stating none. `issue1453.pdf`'s title, `[(The)-169(New)-140(Lady/Blu)-31
(Collection)] TJ` in an AvantGarde subset stating no space, reads "The New Lady/BluCollection":
0.14 em is a word gap and 0.031 em is not. The cost, named: TeX's thin space in a Computer Modern
subset (a sixth of an em) now reads as a word gap, so `cweb.pdf`'s `[LTMC→T]` reads `[LT MC→ T ]`.

## 4. What it moved, and the references as evidence only

Over the same 2808 pages: words 682 274 → 682 983 on 138 pages of 52 documents (0.4 would move 206
pages, 0.7 146). Against `pdftotext`'s page word counts, 87 of the 138 pages moved towards it and 50
away (0.4: 115 and 87; 0.7: 49 and 95); against `PDFBox`'s frozen text, 2 documents towards and none
away. `text_extraction`: word agreement unmoved (pdf.js 99.3%, `PDFBox` 99.8%, whitespace folded);
the word-box judged set 509 → 510 and matched pairs 12 513 → 12 573, cross-axis 9597 → 9650 — both
floors raised with this reason, each measured against 0.6 on the same build. Fixtures:
`content::text::tests::a_display_face_with_no_space_breaks_at_an_eighth_of_an_em` and
`a_gap_nearer_a_space_than_none_is_a_word_break`, each failing under 0.6.
