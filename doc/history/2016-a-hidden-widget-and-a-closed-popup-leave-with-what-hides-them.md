# 2016 — A hidden widget and a closed popup leave with what hides them

Session 2016, a single round, PDF/A lane's worktree: R16-annotations-and-forms, its sites largest first.
ADRs 2850, 2851.

**The premise.** Half true. HEAD reproduced every census line of R16's 17 documents under all eight profiles at
both parts. Under `as-if-printed` R16 holds four: the two XFA forms (X1 besides), `checkbox_no_appearance` and
`poppler-395-0-fuzzed`; the last two are its only frees. Under `as-shown` the sites were a hidden widget (3), a
hidden markup annotation with a closed popup (2), an `Invisible` popup (`pr7352`), an appearance on an appended
page of a tagged document (4, three of them R15 besides), and the two above. The signature-widget row stopping
`poppler` and the XFA forms was not a refusal of the widget: the compound row read the table's answer to the flags
rule and never the operator's, which both shipped profiles give as `preserve`.

**Built.** ADR 2850 (`archive/decision.rs`): `decide_as_underlying` asks the operator's answer of each rule the
compound names before the table's. ADR 2851 (`archive/in_place.rs`, `in_place/linked_popups.rs`): the per-shape
plan discards a hidden widget as the site-wide removal does (§12.7.2: a field "may have" widget children; its
field and value stay), reads a widget's `/Parent` as its field (Table 191), lets a closed popup whose flags conform
leave with its hidden parent (Table 186's `/Open`, Table 167's Hidden), a popup no page holds included, and reads
`Invisible` on a Table 171 subtype as the bits beside it (Table 167). Ledger 12.5.3 and 12.7.2 stay `implemented`,
their notes, code and tests extended.

**Freed** under `as-shown` at 2b and 4: `160F-2019`, `PDFBOX-6018-099267-p9-OrphanPopups`,
`annotation_hidden_noview`, `annotation_hidden_print`, `pr7352`, `widget_hidden_print`. Nothing under
`as-if-printed`. **Pixels** (CPU, 72 dpi, `/OutputIntents` renamed): all twelve outputs identical to their
sources, 0 pixels.

**Left.** `checkbox_no_appearance`: the reader refuses its on widget (no `/AP`, no `/CA`), so a build is its
removal, which touches the field. `poppler-395-0-fuzzed`: a `FreeText` `/DA` font `/DR` does not define, which the
window sets in ADR 0112's stand-in; a build writes that under a loss naming the face. The tagged appended page
(R15 besides on each), `issue7446`'s removed annotation stating no normal appearance, a 3D page at 4. `hesr13613`'s operator is `> Tj` three times in a text widget's
appearance, a stray delimiter the check reads as an operator; it needs an argument of its own.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0 on `pdf-transform` (101 twice first: a redundant closure
and a one-element loop in the tests); nextest of `pdf-transform` and `pdf-archive` 0 (1526 passed, 26 skipped); the
four new tests fail under six plants (two runs: four, then two); `cargo test -p conformance` 0 after `git add -N`; archive
tier 0 and 0; censuses against round 2014 (`cmp.py`): shipped 12 NEW (the six freed files under `as-shown` at 2b and
4), stating 0 NEW, 0 GONE in either, every other output identical masked (5404, 4812); veraPDF 1.30.3: 12 runs over
the new outputs PASS; `rank.py --table`: R16 14/17 to 6/9 under `as-shown`, sections 2 and 9 replaced;
`tools/state.sh --round 2016 docs` 0. No tier 2: no renderer and no `pdf-font` change.
