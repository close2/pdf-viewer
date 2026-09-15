# 1086 — §12.3.5.2's case normalization is a table the compiler lays out

Session 1072. Status: **accepted**. Decides where the Unicode case-folding data lives and in what
form it reaches the program. Adds `data/unicode/`, `crates/pdf-model/build.rs` and
`pdf_model::case`; changes `pdf_model::collection::case_normalised` and `/NOTICE`.

`§N` is ISO 32000-2 and nothing else. A section of any other document is written *section*.

## 1. What the clause asks, and what was answered

§12.3.5.2 restricts a portable collection's names and then compares two of them with the case
taken out:

> In addition to the restriction on naming folders, as just described, it is further required that
> two file names in the same folder do not map to the same string following case normalization.

It defines no mapping and names the document that does:

> See "Unicode Standard Annex #21, Case Mappings" for information on case normalization.

Annex #21 defines caseless matching as the comparison of two strings after the operation it calls
toCasefold — map each character to its case folding — and names `CaseFolding.txt` in the Unicode
Character Database as the data for it (its sections 2.3 and 2.5). ADR 1050 applied all six
restrictions and left this one sentence approximate: `case_normalised` called
`char::to_lowercase`, which is a different function.

**The difference is not a rounding error, it is the direction that reports nothing.** U+00DF
lowercases to itself and folds to `ss`; U+017F lowercases to itself and folds to `s`; U+03C2
lowercases to itself and folds to U+03C3, which is what U+03A3 folds to as well. In each of those
the lowercasing answer to "are these two names the same" is *no*, so the clause's restriction goes
unreported and the panel says the document conforms. An instrument whose failure mode is silence
cannot be left approximately right, which is why one sentence of one clause was worth a data file.

## 2. The decision

**Three things, and the third is the one a later round must not re-litigate.**

1. **Full case folding, which is `CaseFolding.txt`'s status C plus status F.** The file's own usage
   note gives that combination as the full folding and C plus S as the simple one. The simple
   folding exists for implementations that cannot let a string grow, and it leaves U+00DF folding
   to itself — the defect above, rebuilt from the data file.
2. **The Turkic rows, status T, are excluded**, which is the data file's own stated default. A
   folder name in a PDF carries no language, so nothing in a document could ask for the Turkic
   reading, and taking it would make `FILE` and `fıle` one name in every document there is.
3. **The data file is committed and the table is generated at build time into a `static`**, rather
   than parsed at run time, vendored as a pre-generated `.rs`, or taken as a dependency.

## 3. Why generated, and why committed

`CLAUDE.md` principle 2 is the first half: *no parsed data at startup*. The Arlington tables are
the standing precedent — compiled-in `static` data, so the object model costs zero parse time at
launch — and this is the same shape one crate over. The table is 1585 rows the compiler has
already laid out; a lookup is a binary search over `.rodata` and no file is opened, which is what
the rule asks of any future data resource.

The alternatives, and what each costs:

- **Parsing `CaseFolding.txt` at first use** (`OnceLock`) would keep the launch path clean and
  still put 87 KB of text through a parser the first time a document states a `/Collection`. It
  buys nothing: the table is small enough to compile in, and the parse could only fail.
- **Committing the generated `.rs`** would make the data file and the table two sources of truth
  for one fact, and the one a reader would edit is the wrong one. `pdf-spec` and `pdf-font` both
  took the generator, and their reason holds here.
- **A crate** would be a dependency bought for one clause, and `doc/stack.md`'s rule is that each
  one is argued. Nothing in this graph folds case today.

**Committed rather than fetched at build time** for the reason `data/standard-fonts/PROVENANCE.md`
gives about optional submodules: what two names compare equal to must not be a property of which
machine built the binary, or of whether that machine had a network.

**Under `data/` rather than `doc/`**, which is the one place this departs from the round's brief
and is a distinction the tree already draws: `data/` is what is compiled into the binary — the
fourteen font programs, the 239 CMaps, the sRGB profile — each with a `PROVENANCE.md` and a line
in `/NOTICE`, and `doc/` is what a person reads and no binary carries. This file is compiled in,
so it sits with the other three and carries both.

**Nothing is silently dropped in the generator**, which is `pdf-spec/build.rs`'s rule and is
load-bearing here for its own reason: a status this script did not recognise would narrow the
table, and a narrower table answers *no collision*. An unknown status aborts the build.

## 4. What it is not

**Folding is not normalisation.** Annex #21 says so itself, and this table does not compose or
decompose anything: two names that differ by a combining sequence are two names here. The clause
asks for case normalization and nothing else, and adding NFC would be this program deciding a
question §12.3.5.2 does not put.

**And it is not a refusal.** ADR 1050's choice stands unchanged: a name that breaks a restriction
is reported and the file is shown. What changes is that the sixth restriction now finds the pairs
it was written for.

## 5. Where it is calibrated

`pdf_model::case`'s own tests compute the lowercasing answer beside the folding one for all three
characters, so the assertions name the collisions the old function missed rather than asserting a
table's contents against itself (trap 13), and
`collection::two_names_that_differ_only_by_a_sharp_s_are_both_reported` takes the same pair through
`Collection::read` from a document's bytes.
