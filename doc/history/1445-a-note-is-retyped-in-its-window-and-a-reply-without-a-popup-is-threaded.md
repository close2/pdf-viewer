# 1445 — A note is retyped in its window, a reply without a popup is threaded, and a leader crosses the C ABI

Slot 2 of batch seventy, 2026-10-08, a host round. ADRs 1726, 1727; no row moved, no question.

**Premise.** Held for (1): `set_note_text` existed and nothing reached it; what was also missing was
a way to name the note, since a window is keyed by its popup and its text is a square's in some.
Held for (2): `popups` read a reply only through a popup it stated. Held for (3): `abi_note` added
"a tab leader in a popup window". Found: `quorra-gtk` clipped every thread below its window's edge
(trap 132's overlay child, allocated its narrowest-width height); `quorra-confined` draws no popups.

**Built.** (1) `Edit::SetNoteText` and `PopupWindow::note` (`pdf_model::popup::retyped_by`,
`retypable`); a press on a note's window gives it a `GtkTextView` (controls' layer, the press read
in `pointer`) or a `QPlainTextEdit` (Escape taken off its shortcut override), and in `quorra` the
keyboard at the note's end with the window ringed (ADR 1726); an edit naming no note is said, not
logged; wire `PDFVCF11`. (2) A markup reply stating no `/Popup` is a comment in its thread, the
climb reaching a note's own window, and a later write to the window stands over a folded `/Open`
(ADR 1727) — a hunk in `pdf-model`'s `popup.rs` and `is_markup` made `pub(crate)` in
`submission.rs`, outside this slot's files. (3) Six C entry points: the note and its retyping, a
stop's leader and its text, and the windows' grid; `leader_unapplied` deleted.

**Drive.** Steps 64 (press, " typed", Escape, save: `/Contents` "A red word. typed") and 65 (707,
857 and 886 blue pixels of the reply in the open note's window, 0 closed), photographed and looked
at in the three windows.

**Unfinished.** Taken from slot 3 (ADR 1728): no window calls `viewer_host::policy`'s policy fetch —
a `viewer-core` event carrying each signature's `published()` list, answered as `NeedsFile` is, and
`restriction::SUBMITTING`'s words with it; this slot changed only `viewer-host`'s Cargo comment.
`quorra`'s note caret cannot move inside the note. `launch_path` not run: `Command::Open` unchanged.

**Gates.** rustfmt `--check` on my files: clean. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model
-p viewer-core -p viewer-host -p viewer-ffi -p viewer-confined -p viewer-ui -p viewer-gtk -p
viewer-qt --all-targets`: exit 0. `cargo nextest run`: the seven host crates 923 passed, 0 failed;
pdf-model 2 003 passed, 0 failed. Two planted defects (no popup-less reply read; no window naming
its note) each failed one new `headless` test. `cargo check --manifest-path fuzz/Cargo.toml`: exit
0. `cargo test -p conformance`: exit 0, 417 passed. Behind the lock, `--tree 6`: steps 63 to 65 in
the three windows exit 0, 9 works (53 s); `--tree 12`: the whole drive exit 0, 226 works, 0 wrong,
0 to look at, 6 not offered (707 s).
