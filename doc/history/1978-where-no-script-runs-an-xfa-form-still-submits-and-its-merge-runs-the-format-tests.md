# 1978 — Where no script runs, an XFA form's events still submit and its merge runs the format tests

Session 1978, a single ledger round (K.2), the ledger lane. ADR 2767; 2768 not used; no question.
**The slice, and why.** The brief's first two: a submit at `off` and the merge's format tests at
`off`. Both are one fact — at `off` `run_form` returned before the form was activated — so one
construction answers both, and the `xdp` and `xml` packagings (a new body and a new submission
format in every host) did not fit beside it.

**The premise held.** XFA 3.3 page 703 gives an `event` a `submit` *in place of* a `script`, and page
382's format test is a picture clause, so neither is a script; ADR 2738 left the first and ADR 2727
the second as effort only. The host's submit level (`doc/questions/A98`), not the scripts level,
already decides what leaves the machine, as it does for an AcroForm `SubmitForm`. No owner's word.

**Built.** `Resource::activate_unscripted`: the same activation with no runner (`Session::runner`
is an `Option`); `dispatch` passes each `script` over unrun and unsaid, `settle` (the calculations)
and `script_tests` return at once, `unfollowed` says nothing, `perform` and the format tests run
unchanged. `ViewState::run_form` calls it where no runner is supplied, for every stimulus
(`format_test_unscripted` is gone), and `viewer-core` raises the form's pointer, focus and selection
events at every level (the `runner` gates in `pointer.rs`, `editing.rs` and `viewer.rs` removed), so
`send_form_submits` carries a submit out at `off` as at `on`. `notes::about`'s sentence says both.
The commitKey residue (a choice list's item click, `setFocus`, a page turn) is page 402's reading —
its four values are each a field that "lost focus" — and K.2's note now says so rather than owing it.

**Tests.** `cascade.rs`: a click's submit held unscripted with its click script unrun, a
`preSubmit`'s submit still refused; the merge failing `12` against `AAA-999`, the empty field
untested, no calculation. `script_form.rs`: the click's PDF submission at `off` without its
`preSubmit` script; a `mouseUp`'s `urlencoded` submit at `off`; the merge's notice at `off` and `run`.
**Plants**, each applied and reversed: every non-script property skipped where unscripted (three
tests fail), the format tests skipped where unscripted (three fail); reversed, all pass.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` of `pdf-model`, `pdf-formcalc`,
`viewer-core`: exit 0. nextest of the three: 2963 of 2963, 24 skipped. `cargo test -p conformance`
after `git add -N`: exit 0. `state.sh --round 1978 docs`: exit 0.
Tier 2 under `bounded.sh --lock`: `pdf-model script_corpus` 0 (67 445 PDFs walked, safedocs'
65 968 among them; tika-issue-tracker and openpreserve absent; 7 261 s under a load near 300),
`selection_census` 0; `accessibility_census` still queued behind the other lane's walks at the
report. No host changed: the change is in `viewer-core` and below, which every host reaches.

**Left of the row**: the other events and RFC 0011 step 5's; the `xdp` and `xml` packagings; an
`execute`'s WSDL call; XML signing and encryption.
