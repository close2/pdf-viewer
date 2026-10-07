# 1409 — A popup's runs are each in their own face, one paragraph's order, and a C caller reads them

Host slot of batch sixty-four, 2026-10-07. ADRs 1654, 1655. No ledger row moved (§12.5.6.2 is slot
1412's row). No question written.

**Premise.** Held: `draw_rich` was the one site, `quorra.h` had no rich entry (`grep -c rich` 0),
round 1403's three unfinished items were as listed. Not as stated: the spacing and the two scales
were not carried at all — `RichRun` had no field for them — so drawing them needed a hunk in slot
1412's `popup/rich.rs` (three fields, `RichSpacing`, the two phrases removed from `unapplied`), and
one-line re-exports in `rich_text.rs` (`parts::Spacing`) and `popup.rs` (`RichSpacing`).

**Faces, order, spacing, scales** (ADR 1654). `quorra`'s layout moved to `chrome/rich.rs`: a run is
set in the one of §9.6.2.2's families its `font-family` path reaches (metric-compatible names
included), else the family `pdf_font::substitute::Request::derive` classifies its first name into,
said nowhere (CSS2's matching ends in the user agent's face); faces load on first use. A paragraph's
runs are one UAX #9 paragraph: levels and joining over all of it, rules L1/L2 per line by glyph, L4
mirroring; an unaligned right-to-left paragraph starts at the right (a choice; Qt's `html` now writes
no `align` for it). A glyph advances `(w × em + spacing) × h` and is `em × v` tall, a field's rule.
GTK and Qt draw `letter-spacing`; neither markup states a glyph scale, so both say it
(`toolkit_unapplied`). The confined wire carries the fields; greeting `PDFVCF08`.

**C ABI** (ADR 1655): runs, not XHTML — six entry points (`quorra_popup_rich`, `_paragraph`, `_run`,
`_text`, `_family`, `_unapplied`), `quorra_rich_paragraph`, `quorra_rich_run`,
`QUORRA_RICH_ALIGN_*`; note 0 is the window's, `reply + 1` a reply's; the C program reads a red bold
Courier run spaced 2 pt off the form fixture. Entry points 214 → 220.

**Gates.** `rustfmt --check --edition 2024` on the 15 touched `.rs` files: exit 0. `RUSTFLAGS="-D
warnings" cargo clippy --all-targets` on `pdf-model`, `viewer-host`, `-confined`, `-ui`, `-gtk`,
`-qt`, `-ffi`: exit 0. `cargo nextest run` on the six host crates and `viewer-core`: 886 passed;
after the last edits the six: 545 passed; `pdf-model --lib`: 584 passed. Planted (trap 13): the
face walk and the visual order each disabled, their two panel tests failed; the ABI's spacing
zeroed, the C program's test failed; restored. `cargo test -p conformance`: exit 101 on one test,
`records`, naming only slot 1412's record, still being written. Behind the lock,
`tools/drive-windows.sh`: exit 0, 190 works, 0 wrong, 0 to look at, 6 not offered (the floor's 4 and
step 54 in the two toolkit windows); steps 50 to 54 looked at (trap 1). The first drive was lost to
an edit of the script while it ran. `accessibility_census` not run: no node changed.

**Unfinished.** Qt and GTK draw no font scale (Qt's route is a document built with
`QTextCharFormat::setFontStretch`, not its markup); a list tag stays at a right-to-left paragraph's
left; tab stops are still said.
