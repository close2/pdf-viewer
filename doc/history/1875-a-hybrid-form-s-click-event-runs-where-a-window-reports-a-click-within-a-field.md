# 1875 — A hybrid form's `click` event runs where a window reports a click within a field

Slot 5 of batch 128, 2026-10-29, Opus 5.5, one session. ADR 2584 used; ADR 2585 and Q751 unused. Ledger: §K.2 stays
`partial`, and its note now names `click` among the events built and what remains.

**Built.** `Stimulus::Click` fires the field's own `click` events (XFA 3.3 page 391) and then the calculations that
read what they changed (page 379). `ViewState::activate_form_on_click` runs it through the host's runner under ADR
2525's level, deadline and clock. Its one call site is `viewer-core`'s `raise` at Table 197's `/U` on a widget, before
the widget's own scripts. `/U` is raised only for a press and a release within the same annotation. An exclusion
group's widget, and a name reaching a container that is not a field, are reported. An event's `submit`, `execute`,
`signData` or `encryptData` (page 703) is now kept and reported where it fires. No wire, `Event` or `hosts/` change.

**Premise.** The hypothesis held: one `Stimulus` arm, one call in `raise`. Two parts of the brief did not. Nothing
bubbles: page 390 gives a subform and an exclusion group no `click`, and a container hears a field's events only
under page 387's `listen="refAndDescendents"`, which is not this slot. And XFA names no keyboard trigger: page 391
names a mouse click, page 479 does not say how a button is selected, and no window raises `/U` from a key.

**Plant.** A `click` script that never ends (`for (;;) {}`, unbounded recursion, a loop only the clock ends) runs
after a person's commit in another field. The confined worker stops it within 5 s, the entry stands, the event's own
assignment is dropped, and the report names the event and the limit. Three planted defects were caught. Dropping
the call in `raise` fails `script_form`'s click test. Moving it to `/D` fails the test's "released outside the field"
assertion. Letting the click climb the container chain fails `a_click_fires_the_fields_own_click_event_and_no_other`
(`Button!A!`). A press on no widget raises nothing (the same test). **Bounds.** The click fires on one container. One more `click` event than an activation runs ends on the limit and
keeps nothing: 20.9 MB high-water by `getrusage`, beside 19.2 MB for the same binary's `exit` case, which HEAD has.
A hundred thousand submitting `click` events are walked in 0.27 s, held to the XML reader's element bound, and give
one sentence: 116 MB high-water for a 4 MB template in a debug build.

**Gates.** A sibling's mid-edit in `pdf-signature` broke the shared build, so the crate gates ran once in an export
of HEAD plus this round's files (habit 138). `cargo fmt --check` on pdf-model, viewer-core, pdf-formcalc and
pdf-script-worker: exit 0 each. Clippy `-D warnings --all-targets` on pdf-model, viewer-core and pdf-formcalc: exit 0;
on pdf-script-worker `--features engine`: exit 0. nextest: pdf-model, viewer-core and pdf-formcalc 2 844 passed;
pdf-script-worker `--features engine` 46 passed. `-p pdf-model --test xfa_datasets`: 18 passed. `-p pdf-model --test
script_corpus -- --ignored` under the lock: 1 passed, exit 0. Rustdoc `-D warnings` on pdf-model and viewer-core:
exit 0. `--bin cited` 0, `--bin quotations` 0. `cargo test -p conformance --no-fail-fast` in the shared worktree: exit 0, 470 passed, no red. Not run: the six arms, since scripts are `off`
in every arm and no pixel can move; `pdf-script --features engine --test script_corpus`, since nothing it reaches
changed.

**Owed** (ADR 2584): a click within one member of an exclusion group, performing page 703's other properties.
