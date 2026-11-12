# 2815 — An insertion raises the version, Annex I's warning names saving, and a written date is a whole date

Round 2000, the audit-fix round, group TR (transform and archive).
Status: **accepted and built** in `pdf-transform` (`update.rs`: `raise_version`, `validate`,
`keeps_the_required_date`, `restated_packet`) and `viewer-core` (`notes.rs`, the Annex I notes).
It answers findings of `doc/reviews/1983-specification-audit.md` and supersedes, for the points named
below, ADR 0855 (the date check of `SetInformation`) and ADR 0206 (which half of Annex I's sentence the
notes carry).
Context: ADRs 0206, 0207, 0854, 0855, 1171, 1473.
Clauses: ISO 32000-2 Annex I.2, §7.7.2 (Table 29 `Version`), §7.9.4, §14.3.3 (Table 349 `ModDate`), §14.5.

## 1. Inserting a newer document raises the version

Annex I.2: "If a document is modified by inserting the contents of another PDF file into it, the saved
document's version should be the most recent of the original PDF file's version, and the inserted PDF
file's version." The ledger's I.2 row said this was "about a verb this program does not have";
`update::Edit::InsertPages` is that verb (ADR 0854). It now writes Table 29's `/Version` into the catalog
the update already rewrites, set to the inserted document's version, where that one is the newer. The
header cannot move — §7.5.6 keeps the original bytes intact — and §7.7.2 has a catalog `/Version` later
than the header's take its place. Where the original is the newer or the two are equal nothing is
written, by the same paragraph's "a PDF file's version should never be changed to an older version".
`merge` already wrote the highest version of its inputs.

## 2. Annex I's warning carries both halves

The sentence names two causes and one warning with two parts: "it should warn the user that it is
unlikely to be able to read the document successfully and that the user may not be able to change or
save the document." ADR 0206 read "the second half" as the requirements loop; it is the second part of
the warning. The version note now says both parts, and a note after the requirements names the same
two parts once for every unmet requirement.

**The first-error recommendation is assessed and declined, deliberately.** "Upon the first error that is
caused by encountering an unrecognised feature, the PDF processor should notify the user that an error
has occurred but that no further errors will be reported." This program reports every unrecognised
feature it meets, as a structured `pdf_model::Unsupported` per occurrence (worded by
`viewer_core::report`), and every document-level fact as a note. Suppressing the second and later
reports would hide what is missing from a page, which trap 5's rule (unsupported input stays loud)
forbids; the clause's word is *should*, and its own parenthesis already exempts the errors a person must
always see. The recommendation's purpose — not to bury a person in errors — is served by the page report
being one list per page rather than one dialogue per error. The owner is asked to confirm (results file,
"owner must decide").

## 3. A date written into somebody's file is a whole §7.9.4 date

ADR 0855 held `SetInformation`'s dates to the prefix, four year digits and the clause's alphabet, on the
reading that "the field-by-field ranges are the producer's business". When this program writes the date
it *is* the producer, so the ranges are its business: §7.9.4 gives each field its range (`MM (01-12)`,
`DD (01-31)`, …) and says "all other fields may be present but only if all of their preceding fields are
also present", so `D:20261301` and `D:2026010` are not dates. `validate` now asks
`pdf_syntax::date::Date::parse`, the one reading of §7.9.4 the tree has, and refuses what it cannot read.
`restated_packet`'s branch for an unreadable date now leaves the packet's property as it stands rather
than removing it; it is unreachable through `validate`, and it no longer loses a date if a caller ever
skips the check.

## 4. `/ModDate` stays where `/PieceInfo` requires it

Table 349: `ModDate` is "( Required if PieceInfo is present in the document catalog dictionary;
otherwise optional; PDF 1.1 )". A removal of `/ModDate` from a document whose catalog states
`/PieceInfo` is now refused by name; any other edit of the date, and the removal from a document without
`/PieceInfo`, are unchanged.

## Consequences

The I.2 ledger row's sentence about the insertion verb is replaced by what §1 builds. The audit's
`annex_i_insert_version`, `date_spec_audit` and `metadata_requirements_audit` regressions pass.
