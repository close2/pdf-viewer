# Round 2233 — a layer row's language, a title's own escape, and the accessibility gate's population

A fix round in the `audit` worktree: round 2231's leftovers from review 2229 (B04's layer half,
§7.9.2.2.2 in an outline title) and the accessibility gate's whole-population floors.

## What changed

- ADR 3633: a panel row carries the **runs** of language its text is in, `(byte, tag)`.
  `structure::language_runs` resolves §14.9.2.1's catalog `/Lang` against §7.9.2.2.2's escapes
  (an escape at 0 replaces it, a later one starts a run). The model keeps what decoding removes
  (`Item::title_languages`, `OptionalContent::name_languages`, `Presented::Collection::label_languages`);
  `viewer_core::Layer` carries `languages` on both variants across the confined wire, so
  `Answer::Layers` keeps its shape; `PanelRow::languages` replaces `PanelRow::language`; GTK sets
  a Pango language attribute per run; the C ABI gains `quorra_outline_language_run` and
  `quorra_panel_language_run` (header, C#, counts 394 and 375).
- 2229-B04 `fixed`. Review 2197's `outline_item_style_spec_audit.rs` gained
  `title_languages: Vec::new()` in an `Item` literal (compile only, as 2231 did for `Outline`).
- ADR 3634: the accessibility census's whole-population floors restated to the run with
  `ITU-T-H.264-202606.pdf` (tagged, 854 pages) and `ISO-IEC-14496-12-2012.pdf` (untagged) in
  `doc/`, both added to `RATCHETED_SPECIFICATIONS`: structure 114→115, pages 2659→3513, elements
  248 172→348 582, `/Alt` 673→682, placed 14 035→19 037, own marks 211 412→302 837, header cells
  24 095→27 824, associations 34 874→40 703, annotations 13 037→18 024, caret 135 497→189 341,
  lines 223 573→314 342, characters 5 700 393→8 701 438, untagged honest 901→902. `gate_ratchet`
  has no update mode; each bound is the run's printed figure, and the restated run passed.

Ledger: 14.9.2.1 `partial` → `implemented`; §14.9.2 and §14.9 derived back to `implemented`;
7.9.2.2.2 note and tests. Frontier map: the two §14.9 entries removed.

**Gates.** fmt exit 0; clippy `-D warnings --all-targets -p pdf-model -p viewer-core -p
viewer-host -p viewer-confined -p viewer-ffi -p viewer-gtk -p viewer-qt -p viewer-ui` exit 0;
`pdf-sandbox` 0; nextest `--lib --bins --tests` of the eight (`xvfb-run -a`): 4637 run, 4635
passed, 2 failed — confined's hand-written deep-layer bytes (fixed: a runs count) and GTK's clip
timing under load; both pass re-run; `audit-diff.py`: nothing else. rustdoc `-D warnings
--document-private-items` of the eight exit 0; `fuzz` check exit 0; `state.sh accessibility` exit
0; `cargo test -p conformance --no-fail-fast` exit 0.

## Left

1. Qt's item views and quorra's panels read no per-row language (no toolkit hook; ADR 3627).
