# 1290 — A chrome label reads in its own direction, and Qt's panels take the machine's face

The HOST-UI round, batch forty-five. ADRs 1417, 1418. No ledger row moved (the contract named none);
no question written.

## Before (driven under `Xvfb`, three windows)

Fixtures: an outline with "الفصل 12: السلام" and "مقدمة (أ)", one with "פרק 3 שלום" and "מבוא (א)",
a file named `ملف.pdf`, and one outline mixing Chinese, Arabic and Latin.
- `quorra-gtk` (Pango) and `quorra-qt` (`QTextLayout`): joined, UAX #9's order, brackets mirrored,
  the tab `pdf.ملف`, every row left-anchored. But `quorra-qt` drew a Chinese outline title as boxes.
- `quorra`: stored order left to right, isolated forms, `(أ)` unmirrored; the find bar the same.

## Built

- `pdf_font::shaping::Label` (additive): joining, then UAX #9 over the label as one paragraph and
  one line, L4 mirroring, and `boundary` for a caret by ADR 1413's convention.
- `viewer-ui`'s chrome: `Chrome::laid_out` is the one place a label becomes glyphs; `width`, `text`,
  `caret`, `without_a_code` go through it; the find bar's caret follows display order; an elided
  label is re-measured as drawn. `quorra-confined` shares the path (it draws no panel).
- The direction is the label's own (P2/P3), never `/Lang` or `/Direction`: §12.3.3, §14.9.2.1,
  Table 147 read, the standard silent, recorded as a choice (ADR 1417). Rows stay left-anchored.
- `quorra-qt`: panel rows and page labels ask `machine_faces` as tabs do; `pumpFaces` sets every
  panel view in the registered families (ADR 1418). `quorra-gtk` unchanged.

**After.** `quorra` draws the three fixtures as GTK and Qt do; Qt draws the Chinese title.
Launch, outline panel open with Arabic items, release, `Xvfb`: first present 121–124 ms cold and
119–132 warm after, against 126–128 and 134–150 before; no frame's host step above 0.5 ms either
side. `Label::new` costs 0.9–2.7 µs a label.

**Tests and gates.** `label::tests` (5), `chrome::tests` (3, each failing with the reordering planted out), `viewer-qt`
`a_chinese_outline_title_asks_the_machine_for_a_face`, `viewer-gtk`
`an_arabic_outline_title_reaches_pango_as_stored`. Tier 2: `launch_path` (26 banded, 0 outside),
`accessibility_census`, `selection_census` — all exit 0.

**Left.** A `GSUB`-only face is not chosen for a form; a label may change face inside a word; a popup's
wrapped paragraph takes its direction per line.
