# 1676 — A styled check box is saved as its two constructed states

Status: accepted and **built**. Session 1420. Supersedes ADR 1665 section 3 (the saved file stating
the glyph in `/MK` and leaving the states to Table 224's `/NeedAppearances`); builds on ADR 1617
(a saved file shows what the viewer drew) and ADR 1425 (a machine face is subset and embedded).
Code: `crates/pdf-model/src/appearance.rs` (`ForSaving::States`, `SavedStates`, `toggling_states`,
`on_state`), `crates/pdf-model/src/view.rs` (`Update::write_states`, `Update::form_stream`).
Tests: `crates/pdf-model/tests/script_properties.rs`
(`a_style_is_the_check_box_caption_and_its_states_are_written`,
`a_style_with_no_on_state_is_owed_and_flagged_in_a_direct_form`).

## 1. What the page draws, and what the file now says

A check box or radio button whose script set `style` is drawn by constructing it (ADR 1665): its on
state the glyph Table 192's `/CA` names in the `/DA` font, its off state the background and border
alone. ISO 32000-2 §12.7.5.2.3 keeps each state "defined by an appearance stream in the appearance
dictionary of the field's widget annotation", and "[t]he appearance for the off state is optional
but, if present, shall be stored in the appearance dictionary under the name Off". So a save now
constructs both states — the widget posed in each by its `/AS`, the entry the clause has decide
which appearance is used — and writes them as Table 170's subdictionary of states under `/N`. The
next reader draws the glyph from the file, and the widget is no longer in `Written::unconstructed`,
so `/NeedAppearances` is not written for it.

## 2. The choices

- **The on state keeps the name the file gives it**: the one state of `/AP /N` other than `Off`,
  or, with no states dictionary, the widget's `/AS` or the field's `/V` where either names one. The
  clause names only the off state, so where the file names no on state, or two, nothing is written
  and the widget is owed as before.
- **Both states are written, not the on state alone**: the page draws the off state as the
  construction too, so keeping the producer's `Off` would save a picture the viewer did not show
  whenever the script also changed `/MK`. An off state that draws nothing is an empty stream.
- **New objects, never the producer's numbers**: generated forms share one `Off` stream among
  many widgets, and replacing it would redraw them all. The producer's streams stay in the file,
  unreferenced from this widget, as §7.5.6 keeps every earlier byte.
- **`/R` and `/D`, where stated, name the same states**: Table 170 defaults each to "the value of
  the N entry", and a constructed widget draws one appearance per state whatever the pointer does;
  a stored down state would show the producer's glyph the moment the box is pressed.
- **The appearance dictionary is written onto the widget**, not into an indirect `/AP` another
  widget may share.

## 3. Looked at

The saved fixture rendered at 6 px per unit with no script: ✘ inside the border in the on state,
the border alone with `/AS` set to `Off`. The glyph sits at the left of the box, which is
§12.7.4.3's quadding 0 that the construction applies on the page as well.
