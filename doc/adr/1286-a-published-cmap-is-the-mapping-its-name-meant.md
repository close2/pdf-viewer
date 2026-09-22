# 1286 — A published CMap is the mapping its name meant

Status: accepted and **built**.
Context: `crates/pdf-transform/src/archive/{cmaps,remedies,config,mod,prepare,rewrite,report}.rs`,
`crates/pdf-font/src/predefined.rs` (`program`, `builds_on`),
`crates/pdf-archive/src/table/fonts.rs` (`cmap_is_predefined`, `fonts_naming_an_undefined_cmap`),
`doc/profiles/keep-everything.toml`, `crates/pdf-transform/tests/archive.rs`,
`doc/third-party-data.md`.
Answers: `doc/todo/66`'s three `CMap` rows of `keep-everything` at PDF/A-2b.
Builds on: ADR 0140 (the 239 programs in `data/cmaps`), ADR 1188 (a `supply` that chooses among
the file's own statements), ADR 1285.
Clauses: ISO 32000-2 §9.7.5.3 (Table 118), §9.7.5.4, §9.7.5.1; ISO 19005-2 sections 6.2.11.3.1
and 6.2.11.3.3; ISO 19005-4 section 6.2.10.3.3.

## 1. Embedding a published CMap invents nothing

ISO 19005-2 section 6.2.11.3.3 requires every CMap a file uses to be embedded unless the base
standard predefines it. The refusal's reason was that supplying a CMap writes an encoding its
producer did not — true of an invented one, false of one Adobe publishes: the name *is* that
mapping, and a reader that knew it would have used these bytes. So `preserve` with
`source = "shipped-cmaps"` writes §9.7.5.3's stream from `pdf_font::predefined::program`, byte for
byte, with Table 118's entries stating what the program states:

> The name of the CMap. It shall be the same as the value of CMapName in the CMap file.

> The value of this entry shall be the same as the value of CIDSystemInfo in the CMap file.

and `/WMode` likewise. The validator is the oracle: the fixture's output conforms, and the stream's
decoded bytes are the published program.

## 2. What refuses, each by name, before anything is written

- A program that builds on a CMap off the list: the clause's second paragraph lets a CMap reference
  only predefined ones, and §9.7.5.4 a) would make the dictionary name it too. Adobe's vertical
  files mostly build on their unlisted horizontal twin, so they refuse; writing the two programs as
  one is not built.
- A program using an operator §9.7.5.4 b), c) or e) forbids in an embedded CMap, or one that is not
  `CMapType` 1.
- A descendant CIDFont whose Registry or Ordering differs, or whose Supplement is below the
  program's: ISO 19005-2 section 6.2.11.3.1 holds an embedded CMap and its font to one collection,
  so embedding would move the failure rather than end it.

## 3. The chain site cannot take the same answer, and says so

`fonts/cmap-uses-only-predefined-cmaps` reports an embedded CMap that references one off the list.
Embedding that one too leaves the reference in place, still pointing off the list, so the
requirement would fail as before. `source = "shipped-cmaps"` there is now `NotBuiltThatWay` with
that sentence, the catalogue stops offering it, and `keep-everything` stops at the site.

## 4. The write-mode disagreement is the operator's to settle

`fonts/embedded-cmap-states-its-own-write-mode` is a `supply` on ADR 1188's shape: the file states
its writing mode twice and nothing in it says which is the mistake. `write-mode = "program"` writes
the program's value into the dictionary; `write-mode = "stream"` replaces the digit of the program's
own `/WMode … def` with the dictionary's — a one-byte edit inside the producer's bytes — and refuses
a program stating no entry, since writing one would be a line of CMap syntax this converter
composed. Both are reported and recorded as the operator's, as every `supply` is.

## 5. The licence travels with the bytes

Adobe's programs are BSD-3-Clause and each carries its notice in its own `%%Copyright` header,
which the stream keeps. `doc/third-party-data.md` states that CMaps now leave with a converted
document, beside the ICC profiles.
