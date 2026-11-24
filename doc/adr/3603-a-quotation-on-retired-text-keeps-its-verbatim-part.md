# 3603 — A quotation on text an agreed erratum retired keeps its verbatim part and states the rest in italics

Session 2225. Status: **accepted**. Context: ADR 3581, ADR 3589 (`spec-errata` reads the site),
`doc/errata-read.md`'s rule for #534, the `every_quotation_is_the_standards_own_words` gate.

## Context

Round 2222 taught `spec-errata check` the site's errata. It found 182 quotations in this tree
that sit on text only the site strikes. They span 62 issues, and every one of those issues is
published on pdf-issues.pdfa.org, which ADR 3581 takes as agreement. The gate verifies every
rustdoc blockquote against `doc/md/`, which is the printed text. So a quotation cannot simply be
rewritten to the corrected wording.

## Decision

The rule is the one `measurement.rs` uses for #534, now applied everywhere:

- A blockquote or a quotation in marks is cut to the verbatim part the erratum does not touch.
- The corrected words follow in *italics*, naming the issue and its publication. Otherwise the
  requirement is paraphrased with no quotation marks.
- A ledger note follows the same rule.
- Records — `doc/adr/`, `doc/history/`, `doc/reviews/`, and `doc/errata-read.md`'s earlier
  sections — keep the words they quoted when they were written. Their landings are explained,
  not edited.

Each erratum's verdict is a row in `doc/errata-read.md`'s section for the site's errata.

## Consequences

The site-only landings `check` prints outside the records fall to the ones listed in that
section, each with its reason. The gate stays the authority on verbatim text, and the italics
carry what the printed text no longer says.
