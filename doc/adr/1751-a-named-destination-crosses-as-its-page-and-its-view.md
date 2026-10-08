# 1751 — A named destination crosses as its page and its view

Status: accepted and **built**. Session 1457. Supersedes ADR 1724's `gotoNamedDest` row (a page turn,
the destination's view reported as not applied) and extends ADR 1736's `ViewChange`.
Code: `crates/pdf-model/src/view/script_model.rs` (`ViewChange::Destination`),
`crates/pdf-model/src/view/scripts.rs` (`go_to_named`, `ask_view`), `crates/pdf-script/src/wire.rs`
(version 12, `put_destination_view`, `destination_view`), `crates/viewer-core/src/viewer.rs` (one named
arm in `carry_out_view_requests`), `crates/pdf-script-worker/examples/wire_seeds.rs`.
Tests: `pdf-script`'s `pages_and_choices.rs`
(`a_named_destination_shows_its_page_and_its_view_and_an_unknown_one_shows_none`), `tests/wire.rs`,
`viewer-core`'s `tests/script_view.rs` (two).

## The decision

`this.gotoNamedDest(cName)` names §12.3.2.4's destination; the realm sends only the name
(`ScriptEdit::Destination`), the view state looks it up where the clause keeps it, and **holds one
`ViewChange::Destination { page, view }` — the destination's page and Table 149's view together —
in place of the page request it held**. One request rather than a page request and a view request,
because Table 149 gives the view *of that page*: a host that carried out the turn and then lost the
view, or the reverse, would show a place no destination names, which is the reason `viewer-core`'s
link path takes both from one destination. The host shows it as a link's: the view goes to
`pending_views`, applied at `settle` against the drawn page, and the page is turned to where it is
not the one showing. A page past the document's count is refused as a scroll's is.

ADR 1736 cited the destination syntax as Table 151; it is Table 149 (Table 151 is an outline item's
entries).

## What it costs

`ViewChange` crosses the worker's wire inside `ScriptEdit::View`, so the wire carries the new form
too — tag 4, the page, then the view's own tag and numbers, a null a 0 byte — and moves to version
12, which turns every `script_wire` seed on disk into a version refusal until the seeds are
re-written (`fuzz/seeds.sh`'s `script_wire` arm). A realm that sends the form itself asks for a page
and a view of it, which is no more than `scroll` and `zoom` already reach.
