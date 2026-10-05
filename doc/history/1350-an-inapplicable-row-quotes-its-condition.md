# 1350 — An `inapplicable` row quotes its condition, and fourteen rows were not `inapplicable`

Ledger slot of batch fifty-four. ADR 1535. No question written.

**Instrument (ADR 1535).** `ledger::grounding` asks whether an `inapplicable` row's note quotes, in
double quotation marks and at least four words long, a sentence `holds_quotation` finds in the row's
clause, its parent below the top level, or a clause the note cites. A row that fails is
`Problem::ConditionUnquoted`, admitted by `CONDITION_UNQUOTED_CEILING` (0, `==`). `--bin ledger`
prints the counts and the `out-of-scope` rows naming an exclusion. A calibrated unit test plants six notes.

**Before.** 48 rows: 15 quoted and verified, 31 prose only, 2 quoting text their clause does not hold.
§14.11.1 had reworded its clause, and §14.10.5.4 had a paraphrase in quotation marks. Six of the 15
quoted something other than a condition.

**After.** 34 rows: 34 / 0 / 0. Out-of-scope 94 of 94 name their exclusion. Implemented 699.

**Moved to `implemented` (14).** Intros with no `shall`: §8.4.3.1, §8.6.6.1, §11.6.1, §11.7.1, §12.7.1,
§14.8.3.1, §14.11.1, §14.12.1. Each names its family's evidence, as §6.1 and §10.4.1 already did.
Headings with no text whose subclauses include an `implemented` one: §10.6, §10.8, §14.8.3, §14.11.6.
§10.8.1 is a permission exercised: "Whether separations are produced is up to the processing
software", and `pdf_render::separation` produces them under §10.8.3. §14.8.5.4.2 was false of the
tree: `Tree::writing_mode` reads `/WritingMode`, inherited as "[t]he specified layout directions shall
apply to the given structure element and all of its descendants" requires.

**Quoted (the rest).** §10.6.1 to §10.6.4 quote §10.6.1's "Halftoning is not required for such devices".
§10.6.2 adds "Most colour printers, but not colour displays, work this way", and §10.6.4 adds its
bilevel procedure sentence. The layout rows (§14.8.3.2, §14.8.5.4.1, .4, .6, .7) quote §14.8.5.4.1's
"Layout attributes specify parameters of the layout process…". Web capture's sixteen quote §14.10.1's
deprecation and §3.15's definition. §14.11.4, §14.11.6.1, §14.11.6.3 and §14.11.7 quote their own
deprecation sentence and §3.15. The rows §14.2, §14.10, §14.10.6, §14.11.2.2 and §14.12.4.2 already
quoted theirs.

**Cousins corrected.** §10.1 (it called §10.5 inapplicable), §14.8.5.4, §14.8.5.4.3 (it called
`/SpaceBefore` and `/SpaceAfter` unexecuted, but `Tree::allocation` reads them), §14.8.3.3, §14.11 and
§14.12. `doc/todo/65` has no `inapplicable` paragraph, and none of the moves is `partial` or `reported`,
so the frontier map is unchanged.

**Gates.** `rustfmt --check` exit 0 on the three files. `cargo clippy -p conformance --all-targets -D
warnings` exit 0. `cargo test -p conformance`: 376 passed, 0 failed (exit 0), frontier map included.
No output changed, so there is no tier 2.
