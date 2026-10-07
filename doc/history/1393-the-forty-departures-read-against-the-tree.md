# 1393 — The forty departures read against the tree, and a departure is from a `shall`

Ledger slot of batch sixty-one. ADRs 1622 and 1623; no question.
**The rule.** `departed` is kept for a `shall` addressed to this program that it does not carry
out, decided and priced. A `should` declined, a `may` not taken, a NOTE, or a method the standard
offers another kind of processor departs from nothing: the row is `implemented`, its first sentence
naming the choice and the sentence that leaves it (ADR 1535's shape). `doc/PLAN.md` §5a says so.
**Stand (20).** §7.4.2, §7.4.8, §7.5.5, §8.5.3.3.1, §8.7.4.5.7, §8.7.4.5.8, §10.7.4, §10.7.5,
§11.5.3, §11.6.6, §11.7.2, §11.7.5.3, §12.2, §12.5.2, §12.5.5, §12.5.6.23, §12.6.4.6, §12.7.4.1,
§12.7.8.3.3, §13.4. Each note ends on one current sentence naming ADR 1622; §11.6.6's and §11.7.2's
first sentences now name ADR 1254. §12.5.6.23 is re-stated as departed on two refusals (ADRs 1363,
1371); its overlay "should be drawn" and is a choice. §10.7.4's first sentence names all four
departures. §10.7.1's NOTE licenses none of them (ADR 1560); §10.7.5's note said otherwise twice.
**§10.7.4 (1).** `oracle-held`: of 47 held contradicted pages 0 are ours, 14 the references', 33
choices. Of the two pages (1) is drawn on, `issue7891_bc1` and `issue4436r` are references' (ADRs
1560, 1572). No contradicted verdict rests on any of the four.
**To `implemented` (11).** §6.3.2.1, §8.6.5.7, §10.4.2.3, §10.4.2.5, §11.3.4, §12.3.5, §12.3.5.1,
§12.5.6.11, §12.5.6.12, §12.7.8.3.1 (on §3.15's "should be ignored"), and §12.3 as a heading.
**To `partial` (ADR 1623).** §7.10.2: A72's bound answers ADR 0098's ground that a chosen spline would be invented.
Rich text formatting: the XFA exclusion never reached it (ADR 1197), and XFA 3.3 is held. The
Internet Archive's 2026-08-19 copy of the PDF Association's normative reference is at
`/home/AI/specs/XFA-3_3.pdf`, SHA-256 `a3344e7e…a01e`. Its preface permits display software. Moved:
§12.7.4.3, §12.7.5.3, §12.7.8.3.2, and the `implemented` §12.5.6.6 and §12.5.6.2 on the same ground.
Headings derive `partial`: §7.10, §12.5, §12.5.6, §12.7, §12.7.4, §12.7.5, §12.7.8, §12.7.8.3.
**Counts** (`grep -c`): departed 40 → 20, implemented 699 → 705, partial 11 → 25.
**Frontier map.** `doc/todo/65`: bucket 4 holds the five rich text rows as one build; bucket 6
§7.10.2; eight aggregates added. Bucket 1, bucket 3, bucket 5 and the not-owed prose corrected.
Also `doc/todo/01`'s list, todo 22, 26, 42 and README's row, and two state-of-play sentences.
**Left.** `pdf-model` comments in `form.rs`, `forms_data.rs`, `popup.rs`, `appearance.rs` and
`xfdf.rs`'s owed sentence still give the exclusion as rich text's reason; the build corrects them.
`doc/third-party-data.md` owes XFA 3.3 a section before that build.

**Gates.** No Rust file touched, so no rustfmt or clippy. `cargo test -p conformance` after each of
eight ledger chunks; the last run, `--no-fail-fast`: exit 0, 395 passed, `the_frontier_map` among them.
`tools/batch.sh check`: 1, on `cargo fmt` of siblings' `viewer-core` and `viewer-ui` files only.
No heavy walk. Tasks of user AI beside the last conformance run: 349.
