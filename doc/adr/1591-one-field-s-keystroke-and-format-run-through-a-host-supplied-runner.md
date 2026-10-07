# 1591 — One field's `/K` and `/F` run through a host-supplied runner, against a bridge that refuses the rest by name

Status: accepted and **built**. Session 1377. Builds on RFC 0008 sections 4.2, 4.3, 6.3, 6.4 and
11 item 3 (a) (the owner's `A193`), on ADR 1579 (Tier 0's dispatch, which this extends) and on ADR
1590 (the engine and its budgets).
Context: ISO 32000-2 §12.6.3 (Table 199), §12.6.4.17 (Table 221); `CLAUDE.md` principle 1's
immutable `Document` and principle 3's levels.
Code: `crates/pdf-model/src/view/scripts.rs` (`ScriptRunner`, `FieldEvent`, `FieldResult`,
`ViewState::run_scripts_with`, `supplied_script`), `crates/pdf-model/src/view.rs` (the `runner`
field, `set_field`'s verdict), `crates/pdf-script/src/engine/bridge.rs`,
`crates/pdf-script/src/surface.rs`. Tests: `crates/pdf-script/tests/hook.rs`, `fixtures.rs`;
the Tier 1 column `crates/pdf-script/tests/script_corpus.rs`.

## 1. The hook is a trait object in `ViewState`, and its absence is `off`

`pdf-script` depends on `pdf-model` (RFC 0008 section 6.1), so `pdf-model` cannot name the engine:
it names a **trait**, `ScriptRunner`, and a host hands an implementation to
`ViewState::run_scripts_with`. A script Tier 0 reports as not run, at `/K` or `/F`, whose trigger's
action is one ECMAScript action with readable text and no `/Next`, goes to the runner where one is
supplied; with none, it is reported exactly as before. That is the one place a host's level reaches
this crate: `off` supplies nothing, `on` and `warn` supply a runner, `ask` supplies one that asks once
and remembers — so an `ask` is never a refusal that cannot become one. **No host supplies one this
round**, so nothing runs in any window; `viewer_host::policy`'s `Scripts` level is RFC 0008 section
11 item (e). The trait is data in and data out (`FieldEvent`, `FieldResult`) so that the runner can
be a proxy for the confined worker of item 4 without the trait changing.

`ViewState` stays `Clone` and `PartialEq`: the runner is an `Arc`, compared by identity.

## 2. Where each trigger hands over

- `/K` typing (`set_field`): `event.value` the field's text, `event.change` the whole value the host
  hands, the selection the whole text. `rc` false takes nothing; a rewritten `event.change` is what
  the field takes. A rewritten `event.value` at typing is ignored — Adobe's "Form event processing"
  gives the typing keystroke's `value` as the text before the change.
- `/K` commit (`commit_field`): `willCommit` true, the selection at the end. `rc` false puts back
  what was there, as Tier 0's refusal does; a rewritten `event.value` is written as an edit into the
  same map a typed value lands in (RFC 0008 section 6.4) — so the document is never touched.
- `/F` (`displayed_value`): the formatted text is answered. **The drawn appearance does not yet ask
  the runner** — `crate::appearance` reads Tier 0's site alone — so a field whose `/F` only the engine
  runs is drawn with its raw value while `displayed_value` answers formatted. At `off` the two agree;
  the host step that supplies a runner (item (e)) owes the appearance the same hand-over.
- `/V` and `/C` stay reported as not run; `pdf_script::run` declines them by name.

The runner's sentences are recorded in `script_reports` from the two `&mut` paths. `displayed_value`
answers a question and records nothing, so `pdf_script::Engine` keeps its own log of every run.

## 3. What a script reaches

`event.value`, `rc`, `willCommit` (read-only), `change`, `selStart`, `selEnd`, `target`; the target as
a field whose `value` reads the field's value and refuses a write (a `/K` or `/F` changes its field
through `event.value`); `this.getField` of the event's own field, and of any other name a refusal
naming it — the bridge holds one field; `console.println`, into the outcome's log; and **the
twenty-one `AF*` functions as natives over `pdf_model::aform`** — the format, keystroke and validate
families run `Call::format`, `Call::keystroke`, `Call::validate` against the event, the four helpers
answer values, and `AFSimple_Calculate` is refused because it reads other fields. A format called
from a script therefore writes exactly what a one-call `/F` writes (`fixtures.rs` compares them).

Everything else RFC 0008 section 4.2 admits, and everything section 4.3 excludes, is a non-configurable
accessor that throws a `NotAllowedError` on read and on write, whose message names the member and its
tier — section 4.3's reason for a Tier 2 member, "Tier 1 admits it and this bridge does not carry it"
for the rest — and is recorded in the outcome whether or not the script catches it.
`crates/pdf-script/src/surface.rs` is the list; a later round bridges a member by deleting its line.
`app.launchURL` and `this.submitForm` are refused here: section 4.4's request across the wire waits on
the host carrying a script's request, so this bridge says so in the sentence rather than raising one.

The document's members are the global object's, since a script's top-level `this` is that object;
Adobe's reference makes the document `this`, and the two are the same property.

## 4. What the Tier 1 column found

Over the census population (90 763 documents, 36–56 s behind the lock), 2 537 fields carry a `/K` or
`/F` Tier 0 does not run, and the walk made 4 924 runs: 439 finished, none stopped by a budget, none
unparsed, 4 485 threw. **Few of the throws are this bridge's refusals** (92 `NotAllowedError`s):
4 393 are `ReferenceError`s, nearly all naming a function the field script calls and does not
define (`setNumeric` 972, `TFTemplate_KeyStroke` 912, `entier` 592, `setUpper` 546, …) — the shape
of a document-level script, which Table 32's name tree carries and of which §12.6.4.17 says "all of
the actions in this name tree shall be executed" when the document is opened; this round loads none,
and a few name Adobe globals the surface does not list (`display`). The refused members are
`util.printx` 55, `app.viewerVersion` 18, `event.commitKey` 8, `event.fieldFull` 6, `getField` of
another field 5. So the next build is the open sequence's name tree in a persistent realm (the
worker's), then `util`, then `app`'s answers — the census ranking the work, never capping it.
