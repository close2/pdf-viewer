# 2545 — A form's JavaScript runs in a realm of its own whose object model is the form's containers, as a host answers for them

Session 1855, a script round in batch 126. Follows ADR 2524 (FormCalc's conversation with the host) and ADR 2482 item 5
(`pdf-model` names no engine).

Status: implemented. The realm and its object model are built (`pdf_script::xfa`), and the activation carries a JavaScript run to it in the confined worker.

## Context

§K.2's note leaves a hybrid form's JavaScript calculations and events unrun: `Resource::activate` refuses a script whose
`contentType` is JavaScript by name (XFA 3.3 page 412 names the two languages). The worker already runs FormCalc as a
conversation in which the form never crosses (ADR 2524), and a document's AcroForm scripts in a Boa realm under
`Budget::FIELD_EVENT` (ADR 1590). The slot's hypothesis was ADR 2524's conversation with a JavaScript realm and a host
object of the form's fields; it held for the conversation and needed one question more (item 4).

## Decision

1. **A realm of its own, kept for the form's life** (`pdf_script::xfa::FormRealm`), apart from the document's realm:
   page 414 shares nothing between engines but `xfa`, and globals persist across one engine's scripts. It is built as
   the document's is — the loader that reads no file system (trap 118), Boa's loop, recursion and stack limits, the
   optimizer off, the guards on what an argument can make large (ADR 1590) — and each run is sliced against the steps
   and the wall clock, so a runaway script is stopped inside the worker and reports itself past what it was handed,
   which ends the activation (page 380).
2. **The object model is the subset a calculation or a value event reaches**, everything else refused by name:
   `this` is the script's container (page 415); a naked reference (page 413) is the container page 114's search finds;
   `rawValue` is read and assigned and `isNull` read (page 106); a child of a container is reached by its name (page 90);
   `resolveNode` and `resolveNodes` take a SOM expression in a string on `xfa`, `$xfa` and any container (page 107), the
   expression read by `pdf_script::xfa::som`, which refuses `..`, `.#`, predicates and escapes by name. `value` (a node,
   page 107's note), `formattedValue`, `presence`, `xfa.host`, `xfa.event` and the DOMs are refused by name, and a
   refusal the script caught still makes the run's outcome the refusal, since what it computed rests on it.
3. **Three choices where the pages state none**, each written as a choice. A name the engine defines (`Math`,
   `parseInt`) is the engine's before any container's. A name a script declares is its own unless a container answers
   it, and assigning to a container's name is refused, since a container is set through `rawValue`. A calculation's
   value (page 379 evaluates the script as a function and assigns its return value) is the script's completion value, as FormCalc's is
   its last expression's (page 1050), and a script whose top level returns is taken as a function body and its return
   value used; `undefined` and `null` are null, a boolean 1 or 0, an object refused.
4. **The host answers one question more**: `FormHost::resolve` takes the container a path starts from, because a
   child of a container object (`Detail.Row`, `node.resolveNode(...)`) is found from that container and not from the
   script's (page 107 gives every node a `resolveNode` method). The wire question is `ResolveFrom` (below).
5. **The bounds, each tested at the bound**: a SOM string of at most 64 names of at most 4 096 bytes; at most 4 096
   containers held as objects a run, the script's own included (a test process's peak was 23 MB against 14 MB with
   none, about a kilobyte each); at most 2^20 container numbers in a run's lists; one question per naked name a run.

## Consequences

- `cargo test -p pdf-script --features engine --test xfa_javascript`: eight tests, each expected value computed from
  the values the test's host states or a documented refusal.
- **Built**: `pdf_model::xfa::FormLanguage` beside `Runner::run` and `ScriptRunner::run_form_script`, widened at
  every call site in `crates/` and `raster/`; `Resource::activate` hands JavaScript to the runner and refuses only a
  language page 412 does not name. `form_wire`'s `FormRun` carries a language byte and `ResolveFrom` (kind 14) is a
  path from a container the worker was told of, answered as that path qualified by `$` from there; the greeting's magic
  is `PDFSCW03`, so an older worker is refused at its greeting. The worker's `Conversation` is the `FormHost` of one
  `FormRealm` kept for the worker's life, apart from the document's realm, under `Budget::FIELD_EVENT` and the steps
  the activation handed; the host's per-run deadline, `MAX_QUESTIONS` and the sequence clock of ADR 2525 hold either
  language alike.
- The plant: `cargo test -p pdf-script-worker --features engine --test form_script`'s
  `a_javascript_calculation_that_never_ends_is_stopped_and_the_entry_is_kept` drives `for (;;) {}`, a nested busy
  loop, unbounded recursion and a loop that only the clock ends through the view state's entry and commit — the calls
  `viewer-core` makes — and each is stopped in the worker within 5 s, the person's entry kept, the activation's other
  values dropped and the report naming the steps limit; `viewer-core`'s `script_form` asks about and runs a JavaScript
  calculation as one. The wall clock alone is planted on the realm (`xfa_javascript`'s
  `a_script_that_spends_no_limit_but_time_is_stopped_by_the_clock`).
- Remaining under §K.2: the events other than `change` and `initialize`, validations that are scripts, and page 642's
  dismissal of a `warning`.
