# 1972 — Enter and Escape are told on a field's commit, and an XFA `exit` reads them as `commitKey` 2 and 0

Session 1972, a single ledger round (K.2), the ledger lane. ADRs 2755 and 2756; no question. **The slice, and why.**
`commitKey` 2 and 0 (XFA 3.3 pages 392, 398 and 402), which the brief put first: both need the same thing — a host
saying how a field's value ended — through `viewer-core`, the confined protocol, the C ABI and every host, so they are
one change to every host rather than two.

**The premise held, with one correction.** `Command::CommitField` carried no key, so a choice list's selection `exit`
refused `commitKey` by name; nothing abandoned typing. But two hosts did have an Escape in a field: `quorra` and
`quorra-confined` used it to give the keyboard back with the typing kept. Effort was the only obstacle: `ViewState`
already kept what a field showed before typing (a refused commit restores it).

**Built.** `pdf_model::xfa::Exited` (page 402's 0–3); `Stimulus::Exit::committed` takes it and `Stimulus::Selected`
gains `committed`. `ViewState::abandon_typing` restores the record and runs the one-call calculations again.
`viewer-core`: `CommitField { field, by: Committing }` (`Enter`, `Unstated`); a held choice list's selection `exit`
reads Enter as 2, an item click unoffered (within the field, none of page 402's clicks outside it). `CancelField {
field }` abandons the typing, logs `Done::AbandonField` so an undo's replay keeps it abandoned, and takes a focus held
on the field's widget off it: `/Bl`, and `exit` with 0. Confined protocol: a byte after `COMMIT_FIELD`'s name, kind 48
`CANCEL_FIELD`. Hosts: `quorra` (Escape cancels; a file-select's file is chosen at Enter instead), `quorra-confined`,
`viewer-gtk` (a key controller per text control), `viewer-qt` (`returnPressed` noted; Escape in the event filter,
before the window's shortcut), the WinUI host (`KeyDown`, the combo's `TextSubmitted`), `viewer-ffi` (four entry
points, 362→366).

**Tests.** `cascade.rs`: the exit test gains 0 and 2; a choice list's selection exit reads 2, unoffered on an item
click. `script_form.rs`: Enter in a focused choice list is 2; Escape in a focused field abandons and its exit reads 0,
a field not focused fires nothing. `field_commit.rs`: abandon restores, survives an undo's replay, a field nobody
typed into answers nothing. **Plants** (a patch, applied and reversed): 0 numbered 1 fails two; Enter mapped to
unstated fails one; the replay's abandon a no-op fails one; reversed, all pass.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` of `pdf-model`, `pdf-formcalc`, `viewer-core`,
`viewer-confined`, `viewer-ui`, `viewer-gtk`, `viewer-qt`, `viewer-ffi`: exit 0. nextest of the eight (after `cargo
build -p pdf-sandbox --bins`): 3356 of 3356, 28 skipped. `cargo test -p conformance` after `git add -N`: exit 0.
`state.sh --round 1972 docs`: exit 0. Tier 2 under `bounded.sh --lock`, in order: `pdf-model script_corpus` (its
population now holds safedocs' 65 968 PDFs), the `viewer-core` censuses, `viewer-confined awkward_classes`, `viewer-ui
launch_path`: 0, 0, 0, 0; `launch_path` 101, refused by the hang marker before it measured, owed to the clocks.

**Left of the row**: the other events; the `xdp` and `xml` packagings; an `execute`'s WSDL call; XML signing and
encryption; a submit at `off`; `commitKey` on a choice list's item click, a script's `setFocus` and a page turn, which
no page-402 value describes; the merge's format tests at `off`; a save's `cancelAction`; a transmitted submit's
`postSubmit` with no answer; RFC 0011 step 5's events.
