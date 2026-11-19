# 3208 — A portable test's quotation is held to its manifest's clause by the conformance gate

Session 2126. Status: accepted. Context: RFC 0016 section 4; ADR 3206; CLAUDE.md principle 5 ("Quotation
marks mean verbatim"); trap 13.

## Decision

`tools/conformance/tests/portable_quotes.rs` reads every `portable-conformance/tests/*.toml` and fails
when a sentence in its `quote` array is not in the clause its `clause` names, as written
(`ClauseIndex::holds_quotation`, the comparison the source's blockquotes get), when its `id` is not
its file's name, or when the id does not begin with the clause. A second test plants a changed word
and a sentence from another clause and requires both to be reported.

## Why

- **The suite is for readers who do not hold the standard.** Its quotations are the one claim a
  manifest makes that such a reader cannot check, so the tree checks them where it can:
  `doc/md/` is held here (ADR 0187) and not shipped with the suite.
- **Held to the clause, not to the whole text.** On its first run the gate found a manifest for
  §7.3.10 quoting §7.3.7's sentence on null entries. The sentence was verbatim but in the wrong
  clause, and a whole-text search would have passed it. The fix moved the quotation to the §7.3.7
  test and left §7.3.7 in the §7.3.10 test's prose.
- **A manifest names one clause.** A test resting on two clauses quotes the one it is filed under
  and cites the other in prose, the rule CLAUDE.md already sets for every comment in the tree.

## Cost

The manifests are read line by line (a `quote` array holds one double-quoted string per line), not
by a TOML parser, because the conformance crate's `toml_subset` reads the ledger's shape and not the
suite's inline tables. A manifest written in another layout is read as quoting nothing, and the gate
reports that too.
