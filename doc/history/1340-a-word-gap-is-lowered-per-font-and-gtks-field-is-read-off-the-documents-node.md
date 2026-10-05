# 1340 — A word gap is lowered per font, and GTK's field is read off the document's node

The HOST-UI round of batch fifty-three. ADRs 1515 and 1516. No ledger row moved; no question.

**Word gap (ADR 1515).** The walk keeps a provisional space for every gap over a quarter of the
font's space and records each step; `settle_word_gaps` reads each font's threshold when the page is
finished and takes back the rest, moving every range over the text. The rule only lowers half a
space: forty steps, an empty interval reaching into 0.25–0.5 at least a factor of two wide, and most
of the steps it moves leading into more than one code. One build, scratch switch, 2808 pages: 39
pages of 17 documents change (words 682 983 → 683 316); token F1 against `pdftotext` 13 better, 1
worse; the two-way 0.25–0.75 rule first built moved 169 pages, 27/14, and joined a contents page's
numbers to their leaders. Gains `issue10640.pdf`, `issue17056.pdf`; witnesses `issue1453.pdf` and
`tracemonkey.pdf` unchanged, `cweb.pdf` only page 12. At the constant the text is byte-identical.

**Replacement spans (ADR 1515 section 5).** The A/B's 50 differing pages were codes inside §14.9.4
replacements whose spans pointed at stale bytes; each now spans the whole replacement
(`push_replacement`). Elements a caret reaches 121 463 → 135 428 over the whole population.

**GTK (ADR 1516).** GTK 4.22.5's `GtkEntry` reaches AT-SPI's `Text` through `GtkEditable`, which
answers `GetCharacterExtents` `NOT_SUPPORTED`; a bare `GtkText` (built, driven, reverted) answers a
0 × 0 box because GTK asks `get_extents (offset, offset)`; a widget of our own needs GTK 4.16. The
document's node did answer, but in the page area's pixels: `Reading::at` now carries the page area's
place in the window as the document node's transform. Its character 1 lies inside GTK's entry.

**Driven** (release, Xvfb :142): 99 works, 0 wrong, 3 not offered; `29-field-extents` works in all
three windows.

**Gates.** fmt (my files, one by one): 0. clippy `pdf-model viewer-gtk viewer-accessibility
viewer-core --all-targets`: 0 in my files (a sibling's `tests/banded_decodes.rs` holds `-D
warnings`). nextest those four: 2140 passed, 0 failed. `conformance`: 0 but `records`, which fails
on siblings' 1341 and 1343 records. `text_extraction`: exit 0 (cross-axis 9650 → 9708, pairs
12 573 → 12 631). `selection_census`: exit 0. `accessibility_census`: exit 0 (floors raised as
above; field characters 1568). `launch_path`: exit 0 (load 14.6, clocks unjudged). Drive: exit 0.

**Left.** `viewer-qt` places its bridge by the window's client area, not the page area's (ADR 1516
section 3, unmeasured); TeX's thin spaces still read as word gaps (a raised threshold cost more).
Files outside the brief, one targeted edit each: `content.rs`, `content/run.rs`,
`content/transparency.rs`.
