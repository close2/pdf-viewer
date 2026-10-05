# 1333 — A field's characters are placed, and a word gap is half a space

The HOST-UI round of batch fifty-two. ADRs 1501 and 1502. No ledger row moved; no question.

**Characters (ADR 1501).** §12.7.4.3's layout places every glyph of the appearance it writes, so
`variable_text` now answers `Asked::glyphs` from the same walk as the caret: each glyph's line, byte
range and box (a comb glyph its cell), through the `/DA`'s `Tm`, then `/R`, `/Matrix` and §12.5.5
onto the page (`appearance::glyphs`, `ViewState::field_glyphs`). `AccessibilityNode::value_lines`
carries them in device pixels, in display order (ADR 1465's convention); `PDFVCF06` → `07`;
`tree::held` builds the field's runs with the paragraph's `line_runs`. Census floor "field
characters with extents" 1568 (79 of 80 filled fields; the eightieth, `PDFBOX-3148-2-fuzzed.pdf`, is
named each run). Drive step `29-field-extents`: `GetCharacterExtents(1)` inside the field in `quorra`
and `quorra-qt`; GTK 4's own text widget answers the call with an error, so "not offered" there.

**Word gap (ADR 1502).** A scratch switch over 2808 pages, 1.54 M steps along a line: fonts stating a
space show a flat trough from 0.3 to 0.75 of it, fonts stating none one from 0.065 to 0.185 em.
Chosen: half a space (fewest steps near the threshold), the quarter em kept (between the stated
spaces' quartiles). `issue1453.pdf` now reads "The New Lady/BluCollection"; cost: TeX's thin space in
`cweb.pdf`'s math reads as a gap. Words 682 274 → 682 983 on 138 pages; against `pdftotext` 87
towards, 50 away. Floors: judged 509 → 510, cross-axis 9597 → 9650, A/B'd on one build.

**Driven** (release, Xvfb :133, lavapipe): 90 works, 2 wrong, 4 not offered. The two wrong are the
device-refusal steps: `27-processor-fallback` (the cycle page now stops at `MAX_LIST_BYTES` and the
device draws it) and `28-confined-refusal` (blank, "0 page(s) fall back"), both on in-flight sibling
display-list and raster work, not this round's files.

**Gates.** fmt (13 files); clippy `pdf-model viewer-core viewer-accessibility viewer-confined`
`--all-targets`: 0 in this round's files (a sibling's `image.rs` holds `-D warnings`); nextest those
four: 2211 passed, 0; `conformance`: 0; `text_extraction`: 0 (judged 510, pairs 12 573);
`selection_census`: 0; `accessibility_census`: 0 (field characters 1568); `launch_path`: 101 —
`xfa_filled_imm1344e.pdf`'s open at 1823.4 k instructions against a ceiling of 1820, beside a
sibling's in-flight per-command list budget (ADR 1507) on the same path; not argued here.

**Left.** Per-font word-gap thresholds would need a pass over the page before its first space; GTK 4
places no character of its own entry.
