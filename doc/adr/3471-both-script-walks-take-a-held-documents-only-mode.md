# 3471 — Both script walks take a held-documents-only mode

Session 2192. Status: **accepted** and built. Builds on ADR 1625 (Tier 1's column), ADR 3453 and record 2188's
"Why the train saw it". Code: `crates/pdf-model/tests/support/script_population.rs` (`walked`), both
`tests/script_corpus.rs`.

## Context

Both walks read RFC 0008 section 3's census population: every PDF under five roots, each opened to learn whether
a field of it carries a Table 199 script. Whole, they take one and a half to two and a half hours, so `doc/todo/02`
section 2 maps them to the train and a round that changes how a field's value is read, set or committed runs
neither (three commits moved Tier 0's held file unseen, record 2188). The documents that matter to such a round
are the ones Tier 0's held file names: the ones with a field script.

## Decision

1. `PDFVIEWER_SCRIPT_CORPUS_HELD=1` makes either walk read only the documents the first column of
   `crates/pdf-model/tests/script_corpus.tsv` names that are on this machine, and say how many of each.
2. Tier 0 compares those lines as it compares the whole walk's, and its `update` mode under the switch rewrites
   them; a document newly carrying a field script is found only by a whole walk, which is what adds a line.
3. Tier 1 holds the held documents to figures of their own, `HELD_ONLY_FIGURES`, beside the census population's
   `WHOLE_FIGURES`: the population differs, so the floor does too.
4. `doc/verify.md` names the switch for a round that touches field values.

## Measured

Built, Tier 0 over the 382 held documents takes 43 s and Tier 1 about eight minutes, under four rayon threads.

## Cost

A second set of Tier 1 figures to keep, raised and lowered by the same `ratchet:` lines. The held file is the
population's definition here, so a document the held file loses (ADR 3453's 25) leaves the held-only walk too.
