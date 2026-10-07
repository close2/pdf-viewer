# 1415 — A popup's tabs are set, its scales drawn in Qt, and a right-to-left list tag stands right

Host slot of batch sixty-five, 2026-10-07. ADRs 1666, 1667. No ledger row moved (the popup's rows
are not this slot's). No question written.

**Premise.** Held: `grep -rn setFontStretch crates/viewer-qt` found nothing, the letter-spacing site
was `pango_span`, `toolkit_unapplied` was the note, round 1409's three items were as listed. Not as
stated: a tab never reached a host — `popup/rich.rs` turned it into a space and said it — so the
stops needed a hunk there (`RichParagraph::tab_interval`, `tab_stops`, `RichTabStop`, `RichTabAlign`,
a `'\t'` per stop), a one-line `parts` re-export in `rich_text.rs`, the re-export in `popup.rs`, and
the wire in slot 1's `protocol/panels.rs` and `protocol.rs` (greeting `PDFVCF09`, its round trip).

**Built** (ADR 1666). Tab stops in all three windows from one placing
(`viewer_host::popup::tab_stops`): `quorra` lays them out, Pango gets a tab array (`viewer-gtk` names
`pango` with `v1_50`; `gtk4::pango::` paths became `pango::`), Qt block tab positions; a paragraph
stating no stop loses its tabs; a tab in a right-to-left paragraph, a leader, and (toolkits) a tab
past the last stated stop with no interval are said. Qt's note is a `QTextDocument` built run by run
(`RichNoteView`): size × vertical, `setStretch` horizontal ÷ vertical, the face pinned by style name
(step 55 measured a condensed design matched at 83 px against 74 without the pin, 72 with it). Pango
has no per-run scale: said, steps 54 and 55 not offered in GTK. A share-of-space spacing is measured
in both toolkits. A list tag is at the start edge, isolated with LRI/PDI in the toolkits. C ABI
(ADR 1667): `quorra_popup_rich_tabs`, `_tab`, `QUORRA_RICH_TAB_*`, as stated; entry points 220 → 222.
Slot 5's hunk taken: `viewer-core/src/located.rs` calls `AffineRegistration::fit` (their patch).

**Gates.** `rustfmt --check --edition 2024` on the 17 touched `.rs` files: exit 0. `RUSTFLAGS="-D
warnings" cargo clippy --all-targets` on `pdf-model`, `viewer-core`, `-host`, `-ui`, `-confined`,
`-gtk`, `-qt`, `-ffi`: exit 0. `cargo nextest run` on the seven host crates: 890 passed; `pdf-model
--lib`: 593 passed. Planted (trap 13): the tab's stop and the right-hand tag each disabled, their two
panel tests failed; restored. `cargo test -p conformance`: 403 passed, exit 0. Behind the lock,
`tools/drive-windows.sh`: exit 0, 199 works, 0 wrong, 0 to look at, 6 not offered (the floor's 4 and
steps 54 and 55 in `quorra-gtk`; Qt's 54 now works). Steps 54 to 57 looked at (trap 1). `accessibility_census` not run:
no node changed.

**Unfinished.** A tab in a right-to-left paragraph is said, not laid out leftward (chapter 2, page
61), in all three; a leader is said. Slot 5's optional item — a window writing eastings for a
projected `/DCS` — not reached: `Geospatial::display` was not in the tree when looked for.
