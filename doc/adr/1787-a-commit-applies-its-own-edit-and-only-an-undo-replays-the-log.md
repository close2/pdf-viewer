# 1787 — A commit applies its own edit, and only an undo or a redo replays the log

Session 1475. Status: **accepted** and **built**. Amends ADR 0120's edit log as `viewer_core::Open`
implements it, and the trial ADR 1592 made of a refused keystroke. Context: ISO 32000-2 §12.6.3
Table 199; RFC 0008 section 6.5; ADRs 1579, 1592, 1786.
Code: `crates/viewer-core/src/open.rs` (`Open::apply`, `commit`, `replay`),
`crates/viewer-core/src/viewer.rs` (`refused_keystroke`).
Tests: `crates/viewer-core/tests/script_keystrokes.rs`.

## 1. What was found

Driving ADR 1786 under Xvfb, one arrow-made selection logged two keystrokes: the earlier
selection's option with `keyDown` true, and then the new one. `Open::commit` pushed the edit and
then replayed the whole log from a cleared state. So every `set_field` and every `commit_field`
logged before ran again at each new edit, and so did every script they hand to the runner. A field
typed into character by character ran its `/K` once for every earlier character at every keystroke.
Each replayed run read the keys the window held now, not the keys it held then. And
`refused_keystroke` tried each typed value on a copy of the view state before logging it, so a
script the runner runs was handed every keystroke twice.

Table 199's `/K` "shall be performed when the user modifies a character … or modifies the
selection". A run at the next edit is a run when nobody modified anything.

## 2. The decision

- **`Open::commit` applies the new entry to the state and no other.** The state already is the
  replay of the log up to the cursor: an undo or a redo replayed it, and each commit since applied
  its own entry. So applying one entry gives the state a full replay would, wherever every entry is
  a function of the state before it. A script is not such a function, since its realm keeps what
  earlier runs did. That is why applying once is the true history and replaying is not.
- **`Open::replay` stays for undo and redo**, and their scripts run again. That is the cost of
  ADR 0120's choice of replay over inverses: rebuilding the state means running the entries. A
  person who undoes asked for the state to be rebuilt.
- **`refused_keystroke` tries the copy only for a one-call keystroke.** The `AF` library runs in
  this tree without the runner, so trying it is free of side effects. A script's own rejection is
  the runner's result, and its trial would have been a second run.

A script's write to a field is no longer cleared by the next edit's replay. It stands until an undo
or a redo rebuilds the state, where the scripts run again.
