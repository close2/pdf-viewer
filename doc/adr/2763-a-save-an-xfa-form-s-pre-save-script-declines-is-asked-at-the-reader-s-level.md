# 2763 — A save an XFA form's `preSave` script declines is asked at the reader's level

Session 1976, a ledger round (K.2). Status: implemented. Revises decision 5 of ADR 2702 (a save's
`cancelAction` reported and the file written); extends ADR 0803 (the four levels, one place) and
ADR 1961 (a restriction a caller hands to `Level::verdict` itself).

## Context

XFA 3.3 page 402: `cancelAction` "is cleared by the XFA processor at the beginning of an action",
and "[s]etting this flag at any time before the action is consumated causes the action to abort and
a cleanup process to take place". Page 400 says what the cleanup is: "pre-events push their matching
post-events onto a last-in first-out stack. The cleanup process pops stacked post-events and fires
them". Page 388 fires `preSave` "just before the form data is written out" and `postSave` "just
after". ADR 2702 carried the flag for a save and did not obey it, because a save is the reader's own
operation, a document's script stopping it is a document's restriction on its reader, and
`CLAUDE.md` makes such a restriction the reader's to set at four levels — a policy that was not built
for it. The policy exists for every other restriction: `pdf_model::restriction::Level::verdict`, one
level per `Operation` in `viewer_core::RestrictionPolicy`, carried on the confined wire, the C ABI
and every host's reader words.

## Decision

1. **`Operation::Save` is the tenth operation, and `Restriction::SaveCancelled` its one reason.**
   No clause of ISO 32000-2 restricts a save, so `Operation::bit` answers `None` and
   `restriction::asserted` never returns the reason: the activation that ran the `preSave` events is
   the only thing that knows, so `Viewer::save` hands it to the verdict itself, as the media
   hand-off hands `/TF`'s (ADR 1961). Table 257 does not bind a save: what it writes are edits each
   of which was asked about as its own operation.
2. **`Viewer::save` asks once, after `/WS` and `preSave`**: `off` (the default) writes and says
   nothing; `warn` writes and then says why; `on` writes nothing and says why; `ask` holds the save
   (`Held::Save`) and `Command::Answer` writes on a yes. The reader word is `save`; the wire's and
   the C ABI's number is 9 (`QUORRA_RESTRICTED_SAVE`).
3. **A save not written fires `postSave`**, page 400's cleanup of the `preSave` that fired, and not
   Table 200's `/DS`, which runs "after saving a document". A held save fires neither until the
   answer.
4. **`ViewState::activate_form_before_save` returns the `FormAction`** and words nothing; the
   sentence a person reads is the verdict's (`notes::restricted`).

## Consequences

- At `off` the flag is no longer said, as no restriction is at `off`: the level asks to be told
  nothing (ADR 0803).
- A save's validations still never stop it: page 1340 writes it "regardless" of them (ADR 2702).
- Every window offers the level by its reader word, through `RestrictionPolicy::operation_named`.
