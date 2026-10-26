# 2346 — Table 15's `/Size` sentence is re-read against the rebuild, and the departure stands

Status: accepted. Session 1756, 2026-10-26, the clause slot of batch one hundred and fifteen.
Clauses: ISO 32000-2 §7.5.5 (Table 15), §7.5.8, §C.4. §7.5.5 stays `departed`; its note is
rewritten to what is. Context: ADRs 0048, 0789, 1035, 1165, 1622. I worked in `scratchpad/r1756/`.

## 1. The question, and the premise that did not hold

Table 15's `/Size` row ends "Any object in a cross-reference section whose number is greater than
this value shall be ignored and defined to be missing by a PDF reader." ADR 0048 priced it at 66
documents losing their page tree, before this tree could rebuild a table that names no catalog. The
brief asked whether the sentence can now be obeyed on the section as read, with the 66 going through
"§7.5.8's reconstruction path". §7.5.8 is cross-reference streams and reconstructs nothing. The
rebuild this tree has is `xref::rebuild`, which §C.4 licenses in an informative annex: "When a PDF
processor reads a PDF file with a damaged or missing cross-reference table, it may attempt to rebuild
the table by scanning all the objects in the file." Two things reach it: `xref::read`, when the table
is absent, unreadable or empty, and `Document::open`, when the table leads to no catalog. The
hypothesis was tested against those two.

## 2. Who the sentence reaches

`cargo run --profile gates -p pdf-model --example size_census -- doc/pdf.js/test/pdfs` counts the
documents whose table was read from the file and states an in-use entry numbered above `/Size`,
walks the trailer's reference graph, and names who reaches such an entry. All 66 do, and 65 of them
name `/Root` above `/Size`. These producers wrote the number of their entries where the row asks for
"1 greater than the highest object number defined in the PDF file". `bug1027533.pdf` states entries
0, 20, 100, 110 and 121–125 under `/Size 8`. The one exception is `issue15716.pdf`, whose catalog
(1) and page tree root (2) sit below its `/Size 8`, while its page (10), that page's content, pattern
and font, and two optional content groups (1000, 1001) sit above it.

## 3. The sentence obeyed, measured

I built an export of HEAD with one change: each section's in-use entries numbered above its own
trailer's `/Size` are dropped before the sections merge. Free entries are kept, because a free entry
describes no object. Every repair HEAD has was left as it is.

- The census under that reader: tables rebuilt by scanning went from 27 to 92, and documents with no
  page one from 5 to 6. Each of the 65 was rebuilt, by `xref::read` where nothing was left in its
  table or by `Document::open` where what was left named no catalog.
- `display_list_digest` over the 66 first pages: 65 identical, list and reports. `issue15716.pdf`
  has no page. Its table is not empty and it names its catalog, so neither rebuild runs, and
  `Pages`' recovery scan only asks the numbers the table names.
- Instructions to page one (callgrind, `size_census --page-one`) across the 66 went from 5 238 767 to
  100 975 486. For `issue8795_reduced.pdf` (4 244 537 bytes) they went from 428 558 to 61 794 567.
  The rebuild reads the whole file, so the cost grows with its length. None of `t2-launch_path`'s six
  documents states an entry above `/Size`, so the launch gate never meets the change.

## 4. Why the reading does not hold

- **The repair restores what the sentence ignores.** A rebuilt table has no cross-reference section,
  so the sentence has nothing to apply to there. The 65 documents are drawn from every object the
  sentence said to ignore. "Obeyed on the section as read" is true of the section and false of the
  document.
- **It takes effect only where the table is otherwise sound.** `bug1027533.pdf` loses every entry to
  the sentence and is rebuilt whole. `issue15716.pdf` loses one subtree and keeps the table that lost
  it. A rule that does less harm the more of a table it removes is choosing documents by the damage
  it does to them, not reading them.
- **The next repair would empty the sentence of effect.** `issue15716.pdf` could keep its page by
  finding an ignored number through its own `N G obj` header. That is ADR 0789's route for a number
  declared and unread. But it would bring back every ignored object at first use, and the sentence
  would then change nothing on any document. That is the departure under another status.
- **A page that draws differently is its own decision or it is taken back** (the batch's
  discipline). The export's change is taken back, and `pdf-syntax` is unchanged.

## 5. Decision, and what a later construction owes

The departure stands. Its ground, rewritten into the row: under this tree's own repairs, obeying the
sentence restores every ignored object wherever it leaves no catalog, and it costs a page where it
leaves one. Separately, a number above `/Size` is looked up like any other and fails like any other
if its offset is wrong. `size_census` is the command that counts the price, so the next re-reading
counts it rather than quoting ADR 0048.

A construction that does drop entries owes the writers one more thing. Five of them number a new
object past `object_numbers().max()`: `pdf-model`'s `view.rs` and `view/writing.rs`, and
`pdf-transform`'s `attachments.rs` and `update.rs`, each taking the larger of that and `/Size`, and
`archive/prepare.rs`, which then skips a number that resolves to something. In these documents
`/Size` is the understated figure. A table that drops numbers would let an incremental update reuse
a number the file still holds and its references still name. So such a table must carry the highest
number its sections stated.
