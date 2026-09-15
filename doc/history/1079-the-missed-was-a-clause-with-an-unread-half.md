# 1079 — The `missed` was a clause with an unread half, and the limit had a witness

Session 1079, the validator slot. 2026-09-15. ADR 1093 argues both findings. Touched
`crates/pdf-archive/src/{table/metadata.rs,table/file_structure.rs,coverage.rs}`,
`pdf-archive/tests/unwitnessed.rs`, `crates/pdf-transform/src/archive/decision.rs`.

**The one `missed`** was `veraPDF test suite 6-6-2-3-3-t03-fail-b.pdf`, PDF/A-2b. Its value type
omits `pdfaType:field`, which `TechNote 0010` A029 permits and reads as an empty array — and the
packet then states `cvt:field1` for a property of that type. The clause is ISO 19005-2 section
6.6.2.3.1, whose extension half nothing here had read inside: `metadata/properties-use-known-schemas` reads *using* a schema as carrying the value type the
schema gives, and applies that to the predefined half alone. The new row `metadata/extension-schema-structure-fields-are-described`
is the same reading on the other half, in its narrowest form — a structure carries only the fields
its own value type describes — with every other shape passed over and the row that owns it named,
an incomplete description included. The converter refuses it by name: describing the field needs a
sentence only the producer holds; cutting it is a section 3.9 loss.

**ISO 19005-2 section 6.1.13's indirect objects.** Session 1049 said the smallest failing document
was beyond a test. It is not. The clause counts what a file contains and section 6.1.4 makes the
cross-reference table that statement, so the witness is a cross-reference *stream* naming
8 388 608 numbers in use: 58 MB of records, `Document::open` in 345 ms. `/Size` is not that count
and the predicate reads none; it now takes `XrefTable::len` rather than `Examination::objects`,
whose fetch is 6.6 s and a gibibyte of the 6.9 s a whole report costs there, with a test holding
the two to each other. The fixture is judged against its own predicate — 0.58 s, 640 MiB for both
halves — and the boundary is the calibration: 8 388 607 conforms.

**The reach over the seven corpora**, `examples/withdrawn.rs` with `--skip
isartor-6-1-12-t01-fail-a` as 1007 and 1049 ran it: 4361 documents against 4148, 27
unreadable against 12, 121 with an exempt population against 107. Fifteen rows narrowed where 1007
had fourteen, the fifteenth `graphics/rendering-intent-entries-name-one-of-four` on
`AIAA-2002-4016-606.pdf`, which ADR 1026 section 5.2 saw only in the openpreserve crawl. **No row
went quiet**: every row 1007 measured still fails at least as many, the rows failing none are
eleven of 176 where they were eleven of 175 — the same eleven by clause `tests/unwitnessed.rs`
pins — and no `over-narrowing?` printed. The new row fails one document in the 4361, the `-fail-`
witness above.

**Gates.** `fmt --all --check` 0 · `clippy --workspace --all-targets` (`-D warnings`) 0 · `nextest
--workspace --no-fail-fast` 4852/4852 · `--doc` 0 · fuzz fmt 0, fuzz clippy 0 · `-p conformance` 0
· `pdf-archive --test corpus` 0, `missed` 0 and `over` 0 on all six, PDF/A-2b `elsewhere` 7 → 8 ·
`pdf-transform --test archive_corpus` 0, PDF/A-2b 376 conforming (was 377), 173 refused by name
(was 172) · `cross_check` 0 · `tools/state.sh archive` 0 · `archive_census` unconsidered 0.
