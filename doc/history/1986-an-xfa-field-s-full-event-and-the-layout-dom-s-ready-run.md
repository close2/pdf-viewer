# 1986 — An XFA field's `full` event and the Layout DOM's `ready` run

Session 1986, a single ledger round (K.2), the ledger lane. ADRs 2783 and 2784; no question.

**The slice, and why.** Of round 1983's four, the two whose moment the window already has: the
whole text a person typed reaches `set_field` before the field's room cuts it, and a static form's
layout is the pages the file holds, complete at the open sequence's end. Both are `pdf-model` alone.
`preOpen`/`postOpen` need four hosts, a confined frame and two C ABI entries: priced, not built.

**The premise.** No stimulus raised `full`; `$layout` fell to `Bound::Other` and was reported as
unrun. Round 1983's "each needs the window to report a moment" held for the drop-down pair only.

**Built.** `xfa::Typing::full`, set by `view::scripts::fills` where the window's text is longer than
the field takes or holds `/MaxLen` characters (pages 391, 406), the `DoNotScroll` measure shared with
the value; `full` fires after `change` at the commit (page 406's rule 2), its `$event` page 399's
row plus page 400's `selStart`/`selEnd`, no `name` or `keyDown`. `Bound::Layout` for `$layout` and
`xfa.layout`; its `ready` runs once between the Form DOM's `ready` and `docReady` (a choice, ADR
2783); the Form DOM's `ready` skips `pageSet` containers (page 388's note).

**Tests.** `cascade.rs`: `full` after `change` with `fullText|change|newText`, none where not full,
`$event.name` refused; the ready order `form!alias!layout!laid!doc!` with `$data` reported; the page
background hears the layout's `ready` only. `script_form.rs`: `/MaxLen 3` typed `12`, `123`, `1234`
and at `off`; the presented form runs a `$layout` ready and reports a `$data` one.
**Plants**, together and reversed: `full` before `change`, the layout's `ready` dropped, the
page-background skip off, `fills` always false, `name` offered to `full` — all five tests fail;
reversed, all pass.

**Gates.** fmt `--check` of `pdf-model`, `pdf-formcalc`, `viewer-core`: exit 0; clippy `-D warnings
--all-targets` of the three: exit 0 (`CARGO_INCREMENTAL=0`); nextest of the three: 2981 of 2981, 24
skipped; conformance after `git add -N`: exit 0; `pdf-model script_corpus`: exit 0 (67 445 walked, safedocs' 65 968; tika-issue-tracker and openpreserve absent; 5 982 s);
`fuzz cargo check --bins`: exit 101 at HEAD on errors in policy_document.rs and formcalc_cascade.rs that main fixed in cd743e60, none this round's; with that commit's two files applied, exit 0; `state.sh --round 1986 docs`: exit 0. Load 100–860.

**Left of the row**: `preOpen` and `postOpen` (page 392), ADR 2784's construction.
