# 1186 — What crosses from another file, and what only names one

Status: **accepted**.
Context: `crates/pdf-model/src/forms_data.rs` (`FdfField::icon_fit`, `Import::icon_fit`,
`read_field`), `crates/pdf-model/src/view.rs` (`AnnotationView::icon_fit`),
`crates/pdf-model/src/appearance.rs` (`IconFit::read`),
`crates/viewer-core/src/viewer.rs` (`attachments_in_view`),
`crates/pdf-model/src/attachment.rs`.
Builds: ADR 0295 (the entry nobody wired), ADR 0907, ADR 1145, ADR 1155 (a file a document named).
Clauses: ISO 32000-2 §12.7.8.3.2 (Tables 249 and 250), §12.5.6.19 (Table 192), §12.7.6.4,
§14.13.4 (Table 31's `/AF`), §14.13.1, §14.13.3.

Two decisions, and they are one question asked twice: **when may this program act on something
stated somewhere other than where it is drawing?**

## 1. Table 249's five unapplied entries, and the one that crosses

§12.7.8.3.2's binding sentence is indicative rather than modal, which ADR 0907 already records:
"Unless otherwise indicated in the table, importing a field causes the values of the entries in
the FDF field dictionary to replace those of the corresponding entries in the field with the same
fully qualified name in the target document." *Entries* there is every entry of Table 249, so each
of the five this tree named and did not apply is a requirement unmet and not a permission declined.

They do not divide by difficulty. They divide by **where the value lives**.

- **`/IF` crosses, and is applied.** Table 250's icon fit dictionary is four entries and every one
  of them is a name, a pair of numbers or a boolean: `/SW`, `/S`, `/A`, `/FB`. Nothing in it
  reaches an object of the FDF file, so the dictionary carries to the target document whole. And
  it has a corresponding entry to replace: Table 192's `/IF` in the widget's `/MK` names the same
  Table 250 dictionary — the table is printed under §12.7.8.3.2 and §12.5.6.19 points at it — so
  the import is that sentence applied literally. The icon it fits is the widget's own `/MK /I`,
  which is in the target document where it always was. `Import::icon_fit` carries it,
  `AnnotationView::icon_fit` hands it to the appearance, and `IconFit::read` prefers it over the
  widget's.
- **`/AP` does not cross.** "[T]he values of the `N`, `R`, and `D` entries shall all be streams",
  and those streams are objects of the FDF file: each has its own `/Resources`, which name more
  objects of that file. Drawing one means a second `Document` reaching the interpreter, which is
  the design question §12.7.8.3.4's row has held open for Table 254's annotations since the
  hundred-and-fifteenth session. One question, two sites; deciding it here would decide it there
  by accident.
- **`/APRef` does not cross, and is the sharper case.** It is "[a] dictionary holding references
  to external PDF files containing the pages to use for the appearances of a push-button field",
  whose `/N`, `/R` and `/D` are Table 253 named page references. Table 253's `/F` is a file a
  *document* named, which is §12.7.6.4's hazard exactly: `viewer_host::read_import` answers that
  one with a directory a **person** supplied, and ADR 1155 fixed the position that nothing may be
  looser than it. `pdf-model` has no filesystem and must not acquire one, so this entry is a host
  question before it is a rendering question.
- **`/A` and `/AA` do not cross.** An action read out of an FDF file resolves its own references
  *there*, and this tree reads a widget's actions from the target document at the moment it is
  activated (`pdf_model::action::for_annotation`). An imported action would therefore be a value
  carried from one object space into another, where a destination written as an object reference
  names a different object or none. Installing one is a log entry beside the document like every
  other edit — never a mutation — and what it needs first is a rule for the references it brings
  with it.
- **`/RV` is excluded**, by `CLAUDE.md`'s closed list: it is XFA rich text.

Each of the four is named on `FdfField::owed`, so a person importing such a file is told what the
file said and this program did not do (trap 5). §12.7.8.3.2's row stays `partial` on those four
and its note says so in the clause's own words.

## 2. §14.13.4's page, and which page an answer carries

§14.13.4 states that "[o]ne or more files may be associated with any PDF page by including a file
specification dictionary … for each file as one of the members of the array value of the AF key in
the appropriate page dictionary". `attachment::associated` has read that array from any carrier
since §14.13 was implemented — and **no caller in this tree ever handed it a page dictionary**.
That is the shape ADR 0295 named for §12.5.6.15's `/FS` and that §14.13.3's catalog array was in
before `attachment::attachments` reached it: a reading that exists, a row that calls it
implemented, and a payload no panel could list.

**The answer carries the page the reader is on, and that is principle 2 rather than a shortcut.**
A panel listing every page's associated files has to walk the whole page tree, and every host asks
`Query::Attachments` when a document opens — which is the launch path `CLAUDE.md` forbids a full
page-tree walk on, and the measurement ADR 0295 took for the neighbouring clause is 78 to 123 ms
cold over ISO 32000-2's 1023 pages. The page in view is one this crate has already built, so the
entry costs a dictionary lookup, and the list follows the reader exactly as `attached_files`
follows the pointer.

**What that costs is said rather than hidden**: a file associated with page 40 is not listed while
the reader is on page 1. The alternative was not "list them all" — it was "list them all at the
price of the open" — and a panel is not worth that. A document-wide list of page associations
remains available to anything that has already walked the pages for another reason, and
`attachment::associated` is the same function it would call.

## 3. What this does not decide

Whether an associated file should be grouped *under* its page in a host's panel. `Attachment`
carries no page, every host renders one flat list today, and adding a carrier to the structure is
a `doc/ui-boundary.md` question rather than a clause one.
