# 2424 — `App`'s keyboard is continued in five modules, by what a key press does

Session 1795, the standing code-quality slot of batch 119. Status: implemented; no behaviour changed, no bug fixed. ADR 2425 is unused.

`tools/state.sh gods` named the impl block of `App` in `crates/viewer-ui/src/bin/quorra/typing.rs` first (1 039 lines of
twenty-five methods, in a file of 1 354), the one ADR 2406 left next.

**What it is now.** `typing.rs` keeps the types (`Typing`, `Target`, `Aim`, `Pressed`, `Choosing`, `Drawing`), the caret
arithmetic (`caret_boundary`, `before`, `after`, `spliced`, `stepped`) and its tests, 315 lines. Beside it, in `typing/`,
each declared with `#[path]` the way `surface` is:

- `clipboard.rs` — `copy_selection`, `copied`, `clipped`: what a copy, a cut and a paste carry;
- `aiming.rs` — `aim_at_field`, `aim_at_free_text`, `aim_at_focus`, `aimed`, `control_at`, `single_line`,
  `takes_characters_at`, `commit_on_enter`, `toggle_button`: which control a press or a focus addresses;
- `choices.rs` — `open_choices`, `choices`, `close_choices`, `press_on_choices`, `arrow_on_choices`, `commit_chosen_file`;
- `annotations.rs` — `press_on_note`, `draw_free_text`, `note_offset`, `note_line`, `typed_into_note`;
- `keys.rs` — `typed`, `typed_into_value`: one press while a field has the keyboard.

**Why these seams.** A reader asking "what does Ctrl + V do" or "how does a drop-down take a row" finds the file by the
question; the shared caret arithmetic stays in the root so every child reaches it through `super`.

**Cost.** Eight methods a sibling calls became `pub(super)` (E0624 named them); the types' private fields are reached
by the children as descendants of the module that defines them. Each child carries its own `use` lines, pruned by the
compiler's unused-import warnings. The hypothesis (four to six modules, root under 500) held: five, root 315.

Alternative not taken: separating the field's keys from the note's keys at the `typed` seam alone (two files, the
aim and choice code left in the root at about 700 lines).
