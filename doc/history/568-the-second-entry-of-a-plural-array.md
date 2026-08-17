# 568 — The second entry of a plural array, and the grep that classed a corpus as binary

**Finding.** §12.8.2.4's `FieldMDP` transform is read, and the round that read it found that the
reason it had not been read was false. The row was `reported` and its note said the clause's
protection was covered by §12.7.5.5's signature field lock instead — on the belief that no corpus
document states a transform. **The corpus's one certification signature states one**, and has for as
long as this project has been citing that file by name: `xfa_filled_imm1344e.pdf`'s signature carries
a `/Reference` array of **two** signature reference dictionaries, a `DocMDP` and a `FieldMDP`, and
every reader this tree had for that array returned on the first entry that matched what it wanted.
`MOZILLA-1671648-0.pdf` under `doc/corpora/format-corpus` is a second witness from the same producer;
across every PDF under the pdf.js corpus, the four `doc/corpora/` submodules and this project's own
fixtures those two are the whole population.

**The instrument that had said zero was `grep`.** `grep -rl FieldMDP doc/pdf.js/test/pdfs` prints
nothing on this machine; `grep -arl` prints the file. Same walk, same pattern, same `-l` — without
`-a`, grep classes these files as binary and the listing never names them. Trap 8 already says a
census whose predicate is the code under test is not independent of it; this is the other side of it,
where the *independent* instrument lied and the code under test was right. The rule that survives is
run both and require them to agree, and `doc/habits.md`'s *Measuring* section carries the one-line
form.

**What it decided.** `Restriction::FieldCovered` is its own reason and not a second name for
`FieldLocked`, because the two clauses say different things about the same list of fields: §12.7.5.5
is a prohibition — the values "shall no longer be changed after this signature has been signed" —
while §12.8.2.4 states a consequence, "any modifications to specific form fields shall invalidate that
recipient's signature". A person deciding whether to fill in a field is owed which of the two they are
being told, and `CLAUDE.md`'s four levels answer them differently. Where a document states both — the
shape Table 259 tells a writer to produce, "copied from the corresponding fields in the signature
field lock dictionary" — both are returned, which is §12.8.6's composition rule rather than a
duplicate.

`FieldLock` is `FieldSelection` now, one type for Table 236 and Table 259, because the standard makes
the second a copy of the first and a second enum with the same three variants would have claimed a
distinction the vocabulary does not have. `FieldLock::locks` is `FieldSelection::covers`.

**And the witness settled something an argument could not.** `covers` matches §12.7.4.2's fully
qualified name, on the argument that a partial name repeats across a field tree. The transform in
`xfa_filled_imm1344e.pdf` names `form1[0].SignatureField3[0]`, and `pdf_model::form::fields`
independently derives exactly that string as the fully qualified name of the document's one field —
two readers, two clauses, one name.

**What is still owed.** §12.8.2.2.2's comparison of the signed revision with the current one, which
is what "FieldMDP signatures shall be validated in a similar manner to DocMDP signatures" points at
and what keeps §12.8.2.2 `partial` as well. Table 256's `/Data`, required only for this transform, is
what that comparison would be scoped by and is unread for the same reason. The restriction withholds
nothing this program currently does: the covered field in both witnesses is a *signature* field, and
`ViewState::set_field` fills in text and choice fields — it is asserted anyway, because `asserted`
answers what the document says rather than what this program happens to be able to do.

**Date.** 2026-08-18. **ADR.** `doc/adr/0403-the-second-entry-of-a-plural-array-and-the-grep-that-classed-a-corpus-as-binary.md`.

**Code.** `crates/pdf-model/src/signature.rs` (`FieldSelection`, `covers`, `field_mdp`,
`for_each_signed_field`, `walk_signed_fields`, `read_selection`); `crates/pdf-model/src/restriction.rs`
(`Restriction::FieldCovered` and its arm in `asserted`); `crates/viewer-core/src/notes.rs` (the
sentence a person is told).

**Tests.** `crates/pdf-model/tests/forms_data.rs`
(`a_signature_covers_the_fields_its_field_mdp_transform_names` over all three of Table 259's actions
and the unsigned condition, `a_lock_and_the_transform_copied_from_it_are_two_reasons_rather_than_one`);
`crates/pdf-model/tests/restrictions.rs` (`the_corpus_certification_also_covers_a_field_by_name`);
`crates/pdf-model/tests/signatures.rs` (`the_corpus_states_the_fields_one_signature_covers`, which
pins the population and the two instruments); `crates/pdf-model/tests/save_round_trip.rs` (the census
line for the new reason).

**Touched.** `doc/conformance/ledger.toml` (§12.8.2 `partial`, §12.8.2.4 `reported` → **`partial`**,
§12.7.5.5's `/Lock` census sentence), `doc/habits.md` (*Measuring*), `doc/adr/0403-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean. `cargo clippy --all-targets -- -D warnings` silent — six
`doc_markdown` errors were mine and are fixed by backticking `FieldMDP`, which is the convention the
neighbouring `DocMDP` quotations already use. `cargo nextest run` **2117 tests run: 2117 passed, 15
skipped**, four of them new. `cargo test --workspace --doc` green. `cargo test -p conformance --
--nocapture` green — 157 unit and 5 gate tests, every citation naming a clause the standard has and
every quotation verbatim, which is what checked the four new blockquotes. Corpus gate run; no oracle
run, because `git diff --stat -- crates/` names `pdf-model`'s signature, restriction and test files
and `viewer-core`'s notes, and no line of them can reach a raster.
