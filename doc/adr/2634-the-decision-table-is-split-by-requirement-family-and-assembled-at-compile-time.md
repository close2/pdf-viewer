# 2634 — The decision table is split by requirement family and assembled at compile time

Session 1900, slot 10 of batch 130, a docs round. Status: **accepted** and **built** in
`crates/pdf-transform/src/archive/decision.rs` and `crates/pdf-transform/src/archive/decision/`.
Context: the owner's code-quality slot; `doc/pdfa-blocking-issues.md` sections 3 to 6 (the families);
ADR 1023 (a moved comment carries no history). ADR 2635 and Q776 are unused.

## The family map

`decision.rs` held, past 4 400 lines, every requirement row's answer beside the types that state an
answer. It now keeps the shared types (`Loss`, `Losses`, `Authorisations`, `Remedy`, `Conditional`,
`Shape`, `Because`, `Decision`), the dispatch (`decide` and its helpers) and the four tables'
documentation; six modules under `decision/` hold the rows and the sentences the rows quote:

| module | rows whose requirement begins | blocking-issues section |
|---|---|---|
| `fonts` | `fonts/` | 3 |
| `colour_and_graphics` | `graphics/` | 4 |
| `content_stream_fences` | the `REFUSED_BY_NAME` rows answered by the fence's sentences, whatever their prefix | 5 |
| `files_and_structure` | `file-structure/`, `implementation-limits/`, `document-requirements/`, `logical-structure/`, `optional-content/` | 6 |
| `annotations_and_attachments` | `annotations/`, `forms/`, `actions/`, `embedded-files/`, `alternate-presentations/`, `signatures/` | 6 |
| `metadata` | `metadata/` | 6 |

A constant or function goes with the only family that reads it; one read across families stays in
`decision.rs`. Two moved items other modules name by their old path are re-exported from `decision.rs`
(`THE_GLYPH_IS_NOT_IN_THE_PROGRAM`, `odd_hexadecimal_digits_on_a_page`), so no caller changed.

## The const-fn rebuild

Each family declares its own `REMEDIES`, `CONDITIONAL`, `SHAPES` and `REFUSED_BY_NAME` slices, and
`decision.rs` builds each table, a constant array, from `*_BY_FAMILY`, a list of those slices, by two
`const fn`s: `rows` sums the lengths, `concatenated` copies the rows in. The tables stay compiled-in
data (principle 2, *no parsed data at startup*) and stay `&[T]`, so no consumer's type changed. A
`const fn` cannot build an array element by element without `unsafe`, so `concatenated` seeds the
array with the first family's first row and overwrites every slot; it asserts at compile time that
the families' rows fill the array exactly.

## The order change, and why it is not a behaviour change

Each table's rows now run family by family rather than in the order they were written. Every
consumer looks a row up by its requirement (`find`, `any`, `config.rs`'s `BTreeMap` of loss sites,
`prepare.rs`), and each requirement has one row a table, so no answer and no output depends on the
order. The proof is the round's gates: the census without and with the CJK pack identical line by
line to the base's, the harness's one-minute notice aside; `--test archive` and `--test profiles`
the same counts; a line-for-line comparison finding every moved line unchanged but for the module
path one level deeper, a visibility word and one import.

## What it costs a reader and buys them

A reader of a requirement's answer now opens two files: the family module for the row and its
sentence, `decision.rs` for the types the row is written in. A reader who wants the whole table in
one order no longer has it in the source; the census prints it. What it buys: a family's rows,
sentences and helpers sit together under the same heading the blocking-issues document uses, the
largest module is about 930 lines, and a round working on one family's remedy touches one file that
its siblings in the same batch do not.
