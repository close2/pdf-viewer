# 2026 — A redaction's `/RO` is drawn strictly after the removal

Session 2026, a single ledger round in the audit worktree. Row 12.5.6.23; owner answer A842 (on round 2011's Q842);
ADRs 2868 and 2869.

**The slice.** A842's three owes: restore ADR 2841's withdrawn construction, read the authoring line's "overlay" as
`/OverlayText` and `/IC`, and change the one audit test A842 names.

**The premise.** At HEAD nothing drew `/RO`; the removal was proven (`redaction_is_removal.rs`) and a page with any
overlay key departed. Effort was the only obstacle; no question was needed.

**Built.** `redact::overlay` (ADR 2868): after `plan_page` returns the edited content, each redaction's `/RO` is
appended as `q 1 0 0 1 x y cm /OverlayRO<n> Do Q`, behind a `q` and enough `Q`s to close the content back to the
page's first state; `(x, y)` is `/Rect`'s lower-left corner less the form's `/Matrix` `(e, f)`, so form space's
origin lines up (Table 195, §8.10.1). Not drawn, each a per-page departure: a `/RO` that is no form with a `/BBox`,
one whose references reach the page's original content or a `/Redact` annotation, a redaction with no `/Rect`, and
content that pops a state it never pushed (§8.4.2). With `/RO` present, `/IC` and `/OverlayText` are ignored as the
table says; without it they are still not composed and still reported. `CLAUDE.md`'s sentence and
`doc/scope-ratifications.md` name `/OverlayText` and `/IC` as the excluded overlay (ADR 2869, A834, A842). ADR 2841
carries a forward pointer to 2868.

**The audit test.** `redaction_projection_spec_audit.rs::a_redaction_ro_is_reported_while_removal_proceeds_without_composing_the_overlay`
now expects the form's green at (15, 45) and no departure, its removal checks unchanged; its name is kept, since
A842 sanctions the expectation only. No other audit test was touched.

**Tests.** `redaction_is_removal.rs`, six: the removal by extraction from every stream, then the form invoked once,
after every operator of the edited content, unaltered; the origin under a `/Matrix` and a reversed `/Rect`; a `/RO`
that is no form; one reaching the removed content; the first-state closing and the §8.4.2 refusal; the control.
**Plants**: the forms written before the edited content failed three; the producer's content kept under the form
failed all six and the audit test ("KEEP\nSECRET").

**Gates.** `pdf-transform`: `cargo fmt --check` exit 0 (after one formatting pass); clippy `-D warnings
--all-targets` exit 0; nextest 1196 of 1196 passed; private rustdoc exit 0. The transform gate exit 101 twice, `render` at 24.9 and 19.3
pages/s against its floor of 40 at load 42 to 120; round 2000 read 15.4 and 16.4 here, and `render` reaches no `redact` code.
`cargo test -p conformance` after `git add -N`: exit 0, twice. `cd fuzz && cargo check --bins`: exit 0.
`state.sh --round 2026 docs`: exit 0. Tier 2's `corpus` and `raster_golden` not run: `pdf-model` does not depend on
`pdf-transform`, so their binaries are HEAD's.

**Left of the row.** 12.5.6.23 stays `departed` on its two `JPXDecode` refusals; `/OverlayText` and `/IC` stay a
recorded choice on the far side of the authoring line.
