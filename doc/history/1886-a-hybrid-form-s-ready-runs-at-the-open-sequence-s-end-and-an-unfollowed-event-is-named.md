# 1886 — A hybrid form's `ready` runs at the open sequence's end, and an unfollowed event is named

Slot 6 of batch 129, 2026-10-29, Opus 5.5, one session. ADR 2606 used; ADR 2607 and Q762 unused. Ledger: §K.2 stays
`partial`, and its note now names `ready` and `docReady` among the events built and `listen` among the reported.

**Built.** The template reader keeps an event's `ref` as `Bound` (its container, `$form`, `$host`, or another SOM
expression as stated) and reads `listen` (XFA 3.3 pages 387 and 706). `Stimulus::Ready` fires the `ready` events
bound to the Form DOM, which page 388 fires after the merge's calculations and validations (page 213's step 7),
then the `docReady` events bound to the application, which page 388 puts after it; the calculations that read what
each changed follow. `ViewState::activate_form_on_merge` raises it after the merge and the entries it re-raises,
under the clock the merge started, so the two open-sequence sites in `viewer-core` raise it once a sequence. The
same activation names every event with `listen="refAndDescendents"` (run for its container alone) and every event
whose `ref` names another object, `$layout` among them (never run), by container and activity.

**Premise.** The hypothesis held: one `Stimulus` arm, one attribute, one report walk. "Before the first draw" did
not: ADR 2525 item 2 made the merge's completion the first frame, so `ready` runs after that frame and what it sets
is drawn at the next (ADR 2606 item 2). The census's `$form` binding held; a `ready` bound to the field itself fires
nothing, since page 703 requires a `ref` that can generate the activity. `$layout`'s `ready` is reported, not run.

**Plant.** A `ready` script that never ends (`for (;;) {}`, unbounded recursion, a loop only the clock ends) is
stopped within 5 s by the merge's sequence clock in the confined worker; the merge's calculated value stays drawn
and the event's assignment is dropped. Planting the defect back — dropping the `Ready` call — failed all three
`ready` tests in `form_script` and `script_form`. **Bounds.** One more `ready` event than an activation runs ends on
the limit and keeps nothing. Two hundred thousand unfollowed events are walked in 0.65 s, one sentence for the
repeated one and the report at its cap of 257; `getrusage`'s high-water 516 492 KiB beside 516 744 KiB for the same
template under the merge alone (HEAD's path), so the walk adds nothing measurable.

**Gates.** A sibling's mid-edit in `pdf-font` broke the shared build, so the crate gates ran in an export of HEAD
plus this round's files (habit 138) and the work landed by `git merge-file` (exit 0 each), `cargo check --tests` on
the four crates after it exit 0. `cargo fmt --check` on pdf-model, pdf-formcalc, pdf-script-worker and viewer-core:
exit 0 each. Clippy `-D warnings --all-targets` on pdf-model, pdf-formcalc and viewer-core, and on pdf-script-worker
`--features engine`: exit 0 each. `cargo test -p pdf-formcalc`: exit 0; `-p pdf-model --test xfa_datasets`: exit 0;
`-p pdf-script-worker --features engine`: exit 0; `-p viewer-core`: exit 0; nextest `-p pdf-model`: 2 361 passed.
`--bin cited` exit 0, `--bin quotations` exit 0. Rustdoc `-D warnings` on pdf-model: exit 0. `-p pdf-model --test script_corpus -- --ignored` under the lock: 1 passed, exit 0. `cargo test -p conformance --no-fail-fast` in the shared worktree: exit 101, one red, `records.rs` on session 1885's record mid-write, not this round's. Not run: the six arms, since scripts are
`off` in every arm and no pixel can move; `pdf-script --features engine --test script_corpus`, since nothing it
reaches changed; the census, since no conversion path changed.

**Owed** (ADR 2606): firing `listen`- and `ref`-bound events, `mouseUp`, `mouseDown`, page 392's choice-list events.
