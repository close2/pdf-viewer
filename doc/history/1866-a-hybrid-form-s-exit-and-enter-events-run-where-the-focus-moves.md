# 1866 — A hybrid form's `exit` and `enter` events run where a window moves the keyboard focus

Slot 6 of batch 127, 2026-10-28, Opus 5.5, one session. ADR 2566 used; ADR 2567 and Q732 unused. Ledger: §K.2 stays
`partial`, and its note now names the two events built and what remains.

**Census.** A subagent counted the `event` activities through `pdf-model`'s template reader. The brief's two trees
hold six readable hybrids, too few to rank. The crawl cache holds 151 static templates. After `change` (2 177 events,
43 documents) the most frequent are `exit` (1 491, 45), `click` (941, 69), `mouseUp` (363, 13), `ready` (297, 42,
bound by `ref` to `$form`), `initialize` (258, 27) and `enter` (193, 21). Every window already reports the focus.
`exit`, the most frequent, was built with its mirror `enter` (ADR 2566).

**Built.** `Stimulus::Exit` and `Stimulus::Enter` fire on the field and on each subform and exclusion group the focus
crosses (XFA 3.3 pages 390–391), then settle the calculations that depend on what changed (page 379). The view state's
`activate_form_on_exit` and `activate_form_on_enter` run them through the host's runner under ADR 2525's clock. Their
one call site is `viewer-core`'s `raise`: the `exit` at `/Bl` after the commit, the `enter` at `/Fo`. The order inside
each half is a written choice. No wire, `Event` or `hosts/windows` change.

**Premise.** The hypothesis held (an event is one more activity, its trigger one `viewer-core` call site); the census
population did not, since the two trees the tests read are too thin, so the ranking is the crawl cache's.

**Plant.** An `exit` script that never ends (`for (;;) {}`, unbounded recursion, a loop only the clock ends). It is
raised after the person's commit and stopped in the confined worker within 5 s. The entry stands, the event's own
assignment is dropped, and the report names the event and the limit. Planted defects were caught: drop the "also
holds the other field" filter and `focus_within_one_subform_fires_only_the_fields_events` fails; drop the call in
`raise` and the tab test in `script_form` fails. **Bounds.** The walk is held to the container count and the XML
depth of 256 (a test at 100 levels). A field with 10 001 `exit` events ends on the activation limit and keeps nothing.
That case peaks at 19.4 MB by `getrusage`, against 12.2 MB for a test of the same binary that HEAD also has.

**Gates.** `cargo fmt --check` on pdf-model, viewer-core, pdf-formcalc and pdf-script-worker: exit 0 each. Clippy
`-D warnings --all-targets`: exit 0 for pdf-model, viewer-core, pdf-formcalc and pdf-script-worker (`--features
engine`); the first pdf-model and pdf-formcalc runs failed in a sibling's `pdf-render`, and one finding of mine was
fixed. nextest: pdf-model 2 353 passed after the dev sandbox worker was rebuilt (trap 10); its first run lost seven
`ccitt` tests to the stale worker. viewer-core 432 passed, pdf-formcalc 51 passed, pdf-script-worker `--features
engine` 44 passed. `-p pdf-model --test xfa_datasets` 18 passed. `--bin cited` 0, `--bin quotations` 0, rustdoc `-D warnings` on pdf-model and viewer-core 0 after one private link fixed.
`cargo test -p conformance --no-fail-fast`: one red, `records`, on this record's gate line, fixed; `--test records` 3 passed. Not run: the six arms, since scripts are `off` in every arm
and no pixel can move there; `pdf-script --features engine --test script_corpus`, since nothing it reaches changed;
the protocol fuzz target, since the wire is unchanged.

**Owed.** `click` (the most documents), `ready`, `mouseUp` and `mouseDown`, each one call site at `/D` or `/U` in
`raise`. Also owed: page 392's choice-list `exit`/`enter`, and `ref`- or `listen`-bound events.
