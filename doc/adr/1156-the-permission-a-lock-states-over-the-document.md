# 1156 — The permission a signature field lock states over the document

Session 1159. Status: **accepted**. Revisits ADR 0502 section 3.1, whose deferral is discharged.
Context: `crates/pdf-signature/src/signature.rs` (`field_lock_permissions`),
`crates/pdf-model/src/restriction.rs` (`Restriction::LockPermission`, `asserted`),
`crates/viewer-core/src/notes.rs`, `crates/pdf-transform/src/lib.rs`,
`crates/pdf-model/tests/forms_data.rs`.
Builds: ADR 0502 (the measurement and the deferral), ADR 0284 (the lock as a restriction),
ADR 0212 (the reading), ADR 1144 (a level per operation, `off` by default), `doc/todo/38`.
Clauses: ISO 32000-2 §12.7.5.5 (Tables 235 and 236), §12.8.2.2 (Table 257), §12.8.6, §7.5.6.

## 1. The owner's revisit note, claim by claim

`doc/adr_revisit/0502-…` argues that the deferral's priced cost expired. Each of its claims was
checked against the tree before anything was built.

- *"the incremental write surface grew … an unread `/P 1 /Action All` now leaves each new write
  operation ungated, not one."* **Held.** `asserted` consulted a field lock only for
  `Operation::FillInForm`; `Edit::Attach`/`Detach` are `Operation::Modify` and an annotation is
  `Operation::Annotate`, so two more operations went ungated than when ADR 0502 counted one.
- *"the four levels now exist, so the permissive default the deferral wanted is a policy value
  rather than an argument."* **Held, and it is the claim that decides the round.**
  `RestrictionPolicy::default` is `RestrictionLevel::Off` for all six operations, in the viewer as
  in every other face. ADR 0502 §3.1 refused the reading because it "adds a **default refusal** on
  28 real documents"; there is no longer any such thing to add.
- *"0502's settle-path is built and waiting."* **Held.** The entry goes through
  `restriction::asserted` exactly as the row said it would, and nothing else had to move.
- Errata issue #131 is recorded in `doc/errata-read.md` and was re-read. It carves a DSS-only or
  timestamp-only incremental update out of this `/P`, which changes no verdict here — this program
  appends field values, annotations and attachments, never a bare DSS or timestamp — and it is
  evidence for the reading rather than against it: **an erratum that carves an exception out of a
  permission is about a permission.**

## 2. The reading, and what settles the two voices

ADR 0502 left the entry two-voiced and said the choice deserved its own round. The entry decides
it. Its first sentence is "[t]he access permissions granted for this document"; three later ones
address a processor that changes the file — "The new permission applies to any incremental changes
to the document following the signature of which this key is part", "That is, permissions can be
denied but not added", and "If the document does not have an author signature, the initial
permissions in effect are those based on the number 3". §7.5.6's update is exactly those
incremental changes.

The sentence that used to dispose of it — "absence of this key shall result in no effect on
signature validation rules" — is about an **absent** `/P`, and a sentence about absence describes
nothing a present one states. That is the whole of the correction.

**What makes reading it safe is not confidence.** It arrives as one `Restriction` among six, so
`CLAUDE.md`'s four levels decide what happens about it and every face opens at *off*. If this
reading is wrong, nothing is withheld from anybody who did not ask for it to be — which is the
asymmetry ADR 0502 reasoned from, now pointing the other way.

## 3. Where it sits, and how several compose

`Restriction::LockPermission { level }`, beside `Certified` rather than beside `FieldLocked`. The
`/Action` and `/Fields` of the same dictionary name *fields*; the `/P` names none and states a
regime over the file. And it is a different clause from §12.8.2.2's, reached without §12.8.6's
permissions dictionary, so a person is owed which of the two they are being told — `FieldLocked`
against `FieldCovered`'s rule, applied again.

The values are Table 257's, word for word bar two verbs, so `Modification` is the type for both and
`certification_permits` asks one question of either; the levels part company where that table parts
them, `/P` 2 permitting form filling and withholding annotating.

**Several compose as a minimum, and the entry's own words are why.** A number is taken only where
it is "less than or equal to the permissions already in effect" and is "ignored" otherwise, which
makes the sequence a running minimum — and a running minimum over a set is that set's minimum
whatever order it was read in, so a walk that does not promise the document's order cannot get it
wrong. `/DocMDP` is the other MDP permission that sentence speaks of and is asked separately; each
restriction refuses on its own, which is §12.8.6's composition.

A value outside 1..=3 states no level and is passed over, on `Modification::Unknown`'s standing
reading: refusing on a number the table does not define would let a malformed integer lock a
document a person is entitled to change.

## 4. What was measured, and what was not

`a_signed_locks_permission_entry_states_what_may_still_be_changed` was calibrated in both
directions (trap 13): with `field_lock_permissions` answering `None` it fails, and with it
answering a level unconditionally it fails, taking four other tests with it. The witness is
hand-built (trap 8) beside ADR 0502's crawled population — 28 of 65 944, 0 of 1251 curated — which
this round did not re-walk; `save_round_trip` over the curated corpus moved no count, which is what
a population of zero predicts.
