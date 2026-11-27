# 3825 — The tracker's moves of 25 November read: none agreed against a clause this program implements

Session 2281. Status: **accepted**. Context: ADR 3581 (an agreed erratum is the standard), ADR 3583
(`doc/errata/` held, `tools/fetch-errata.sh --check` on the train), ADR 3589 (`spec-errata` reads the
site), `doc/errata-read.md`'s last section (the table this decision reads).

## Context

The train's daily `--check` failed: `issues.jsonl` and `comments.jsonl` moved, and no page of
`site/` did. The re-fetch holds `pdf-association/pdf-issues` at `d9b2519a`. Twenty-nine issues
changed, one is new (#865) and six comments are new.

## Decision

1. **Each move gets a verdict by ADR 3581's criterion**: an erratum is the standard where it is
   published on the site or recorded as agreed in its thread. Closing an issue is not agreement.
   Eight PRC issues were closed as completed with no comment, and the site carries no ISO 14739-1
   page change. #817's one new comment shows the proposals moving to the 3D PDF TWG's own source,
   which is not yet the standard.
2. **#865 (ISO 32000-2 §12.5.5) is evidence.** It is open and its thread records no agreement. Its
   proposal changes what a writer *should* do and leaves a reader's rule as printed. The
   construction it reports, an empty `/N` subdictionary with no `/AS`, is drawn as nothing by
   `pdf-model` and named under Table 166's requirement. That is the printed clause's reading.
3. **The twenty-eight PRC moves are inside the exclusion.** Rendering PRC artwork is excluded
   (`doc/scope-ratifications.md`) and no code in this tree reads ISO 14739-1. They are recorded so
   that the round meeting the trigger reads them, closed or not.
4. No ledger row, quotation or code changes. `spec-errata check` reads only the site, which did
   not change, so its output is the same.

## Cost

One table section in `doc/errata-read.md`. The re-fetch adds about 60 KiB.
