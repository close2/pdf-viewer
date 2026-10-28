# 2524 — A form's FormCalc runs in the confined script worker as a conversation with the host that holds the form

Session 1845, a script round in batch 125. Status: accepted. ADR 2525 holds the view state's two stimuli and what `override` reaches a window as.

## Context

ADR 2482 built `pdf-formcalc` to run "in the confined script worker beside Boa, under the same
budgets and the reader's `Scripts` level, `off` by default (ADRs 1904, 1616)", and ADR 2506
built `Resource::activate`, which orders a form's calculations and takes a `Runner` that runs one
script against a `Form`. No window called either. The slot's hypothesis was that the worker gains
a FormCalc realm "behind the protocol's existing message". **It does not hold**: that message is a
request (a site, a field, an event and the fields' state) and an outcome, and a FormCalc script
reads and writes a form the worker does not have — `$`, `Units * Price`, `Total = Sum(Row[*].Cell)`
are paths the host's `Form::resolve` answers (XFA 3.3 pages 114–115), over values only the host
holds. Shipping the form across would put the whole template's containers, names and values in a
process whose profile is the narrowest of the three (ADR 1608), for scripts that read a few of
them.

## Decision

1. **A FormCalc run is a conversation, not a request.** The host sends `FormRun` (the container the
   script belongs to, the steps it may spend, its text) and answers each question the interpreter
   puts — resolve a path, read a container's value, assign one — until the worker says the run is
   over (`crates/pdf-script-worker/src/form_wire.rs`, kinds 6 to 13). The worker's `Host` for
   `pdf_formcalc` is those questions; the host's answers come from `Form::resolve`, `Form::value`
   and `Form::set`, so **the form never crosses** and every assignment is charged to the activation's
   limit on assigned text (ADR 2507) in the host's `Form`, where a script run elsewhere cannot go
   round it. The greeting's magic moves from `PDFSCW01` to `PDFSCW02`: an older worker cannot
   answer a form run, and the greeting is where a mismatch is found cheaply, before a run is sent.
2. **`ScriptRunner` gains `run_form_script`**, the one call that carries a form's script out, with
   the signature of `pdf_model::xfa::Runner::run`. The default refuses by name, so a runner that has
   no FormCalc is a sentence in the report and not a silent skip; `pdf-model` still names no script
   language (ADR 2482 item 5). `ScriptWorker` answers it; `viewer-host`'s `warn` wrapper forwards it
   and says, once per activation in which a script ran, that it did (`form_runs_note`); the
   withheld runner of the *ask* level records the first form script as the question's subject.
3. **The bounds, each tested at the bound** (`form_wire.rs`, `tests/form_script.rs`): a path of at
   most 64 names of at most 4 096 bytes, a node list of at most 65 536, a text of at most 1 MiB, a
   count read against the bytes that remain before anything is reserved; **10 000 questions a run**,
   after which the host answers each with a sentence and the script ends with that error (a
   compromised worker that ignores its step budget is the case; `Budget::CALCULATION` is 100 000
   steps and keeps an honest one under it); and the run is held to the worker's deadline as a
   JavaScript run is, a run that reaches it costs the worker, reports itself past the activation's
   steps so that the cascade ends there (page 380 recommends a means of ending a seemingly infinite
   loop), and the next run starts another worker. A worker busy holding a script on a person's
   answer is not asked: the form script is refused with that sentence.
4. **A container crosses as the host's number for it**, which the worker can use only to ask the
   host again. A number it was never given reads as null and refuses an assignment
   (`Form::value` and `Form::set` look up, never index). The worker can read any container's value
   by guessing numbers; the JavaScript realm is told every field's value at its first run (ADR
   1602), so a form script learns nothing the document's scripts did not already hold.

## Consequences

- **Measured** (load average 5–8, debug profile): a worker's start and 10 000 questions together take 0.08–0.11 s, about 8 µs a
  question, so the 250 ms deadline allows about three times the cap. The test process's peak is
  12 MB with a script that builds a mebibyte and assigns it 200 times; the worker's own ceiling
  is the profile's address-space limit (ADR 1608, `end_to_end.rs` asserts 96 MiB), and the host-side
  copy of an assigned text is the mebibyte of one frame.
- The transport's frame bound from a worker (`confined_transport::frame::MAX_MESSAGE`) is
  inherited as the JavaScript replies inherit it; it is not new here.
- The confined window pins the level `off` (ADR 1616) and so runs none of this; the windows that
  run scripts reach it through `viewer-host`'s `Workers`.
- Owed: the JavaScript half (§K.2's note), and the events other than `change` and `initialize`.
