# 2986 — `/PV` and `/PI` scripts run where the arrangement gains or loses a page

Session 2076. Status: **implemented.** Supersedes ADR 2814 section 3 on when `/PV` and `/PI` scripts run. Context:
review 2038 (finding B78), ADR 2967. Clauses: ISO 32000-2 §12.6.3 Table 197.

Table 197's `/PV` is performed "when the page containing the annotation becomes visible", `/PI` "when the page
containing the annotation is no longer visible", and §12.6.3 says why they exist apart from `/PO` and `/PC`: "[t]he
PV and PI entries allow a distinction between pages that are open and pages that are visible." ADR 2814 raised the
two pairs' non-script actions at their own transitions but left their *scripts* riding with the current page's open
and close (`ViewState::run_page_scripts`), so a page that appeared beside the current one ran no `/PV` script, one
that left the arrangement ran no `/PI`, and a turn between two pages both on the screen ran both spuriously.

Now `run_page_scripts` runs the page's `/O` or `/C` and its annotations' `/PO` or `/PC` only, and
`Viewer::raise` runs a `/PV` or `/PI` script through `run_annotation_scripts` where `visibility_of` raises it. The
open sequence is the one exception: it is the first moment any script may run, and it runs the presented page's
`/PO` and `/PV` (`annotation_triggers(trigger, visible)`); before it, a `/PV` raised by the first arrangement hands
nothing over (`ViewState::scripts_opened`). A page beside the presented one at the open has its `/PV` actions
performed and its script not run, because no script runs before the open sequence; that is recorded here as the
one gap.

Tests: `viewer-core` `page_visible_script_runner_spec_audit` (three).
