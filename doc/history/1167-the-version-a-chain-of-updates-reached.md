# 1167 — The version a chain of updates reached, and a table read as a table
Two clause-7 rows from `doc/todo/65` bucket 6, both closed; §7.7 flipped with its last child.

## §7.5.6 `partial` → `implemented` (ADR 1171)

Errata Collection 3 Issue #399: an update's catalog `/Version` *upgrades* the version the document
has reached, and an update's catalog shall not reduce it by that entry's value **or by its
absence**. `Document::version` answered the larger of the header and the newest catalog alone, so
two files read low — one whose later update states less, and the likelier one whose update rewrote
the catalog and dropped the entry, which the row had never named. `xref::walk_revisions` now walks
the `/Prev` chain **forwards**, one pass, laying each section over an accumulating table
(`XrefTable::lay_over`); after the *n*th section the table is the one a reader opening the file at
the *n*th `%%EOF` would have built. `Document::as_of` reads a revision by substituting that table,
carrying the file encryption key because §7.6 makes it the file's; a catalog the document's own
table still names is read through this document and its cache, so the ordinary file pays no clone
and no second parse. The recorded reason for not building it — "work on the open path" — is false:
nothing in `Document::open` asks for the version, and the one caller that makes Annex I's warning is
behind a `OnceLock` `open.rs` keeps off the launch path by measurement.

## §7.7.3.3 `partial` → `implemented` (ADR 1172)

The row was `partial` on a count of Table 31 *entries* whose values nothing reads, where the
ledger's word counts *requirements*. Read one at a time, all ten — the seven listed, `/PZ`, and
`/AF` and `/DPart`, which the list had never held — hand their meaning to a clause whose own row
has disposed of it: §14.5, §14.10.6, §14.11.2.2, §14.11.4 (`inapplicable`), §12.7.7, §14.3.2,
§14.13.4 (`implemented`), §14.12.3 (`writer-side`). `/OutputIntents`, the third PDF 2.0 entry the
list lacked, is read already; `/PZ`'s premise had no ADR and is re-derived from §14.10.6's `may`;
and the clause's last sentence — a page tree shall not name one page object twice — binds whoever
builds a tree, which this tree's three builders cannot break.

## Files and gates

`crates/pdf-syntax/src/{xref.rs, version.rs, document.rs}`, `tests/incremental_update.rs` (five new
tests, four in `version.rs`); `doc/conformance/ledger.toml` (§7.5.6, §7.7.3.3, §7.7),
`doc/adr/{1171,1172}`, `doc/todo/65`, `doc/errata-read.md`. `rustfmt --check` on the four code
files: 0; `clippy -p pdf-syntax --all-targets -D warnings`: 0; `nextest run -p pdf-syntax`: 274
passed, `on_disk` included; `cargo test -p conformance`: the ledger check passes with 883 rows
either side, the quotation check fails on sibling files mid-edit (its list changed between two runs
an hour apart, none of it mine); `-p pdf-model --test corpus --profile gates -- --ignored` behind
the lock: exit 0, five ratchets at ceiling, slack 0 (0 / 10 / 1 / 5 / 61), peak 2.14 GiB.
