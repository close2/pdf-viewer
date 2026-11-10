# 2777 — An XFA form's `prePrint`, `postPrint` and `docClose` events run

Session 1983, a ledger round (K.2). Status: implemented. Extends ADR 2702 (a save's and a submit's
pre- and post-events) and ADR 2763 (a `preSave` script's `cancelAction` is asked at the reader's
level); keeps ADR 1616's confined window at `off`, where none of the three runs a script.

## Context

XFA 3.3 page 388 names six application events, bound by the `ref` `$host`: `docClose`, `docReady`,
`prePrint`, `postPrint`, `preSave` and `postSave`. Four of them ran (ADRs 2606, 2702); `prePrint`,
`postPrint` and `docClose` did not. Each needs a moment the window already has: a print operation
has a beginning and an end (`Command::Print`'s `Printing::Start` and `Printing::Finish`, ADR 1180,
where Table 200's `/WP` and `/DP` already run, ADR 1614), and `Command::Close` already runs `/WC`.
So no host, no frame of the confined protocol and no entry of the C ABI changes, and the group is the
one that finishes a class of page 388's events. Effort was the only obstacle.

## Decision

1. **`prePrint` and `postPrint` sit beside `/WP` and `/DP`.** `prePrint` "fires just before
   rendering for print begins", so it runs after `/WP` and before the document is put into print
   intent, and what a script writes is on the paper; `postPrint` "fires just after the rendered form
   has been sent to the printer, spooler, or output destination", which for this window is the end
   of the print operation, after `/DP`. The pages are not rendered between the two by this crate —
   a host pulls them with `Query::PrintPage` — so the end of the operation is the moment the window
   knows the output has gone.
2. **A `prePrint` script's `cancelAction` is a document's restriction on its reader.** Page 399 has
   the flag "cancel whatever action fired the event", and a print is the reader's own, so the flag
   is `Restriction::PrintCancelled` asserted against `Operation::Print` and asked at the reader's
   level for `print`, exactly as `SaveCancelled` is for `save` (ADR 2763): `off` prints, `warn`
   prints and says so, `on` prints nothing and says why, `ask` holds the print
   (`Held::PrintCancelled`, carrying the sheet and Table 22 bit 12's answer already given) for a
   `yes` or a `no`. The question comes after bits 3 and 12, because `prePrint` fires only for a print
   that is going to begin. Wherever nothing is printed, page 400's cleanup fires the `postPrint` the
   `prePrint` pushed, and `/DP` does not run.
3. **`docClose` runs after `/WC`, and only where every validation succeeded.** Page 388: "This event
   fires at the very end of processing if and only if all validations succeeded. Success in this
   case is defined as generating nothing worse than a warning (no errors)." The window's end of
   processing is the close. So at the close every container is put to page 381's four tests over
   what the form holds then — the null test at its `nullTest`'s level (page 384), the format test at
   its `formatTest`'s and the script test at its `scriptTest`'s (pages 384–385), and the datatype
   test, whose response "is equivalent to a level of error" (page 386) — and the `docClose` events
   fire only where none fails at `error`; otherwise the report names each failure. **A choice where
   the page is silent**: the page does not say whether "all validations" means the ones already run
   or a run at the close. A run at the close is chosen, because the validations this reader has run
   by then depend on which pre-events a session happened to raise (page 396 runs the null test only
   in pre-events), so a reading of what was already run would make the same form close differently
   after a save than without one. The verdicts are not kept, and no `validationState` fires: the
   form is closing.
4. **A `submit` under `docClose` is said and not held.** Page 388 says the event "comes too late to
   modify the saved document", and page 389 has the data a submit may return merged back into the
   Data DOM, which is gone. A choice, beside ADR 2738's for a `submit` under a submit's own events.
5. **Where no script runs, the close validates nothing.** Every script of a `docClose` event is
   passed over at `off` (ADR 2767), and its `submit` would not be performed, so there is nothing for
   the validations to decide. `prePrint` and `postPrint` are raised at every level as the other
   actions' events are: their scripts pass over unrun and their `submit`s are carried out.

## What is left of K.2's events, and why

`full` (page 391), `preOpen` and `postOpen` (page 392) and the Layout DOM's `ready` (page 388) are
effort, each needing the window to say something it does not say today. `preExecute` and
`postExecute` (page 393) belong to a WSDL call, which `CLAUDE.md`'s JavaScript entry keeps outside
(ADR 2771). `preSign` and `postSign` (page 394) fire when a signature "is about to be applied", and
this program applies none — it verifies them (§12.8) — so they have no moment here until it does. `indexChange` (page 395) belongs to an
instance manager, "a dynamic array of same-named sibling objects", which a template whose containers
occur a varying number of times has, and such a template is the dynamic half `form.rs` declines.

## Cost

One `Stimulus` per event, one `Restriction` variant, one `Held` variant, and a validation run at the
close of a form that has a `docClose` event; a form with none is not validated there.
