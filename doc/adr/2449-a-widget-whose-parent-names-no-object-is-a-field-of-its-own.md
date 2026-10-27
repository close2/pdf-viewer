# 2449 — A widget whose parent names no object is a field of its own

Status: accepted. The orchestrator's, at batch one hundred and twenty-one's merge, 2026-10-27; the number
was assigned to round 1807 and unused there. Clause: ISO 32000-2 §7.3.9 and §12.7.4.2. No ledger row moves.
Context: ADRs 2448 (round 1807's resolver), 1653 (fields recovered from the pages), 2242 (a red gate is
judged, re-taken, fixed or asked — never accepted); the held file `crates/pdf-model/tests/script_corpus.tsv`.

## 1. What the merge gate read

`t2-script_corpus` held one document differently from its line:
`corpus-cache/tika-issue-tracker/batch1/PDFBOX/PDFBOX-3891-2.pdf` went from 14 fields, 6 sites not run, to
76 fields, 7 sites not run, with a new digest. Round 1807 attributed it by its one change, reverted and not
in two exports of the batch: `widgets_by_field_name` gives 14 names on HEAD's and 76 on round 1807's, and
the cause is `view/field_tree.rs::named_root`.

## 2. Why 76 is the clause's count

The document's `/AcroForm /Fields` is empty, so every field is recovered from its pages (ADR 1653). Of its
275 widgets, 113 state a `/T`, and most of those state `/Parent 1209 0 R` — an object the file does not
hold, which this reader resolves to null. §7.3.9:

> An indirect object reference (see 7.3.10, "Indirect objects") to a nonexistent object shall be treated
> the same as a null object. Specifying the null object as the value of a dictionary entry (7.3.7,
> "Dictionary objects") shall be equivalent to omitting the entry entirely.

A widget whose `/Parent` names a nonexistent object states no parent, so it is the root of its own
hierarchy and a field under its own `/T` (§12.7.4.2); HEAD took the dangling reference for the root and
walked nothing, which kept only the 14 widgets with no `/Parent` entry at all. The one more site not run
belongs to a field now reached. qpdf, as evidence only, omits the same dangling entry and lists 77 names,
perhaps counting a widget it reports as unreachable from `/AcroForm`; that one name is not read here.

## 3. Decision

The held line follows the reading: `76	0	7	6	0	41a812a7e18907ac`. Nothing else in the gate moved.
