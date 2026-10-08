# 1723 — A text note with no popup opens a window of its own, beside its icon

Session 1439. Status: **accepted** and **built** in `pdf_model::popup` and so in every window that
draws one. Builds on ADRs 0191 (a popup is a host's window), 0294 (Table 175's `/Open` opens the
window), 1700 (a script's `popupOpen` on a note with no popup writes Table 175's `/Open`) and 1720
(a person's click and a script's write are one entry, the later wins).
Context: ISO 32000-2 §12.5.6.4, Table 175, §12.5.6.2 (Table 172's `/IRT` and `/RT`), §12.5.1.
Code: `crates/pdf-model/src/popup.rs` (`own_window`, `OWN_WINDOW`; `popups` reads a text
annotation; `Popup::annotation` and `rect` say so); `crates/viewer-core/src/interact.rs`
(`has_a_window`). Tests: `viewer-core`'s `a_text_note_with_no_popup_opens_a_window_of_its_own`,
`a_text_note_with_no_popup_is_opened_and_closed_by_its_activation`; `tools/drive-windows.sh` step 63
in the three windows.

## 1. The premise, and what the clause asks

Round 1432 saved a script's `popupOpen` on a text note that states no `/Popup` and no window showed
it. The brief supposed `pdf_model::popup` already drew a window for a popup-less note's reply; it
draws none — `popups` walked `/Popup` annotations alone, so neither a script, nor a click, nor the
file's own Table 175 `/Open true` opened anything for such a note. §12.5.6.4 is the rule: "When
closed, the annotation shall appear as an icon; when open, it shall display a popup window
containing the text of the note in a font and size chosen by the interactive PDF processor." The
window is the note's whether or not a popup annotation describes it, and Table 172's `/Popup` is
Optional.

## 2. The construction

`popups` now gives a text annotation that states no `/Popup` and no `/IRT` a window of its own,
keyed by the note — the object a click names, and the one ADR 1720's map and ADR 1700's save already
key such a note's state by. Its text is read as a parentless popup's (`read`): the note's own
`/Contents`, `/T`, `/M`, `/C`, `/Subj`, `/CreationDate` and `/RC` through §12.5.6.2's group rule,
with a person's retyping in its place; it opens where Table 175's `/Open` says so, then as a person
or a script last said. §12.5.1's activation reaches it (`has_a_window`). A note stating `/IRT` has
none: a reply is shown together with what it answers, and a subordinate's window is its primary's.

## 3. Where it goes, a documented choice

The clause states the window's text and its font and size as the processor's, and nothing about its
place. **Three inches by two (216 × 144 points of default user space), its top-left corner at the
icon's top-right one**, moved to the icon's left where it would cross the crop box's right edge and
up where it would cross the bottom — beside the note, so that it covers no more of the page than a
window has to, and in user space, so that it scales with the page as a stated `/Popup` rectangle
does.

## 4. What it moves, and what is left

The corpus has no popup-less text note stating `/Open true` (`examples/open_annotation_census`: 28
text annotations, one open, and that one's popup states its own), so no document opens a window it
did not before; a click on any of the others now opens one. Left: a popup-less *reply* is still shown
nowhere — `popups` folds only replies that state a popup into a thread — which is §12.5.6.2's
threading, not this clause's.
