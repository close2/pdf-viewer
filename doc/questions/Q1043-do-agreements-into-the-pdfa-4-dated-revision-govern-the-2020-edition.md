# Q1043 — Do agreements into PDF/A-4's dated revision govern the 2020 edition, and does #77's later reversal change ADR 3582?

Asked by round 2222 (ADR 3590), on step 4 of your errata plan of 2026-11-24.

**The condition.** `doc/errata-read-pdfa.md` gives every PDF/A issue in `doc/errata/` a verdict.
Two cases need your word, because principle 5's new sentence ("an erratum the standard's working
group agreed is the standard") reads two ways on them.

1. **Seven issues were agreed into the dated revision of ISO 19005-4, not as errata to the 2020
   edition**: #192 (invisible signatures exempt from 6.3.2's flag rules), #197 (rendering intents
   limited to Table 69's four), #232 (an explicit prohibition of the `F` operator), #244, #626, #633
   and #842. Each thread records the working group's agreement ("Change applied!", "already been
   addressed in PDF/A-4 dated revision") but not the revised words, and the site publishes none of
   them. ISO/DIS 19005-4:202x is under ISO ballot. I recorded them `not applicable` to the 2020
   edition, which is what `pdf-archive`'s PDF/A-4 targets name and what veraPDF 1.30.3 validates.
2. **#77, which ADR 3582 applied, was later reversed for the revision.** On 2023-12-11 bdoubrov
   wrote: "The decision of PDF/A TWG is not to make this change in the dated revisions of PDF/A-4
   and PDF/X-6"; on 2024-02-16 it was parked pending a white paper. The errata page still publishes
   *less than or equal*, and veraPDF follows it. I left it `applied`.

**The question.** Which do you want?

1. **As recorded**: the 2020 edition is the target; a revision's agreements apply when the revision
   is published and held, and #77 stays applied because the page publishes it.
2. **Hold the DIS now**: fetch ISO/DIS 19005-4:202x (a public review draft when ISO opens it) into
   `doc/pdfa/`, and apply #192, #197 and #232 from its words to the PDF/A-4 targets.
3. **Something else** for #77: for instance, accept either direction of the comparison until the
   white paper settles it, said in the report as an ambiguity.
