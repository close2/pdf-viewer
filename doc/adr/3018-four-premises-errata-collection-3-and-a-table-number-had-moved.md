# 3018 — Four premises the errata and a table number had moved, corrected on those points

Session 2082. Status: **implemented** (documentation and comments; no behaviour changes). Corrects ADR 0601,
ADR 0803 and ADR 0473 on the points named below; their other decisions stand. Context: review 2038 (findings D03,
D44, D48, D63). Clauses: ISO 32000-2 §12.4.2 Table 161, §13.2.4.2 Table 285, §7.5.4, §7.7.3.3 Table 31.

## 1. Table 161's letters: the conflict ADR 0601 recorded was already settled (D03)

ADR 0601 found two `Review/Accepted` errata on Table 161's `/S` row that cannot both be applied: Issue #432
(`ZZ` struck for `AZ`, an odometer) and Issue #593 (*AAA to ZZZ for the next 26, AAAA to ZZZZ for the next 26,*
inserted, the repeat). It kept the repeat on the published sentence's own count and recorded the disagreement
as a claim that might decay.

The tracker had settled it before that ADR was written. On pdf-association/pdf-issues #432, a comment of
2025-08-14 reopens it because #593 showed the original text was right. #593's proposal (`A,B-Z, AA,BB-ZZ,
AAA,BBB-ZZZ, ...`) was agreed by the PDF TWG on 2025-10-23. On 2025-11-10 #593 was closed completed and #432 closed
with the `duplicate` label. The sponsored PDF's overlay is a snapshot older than that. So the repeat that
`page_label::letters` produces is the amended standard's reading, not only the published count's, and there is
no conflict left to record. The ledger's §12.4.2 note and `doc/errata-read.md` say so. `letters`'s doc comment
belongs to `crates/pdf-model/src/page_label.rs`, which another round holds; it is corrected when that round is merged.

## 2. Table 285's `/MH`: the overlay contradicts the issue it carries (D44)

Errata Collection 3's Issue #179 has a strike over the `/MH` row's *"Table 287 — Entries in a media clip data
MH/BE dictionary"* and a caret that writes *"Table 289 — Entries in a media clip section MH/BE dictionary"* in its
place. Issue #179 (`ISO approved`, closed 2022-05-20) asks for the opposite change: its body calls the `/BE` row's
Table 289 a copy-paste error and asks that it match the `/MH` row's Table 287. §13.2.4.2's own prose, "If BU is in
the MH dictionary and the base URL is not honoured the media clip data shall be nonviable", names an entry that
only Table 287 holds.

**Decision**: the overlay is read as misplaced. `media.rs` reads both of a media clip data dictionary's `/MH` and
`/BE` by Table 287, and a section's by Table 289, which is the issue's resolution. §13.2.4.2's ledger note and a
row in `doc/errata-read.md` record the conflict. A clarification from the PDF Association would settle which
of the overlay and the issue is the error. That request is the owner's to make. It is not a precondition, because
both texts agree that the data's `/MH` is read by Table 287.

## 3. The detached free entry no longer rests on a struck NOTE (D48)

ADR 0803 chose §7.5.4's second free-entry mechanism (entries linking back to object 0 with generation 65,535)
because NOTE 3 said an incremental update's subsections "can never have an object number of zero". Errata
Collection 3's Issue #147 strikes that sentence and changes "positive" to "non-negative". An update may now
restate object 0. The choice stands on a different reason: the detached entry states only what the update frees.
Using the linked list would restate object 0 and thread the new entries into a list whose earlier links stand in
sections the update does not rewrite. `write::incremental_update_freeing`'s doc comment and the §7.5.4 ledger note
now give this reason. The tests that already held the writer (`incremental_update.rs`) are unchanged, because the
behaviour is.

## 4. A page's `/Dur` is Table 31's (D63)

ADR 0473 twice cites "Table 160's `/Dur`" for a page's display duration. The page entry is Table 31's (§7.7.3.3,
Entries in a page object). Table 160 is the navigator dictionary's, and Table 164's `/D` is a transition's duration.
The ADR's decision, one shared clock and the incoming transition's timing, is unchanged. Only the locator is
corrected, and no other file in the tree repeats it.
