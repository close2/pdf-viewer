# 3590 — Every PDF/A erratum has a verdict, and a gate holds the set

Session 2222. Status: **accepted**. Context: ADR 3581 (step 4 of the owner's plan), ADR 3584
(`doc/errata/pdfa.md`'s 34 issues), `doc/questions/Q1043`.

## Decision

`doc/errata-read-pdfa.md` gives each of the 34 issues one of three verdicts — `applied`,
`not applicable`, `recorded disagreement` — with the deciding comment. Five are `applied`, all
already in the tree (#77 ADR 3582; #104 Annex Q in `survey.rs`; #123, #253 and #314 in
`pdf-archive/src/errata.rs`), and none is a recorded disagreement. Seven were agreed into ISO
19005-4's dated revision without the revised words and are `not applicable` to the 2020 edition
this tree validates; Q1043 asks the owner whether that is the reading wanted, and about #77's
reversal for the revision (2023-12-11), which the errata page has not followed.

`tools/conformance/tests/pdfa_errata_verdicts.rs` selects the issues by `pdfa.md`'s rule (a
`PDF/A-` label, or `19005` or `PDF/A` in the title) plus every `data-issue` on the site's ISO 19005
pages, and fails on an issue without exactly one row, a verdict outside the three, a row with no
reason, an `applied` row naming neither ADR nor code, or a row naming an issue the snapshot does not
hold. It reads `doc/errata/` itself: `conformance` does not depend on `spec-errata` (ADR 0252).

## Consequences

The post-merge train's daily `tools/fetch-errata.sh --check` turns a new PDF/A issue into a changed
snapshot, and a re-fetch into a failing gate until the thread is read. No census line moves: no
verdict changes a predicate.
