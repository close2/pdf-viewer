# 1946 — A container's validation state fires its event, and a choice list's selection exits it

Session 1946, a single ledger round (K.2), the ledger lane. ADRs 2706 and 2707; no question.

**The slice.** Of K.2's owed list, the two a person sees: `validationState` (XFA 3.3 pages 396–397)
and page 392's choice-list `exit`/`enter` with `$event.reenter` (page 403) and the
`originalXFAVersion` instruction (page 393). **The premise held.** `Resource::activate` was
stateless, no stimulus fired either event, `reenter` sat in `UNSUPPLIED`, and the XML reader
dropped processing instructions. Effort only; nothing owner-blocked.

**Validation state (ADR 2706).** `xfa::Validity` keeps per container the tests its last run of each
failed; `Resource::activate_kept` takes it, and the view state holds it across activations. Script
tests record their verdicts (an empty field fails none, page 396), `PreSave`/`PreSubmit` the null,
datatype and (submit) format tests where the `config` lists them. A container's state is its own
and its content's (page 397). The merge fires every field's, exclusion group's and subform's event
after `initialize`; later activations fire only where the state or its reason changed. A budget end
restores the state.
**Choice lists (ADR 2707).** `Command::CommitField` on a field whose widget keeps this crate's
focus is the selection commit: `Stimulus::Selected` fires the list's `exit` after it, and the next
commit `Stimulus::Reentered` its `enter` first with `reenter` true. The focus then leaving fires no
second list `exit` (a reading of page 392's correction). A template whose namespace, or whose
`originalXFAVersion` with `LegacyEventModel:1`, names a version before 2.4 keeps the old model.
`reenter` is false on a field that is no choice list and refused on a choice list's non-`enter`.
`notes::about` now names what runs.

**Tests**: `cascade` 6 new, `script_form` 2, `validity` unit 1. **Plant** (blur exit not
suppressed, reenter never true, pre-event verdicts dropped, changes never announced, legacy
ignored, the window never reporting the commit): 7 of 93 fail.

**Left of K.2**: the other events (`preOpen`/`postOpen` need a window that reports a drop-down
opening), an event's other properties, the format test's re-evaluation at a change (page 396), the
unset `$event` properties, a save's `cancelAction` (the reader's four levels), a transmitted
submit's `postSubmit` with no answer, and RFC 0011 step 5's events. Effort only, except the save's.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-model`, `pdf-formcalc`,
`viewer-core`): exit 0, after `xml::parse` and `activate_kept` were split under a hundred lines.
nextest of the three after `pdf-sandbox --bins`: 2914 of 2914. `cargo test -p conformance` after
`git add -N`: exit 0. `state.sh docs`: exit 0, rerun with `CARGO_INCREMENTAL=0` after rustdoc's
first run panicked in `render-cpu` and `conformance`, crates this round did not touch. Tier 2:
`pdf-model` `script_corpus`, `selection_census` and `accessibility_census` exit 0.
