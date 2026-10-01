# 1309 — The three windows driven end to end

The HOST-UI round of batch forty-eight. ADR 1453. No ledger row moved (the contract named none); no
question written.

**Driven.** Every step `doc/verify.md`'s drive list names, in `quorra`, `quorra-gtk` and
`quorra-qt`, release, `Xvfb` 1400×1100, on four fixtures `tools/drive-windows.sh` writes plus
`ArabicCIDTrueType.pdf`; every screenshot looked at. Seven defects found, two in all three.
After: every step works in every window but the Arabic find; `not offered` are GTK's
`/CenterWindow` (ADR 1429) and Control+wheel zoom in the two native windows.

**Fixed (ADR 1453).**
- A toggled check box was saved as a string `/V` with `/AS` unchanged, so all three windows
  reopened it unticked (§12.7.5.2.3): `ViewState::save` writes the name and each widget's `/AS`.
- `quorra`'s selection, find matches and Annex O rectangle hid their words: a `Multiply` into an
  overlay layer over nothing. Now a 0.45 wash.
- `quorra-gtk`: the outline list held the keyboard at launch, after a press on the page and after
  an unclaimed arrow, taking Home, End, `+`, `-`; the page area takes it and a bound key stops at
  the window. `r` opened an empty menu (the host was borrowed); deferred to idle. The pages panel
  moved on a double click only (§12.3.4).
- Shift+Enter in the find bar: previous in all three (GTK did nothing, Qt went forward).

**Found, left.** An Arabic word on the page is never found: the readback holds presentation forms
in visual order (`doc/todo/27`). Qt's popup body has no fill or border.

**Tests.** `pdf-model` `saving.rs` (two, failing with the fix planted out), `quorra`'s
`overlays::tests` (failing at full opacity); the GTK focus, menu and panel fixes are held by the
script's `04-key-home-after-left`, `05-zoom-keys`, `15-restrictions`, `17-nothing-dropped`,
`14-pages-panel` — GTK needs a display, which no gate has.

**Gates.** fmt, clippy (4 crates), nextest (pdf-model 1728; viewer-core, -host, -ui, -gtk, -qt
664), `conformance`, `launch_path` (26 banded, 0 outside), both censuses, `save_round_trip`: all
exit 0. The script's last run: 62 works, 3 wrong (the Arabic find), 10 looked at.

**Left.** The Arabic find's two folds; Control+wheel in GTK and Qt; the script's coordinates are
measured, not asked of the toolkit.
