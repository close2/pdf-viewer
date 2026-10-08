# 1439 — A popup's leader is painted in both toolkits, and a note with no popup opens its own

Slot 2 of batch sixty-nine, 2026-10-08, a host round. ADRs 1722, 1723; no row moved, no question.

**Premise.** Held for (1): ADR 1690 §4's composition was one step away in both toolkits. Did not
hold for (2): `pdf_model::popup` drew no window for a popup-less note or its reply — `popups` read
`/Popup` annotations alone, so neither a script, a click nor the file's Table 175 `/Open` opened one.

**Built.** (1) A leader in `quorra-gtk` (a drawing area over the label, the label's `pango::Layout`
giving each tab's extent, the run's span appended per repetition to a snapshot whose node draws on
cairo) and `quorra-qt` (`RichNoteView::paintLeaders`, `cursorToX` either side of the tab, `QtTab`
carrying the leader); the stop reached, the grid and the rule pieces are `viewer_host::popup`'s,
which `quorra` now draws by too. Both toolkits give every tab's extent in both directions, so the
toolkit windows no longer say a leader; the C ABI still does. (2) `popups` gives a text note with no
`/Popup` and no `/IRT` a window of its own, keyed by the note, three inches by two beside its icon (a
choice, ADR 1723); activation reaches it (`has_a_window`). A one-function hunk in `pdf-model`'s
`popup.rs`, outside this slot's files: the window needs `view.popup_opened` and `note_text`, which
are `pub(crate)`, and `popups` is what every window reads.

**Drive.** Steps 62 (dots, a 3-point rule, a `+` content leader, and a right-to-left paragraph's
dots, each reaching 246 to 354 px and stopping short of the stop) and 63 (599 or 720 red pixels
opened by `popupOpen` and by `/Open`, 0 closed), photographed and looked at in the three windows.

**Unfinished.** A popup-less *reply* is still shown nowhere (§12.5.6.2's threading folds only
replies that state a popup). Qt's super- and subscript dots stand on the baseline. `launch_path` not
run: `Command::Open` is unchanged; the repaint's `Query::Popups` reads a text note once more.

**Gates.** rustfmt `--check` on my files: clean. `RUSTFLAGS="-D warnings" cargo clippy -p viewer-core
-p viewer-host -p viewer-ui -p viewer-ffi -p viewer-gtk -p viewer-qt -p viewer-confined
--all-targets`: exit 0; `cargo clippy -p pdf-model --all-targets`: no warning. `cargo nextest run`:
viewer-host, -ui, -gtk, -qt, -core 782 passed; viewer-core 350 passed; viewer-ffi, -confined 132
passed; pdf-model 2 002 passed. A planted `own_window` that returns nothing failed both new
`headless` tests. `cargo test -p conformance`: exit 0, 417 passed. Behind the lock, `--tree 12`: the
whole drive exit 0, 220 works, 0 wrong, 0 to look at, 6 not offered (668 s).
