# 1224 — An FDF annotation is an annotation, and three markup entries read as themselves

Status: **accepted**.
Context: `crates/pdf-model/src/view.rs` (`ViewState::place_annotations`,
`EXCLUDED_FDF_SUBTYPES`, `Imported::annotations`),
`crates/pdf-model/src/forms_data.rs` (`FdfAnnotation`, `read_annotations`),
`crates/pdf-model/src/popup.rs` (`Popup::subject`, `Popup::created`, `Popup::created_date`,
`Comment`), `crates/pdf-model/src/appearance.rs` (`unapplied_default_style`,
`Refusal::DefaultStyleUnapplied`), `crates/pdf-model/src/markup.rs`,
`crates/viewer-core/src/query.rs` (`PopupWindow`).
Builds: ADR 1223 (a value that crosses is copied), ADR 0907, ADR 1066, ADR 1108, ADR 1212,
ADR 0191, ADR 0199.
Clauses: ISO 32000-2 §12.7.8.3.4 (Table 254), §12.7.8.3.1 (Table 246), §12.5.6.2 (Tables 172
and 173), §12.5.6.6 (Table 177), §12.5.5, §7.9.4.

Two readings, and each of them is a clause that turned out to say less than its ledger row
assumed.

## 1. §12.7.8.3.4 needs no second standard

The row said an FDF annotation was blocked twice over: by the second-`Document` design question,
and — for the XFDF spelling — by ISO 19444-1 sections 6.4 and 6.6, which this tree does not hold.
Read against the clause, the first is ADR 1223's and the second was never about the PDF side at
all.

§12.7.8.3.4 is **one sentence**: "Each annotation dictionary in an FDF file shall have a Page
entry (see "Table 254 -Additional entry for annotation dictionaries in an FDF file") that shall
indicate the page of the source document to which the annotation is attached." Table 254 spells
the ordinal — "The ordinal page number on which this annotation shall appear, where page 0 is the
first page" — and that is the whole of what the clause adds. **Everything else in such a
dictionary is §12.5's**, because an FDF annotation *is* an annotation: Table 246 says as much when
it excludes six of Table 171's types from the array, which is a sentence about a list of
annotation types and not about a second grammar.

So once ADR 1223's copy has made the dictionary name nothing of the other file, there is nothing
left to read it by, and ISO 19444-1 is the definition of **XFDF** rather than of this. The
departure that remains on the row is that text's and is stated as such; `doc/questions/Q97` asks
the owner for it.

## Where a placed annotation goes

Into the log a person's own annotations already go into, and not into a second list. That is not
tidiness: `ViewState::added` is what §12.5.5's "previously painted annotations" means for a page,
`write_additions` is what puts an added annotation into §7.5.6's update with Table 166's `/P`, and
a replay clears and re-applies an import like any other log entry. A second list would have needed
its own copy of all three.

Table 254's own `/Page` is dropped on the way in, because `/P` is what a PDF file states instead
of it and two statements about which page an annotation is on is one more than §12.5.2 defines.

**Two refusals rather than a silence.** A subtype Table 246 excludes is refused by name — a file
that wrote one has broken a rule this reader would otherwise act on — and so is an ordinal this
document has no page for, because only somebody who can see both files can say which of the two
is wrong.

## 2. `/DS` is not an entry of Table 172

§12.5.6.2's row named three unread entries: `/Subj`, `/CreationDate` and `/DS`. The first two are
Table 172's and now reach `popup::Popup` and `viewer_core::PopupWindow` beside the title and the
text, through `markup::group_source` because the grouping sentence makes both group attributes.
`Popup::created_date` parses the entry §7.9.4 types it as and `Popup::created` keeps what the file
wrote — the distinction Table 166's `/M` forces for its own reason, kept here so that a file that
wrote something else still says *when*.

The third is the reading. **Table 172 does not list `/DS`.** Its keys are `/T`, `/Popup`, `/RC`,
`/CreationDate`, `/IRT`, `/Subj`, `/RT` and `/IT`. `/DS` appears in §12.5.6.2 exactly once, inside
the group-attribute list — "Contents (or RC and DS )" — and the entry itself is **Table 177's**,
for a free text annotation: "A default style string, as described in Adobe XML Architecture, XML
Forms Architecture (XFA) Specification, version 3.3".

Two things follow. What §12.5.6.2 owes for `/DS` is the **group rule and nothing else**, and the
group rule is applied — `markup::group_source` is where a subordinate's own `/DS` is ignored. And
its *content* is §12.5.6.6's business, whose row is already `implemented` on the argument that XFA
is principle 5's closed exclusion (ADR 1035).

That argument is right and it was leaving a silence. A note whose producer stated a style this
reader does not apply was drawn under `/DA` with nothing said, which is trap 5's shape inside a
feature otherwise built — so `appearance::unapplied_default_style` reports it, on the entry rather
than on every free text annotation (trap 11). The field-side equivalent has had exactly this report
since ADR 1122; this is the annotation-side half of the same sentence.

§12.5.6.2 is therefore `implemented`: every entry of Table 172 has a reader, Table 173's `/ExData`
was disposed of by ADR 1212, and the one residue belongs to another clause and another table.
