# 1602 — One realm per document, the open sequence after the first present, and the sites outside a field

Status: accepted and **built**. Session 1383. Builds on RFC 0008 sections 6.2, 6.4, 6.5, 6.6 and
11 item 3 (c), accepted by the owner in `doc/questions/A193` (answers 5 and 6); amends ADR 1590
section 3 and extends ADR 1591. ADR 1603 is (b), built beside it; ADR 1609 is the worker that holds
a realm across a process boundary.
Context: ISO 32000-2 §12.6.4.17, §7.7.4 (Table 32), §12.6.3 (Tables 197, 198, 200); `CLAUDE.md`
principle 2 (nothing eager, time-to-first-page) and principle 3 (budgets).
Code: `crates/pdf-script/src/engine/mod.rs` (`Realm`, `Engine`), `engine/bridge.rs`,
`crates/pdf-model/src/view/script_sites.rs`, `view/script_model.rs`, `view/scripts.rs` (`tell`).
Tests: `crates/pdf-script/tests/realm.rs`, `hook.rs`; the column `tests/script_corpus.rs`.

## 1. A realm is one document's, and it persists

§12.6.4.17 has the name tree run at the open for the sake of what comes after:

> When the document is opened, all of the actions in this name tree shall be executed, defining
> ECMAScript functions for use by other scripts in the document.

So the functions it defines must still be there when a field's script calls them, and ADR 1590's
"one context per run" cannot honour the sentence. **`pdf_script::Realm` is one Boa context per
document**: each run installs a fresh `event`, clears its record and evaluates in the same context,
so every global a script leaves stays (ADR 1590 section 3 is amended to this). `pdf_script::run` is
the one-shot form, a realm built and dropped per request, for a caller with no document.

Boa's values are reference-counted per thread, so a realm is not `Send`. **In this process
`pdf_script::Engine` holds its realm on a thread of its own** ("pdf-script realm", 16 MiB stack),
started at the first event a view state hands over and ended when the engine is dropped — the
in-process stand-in for ADR 1609's worker, which holds a `Realm` on its main thread the same way. A
realm whose thread dies answers every later event with the sentence that scripts stopped for this
document (RFC 0008 section 6.8). This is the one thread this crate starts; ADR 1590's "no thread is
spawned anywhere" is amended to it. Every budget is still checked on the thread that runs the script.

## 2. What the realm is told

`this.getField` must reach any field without a round trip across the process boundary, so **the
realm holds the document's fields** as `FieldState`s (`pdf_model::view`) and every request carries
the fields whose state changed since the realm last heard — all of them the first time. The view
state measures *changed* by comparing copies of its four statements about values and visibility
(`edited`, `imported`, `reset`, the hide sets) and the script overrides against the copies taken at
the last telling, rather than by a counter every writer bumps: a comparison cannot miss a site that
writes one, and the copies are as large as the edits, not as the form. A new runner is a new realm,
so `run_scripts_with` forgets what was told.

## 3. The open sequence, and the call a host makes

`ViewState::run_open_scripts(document, page)` runs RFC 0008 section 6.5 step 1: Table 32's
`/JavaScript` entries in the tree's order (§7.9.6 orders a name tree's keys, and `tree::name_pairs`
walks in that order), each entry's `/Next` chain joined; then the catalog's `/OpenAction` where it is
an action; then page `page`'s Table 198 `/O`, which "shall be executed after such an action"; then
each of its annotations' Table 197 `/PO`, which "shall be executed after the O action in the page's
additional - actions dictionary"; then every format a runner runs (ADR 1603). **The host's first
present calls it** — round 1384 names `run_open_scripts` in each window after the first frame — so
the cost to time-to-first-page is nothing by construction (RFC 0008 section 6.6, a documented
choice: *when opened* is not *before the first frame*). With no runner the open reports, once, how
many document-level scripts went unrun. The sequence as a whole is held to one second
(`MAX_SEQUENCE_TIME`), RFC 0008 section 6.8's open-sequence deadline; past it the rest are named as
not run.

## 4. Table 198 and Table 197, scripts alone

`run_page_scripts(document, page, PageTrigger)` runs a page's `/O` and then its annotations' `/PO`,
or its annotations' `/PC` ("shall be executed before the C action") and then its `/C`;
`run_annotation_scripts(document, annotation, Trigger)` runs one of Table 197's ten events, `/U`'s
chain being `/A` where the annotation states one, Table 197's precedence. **Each runs only the
ECMAScript actions of the chain**: `perform_all` already performs every other action of the same
chains in every host, so a host calls both, and the order between a script and a non-script action
in one chain is the one thing not kept. No host calls these yet; the test host supplies the engine
and does (`hook.rs`). Adobe's `event.type` and `event.name` per site are the reference's "event
object" table (`ScriptSite::event_names`): a widget's events are `Field`'s, a link's mouse-up
`Link`'s, every other annotation's `Screen`'s — a documented choice. None of these sites listens to
`event.rc`, as the reference has it.

