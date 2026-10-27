# 2430 — Annex K leaves `out-of-scope`: §K.1 is implemented, §K.2 is partial, and the notice says what runs

Session 1798, the clause slot of batch 120 (ADR 2370's ledger rule).

Status: implemented — `doc/conformance/ledger.toml`'s three Annex K rows, `crates/viewer-core/src/notes.rs`
(`xfa_form`), tested by `notes.rs::an_xfa_form_is_said_to_be_one_and_what_of_it_is_carried_out` and
by `crates/pdf-model/tests/xfa_datasets.rs`.

## Context

RFC 0011 section 11 set the condition for Annex K's rows to leave `out-of-scope`: the owner ratifies
the RFC's static half, and `CLAUDE.md`'s XFA entry says that half is in scope. The owner answered
`doc/questions/A639` ("Q639: recommendation is fine"), which adopted the RFC's question 2 as well, and
the XFA entry now reads "**XFA's static half is in scope**" with the dynamic half, a form whose
template generates its pages, still excluded. Round 1790 recorded the condition met and left the move
(ADR 2414). The rows read: K `out-of-scope` as the aggregate of K.1 and K.2, both `out-of-scope` on
the `xfa` exclusion, K.1's note saying its one `shall` was "conditional on the choice this project did
not make".

## Decision

**§K.1 is `implemented`.** Its one requirement on a processor is "If an interactive processor
supports XFA forms, that processor shall clearly indicate to the user when they are interacting with
an XFA form that they are interacting with an XFA form". The condition now holds — this program keeps
a static form's data (ADRs 2396, 2414) — and the indication is executed by `notes::about`, once per
document, on the report every window asks for after its first frame (ADR 1932). The clause's
permission, "a PDF processor may choose to not implement this feature", is about schema-driven page
generation; it is taken for the dynamic half, and the report says so. A permission taken and said is
a clause read (the permitted sweep's reading of `CLAUDE.md`'s flatness sentence), so nothing in §K.1
is owed.

**§K.2 is `partial`.** Executed: both forms of Table 224's `/XFA` are read; three of the writer's
rules are kept by `ViewState::save` — field objects stay present (the §7.5.6 update replaces and
removes none), "[t]he XFA field values shall be consistent with the corresponding V entries of the
PDF field objects" (the `datasets` splice, in canonical form), and the SOM naming convention is how
`Resource` reads a field name — and nothing the save writes adds an `A` or `AA`. Owed, and said: "all
form-related events such as calculations and validations" are not run, and the configuration is read
for its locale alone. RFC 0011 section 5's steps 4 to 6 are the build.

**K derives `partial`** from its leaves (ADR 1599), and its note opens "Aggregate of the rows below".

**The notice is corrected, because it was no longer true.** It said "this program does not implement
XFA" for every kind of form, which ADRs 2396 and 2414 made false for a hybrid one. A hybrid form's
sentence now says the AcroForm is shown and filled and that saving keeps the form's XFA data
consistent, naming any field it cannot, and that calculations, validations and formatting are not
carried out; the two regenerated kinds say "this program does not generate a form's pages from its
XFA template, which §K.1 permits a processor to choose". No pixel moves; the sentence is a report.

## Consequences

- The `xfa` exclusion now covers no ledger row; `CLAUDE.md`'s dynamic-half entry stays the reason
  §K.1's permission is taken, and a dynamic form is named field by field in `Written::xfa_unkept`.
- `crates/pdf-model/src/xfa.rs`'s module comment opens "This program does not implement XFA", which
  is now stale like the notice was; it sits in the XFA slot's files and is left to that slot.
- §K.2 moves to `implemented` when the template's calculations, validations, formatting and events
  run and the configuration is read whole.
