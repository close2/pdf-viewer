# 685 — The three clauses under six lost agreements

Parallel round, worktree `r685`, branch `round-685`. Takes the item ADR 0513 left owed and 684
carried forward: the six pages that stopped agreeing when a flat reference stopped voting. **All
three documents are read, from their clauses; one defect of this tree came out of them and is
fixed.** ADR 0520 has the readings, the closed forms and the before/after. **No pixel moves** —
the diff under `crates/` touches counters and a report and never the display list.

## The three readings, one clause each

- **`issue17333.pdf` page 1 — §9.6.5.4 runs out and its last sentence says so.** One `Tj`, one
  character code 0, an embedded two-glyph `SymbolMT` subset with `/Encoding /MacRomanEncoding`
  over `/Flags 32`. Walked to the end: no glyph name from MacRomanEncoding, none from the
  `StandardEncoding` fill, so both `cmap` rules have nothing to carry and `post` has no name to
  look up; the font's only subtable is a (1, 0) format 6 covering code 165 alone. What decides
  the page is *"a PDF processor may supply a mapping of its choosing"* — a permission, which
  ranks nothing. `mupdf` and `hayro` supply `.notdef` (glyph 0's two contours are the hollow box)
  and agree to **0.004 of 255**; `poppler`, `ghostscript` and this tree supply nothing.
- **`issue18042.pdf` pages 1–4 — a `DCTDecode` stream of four ASCII digits.** §7.4.1's filter is
  invoked and there is no original form; §7.4.8 requires ISO/IEC 10918 and **no clause states any
  artwork for an image that will not decode**. `mupdf`'s black rectangle is the closed form to the
  digit: §8.9.5.1's unit square over one sixteenth of the sheet is `255 ÷ 16` = **15.9375**.
- **`text_field_own_canvas_calc.pdf` page 3 — §12.7.4.3's regeneration is a splice.** The whole
  stored appearance is inside `/Tx BMC … EMC` and the field has no value, so the clause's closing
  paragraph replaces it with nothing (ADR 0032's reading, pinned both ways in
  `tests/variable_text.rs`). The closed form prices the departure exactly: `0.85` at eight bits is
  217, the rectangle is 200 × 20 of 612 × 792, `(255 − 217) × 4000 ÷ 484 704` = **0.313591**, and
  `ghostscript` and `hayro` each measure **0.313593** at a minimum channel of exactly 217. They
  draw the pre-regeneration artwork and nothing else.

**Two of the three are documented departures and are not chased**, which is principle 5 doing the
work it exists for: on the first the clause hands the answer to the processor, on the third the
clause says the mark the other two draw has been replaced.

**A correction to ADR 0513's figures, which is the instrument and not the renderers**: `hayro`'s
panels carry an alpha channel, so 0.262 and 0.2352 are 0.3498 and 0.313593 with `doc/todo/00` step
5's `-alpha off`. Exactly the factor of ¾ that step names.

## The defect, which is none of the three

`issue17333.pdf` drew **zero commands** and reported `unsupported: []` — `doc/todo/00` step 4's
worst shape, on a page that states a mark and whose clause ended without one.

`Interpretation::codes_without_a_glyph`'s own doc comment names two exclusions; the code had a
third nobody had written down — a code §9.10.2 could not **name** was excluded too. That gate is
right for the *reader's* question and wrong for the page's: whether the program answered is decided
by the glyph the code reached, and `.notdef` or no glyph at all is the program saying it has none
(§9.6.5.2, §9.7.6.3). So `Coverage::empty` stayed 0 and the report that fires on a font which drew
nothing of what it was asked to show had no operand. The branch divides on the glyph now, with the
whitespace exemption — the only one ADR 0270 measured — in front of it.

**Trap 11, printed rather than trusted**, before and after on one tree:

| | before | after |
|---|---|---|
| documents drawing incompletely | 68 | **69** |
| codes reaching no glyph *in silence* | 5 over 2 documents | **129 over 7** |
| codes reaching a glyph the font draws blank | 57 over 9 | 57 over 9 |
| codes §9.10.2 could not name *in silence* | 1226 over 41 | 1225 over 40 |

The incomplete diff is **one line**, and it is the page the round is about. The last row moves by
one document because `in silence` filters on `is_complete()`. The second row's four new documents
were opened rather than counted — code 10, code 0, `.notdef`, and a UTF-8 no-break space written
byte by byte into a simple font — and none of them reports, because each of those fonts draws the
rest of its page.

## Two tests that asserted the opposite, and had been wrong all along

The widened count failed two of them, both on one fixture — a `/Helvetica` whose `/Differences`
names `/a192` and `/a224`, pdfTeX's private labels, on a page showing `(A\300a\340)`.
`pdf-retrieve`'s said *"the substitute draws something for both: this is a reading gap, not a
drawing one"*, and `viewer-core`'s said *"a substitute drew all four"*. **Neither is true and
neither ever was**: a name no encoding defines addresses nothing in the substitute face either, so
the page states `AÀaà` and marks `Aa`. The count could not contradict the prose because it carried
the same exclusion. Both are corrected to 2, and the *decision* each test is about — that this
crosses as a count and not as a report — is untouched, because the font drew two of its four codes
and the report fires only on a font that drew none.

## Gates and sweeps

`doc/todo/02` §2 whole; §5's binaries rebuilt and installed. **The reference cache was pointed at
the shared one** (`PDFREF_CACHE`), deliberately and for the reason §2 gives: three parallel rounds
put the machine at a load average near 90, and the oracle's references run on a thirty-second wall
clock — 99.8% of its 6185 renders came from the cache, so the reference side of this run was taken
on a quiet machine even though ours was not.

**`doc/todo/00` step 7 is not owed and the reason is stated rather than assumed**: the diff under
`crates/` increments counters and pushes an `Unsupported`, and touches no `DisplayList` — our
rasters are byte-identical by construction, and the oracle reproduces every verdict count of 684
exactly.

Sweeps: `quotations` shows no hit in anything this round wrote; `quoted` names no figure in the
note this round rewrote, which is what removing its gate-vocabulary figures bought; `overtaken`
produces one hit this round created and it is the noise its own footer describes — ADR 0520 names
`standard_fonts.pdf` in a list of newly-counted documents, and that document happens to be a
member of `render-quorra`'s 300-name `DIFFERS_IN_SHAPE`, whose prose is about something else.
ADR 0495 collected the identical hit.
