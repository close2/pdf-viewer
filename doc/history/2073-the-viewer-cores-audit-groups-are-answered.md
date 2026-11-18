# 2073 — The viewer core's audit groups are answered

Session 2073, an audit-fix round in `pdf-viewer-lane4`. Groups G100 to G110 of
`doc/reviews/2038-specification-audit/fixes.md`: 30 findings (15 B, 12 T, 3 D).

- **Fixed 28, open 2, argued 0, owner 0.** Each line of `fixes.md` names its ADR and tests.
- ADR 2964: Table 29's open action before page one's `/O` and `/PO`; a GoToR or GoToE
  destination keeps its Table 149 view; `/FitR` and `/FitB` centred; Table 205's `/R /P`
  over a kept lineage (`Open::parents`); §12.5.6.18's ledger note.
- ADR 2965: `/TM` read from the terminal field only and written by the HTML Form body;
  `SubmitForm::fields` and `ResetForm::fields` are `Option`, so `[]` is a list.
- ADR 2966: Table 273's `/V` past PDF 2.0 is unmet and counts in the penalty.
- ADR 2967: `VisiblePages` unions every page the arrangement places.
- ADR 2968: `revocation::Basis` — §12.8.3.4.7's thisUpdate end and §12.8.3.4.8's producedAt
  condition; a compromised TSA path's genTime is not used.
- ADR 2969: a PAdES-signed document reports its verification at the first present.
- 18 expected failures pass and leave `expected-failures-current.json`; no new failure.
- The twelve T tests were each plant-checked (a Sonnet helper on a scratch copy); three
  weak spots are written on their lines.
- Open: B49 (the chain's script slots are in `script_sites.rs`, G028's) and B73 (an image's
  `/Measure` needs the interpreter to report image placements).
- One owner audit file changed: `inherited_reset_defaults_audit.rs`'s `fields: Vec::new()`
  became `fields: None` under ADR 2965's type; its assertions are unchanged.
- `viewer.rs` (named by G021) was edited for B22 and B17: the Presented arm.

**Gates.** fmt exit 1 on one file not this round's (`named_property_actualtext_spec_audit.rs`, as on
main); clippy `-D warnings --all-targets` 0; nextest of the four crates 64 failed, exactly the
manifest's; rustdoc 0; fuzz `cargo check --bins` 0; conformance as main's five.

Habit: a file another group names is checked before a fix spreads into it; B49 was built,
found to need `script_sites.rs`, and withdrawn.
