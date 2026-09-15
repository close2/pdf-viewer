# 1093 — A sentence nobody had read inside, and a limit that had a witness all along

Session 1079. Status: **accepted**. Settles the validator's one `missed` against the veraPDF
corpus, and overturns the claim that ISO 19005-2 section 6.1.13's indirect-object limit has no
witness a test may build. Adds `metadata/extension-schema-structure-fields-are-described`; changes
`crates/pdf-archive/src/table/metadata.rs`, `src/table/file_structure.rs`, `src/coverage.rs`,
`tests/unwitnessed.rs` and `crates/pdf-transform/src/archive/decision.rs`.

`§N` is ISO 32000-2 and nothing else; ISO 19005 clauses are written out. Nothing of `doc/pdfa/` is
quoted here: that copy is licensed to a single reader, so every clause is cited and paraphrased.

## 1. The `missed`, and the sentence it was hiding behind

`tools/state.sh archive` reported `missed` 1 on PDF/A-2b from session 1007 to this one:
`veraPDF test suite 6-6-2-3-3-t03-fail-b.pdf`, filed under ISO 19005-2 section 6.6.2.3.3. Its
value type states no `pdfaType:field`, which is what the corpus built it to test — and
`TechNote 0010`'s A029 withdrew that rule, the working group resolving that a value type
describing no structured field may omit the entry and a validator reading the absence as an empty
array. `tests/corpus.rs` has said since session 940 that the file is non-conforming all the same,
"for a reason this crate does not yet check". This round found the reason and the clause.

**The file contradicts its own description.** Its one property `fs:property1` is typed
`CustomValueType` by the extension schema, and the packet then states `cvt:field1` for it —
a field the type describes nowhere, because A029's empty array is what the absent entry means.
The governing sentence is ISO 19005-2 section 6.6.2.3.1, whose requirement is that every property
specified in XMP form use a predefined schema or an extension schema complying with section
6.6.2.3.2. `metadata/properties-use-known-schemas` already reads *using* a schema as more than
borrowing its namespace — a property the schema gives a value type has to carry a value of that
type — and says so in its own comment. That reading was applied to the predefined half alone, and
nothing in this crate had ever applied it to the extension half, which is where a schema's Table 5
and Table 6 say what is inside a structure.

**So the new row is that same reading, on the other half of the same sentence**, and it is the
narrowest form the question takes: where a packet describes a property, gives it a value type its
own description defines, and then states a structure for it, every field of the structure has to
be one the value type lists. Every other shape is passed over with a reason — an undescribed
namespace is `metadata/extension-schemas-embedded`'s finding, an undefined value type is
`metadata/extension-property-value-types-are-defined` (still `Check::Unchecked`, and the two
questions are now distinguished at both rows), a value that is not a structure states no field.
A packet reaches this finding only by describing a type and then contradicting the description.

**What moved:** `missed` 0 on all six targets, `over` still 0, `elsewhere` 7 → 8 on PDF/A-2b — the
witness is now caught under the clause that states the rule rather than the one whose directory it
sits in, which is what that column is for. The converter refuses it by name: both repairs
(describing the field, or cutting it) need something the file does not say.

## 2. The limit that had a witness

`tests/unwitnessed.rs` said, and ADR 1026 section 5.1 implied, that the smallest document failing
`implementation-limits/indirect-object-count` — 8 388 608 indirect objects — is more than a unit
test may build, because the count came from `Examination::objects`, which fetches every number the
cross-reference table names. Both halves of that are now wrong.

**What the clause counts.** ISO 19005-2 section 6.1.13 bounds the indirect objects a conforming
file *contains*, and ISO 19005-2 section 6.1.4 is what makes the cross-reference table the
statement of that: an indirect object no cross-reference section names is exempt, so the numbers
a section does name are the file's own account of what it holds. A `/Size` is **not** that account
— §7.5.8.2 makes it one greater than the highest object number, which says nothing about how many
entries are in use — and the predicate does not read one. A witness had to be a table, not a
trailer entry.

**What it costs, measured rather than assumed.** §7.5.8 lets a cross-reference section be a stream
of fixed-width records, so a file states 8 388 608 objects in 58 MB of records and nothing else.
`examples/cost.rs` on that file: opening 345 ms, the object population 6.6 s, the predicates 9.4 s,
1.21 GiB peak. The *count* costs nothing — `XrefTable::len` is what the row wanted all along, the
row asks how many and never which — so the predicate now takes it there, and the two remain one
number by construction with a test holding them to each other. The fixture is judged against its
own predicate rather than through `check`, which is what keeps a whole report's 6.9 s out of tier
1: 0.58 s and 640 MiB for both documents.

**The boundary is asserted from both sides**, and that is the calibration rather than a nicety:
the first fixture built for this row named exactly 8 388 607 objects and was met, and a predicate
reading `/Size` would fail that same file.

## 3. What this round did not do

It did not touch `metadata/extension-property-value-types-are-defined`, which needs two
vocabularies this crate does not hold and says so. It did not give the converter a rewrite for
either row. It did not re-status any subclause: ISO 19005-2 section 6.6.2.3.1 was `Bound` and read
sentence by sentence before this round, and the reading now carries one sentence more.
