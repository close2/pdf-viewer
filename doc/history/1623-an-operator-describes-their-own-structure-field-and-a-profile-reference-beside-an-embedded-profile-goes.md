# 1623 — An operator describes their own structure field, and a profile reference beside an embedded profile goes

Slot 1 of batch ninety-eight, 2026-10-20, an archive round. ADRs 2082, 2083; no question.

**Premises and hypotheses.** The `supply` hypothesis held in shape — a configuration key and one
branch in the describing writer — with two additions: the report's facts need the producer's own
undescribed fields read before any editor (`undescribed_fields_in`, shared with the writer's
reading), and a configured `supply` was never asked whether its rewrite's preparation refused, so a
refusal surfaced only as a failing output. "The census's next sites are two or three small losses
with a `preserve` answer" did not hold: over the veraPDF, pdf.js and corpora sets (4 358 of 4 385
opened) no `not-built-yet` site with a document is left at 2b, 2u or 2a; at part 4 the four ranked
are the two duplicate-profile rows (6, 2), which ADRs 0965 and 0982 answer with no mitigation, the
base-standard profile row (2, outside the veraPDF part 4 folders) and `DestOutputProfileRef` (1),
a statement rather than a loss. Habit 133 kept: the identifiers came off the census table first.

**Built.** ADR 2082: `field-descriptions = { "<namespace><local>" = "<sentence>" }`,
`Supplied::FieldDescriptions`, an empty sentence or a namespace-less key refused when read, an
unnamed field keeping the converter's sentence, `described_by` in `described_extension_fields`;
`supply_not_carried_out` refuses a supply whose rewrite's preparation refuses (red before). ADR
2083: `Rewrite::DestinationProfileReferenceRemoved` under both reference rows, `Stated`, where the
intent embeds its profile; the only-profile reference, a `URLs` embedded file and a direct intent
refused by name. ADR 2083 section 2 says why the duplicate-profile rows were not taken.

**Pixels.** `6-6-2-3-3-t03-fail-b` at 2b under a table naming its field and `6-2-3-t04-fail-a` at 4
and 4e: veraPDF 1.30.3 failing before (6.6.2.3.1 test 2; 6.2.3 test 3), compliant after; every page
byte-identical at 72 and 720 dpi. 4f still refuses the second at Annex A.2, as before.

**Gates.** Built and tested in an export of HEAD plus these eight files while a neighbour's
`pdf-colour` split did not compile (`tools/batch.sh export 1623 --as mine`): rustfmt clean per file;
`cargo fmt --check -p pdf-transform` exit 0; `RUSTFLAGS=-D warnings clippy -p pdf-transform
--all-targets` exit 0; nextest `pdf-transform`, `pdf-archive` 858 passed, 0 failed. In the worktree:
`cargo test -p conformance` 443 passed, 0 failed. `tools/state.sh --round 1623 archive` ran every
part and exited 2 on a neighbour's `tools/state.sh` edit after its last: `--remedy-sites` 2b 23 of
67 (24 of 68), 4 20 of 59 (21 of 60); validator `missed` 0, `over` 0 every target; `archive_corpus`
4 235/124 (234/125), 2b 492/119; `cross_check` 0 disagreements. `t3-foreign_corpus` under
`--clock` exit 0, 155 s held, 0 panicked. `tools/state.sh --round 1623 docs` exit 0.

**Unfinished.** The duplicate-profile rows, a reversal of ADRs 0965 and 0982 (by hand, `t03-fail-a`
and `-c` written `/DeviceCMYK` draw identically at 72 and 720 dpi and pass veraPDF at 4, ADR 2083
section 2); the base-standard row's witnesses; a reference that is an intent's only profile. 4 147 s.
