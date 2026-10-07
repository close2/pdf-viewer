# 1640 — A script's page turn is an edit the host makes, counted from the page the event is on

Status: accepted and **built** in `pdf-script` and `pdf-model`; the windows' half is round 1403's
(`Viewer` carries the request out as a page turn). Session 1402. Builds on RFC 0008 sections 4.2
and 6.4 (`pageNum` is Tier 1), ADR 1615 (`setFocus`, the request a view state holds for a host).
Context: Adobe's *JavaScript for Acrobat API Reference*, "Doc properties", `pageNum`, cited by name
and never quoted.
Code: `crates/pdf-script/src/engine/bridge.rs` (`page_num`, `write_page_num`, `page_named`),
`crates/pdf-script/src/wire.rs` (edit tag 6, `VERSION` 5),
`crates/pdf-model/src/view/script_model.rs` (`ScriptEdit::GoTo`),
`crates/pdf-model/src/view/scripts.rs` (`ViewState::take_page_request`, `field_page`).
Tests: `crates/pdf-script/tests/realm.rs`
(`a_write_to_page_num_is_a_page_turn_the_script_reads_back`), `tests/hook.rs`
(`a_field_script_s_page_turn_counts_from_its_own_page_and_waits_for_the_host`), `tests/wire.rs`.

## 1. The write was refused, and three scripts of the census caught the refusal

`this.pageNum` was read-only: a write threw `NotAllowedError`, so three runs of the Tier 1 column
finished refused. The reference makes the property read and written, zero-based, and gives
`this.pageNum = 0` and `this.pageNum++` as its two examples — both fixtures now. What a person does
by hand is turn the page, and RFC 0008 section 4.2 admits exactly that.

## 2. The turn is an edit; which page is shown stays the host's

A write records `ScriptEdit::GoTo { page }`, the script reads the page back for the rest of its run,
and the latest turn of a run replaces an earlier one. All or nothing, as every edit: a run that
throws turns nothing. The view state holds no page of its own, so `apply_edits` checks the page
against the document's and keeps it for `ViewState::take_page_request`, which a host takes after
any call that ran scripts and carries out as a person's page turn — the shape `setFocus` already
has (ADR 1615). For round 1403: after `carry_out_focus_requests`, take the request and
`go_to(PageTarget::Index(page), Turn::Requested, …)`; the leaving page's `/C` and the arriving
page's `/O` then run as for any turn.

## 3. What the reference leaves unstated, decided here

**A value that names no page turns nothing**: the value is ECMAScript's `ToNumber`, truncated
toward zero; one that is not finite or falls outside `0..numPages` is said in the run's notes and
the script goes on — a turn past the last page goes nowhere for a person either, and a throw would
stop the rest of a script whose author wrote `this.pageNum++` on the last page. A `Symbol` still
throws, as `ToNumber` does.

**A field's event is on its widget's page.** `this.pageNum` read the request's page, which every
field event left at 0; a `this.pageNum++` in a field on page 3 would have turned to page 2. A field
event now carries the page Table 166's `/P` names for the field's first widget — the page a person
is on when they type into it — and the open, page and document events keep the page the host
states. A format run only to answer what a field displays keeps 0, since what it does is recorded
nowhere.
