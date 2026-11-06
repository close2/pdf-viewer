# 1956 — A field's format test runs when its content changes, and its exit reads how it was committed

Session 1956, a single ledger round (K.2), the ledger lane. ADRs 2726 and 2727; no question.

**The slice, and why.** Of K.2's remainder, what a person sees and what needs no host to say more: the format
test re-run when content changes (page 396), which puts a message in front of the person and moves the
validation state, and the two `$event` properties this crate can answer from what it already knows —
`commitKey`, from the focus moves `viewer-core` raises itself (a tab, a press elsewhere), and the content types,
from the template. `modifier`, `shift`, `keyDown` and `fullText` need every host's pointer and focus commands to
carry the keys held (82 `Command::Pointer` constructions, and the confined protocol), and
`preOpen`/`postOpen` a drop-down no window draws: work for another round, not a question.

**The premise held.** `event.rs` refused all seven by name; the format test ran only before a submit and in the
write; no window passed how a focus left. Nothing is owner-blocked; no Q797.
**Built.** `$event.commitKey` (pages 398, 402) is `view::CommitKey`, the AcroForm script's numbers (ADR 1626),
carried by `Stimulus::Exit` and `activate_form_on_exit`; `Viewer::raise` passes `Click` from a pointer's raise
and `move_focus` `Tab` for `Next` and `Previous`; an exit the window did not describe refuses it with its own
sentence. The content types (page 403) are `Container::entry`, a `textEdit`'s `allowRichText` (page 865). The
format test (pages 382, 384–385, 396): every field an activation changed, and every field at a merge, through
`validate::conforms`; failures go into `Activation::script_tests` before the script tests' (page 381's order),
`Invalid` now naming its `test`; verdicts are `Validity`'s, so a change fires `validationState`; where no runner
is supplied a person's entry is still tested (`Resource::format_tests`), the test being no script.
`validation_notice` says what page 385 requires where the form gives no message: the input mask.

**Tests.** `cascade.rs`: commitKey 3, 1, 2 and untold, refused on a subform's exit and an enter; the content types
of four `ui`s and refused on a click; the format test at each level, conforming, empty and untouched; the state
changed once over merge, entry, entry. `script_form.rs`: a tab and a click through the window; the entry's message
at `off` and with a runner. `scripting.rs`: the input-mask wording. **Plant**: commitKey never told, content type
always plain, no change-time format test fail 6 tests; `move_focus` passing nothing and no runner-less path fail
the 2 window tests.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-model`, `viewer-core`, `pdf-formcalc`,
`pdf-script-worker`): exit 0. nextest of the four: 2934 of 2934, then `viewer-core` 446 of 446. `cargo test -p conformance`
after `git add -N`: exit 0. `state.sh --round 1956 docs`: exit 0. Tier 2: `dates`, `xmp`, `on-disk`, `ahead`, `ahead-strips`
exit 0; `corpus` and `golden` exit 101 as at HEAD, 62 incomplete documents and 16 moved pages; `scripts` 101, the engine
column's known ratchet (threw 9315 to 4); `transform` and `launch` not run: the wall budget went to the walk lock.

**Left of the row**: the other events; an event's properties other than `script`; `modifier`, `shift`, `keyDown`,
`fullText`, and `commitKey` where no tab or click took the focus; the merge's format tests at `off`; a save's
`cancelAction` (ADR 2702); a transmitted submit's `postSubmit` with no answer; RFC 0011 step 5's events.
