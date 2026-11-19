# 2105 — A rich value in a broken font, and a printed sheet's grid faults

Session 2105, a host round in `pdf-viewer-lane4`. ADRs 3121 and 3122; 3123, 3124 and Q927 unused.

- **Rich text in an unreadable font** (ADR 3121; §12.7.4.3, §9.5). The premise held, and was worse than stated: the
  brief expected the plain-text fallback, and with the change reverted the test finds no appearance at all, the
  widget reported "its /DA's font is unusable". `variable_text::Face::load`, which every face of a rich layout goes
  through, now takes `LoadedFont::substitute_for_unreadable` as `LayoutFonts::load` does and keeps the refusal;
  `rich_text::layout::lay_out` reports `Owed::FontSubstituted` where a glyph was set in a substitute, in the plain
  layout's order. The variant's sentence now says "a font program its value is set in", since a run's face may be
  a `/DR` face of its family. Test: `pdf-model` `forms_suite::rich_text::a_rich_string_in_an_unreadable_font_program_is_laid_out_in_its_substitute`
  (a `/FontFile2` of no program, a green span). **Plant**: the substitution dropped from `Face::load` fails it.
- **ADR 2848's open item** (ADR 3122; §7.10.5.3, §8.7.4.5.2). The premise as worded no longer held: ADR 2946 (round
  2066) built the screen's caller — `viewer-core` asks after every `RenderReady` and the empty `Declined` — after the
  2063 record restated the item. What remained of it is the route ADR 2946 named uncovered, a printed sheet:
  `print_page` worded its reports before the host drew the sheet and kept no watch. `PrintPage` now carries the print
  reading's `AfterDrawing` and answers `faults_after_drawing()`; GTK, Qt and the C session ask it after the sheet's
  draw, a refused one included. The confined print answer is not covered: a type 1 shading is `Uncodable`, so its
  page does not cross. Test: `viewer-core` `function_faults_after_drawing::a_printed_sheets_grid_fault_is_asked_after_the_sheet_is_drawn`.
  **Plant**: `print_page` handing `AfterDrawing::default()` fails it. ADR 2848's item is closed here.
- **Ledger.** 12.7.4.3 and 9.5 name the rich test; 7.10.5.3 names the print test and `query.rs`, and its note the
  sheet's question. No status moved.
- **Golden pages.** None can move: no corpus page's interpretation changes unless a rich field's font program is
  unreadable, and printing is not in `raster_golden`.

**Gates.** Over `-p pdf-model -p viewer-core -p viewer-ffi -p viewer-gtk -p viewer-qt`: fmt `--check` exit 1 once (the
new print test's chain), then 0; clippy `-D warnings --all-targets` exit 0; `cargo build -p pdf-sandbox --bins` exit
0, then nextest `--lib --bins --tests` under `xvfb-run -a`, unpinned, exit 0, 3706 passed, none failed, so nothing
differs from `expected-failures-current.json` (empty) and it is unchanged; rustdoc `-D warnings
--document-private-items` (private target) exit 0; conformance after `git add -N` exit 0. `fuzz/` names no
type this changed.
