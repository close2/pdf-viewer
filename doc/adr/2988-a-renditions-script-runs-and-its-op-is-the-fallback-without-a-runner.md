# 2988 — A rendition's `/JS` runs, and its `/OP` is the fallback where no runner is supplied

Session 2077. Status: **implemented.** Supersedes the reading in `pdf_model::action`'s `rendition` that no runner
is handed a rendition's `/JS` (ADRs 1904 and 1960 on this point). Context: review 2038 (finding B49). Clause:
ISO 32000-2 §12.6.4.14, Table 218.

Table 218 makes `/JS` "a text string or stream containing an ECMAScript script that shall be executed when the
action is triggered", and the paragraph under the table orders the pair: "Either the JS entry or the OP entry shall
be present. If both are present, OP is considered a fallback that shall be executed if the interactive PDF
processor is unable to execute ECMAScripts." The tree read the action for `/OP` alone and handed its `/JS` to
nobody, so a click on a link whose rendition stated both performed the fallback beside a runner able to run the
script, and the script never ran.

Now:

1. `view::scripts::read_script` reads a rendition action that states `/JS` as an ECMAScript action, at every site
   that runs scripts (an annotation's events, a page's, an outline item's, the open action, the document's).
2. `action::Rendition` carries `script`, whether `/JS` is present, so the action path can tell such a rendition
   apart.
3. **"Unable to execute ECMAScripts" is read as no runner supplied** (`ViewState::runs_scripts`): the scripting
   level `off`, or a host that supplies none. Where a runner is supplied the rendition's `/OP` is not performed:
   `ViewState::perform_annotation_chain` treats the rendition as the chain's ECMAScript slot in its place, and
   `viewer-core`'s `interact::handed_over` drops it from every other chain. Where none is supplied `/OP` is
   performed as before.

**The reading is processor-level, not per script.** A script the runner declines, a `/JS` too long to read (ADR
1950), one raised before the open sequence (ADR 1750) or one the reader withholds under `ask`: in each case the
processor holds a runner and is able to run scripts, the reason the script did not run is said, and `/OP` is not
substituted for it. The alternative, performing `/OP` wherever this one script did not run, would make the
media hand-off depend on a script's length or on the order of events, and the table names the processor's ability
rather than the outcome of one script.

Tests: `crates/pdf-model/tests/rendition_scripts.rs` (the script handed over and `/OP` not performed; `/OP` performed
without a runner; a rendition without `/JS` performs `/OP` under a runner; the script in its place in a `/Next`
chain), and the audit's `crates/viewer-core/tests/rendition_preconditions_spec_audit.rs`, whose
`an_enabled_runner_executes_the_rendition_script_and_does_not_use_op_as_fallback` failed before this change.
