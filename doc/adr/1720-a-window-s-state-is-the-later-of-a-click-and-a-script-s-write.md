# 1720 — A window's state is the later of a person's click and a script's write

Status: accepted and **built**. Session 1438. Amends ADR 1700 section 5's second unfinished item;
ADR 0191's session map is retired into the view state.
Code: `crates/pdf-model/src/view/script_annotations.rs` (`Annotations::windows` and
`scripted_windows`, `ViewState::set_popup_open`, `popup_opened`, `written_windows`),
`crates/pdf-model/src/view/scripts.rs` (`this.dirty` measures what a save writes),
`crates/viewer-core/src/open.rs` (`toggle_popup` writes the view state; the `popups` field and
`popup_is_open` are gone), `crates/viewer-core/src/viewer.rs` (`popup_windows` reads `Popup::open`).
Tests: `crates/pdf-model/tests/script_annotations.rs` —
`a_person_s_click_after_a_script_s_popup_open_wins`,
`a_script_s_popup_open_after_a_person_s_click_wins`, `a_person_s_click_alone_is_not_saved`.

## 1. What was wrong

A window's state had two homes. A person's click was `viewer-core`'s `Open::popups`; a script's
`popupOpen` was the view state's map, laid over Table 186's `/Open` by `pdf_model::popup`. The host
read its own map first, so once a person had touched a window no later script could open or close
it, and the realm read back a state the reader was not shown.

## 2. The choice

**One map, in the view state, and both sides write it**: `ViewState::set_popup_open` is what a click
does and what a script's write does, so whichever came later is the window's state, what
`crate::popup` draws and what the realm is told. Table 186 makes `/Open` how the window is
"initially" displayed, so the file states the first frame and the map every change since — the
division ADR 0191 drew, kept, with the log in the place the edit log already is (RFC 0008 section
6.4).

**Only a script's write is saved**, as before: a click is this sitting's, and Table 186's sentence
is about the next one. A window a script wrote is saved **as it is shown when the save is made** —
a later click that closed it saves `/Open false` — which is the rule ADR 1700 gave `Hidden` (a later
hide action wins). `this.dirty` measures that projection, so a click that changes what a save
would write is unsaved work and one that does not is not.

## 3. What stays

A click is not on `viewer-core`'s undo log, as it was not before; a replay leaves windows alone.
