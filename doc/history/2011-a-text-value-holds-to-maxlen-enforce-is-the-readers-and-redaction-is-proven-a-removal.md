# 2011 — A text value holds to `/MaxLen`, `/Enforce` is the reader's, and redaction is proven a removal

Session 2011, a single ledger round in the audit worktree. Rows 12.5.6.23, 12.7.5.3, 12.2 and I.2; owner answer
A834; ADRs 2840 and 2841; Q842.

**The slice.** A834's four items, in the brief's order: redaction as removal, `/MaxLen` in the model, §12.2's
`/Enforce` under the four levels, Annex I.2 pinned.

**The premise.** Redaction already removed every text code, path and image sample, writing a new file (no earlier
revision); no viewer path applies one. Drawing `/RO` after the removal was built as A834's Reading asks, failed
the owner's audit test `a_redaction_ro_is_reported_while_removal_proceeds_without_composing_the_overlay`, and was
withdrawn: A834's own words decline Q834's recommendation 3, so the drawing is Q842. The other three held:
unbuilt, effort only.

**Built.** `held_to_max_len` cuts a `/FT /Tx` value to its inherited `/MaxLen` wherever it enters the edit log:
`set_field`, a script's `write_text`, FDF/XFDF import, template fields. The two audit tests A834 names now expect
ten characters, the one sanctioned edit. `Operation::OverridePreference` with `Restriction::PrintScalingEnforced`;
`RestrictionPolicy` defaults it to `warn`; `viewer_core` asks it last at the print grant, and `Event::Printing`
carries `scaling_held`, carried on the confined wire, `pdf-vfs`'s wire and `QUORRA_RESTRICTED_ENFORCE`. Qt locks
its scaling control; GTK and a C caller say `Defaults::held_note`. Annex I.2: a second unrecognised feature is
pinned as reported.

**Tests.** `max_len_model.rs` (four), `enforced_preferences.rs` (five), `redaction_is_removal.rs` (two, by
extraction from every stream), `objects_not_understood.rs` (one). **Plant**: the producer's content kept and no
image destroyed fails both redaction tests ("KEEP\nSECRET" extracted).

**Gates.** Over `pdf-model`, `pdf-transform`, `viewer-core`, `viewer-host`, `viewer-confined`, `viewer-ffi`,
`viewer-gtk`, `viewer-qt`, `pdf-vfs`, `viewer-ui`: `cargo fmt --check` exit 0; clippy `-D warnings --all-targets`
exit 0; nextest under `xvfb-run -a`, 5177 of 5177 passed (the C-program tests included). `cd fuzz && cargo check
--bins`: exit 0. Conformance after `git add -N`: exit 101 on this record's gates paragraph, fixed, then
`--test records` exit 0. At the merge: tier 2 `corpus` 101 and `raster_golden` 101 as HEAD's (the same pair joined, the
same 23 first pages moved by name as round 2012's walk); `selection_census`, `accessibility_census`,
`awkward_classes` and `script_corpus` (356 held, 0 moved) 0; whole `conformance` 0; `state.sh docs` 0.

**Left of the rows.** 12.2 stays `partial`: GTK's and a C caller's dialogues cannot lock the scale. 12.5.6.23 stays
`departed` on its JPX refusals, `/RO` drawing waiting on Q842. 12.7.5.3 and I.2 stay `implemented`.
