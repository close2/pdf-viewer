# 3041 — A file-select control's file is read at its commit: ADR 3021 §1 built

Session 2086. Status: **accepted, built.** Builds ADR 3021 §1, whose record stays as written ("accepted, not built").
Context: review 2038 (finding D55). Clauses: ISO 32000-2 §12.7.5.3 (`FileSelect`), §12.7.6.3.

## Decision

ADR 1216's rule is "once, when the person finishes with the field. Not per keystroke", and every window now keeps it
through one type rather than three key handlers.

- `viewer_host::form::edit_of` is gone. While a person types, every field's text is a `SetField`, a file-select
  control's path included. `viewer_host::commit_of` turns the path into `Edit::ChooseFile` with the file
  `policy::read_chosen` read, for a file-select control with a path.
- `viewer_host::TypedPaths` keeps the last path typed into each file-select control until its commit. GTK's
  `FieldChange::Commit`, Qt's `commit_control` (`editingFinished`) and the platform layer's `commit_field` take
  their commands from `TypedPaths::commit`. `viewer-ui` reads at Enter through `commit_of`.
- **A refused read abandons the typing** (`CancelField`) instead of committing. Before, a refused keystroke sent
  nothing. Now the path is already in the log as typed text, and committing it would put a path with no file behind
  it under the field's name (trap 5). The refusal is said.
- A chooser's answer is a commit in itself: GTK's chooser sends the commit after it fills the entry, and Qt's emits
  `editingFinished`.
- `policy::chosen_reads` counts the reads a process made. The tests read it.

## Consequences

- D55 is `fixed`. The count is held in `viewer-host` (`host_mappings::a_typed_path_is_read_once_at_its_commit`), in
  Qt by driving `set_control` per character and `commit_control` on the real host, and in GTK by typing into the real
  entry and pressing Enter, with its `FieldChange`s carried through `TypedPaths` as the host's sink carries them.
  `viewer-ui`'s window has no headless constructor, so its read is counted through `commit_of` only.
- §12.7.5.3's ledger note names `commit_of` where it named `edit_of`; its status does not move.
