# 449 — A `shall` that was filed under "signing behaviour"

**Finding.** §12.7.5.5's signature field lock is a `shall` addressed to whoever changes a form
field's value, and this program changes them. The ledger row had called it "signing behaviour",
so a field a signature had locked was filled in and saved without a word — and no corpus document
states a `/Lock`, so nothing could have found it from the demand side.

**Date.** 2026-08-12.
**ADR.** [0284](../adr/0284-a-shall-that-was-called-signing-behaviour.md).
**Touched.** `crates/pdf-model/src/signature.rs`, `crates/pdf-model/src/restriction.rs`,
`crates/pdf-model/tests/forms_data.rs`, `crates/pdf-model/tests/restrictions.rs`,
`crates/viewer-core/src/viewer.rs`, `crates/viewer-core/src/notes.rs`,
`doc/conformance/ledger.toml` (§7.6, §7.7, §12.7.5.5, §14.6, §14.6.1, §14.8.2.6.1),
`doc/todo/01-ledger-partial-rows.md`, `doc/todo/38-a-documents-restrictions-have-levels.md`,
`doc/adr/0284-*`, this file.

## How it was found

`doc/todo/01`'s blame-ordered reading list, re-derived over 597 commits: order the `partial` rows
by the commit that last wrote each `note = ` line and read from the top. Six rows were read that
the previous run had not; five were wrong and the sixth was work.

## What was built

`signature::field_locks` reads Table 236's `/Action` and `/Fields` off every **signed** signature
field, `FieldLock::locks` answers the three actions over §12.7.4.2's fully qualified names, and
`restriction::asserted` gains a fourth `Restriction` beside Table 22's and `/DocMDP`'s. It reaches
a person as `Event::Refused` with the clause in the sentence, and the reader can turn it off, which
is ADR 0212's shape unchanged. The witness is hand-built: none of the 974 states a `/Lock`.

## Gates

Whole `tools/state.sh` sequence, diffed line by line against the previous round's: everything
identical except the ledger (implemented 410 → 412, partial 244 → 242, from §12.7.5.5 and
§14.8.2.6.1), the citation and quotation counts, and one more test — 1620 passing.
