# 1490 — A word gap is measured in text space

Session 1327. Status: **accepted**.
Context: `pdf-model`'s `content/text.rs`, `Interpreter::separate_text` and
`Interpreter::text_space_step`; the readback every consumer of `Interpretation::text` reads (search,
selection, accessibility, extraction). Builds on ADR 1477's mirror reading in `select::axes`, which
left this half open.

## 1. The defect, and the clause

`separate_text` infers a space or a newline between two show operations from where the pen moved,
and compared that movement — the text matrix's translation, in the space `Tm` maps into — with
`word_gap`, a fraction of the space glyph's displacement, which is a text-space length. ISO 32000-2
§9.4.4: "Both the glyph's shape and its displacement (horizontal or vertical) shall be interpreted in
text space." Two quantities from two spaces agree only where `Tm`'s linear part is the identity and
`Th` is 100.

- **A mirroring `Tm`** (`-1 0 0 1 x y`): a `TJ` adjustment that moves the pen back against the
  advance — a right-to-left word placed glyph by glyph in reading order — became a step forward along
  user-space x, larger than the gap, so the readback held a space between every glyph and the word
  was not found.
- **A scaling `Tm`** (`1 Tf` under `12 0 0 12 x y Tm`, which many producers write): a gap of a
  hundredth of an em was twelve times larger than its threshold, so tracking and kerning read as word
  breaks ("reser ved", "nt erna tio nal", "PDF A ssociation"), and half a point of baseline shift as a
  new line.
- **A font stating no space** (a subset without code 32) was given `0.25 × Tfs` as its threshold —
  the whole quarter em, where a stated space gets 0.6 of itself. Its producer's word gap is a `TJ`
  of about `-250`, exactly that quarter em, so `tracemonkey.pdf`'s title read
  "Trace-basedJust-in-Time" before this change and the text-space reading spread the defect to every
  such font under a scaling `Tm`. The fallback is now a nominal quarter-em space under the same 0.6.
  The 0.6 and the quarter em are choices, as §14.8.2.6.2 says any such heuristic is; they were not
  tuned to the corpus, and pages a different choice would read differently remain
  (`issue1453.pdf`'s 0.14 em gaps in a display face now read as one word, where `pdftotext` splits).

## 2. The reading

The step between the last glyph's end and the new show string's start is taken back through the
inverse of `Tm`'s linear part and divided by `Th` (sign and all), which is the space `tx = (w0 ×
Tfs + Tc + Tw) × Th` is stated in with `Th` removed — the space `word_gap` and `Tfs` measure. In it
a glyph advances along +x when `Tfs` is positive and along −x when it is negative (§9.3.1's NOTE
permits a negative size), so "along" is the x component times the sign of `Tfs`; in vertical writing
it is −y times that sign, the column running down. A `Tm` or a `Th` that collapses text space has no
inverse, and the step is read as it stands. Fixtures: `content::text::tests` (a mirrored `TJ`, a
gap opened under a mirror and under `-100 Tz`, `1 Tf` under a `Tm` of 20 against `20 Tf`, a `-250` in a font stating no space), each
checked to fail under the old reading; `headless.rs`'s three-ways page under the mirroring `Tm`, 3
found where the old reading found 2; `drive-windows.sh`'s `25-find-mirrored` in all three windows.

## 3. What it changes on the corpus

An A/B over the same build, the old reading behind an environment switch (scratch only, removed):
over `doc/`, `doc/pdf.js/test/pdfs` and the `PDFBox` inputs, the first sixty pages of each, **1205
of 2809 pages in 109 of 1050 documents read back differently**; on them inferred spaces fell from
494 913 to 447 862 and newlines from 69 686 to 64 956. A mirrored `Tm` is a handful of these; the
scaling `Tm` and the space-less fonts are the rest. `text_extraction`'s word agreement did not move
(pdf.js 99.3%, 24709 of 24888; `PDFBox` 99.8%) because it compares words with whitespace folded;
the word-box gate's judged set rose from 503 to 509 documents and its matched pairs from 11 131 to
12 513 (cross-axis 8266 → 9597, 500 of 509 documents fully in bounds), and both floors were raised
with this reason beside them.
