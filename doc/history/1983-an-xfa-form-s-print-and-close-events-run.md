# 1983 — An XFA form's `prePrint`, `postPrint` and `docClose` events run

Session 1983, a single ledger round (K.2), the ledger lane. ADR 2777; 2778 not used; no question.

**The slice, and why.** Page 388's application events: three of the six did not run, each has a
moment the window already marks (`Printing::Start`/`Finish`, where `/WP` and `/DP` run; `Close`,
where `/WC` runs), so no host, frame or C ABI entry changes, and the class is then whole.

**The premise held.** No stimulus or caller raised the three. Effort only, no owner's word.

**Built.** `Stimulus::PrePrint`, `PostPrint`, `DocClose`; `ViewState::activate_form_before_print`,
`_after_print`, `_on_close`. `begin_printing` runs `/WP`, then `prePrint` before the print intent;
its `cancelAction` (page 399) is `Restriction::PrintCancelled` asked at the reader's `print` level
after bits 3 and 12 (`Held::PrintCancelled` for `ask`), with page 400's `postPrint` cleanup where
nothing prints. `end_printing` runs `/DP`, then `postPrint`. `Close` runs `/WC`, then `docClose`:
every container's null (at `nullTest`'s level), datatype (always error, page 386), format and script
tests over the form as it is, and the events only where none fails at `error`; else each failure
named. A `submit` under `docClose` is said, not held (page 389's returned data has no form to go
into). At `off` the close validates nothing. Choices in ADR 2777: a validation run at the close
rather than a reading of earlier runs; `postPrint` at the operation's end.

**Tests.** `cascade.rs`: print's two halves bound to `$host` only; `prePrint`'s `cancelAction`;
`docClose` over ten cases (null error/warning, format and script error/warning, datatype); a
`docClose` `submit` said and a form with no `docClose` silent. `script_form.rs`: `prePrint` before
the pages and `postPrint` at `Finish`, at `run` and `off`; the cancel at `off`, `warn`, `on`, `ask`
yes and no; the close's report with the null test failing, passing, and at `off`.
**Plants**, applied together and reversed: `docClose` fired whatever the validations found,
`prePrint` read as `postPrint`, the close's submit origin dropped, the cancel ignored — all seven
new tests fail; reversed, all pass.

**Gates.** clippy `-D warnings --all-targets` of the four crates touched: exit 0
(`CARGO_INCREMENTAL=0`; clippy-driver panicked once incrementally); fmt clean but for HEAD's
`truetype_cmap.rs`, untouched; nextest 4107 of 4107; conformance after `git add -N`: exit 0;
`pdf-model script_corpus` 0 (67 445 walked, safedocs' 65 968; the other two absent; 6 288 s);
`state.sh docs` and the two `viewer-core` censuses still behind the walk lock at the report.

**Left of the row**: `full`, `preOpen`, `postOpen` and the Layout DOM's `ready`. `indexChange` is
the dynamic half's; `preExecute`/`postExecute` go with the refused WSDL call; `preSign`/`postSign`
have no moment while this program applies no signature.
