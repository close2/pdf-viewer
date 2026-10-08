# 1750 — A widget's and a page's scripts are requests the host raises at the event

Status: accepted and **built** (the model half). Session 1457. Builds on ADR 1602 section 4 (the two
sites, scripts alone), ADR 1616 (the `Scripts` level is the runner's), ADR 1702 (a site the view
state exposes and a host drives); the windows' half is ADR 1752.
Code: `crates/pdf-model/src/view/script_sites.rs` (`run_annotation_scripts`, `run_page_scripts`,
`before_open`, `page_turn_scripts`, `annotation_triggers`), `crates/pdf-model/src/view/scripts.rs`
(`Scripting::opened`), `crates/pdf-script-worker/tests/script_column.rs` (`raise_events`).
Tests: `crates/pdf-script/tests/hook.rs` (`a_widget_s_six_pointer_and_focus_events_each_run_their_script`,
`a_page_shown_and_left_runs_each_annotation_s_visibility_with_its_open_and_close`,
`an_event_raised_before_the_open_sequence_runs_nothing_and_says_so`).

## 1. The premise, as the tree stood

Round 1450 found that past the open sequence no Table 197 (an annotation's `/AA`) and no Table 198
(a page's `/AA`) script ran in any window: `ViewState::run_annotation_scripts` and
`run_page_scripts` had no caller outside the tests, and `crate::action` reads every `JavaScript`
action as `Action::Refused`, so a window's action path declined each one. The brief named the page
table as Table 196; it is Table 198 (Table 196 is the entries common to every action dictionary).

## 2. The requests are the two methods, and what each now covers

**A host calls `run_annotation_scripts(document, annotation, Trigger)` at a widget's `/E`, `/X`,
`/D`, `/U`, `/Fo` and `/Bl`, and `run_page_scripts(document, page, PageTrigger)` as the page shown
changes — the leaving page's `Close`, then the arriving page's `Open` — beside the non-script
actions it performs at the same event.** Each answers how many scripts the runner was handed; the
level is the runner's, as at every site, so `off` answers 0 and `ask` puts its one question at the
first. They keep their names because slot 3 of this batch built the windows against them while
this was written, and one enum of the two would have bought a host nothing but a rename.

- **Table 197's four page events ride with the page's.** `Open` runs `/O` ("[t]he action shall be
  executed after the O action", Table 197's `/PO`), then each annotation's `/PO` and `/PV`; `Close`
  runs each annotation's `/PC` and `/PI`, then `/C` (`/PC` "shall be executed before the C
  action"). The open sequence runs `/PV` after `/PO` too. A host therefore never runs those four
  per annotation, which is what keeps a host that raises them per annotation for their *other*
  actions from running their scripts twice. *Visible* is taken as *shown*: Table 197 says "more
  than one page may be visible, depending on the page layout", and every window here raises
  `/PV` and `/PI` with the page turn, so the scripts follow the windows — a documented choice whose
  cost is a continuous layout's neighbour pages, which no window raises for either.
- **Before the open sequence has run, nothing runs.** §12.6.4.17 executes Table 32's tree "[w]hen
  the document is opened … defining ECMAScript functions for use by other scripts", so a script
  raised before it meets a library that does not exist and throws a `ReferenceError` a person
  would read as the form's. An `Open` before it is silent — the sequence runs the page shown when it
  runs — and any other request holding scripts says it was not run. A window runs the sequence at
  `Command::Presented` (ADR 1602), so only a host that raises an event before its first present,
  or never presents, reaches the sentence.
- **Cheap where nothing is scripted.** A cursor crossing a page raises `/E` and `/X` on every
  annotation it meets; the chain's scripts are read first and the field tree is walked only once
  one is found, and a page turn's request walks its chains before it builds the page's fields.

## 3. The action path's refusal stays, and the row stays `out-of-scope`

`crate::action::refused` keeps `JavaScript` as a refusal, and `perform_all` performs no script:
§12.6.4.17 is `out-of-scope` under `CLAUDE.md`'s script exclusion until the owner answers
`doc/questions/Q286`, and the requests above run a chain's ECMAScript actions *beside* that path
rather than through it — the shape every site has had since ADR 1602, including a widget's or a
link's `/A`, which Table 197's precedence makes `/U`'s chain. The refusal's sentence is the one
trap 5 asks for where no runner is supplied; where a request answered more than 0 the runner's own
sentence says what ran, and a host drops the refusal for that chain (ADR 1752). The order between a
script and a non-script action of one chain remains the one thing not kept (ADR 1602 section 4).

## 4. The worker column raises them

`pdf-script-worker`'s census walk, after its commits, raises on each of the first 64 pages every
annotation's six pointer and focus events in a click's order (`/Fo` and `/Bl` on widgets only, as
the table says) and turns to the next page, so every Table 197 and Table 198 script of the census
runs through the confined worker, where a `SIGSYS` names a call the profile does not admit. The
in-process Tier 1 column is unchanged, so its held figures say what the open sequence's `/PV`
moved.
