# 3581 — A working-group-agreed erratum is the standard, and the printed text is retired

Session 2220. Status: **accepted**, the project owner's decision of 2026-11-24. Refines ADR 0601
("an erratum is evidence about the standard"). Context: veraPDF-library #1668,
pdf-association/pdf-issues #77, ADR 3447, `CLAUDE.md` principle 5.

## The decision, in the owner's words

On 2026-11-24 the owner answered a five-step plan about errata — fix the known error; hold every
errata source; have `spec-errata` read the site's errata; give every PDF/A erratum a verdict with a
gate that fails on one without; and the habit of checking errata and pdf-issues before calling a
disagreement another implementation's or filing it upstream — with:

> agree in all points (and explicit yes to the principle 5 point)

The principle-5 point is this: **where an erratum agreed by the working group that owns a standard
— published on pdf-issues.pdfa.org, or recorded as agreed in `pdf-association/pdf-issues` —
corrects a clause, the corrected text is the standard and the original is recorded as retired.**
`CLAUDE.md` principle 5 carries that as one sentence citing this ADR.

## Why this refines ADR 0601 rather than contradicting it

ADR 0601 found two *accepted* errata to ISO 32000-2 that cannot both be applied, and ruled that an
erratum is evidence, decided by the published clause and its own arithmetic. That ruling stays for
what it was about: a collection that disagrees with itself. What it cannot be is a licence to keep
a sentence its own authors have withdrawn. ISO 19005-4 6.2.10.3.1's supplement comparison is the
case: the PDF/A Technical Working Group agreed on 2022-02-03 that the printed *greater than or
equal* is wrong and *less than or equal* is meant (#77), the site publishes the correction, and
this tree spent a census and an upstream issue (#1668) defending the retired text against a
validator that had followed the correction. The owner's acceptance of pdf-issues #850 was already
this rule in one instance.

So the order is: an erratum the working group agreed governs; two that contradict each other go
back to ADR 0601's reading; a proposal nobody agreed is evidence, as before. The `data-iso`
attribute of the site's source (`approved`) records ISO's own approval and is not what decides
here: #77's correction is on the page without it, and the working group's agreement is the
owner's criterion.

## Consequences

- ADR 3582 applies it to the one known error.
- ADR 3583 holds the sources on disk, so "recorded as agreed" can be read without a network.
- Before a disagreement with another implementation is called theirs, or filed upstream, the
  held errata and issues are searched for the clause (step 5 of the plan; `doc/habits/` takes it
  when the later round builds the gate of step 4).
