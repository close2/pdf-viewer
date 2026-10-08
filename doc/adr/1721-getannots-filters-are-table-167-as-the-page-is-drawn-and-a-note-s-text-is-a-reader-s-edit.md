# 1721 — `getAnnots`'s flag filters are Table 167 as the page is drawn, and a text note's text is a reader's edit

Status: accepted and **built**. Session 1438. Amends ADR 1700 section 2's filter and `contents`
choices.
Code: `crates/pdf-model/src/view/script_model.rs` (`AnnotationReach`),
`crates/pdf-model/src/view/script_annotations.rs` (`reach`; `contents` through
`set_note_text`), `crates/pdf-model/src/view.rs` (`set_note_text`, `note_text`, the save's text
note), `crates/pdf-model/src/popup.rs` (`retyped_note`), `crates/pdf-script/src/engine/annotations.rs`
(`filter`), `crates/pdf-script/src/wire.rs` (version 9).
Tests: `crates/pdf-script/tests/annotations.rs` (one line per filter, the three refusals, a text
note's `contents`), `crates/pdf-model/tests/script_annotations.rs`
(`each_annotation_is_told_where_table_167_lets_it_go`,
`a_script_retypes_a_text_note_and_its_window_shows_it`, `a_group_s_subordinate_note_is_not_retyped`).

## 1. The reference's filters, read

Adobe's *JavaScript for Acrobat API Reference* at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`,
"Doc methods", `getAnnots`, lists seven `nFilterBy` values: `ANFB_ShouldNone` (all), and six that
keep annotations that "can be printed", "can be viewed", "can be edited", appear "in the annotations
pane", "can be included in a summary" and "in an export". There is no `ANFB_ShouldNoView` and no
`ANFB_ShouldNoZoom`; `NoZoom` is a placement flag no filter asks about. Cited, never quoted in a
blockquote, each rule a documented choice under principle 5.

## 2. The choices

- **Print, View and Edit are §12.5.3's flags as this program draws and hit-tests by them**, asked
  of `crate::annotation::displayed` and `interacts` — one statement of each rule, so a filter and
  the page cannot come apart: Print is bit 3's three sentences on paper (an annotation with no
  appearance stream prints, the flag ignored); View is `NoView` with `ToggleNoView` inverting it
  under the pointer, so a pair that shows on hover can be viewed; Edit is a pointer's reach —
  `NoView` and `ReadOnly`. `Hidden` suppresses all three and is composed in the realm, so a filter
  reads the bit a script has just written.
- **`Locked` and `LockedContents` do not filter**: they are the document's restrictions on its
  reader, which a host applies at one of `CLAUDE.md`'s four levels (`crate::restriction`), and a
  script's question has no level to apply.
- **AppearInPanel, Summarize and Export are refused by name**: Acrobat's comments pane, summary and
  export, which no clause defines and this program does not have.
- **A text note's `contents` is a reader's edit** (`ViewState::set_note_text`): §12.5.6.4's window
  contains "the text of the note", so the retyping is held where a free text annotation's is, shown
  in the window (`/RC` only where its characters still agree) and saved as `/Contents` alone, the
  producer's icon appearance kept. A group subordinate is refused: §12.5.6.2 makes `Contents` a
  group attribute whose subordinate entries "shall be ignored". `contents` on the other fifteen
  subtypes stays refused by name; each that lands is counted by `this.dirty`.

## 3. What is left

No host lets a person type into a text note's window yet: the view state's edit exists and a host
command for it is a host round's. The census's seventeen attachments-shim documents never reach
`getAnnots`: the shim tests `app.viewerVersion < 7` and calls it only for `6 <= v < 7`, and this
program answers 0.1, its own release (ADR 1615).
