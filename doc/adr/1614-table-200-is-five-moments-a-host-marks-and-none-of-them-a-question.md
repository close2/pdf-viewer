# 1614 — Table 200 is five moments a host marks, and none of them is a question

Status: accepted and **built** in `pdf-model` and `pdf-script`; the hosts' calls are round 1390's.
Session 1389. Builds on RFC 0008 section 6.5 step 6 and section 11 item 3 (d), accepted by the
owner in `doc/questions/A193`; extends ADR 1602 (the realm, the sites outside a field) and ADR 1609
(the wire to the worker).
Context: ISO 32000-2 §12.6.3 (Table 200), §7.7.2 (Table 29's `/AA`), §7.5.6; `CLAUDE.md` principle
3, *A document's restrictions are the reader's to set*.
Code: `crates/pdf-model/src/view/script_sites.rs` (`ViewState::run_document_scripts`),
`view/script_model.rs` (`DocumentTrigger`, `ScriptSite::Document`), `crates/pdf-script/src/wire.rs`,
`engine/bridge.rs` (`begin`), `crates/pdf-script-worker/src/client.rs` (`subject`).
Tests: `crates/pdf-script/tests/document_events.rs`, `crates/pdf-script-worker/tests/end_to_end.rs`
(`a_will_save_script_runs_in_the_worker_and_cannot_refuse_the_save`).

## 1. One call, five moments

`ViewState::run_document_scripts(document, trigger, page)` reads the catalog's `/AA` entry for one
`DocumentTrigger` — `/WC`, `/WS`, `/DS`, `/WP`, `/DP` — and hands each ECMAScript action of its
§12.6.2 chain to the runner, in the document's realm, at `ScriptSite::Document(trigger)`. A view
state cannot see a close, a save or a print happen; a host can, so the host marks each, and these
are the calls round 1390 makes (viewer-core owns all three operations for every window, so the
toolkit hosts need nothing of their own):

| moment | where, in `crates/viewer-core/src/viewer.rs` | call |
|---|---|---|
| a close | `Command::Close(id)`, before the document is let go — and every document still open when a window quits | `WillClose` |
| a save | `Viewer::save`, before `ViewState::save` writes §7.5.6's update | `WillSave` |
| after a save | the same, on `Ok(written)` only — a save that failed was not a save | `DidSave` |
| a print | `Viewer::print`, at `Printing::Start` once `Standing::Proceed` — a print the reader's policy refused never began | `WillPrint` |
| after a print | `Viewer::print`, at `Printing::Finish` | `DidPrint` |

`WillSave` comes first so that what its script writes into a field is in the file the save writes —
the RFC's "a save hook that clears a draft watermark field" — and the test reopens the written file
and reads the value back. `page` is the page the host shows, `this.pageNum`.

## 2. `event.rc` is read, reported, and never obeyed

Every row of Table 200 says when its action runs and nothing else. `/WC`'s is

> (Optional; PDF 1.4) An ECMAScript action that shall be performed before closing a document.

— a moment, not a question: the clause gives the action no part in whether the document closes, and
the other four rows say the same of a save and a print. So a script that sets `event.rc` false is
reported — "the document's /WC script set event.rc false, and the close goes ahead: Table 200
performs the script before the close and gives it no say in whether the close happens" — and the
close, the save or the print goes ahead. Adobe's "Event type/name combinations" page agrees on all
five — it says of each that the event ignores the return code — which is evidence that this reading
is the reference's too, and not its source.

**And it is a rule of this program's, beside the clause.** A will-close or will-save script that
could veto would be a restriction a document asserts over its reader that no level turns off;
`CLAUDE.md` principle 3 says of a document's restrictions that "it shall always be possible to
turn them off", and a veto inside a script is one nobody could. The level
that governs these scripts is the host's `Scripts` level (item (e)); a script at any level reaches
the fields it may write and nothing about whether its reader may close or save.

## 3. The event

`event.type` is `Doc`, `event.name` `WillClose`, `WillSave`, `DidSave`, `WillPrint` or `DidPrint`
(the reference's page, a documented choice). `event.target` is the document — the global object
here — and so it now is at the open's two `Doc/Open` sites too, where it was `null`: the same page
makes the document the target of every `Doc` event. No `value`, no `change`, nothing read back but
`rc`. A script's edits are applied as at any other site, and `/CO` is walked after them where they
changed a value or asked (`calculateNow`), then the runner's formats refreshed.

## 4. What else the chain may hold

Table 200's value column makes every entry "[a]n ECMAScript action". A §12.6.2 `/Next` chain can
still hold another type; it is not performed — there is no host-side `perform_all` for these five,
and the table admits none — and the view state says how many: "the catalog's /AA /DP chain holds 1
action(s) that are not ECMAScript, which Table 200 does not admit there, and none was performed".
With no runner supplied the scripts are reported as not run, once each, as the open's are.

## 5. Budgets and the wire

Each script runs under its runner's budget (ADR 1590's field-event budget; ADR 1609's 250 ms
deadline in the worker) and the chain as a whole under ADR 1603's one-second sequence bound, past
which the rest are reported as not run. `pdf_script::wire::VERSION` is 3: site tag 5 carries a
`DocumentTrigger`, so a host and a worker built from trees on either side of this change refuse
each other at the first byte. The worker's own frame (`pdf_script_worker::wire::VERSION` 1) did
not change shape and is unmoved.

## 6. The ledger

§12.6.3 stays `implemented` — its requirements are which events exist and when — and its note now
says Table 200's five run through a supplied runner at the moments a host marks, rather than that
they are unbuilt. §12.6.4.17 stays `out-of-scope` until the owner amends `CLAUDE.md` under A193's
`Owes` line; nothing here runs in a window until a host supplies a runner.
