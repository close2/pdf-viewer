# 1942 — A radio button names its field, an event bound by `ref` runs, and a script test's `warning` asks

Session 1942, a single ledger round (K.2), the ledger lane. ADRs 2698 and 2699; no question.

**The slice.** Of K.2's owed list, the items a person sees or does: a pointer and the focus on one
field of an exclusion group, events bound to another object by `ref`, and page 385's choice for a
script test's `warning`. Page 392's choice-list events were left, because they need the selection
commit that a window does not yet report, the `reenter` property, and the legacy processing
instruction. **The premise held.** `Session::pointed` refused an exclusion group by name,
`unfollowed` named every other `ref` as not run, and `validation_notice` had one button. Effort was
the only obstacle, and nothing needed the owner.

**Exclusion groups (ADR 2698).** The widget's on value is `form::button_on_value`: `/Opt` at its
place among the field's widgets, or else its on state. It names the group's field whose first
`items` value it is, the rule `datasets.rs` already writes by (pages 195–197). A click, a press, a
release, a crossing and the focus on a radio widget run that field's events (pages 390–392).
**Events by `ref` (ADR 2699).** `Step::path`, moved from `pdf-script` so that one reader serves
both, reads a `ref`. It is resolved from the holder (pages 114–115, 706–707) and its events run
when the object generates the activity. `listen` is keyed by the bound object. A `ref` that names
nothing is named at `ready`, and each search is charged the template's size in steps.
**Page 385 (ADR 2698).** A group of warnings is a Yes/No question. Its "No" asks for the focus on
the first failing field. **`Activation::validations` is removed**: a save and a submit already run
those tests over the values they write.

**Tests**: `cascade` 8 new, `script_form` 1, one unit test amended. **Plant** (second `items` value,
`ref`s ignored, focus kept on the group, no focus request): 10 of 171 fail.

**Left of K.2**: the other events, an event's other properties, page 392's choice-list events,
the unset `$event` properties and `cancelAction`, script tests at a save and a submit, and
`validationState`. Effort only.

**Gates.** fmt `--check`, clippy `-D warnings --all-targets` (`pdf-model`, `pdf-formcalc`, `viewer-core`,
`pdf-script-worker`, `pdf-script`, and the last two with `engine`): exit 0. nextest of the five, with
`engine`: 3139 of 3141 passed. `xfa_javascript`'s stale `xfa.event` expectation, which ADR 2646 left
behind, is corrected, and its wall-clock bound passes alone: 8 of 8. `cargo test -p conformance`
after `git add -N`: exit 101 on HEAD's A784 medium only. `state.sh docs`: exit 0. Tier 2:
`script_corpus`, `selection_census` and `accessibility_census` exit 0. `corpus`, `raster_golden` and
the engine `script_corpus` exit 101 with 1941's HEAD readings, line for line.
