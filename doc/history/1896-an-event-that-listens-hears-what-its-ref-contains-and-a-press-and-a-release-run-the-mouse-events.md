# 1896 — An event that listens hears what its `ref` contains, and a press and a release run the mouse events

Slot 6 of batch 130, 2026-11-02, Opus 5.5, one session. ADRs 2626 and 2627 used; Q772 unused. Ledger: §K.2 stays
`partial`, and its note now names `listen` among the built and `mouseDown` and `mouseUp` among the events.

**Built.** `Session::fire` follows XFA 3.3 page 387's propagation: each `change`, `exit`, `enter`, `click`,
`mouseDown`, `mouseUp` or `initialize` on a field, subform or exclusion group runs the object's own events, then the
listening events of each enclosing container innermost first, then the listening events bound to `$form`, and no
further (ADR 2626). `Listeners::of` finds, in one pass, each container's nearest listening container (the template
numbers a container after its parent), and every heard event is charged one run of the activation limit, so the
dispatch is bounded by the runs an activation may make. `ready` and `docReady` happen on a DOM and the application,
which no container contains, so a listening one runs once. The report's `refAndDescendents` sentence is withdrawn.
`Stimulus::MouseDown` and `Stimulus::MouseUp` are the field's own activities (pages 391–392); `viewer-core` raises
`mouseDown` at `/D` on a widget and `mouseUp` at every release over one, wherever the press went down, before the
`/U` that raises `click` (ADR 2627). `ViewState::activate_form_on_mouse_down` and `_on_mouse_up` are the entries.

**Premise.** Held: `listen` as a second pass over the container chain, the mouse events as two more `Stimulus`
arms. Not held as written: the brief's list of activities omits `initialize`, which the merge fires and page 387
does not exclude, so it propagates too; `mouseUp` is not `/U`'s, since page 392 states a release within the region
alone and §12.6.3 ties `/U` to a press on the same annotation; and the order is the processor's to choose (page
387), so innermost-first is a choice written in ADR 2626 item 2.

**Plant.** A `$form`-bound listening `click` that never ends (three scripts) is stopped within 5 s in the confined
worker after the field's own `click` ran and said so; disabling propagation fails four of the new cascade tests.
Memory high-water (debug, `getrusage`): ten thousand and one heard events 19 MiB against 18 MiB for as many own
events; the two-hundred-thousand-event template 505 MiB, ADR 2606's figure.

**Found, not fixed.** `activate.rs`'s former `clicked` carried a sentence with a run of spaces inside it (a
continuation without `\`); its replacement `pointed` writes the sentence whole. No `$event` object exists, so a
listening script cannot read `$event.target` (page 398), owed in ADR 2626.

**Gates.** `rustfmt --check --edition 2024` on my files 0; `cargo fmt --check` 0 for `pdf-model`, `pdf-formcalc`,
`pdf-script-worker` (`viewer-core` 1, in a sibling's `notes.rs` only); `RUSTFLAGS="-D warnings" cargo clippy` on the
four `--all-targets` 0; `cargo test -p pdf-formcalc` 69 passed 0; `-p pdf-script-worker --features engine` 50 passed
0; `-p viewer-core` 435 passed 0; `-p pdf-model --test xfa_datasets` 18 passed 0; `-p pdf-model --test
script_corpus` 0 (1 ignored); `--bin cited` 0; `--bin quotations` 0; `cargo test -p conformance --no-fail-fast` 101,
468 passed 2 failed, neither mine (`adr_status` on a sibling's ADR 2630, `records` on a sibling's record 1898).
No arm ran: only `viewer-core` installs a script runner, at a level a reader sets, and no corpus arm goes through it.
