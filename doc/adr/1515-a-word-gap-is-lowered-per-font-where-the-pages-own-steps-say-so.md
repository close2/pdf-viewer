# 1515 — A word gap is lowered per font where the page's own steps say so

Session 1340. Status: **accepted**.
Context: `pdf-model`'s `content/text.rs` — `WordGaps`, `own_threshold`, `LEAST_WORD_GAP_SHARE`,
`STEPS_TO_DECIDE`, `GAP_TO_DECIDE`, `Interpreter::separate_text`, `Interpreter::truncate_readback`,
`Interpreter::settle_word_gaps`, `Interpreter::push_replacement` — and its three callers: `content.rs`'s `finished` and checkpoint,
`transparency.rs`'s readback mark, `run.rs`'s §14.9.4 replacement. Amends ADR 1502 section 3, whose
half a space it keeps as the default and lowers for one kind of font.

## 1. What the standard states

Nothing new: §9.4.4 places glyphs, §9.3.3 widens code 32, and §14.8.2.6.2 names any word break an
untagged page's reader infers a heuristic. Where a word ends is this program's choice (ADR 1502), and
this records a second choice on the same evidence, made per font instead of once.

## 2. The construction

The readback is written in the walk that draws, so a threshold read off the page's own steps cannot
be known when the first gap is met. The walk therefore puts a **provisional space** in for every gap
wider than a quarter of the font's space — the least any threshold may be — and records each step
(font, distance, space, codes in the show operation after it). When the page and its annotations are
finished, `settle_word_gaps` reads each font's threshold and takes back the spaces it rejects,
moving every range over the text (text layer, §14.9, §14.8.2.2, marked content, associated files,
structural annotations) to match. A cut of the readback (a rewind, a §14.9.4 replacement) forgets the
spaces it cut, so no recorded space can name a byte something else has since been written to.

## 3. The rule, and the variants it was chosen over

One build, a scratch switch (trap 95), the first sixty pages of every document under `doc/`,
`doc/pdf.js/test/pdfs` and the `PDFBox` inputs: 2808 pages. Evidence against `pdftotext` read as
token F1 (words with their punctuation stripped, as a multiset), because a page's word *count*
moves with leader dots that `pdftotext` joins to a page number; both counts are given.

| variant | pages changed | F1 better / worse | word count towards / away |
|---|---|---|---|
| widest gap in 0.25–0.75 of a space, ratio 1.5, either way | 169 | 27 / 14 | 22 / 43 |
| lowering only, ratio 1.5, run test | 60 | 14 / 3 | 9 / 21 |
| **lowering only, ratio 2, run test (chosen)** | **39** | **13 / 1** | 7 / 12 |
| the same, 20 steps · 80 steps · ratio 1.25 | 72 · 44 · 101 | 15/3 · 13/3 · 16/14 | |

The two-way rule joined each page number of an ISO table of contents to its leader ("....1") and split
"writers." from its full stop; raising a threshold bought nothing the lowering did not. So **the rule
only lowers half a space**: where a font shows at least forty steps on the page, the widest empty
interval reaching into 0.25–0.5 of its space, at least a factor of two wide, is its gap below its word
gaps, and its geometric middle is the threshold — provided most of the steps that moves lead into a
show operation of more than one code. That last test is what keeps letter-spaced type as it was
("Ta b l e" on `PDF32000_2008.pdf` page 31 without it): a step before a single code is a letter set
apart, not a word. The factor of two is what keeps `freeculture.pdf`'s kerned "c an" and "“A n", which
a factor of 1.5 split.

## 4. What it moved

39 of 2062 font-pages that show forty steps lowered, on 39 pages of 17 documents; words 682 983 →
683 316. Gains: `issue10640.pdf` ("LATEXsupportforOpenSans" → "LATEX support for Open Sans", F1 0.37 →
0.97), `issue17056.pdf` ("Linktopage1." → "Link to page 1.", 0.10 → 0.78), `ISO-19444-1-2019-preview.pdf`
pages 3 and 5, eight sentence breaks in `freeculture.pdf` ("ﬂy.The" → "ﬂy. The"), "Federal Tort" in
`PDFBOX-3042-003177-p2.pdf` (`pdftotext` agrees, `PDFBox`'s frozen text does not). Costs, named:
`freeculture.pdf` page 7's drop cap ("E NGUIN"), `cweb.pdf` page 12's "exp s." (as `pdftotext` reads it),
a URL's ": //" on one ISO page, a full stop set off on two `PDF32000_2008.pdf` contents pages, and
whitespace doubled where a gap meets a glyph that reads as white space (no word changes). The three
witnesses: `issue1453.pdf`'s display face is unchanged; `cweb.pdf`'s thin spaces still read as ADR
1502 says, since that would need a raised threshold; `tracemonkey.pdf` is unchanged.

Where every provisional space is kept or no font decides, the readback is the constant's byte for byte:
through the settle at half a space the text and the separator count equal the old single pass on all
2808 pages.

## 5. A code inside a replacement reads back as all of it

The A/B found one more difference, and it was not the rule's: on 50 pages the text-layer spans of
codes inside a §14.9.4 replacement moved. Those spans had always pointed at whatever bytes of the
replacement the cut readback had happened to occupy, so a provisional space before them shifted
them; emulating the old offsets in the settle removed all 50 differences, which is how that was
shown. The emulation was not kept, because what it preserved was wrong: "The ActualText value shall
be used as a replacement, not a description, for the content", so `push_replacement` gives each
enclosed code the replacement's whole span. `accessibility_census`, A/B'd on one build: elements a
caret reaches 121 463 → 135 428 over the whole population (lines 209 965 → 223 382, characters
5 681 052 → 5 695 133), 1382 → 1384 tracked; the word-gap rule alone moves none of the three, and
the 36 elements it seemed to cost before this were stale spans landing outside their element.

`text_extraction`, with the rule A/B'd on one build: word agreement unmoved (pdf.js 99.3%, `PDFBox` 99.8%), the
judged set 510, matched pairs 12 573 → 12 631 and cross-axis 9650 → 9708, the floor raised with
this reason. Fixtures: `content::text::tests::a_font_whose_word_gaps_sit_below_half_its_space_breaks_at_its_own_gap`
(fails under the constant), `letter_spaced_single_codes_keep_the_constant` (fails without the run
test), `a_space_taken_back_moves_the_ranges_over_the_text` and
`a_code_inside_a_replacement_reads_back_as_all_of_it`.
