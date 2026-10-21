# 2202 — Script methods follow the events they handle

Status: accepted. Session 1684. Context: todo 74, ADR 2018.

The script-related `ViewState` implementation mixes committing a field, dispatching
runner events, applying edits, recalculating fields, queries, keystroke decisions
and host-facing request/state bookkeeping. Those are seven concerns with different
entry points. Move its 58 methods into seven `view/scripts/` children: `commit`,
`events`, `edits`, `calculation`, `queries`, `keystroke` and `state`. The root keeps
all types, limits, traits, shared free functions and the `Typed` construction.
No field, method body, literal, bound, event ordering or public signature changes.

Visibility retains the original boundary. The 17 `pub(super)` methods become
`pub(in crate::view)`, because another module level must not narrow their callers.
Nine formerly private methods used across the new children become
`pub(in crate::view::scripts)`: `run_field_script`, `field_page`, `apply_edits`,
`window_view`, `read_words`, `rich_value_of`, `set_calculated`, `write_text`, `revert`.
That is their old module's scope, not a new caller population. Other private
methods stay private in their child. The root's fields and declarations keep
identical tokens. Paths formerly relative to the parent `view` module now name
`crate::view`, including documentation links. Children import explicit parent names.

The proof compares every method/signature token stream, preserving string and raw
literal contents, after only these visibility/path substitutions and rustfmt's
optional trailing function-argument commas. The original source is byte-identical
to fixed baseline 020a86f3. The integration agent independently checked every
literal against that original. ADR 2203 states the limits of this mechanical proof.
Teaching and ledger pointers follow the moved methods; statuses do not change.

Validation: all 58 token streams and root declarations pass the calibrated identity
check. Scoped conformance, formatting, warnings-denied Clippy and rustdoc exit 0;
2134 model tests pass, 21 existing tests are skipped (97.321 seconds). The shared
421-second hold peaks at 3.91 GiB. Those model tests also include the separately
owned solid-colour work; the lexical proof isolates this move's lack of behavior change.