## 5. Table 200 is item (d), and none of it is taken here

`/WC`, `/WS`, `/DS`, `/WP` and `/DP` are each "An ECMAScript action that shall be performed" around a
close, a save or a print. A save and a close are host moments no `ViewState` call marks, and this
program prints nothing (RFC 0004 is a draft), so each needs a host hook a later round names; RFC 0008
section 11 keeps them as item (d) on their own, and §12.6.3's row says so.

## 6. The ninth number is the worker's ceiling, and the tenth is the parser's depth

Boa has no heap ceiling and offers no allocator hook — `HostHooks::max_buffer_size` bounds an
`ArrayBuffer` and nothing else — and a realm lives as long as its document, so a script that grows a
global on every keystroke is bounded by nothing a run's budgets see. **The realm's memory budget is
therefore the address space of the process that holds it: 96 MiB, ADR 1609 section 2's measured
`RLIMIT_AS` for the script worker**, and it is that ADR's number, installed by the kernel, rather
than a second one carried in `Budget` that nothing in-process could enforce. The in-process `Engine`
is the test host's and is held to no heap ceiling; no window supplies it. Measured against the 96
MiB (`script_corpus.rs`'s `a_realm_s_heap_is_measured_for_one_document`, one document per process,
the document dropped before the realm is built): the census's largest library (`evince-LINK-46-2.pdf`, 179 030 bytes of script over 38 requests) raises the high-water mark by 19.1 MiB, `REDHAT-1167020-11.pdf` by 6.7, `poppler-LINK-82-0.pdf` by 4.5, and `5712688.pdf`'s 10 482 requests by 1.8 — each inside the ceiling with the worker's own 12.4 MiB start beside it.

**`Budget::nesting`, 128 levels, is new beside ADR 1590's eight.** Boa 0.22's parser recurses once
per nested expression and has no depth limit of its own — upstream's parser is unbounded: five
hundred nested parentheses overflow an 8 MiB stack, which the worker survives as one named loss
(ADR 1609) and which in this process would abort it whole. Measured here, one process per point, in
`dev` and in `release` alike: an 8 MiB thread parses 256 levels and overflows at 320, a 16 MiB one
parses 400, a 2 MiB one 64 and not 128 — about 26 to 32 KiB of native stack a level. So a script
whose brackets nest deeper than 128 — counted over every byte, a bracket in a string included, which
errs towards not running rather than towards overflowing — ends `Exceeded::Nesting` before the
parser sees it: half of what the worker's 8 MiB main thread holds, a quarter of `Engine`'s 16 MiB
thread, and far past any form script (the census's nest in tens). The `script` fuzz target skips past
256, inside which this bound sits. `fixtures.rs` holds 500 refused and 128 run.

## 7. What the column found

The column (`script_corpus.rs`, 90 763 documents, 106-142 s behind the lock) now runs the open
sequence and commits every field whose `/K`, `/F`, `/V` or `/C` is a script: 3 845 fields, 20 699
runs (15 190 calculations, 4 531 keystrokes, 393 formats, 265 validations, 303 document-level
scripts, 17 page opens), 10 966 finished, none over a budget, 9 725 threw, 8 unparsed. **The
`ReferenceError`s for functions a document's own library defines are gone**: ADR 1591 counted 4 393
across the population for `setNumeric`, `TFTemplate_KeyStroke`, `entier`, `setUpper` and their
kind; 9 178 remain, in **15 documents, for 9 names, none of which the document's own library
defines**: `TFMC` (8 892 runs, one form whose seven-entry library defines `TFOpen`, `TFMV` and the
rest but not the `TFMC` its `/C` calls — a function no file defines), `defaultValue` (223, one form's
script slip for the field's `defaultValue` property), `Matrix2D` (40, five documents whose
producer's folder-level script defined it in the author's viewer and not in the file), `goNext`,
`f_insert` and `aaa` (functions no file defines), `AFExactMatch` (a function of the reference's
library Tier 0 does not carry), `AFSpecial_FormatEx` (one it has nowhere), and `cursor` (Adobe's
`cursor` constants, a Tier 1 member not bridged). The commonest refusals are `util.printx` 59,
`app.viewerVersion` 36, `Field.comb=` 29, `Field.getArray` 19 and `util.printd` 13 — the next
build, in that order.
