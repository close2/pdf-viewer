# The copy of the standard this project checks itself against is lossy

Status: **census run and step 2 built in the four-hundred-and-sixteenth session** (ADR 0252). The
census answered the question the item turned on, and it answered it the bad way: the annotations
are the **errata**, and `doc/md/` presents 79 struck-out passages as the standard's current text.
Three of this tree's own quotations were quoting retired sentences. Steps 3 and 4 are open.
Priority: 48 — kept, by the convention that the `40`–`49` band is *the project's own instruments*.
**Its real weight is higher than the number and is now higher than it was**: this is no longer a
question about a copy's fidelity but a list of 76 known-wrong passages in the file principle 5
rests on.
Corpus: —, the subject is `doc/md/` and the fourteen documents under `doc/`
Code: `tools/spec-errata` (built), `crates/pdf-model/examples/spec_annotation_census.rs` (the census)

## What the census found, and what it corrected in this file

The table below **replaces this file's original premise, which was wrong by a factor of thirteen**:
it recorded "882 `/Annots`" in ISO 32000-2 and called them annotations. There are 882 `/Annots`
*arrays* and **11 462 annotations** in them. The item's own rule — census before building — is what
caught it.

| | annotations | what they are |
|---|---|---|
| `ISO_32000-2_sponsored_EC3.pdf` | **11 462** | Link 6031, Popup 2553, **Text 1876, Caret 522, StrikeOut 435**, Widget 38, FileAttachment 6, Highlight 1 |
| `ISO_TS_32002-2022_sponsored_EC3.pdf` | 75 | Link 62, Text 8, StrikeOut 5 |
| `ISO_TS_32001-2022_sponsored_EC3.pdf` | 69 | Link 48, Popup 9, Text 8, StrikeOut 3, Caret 1 |
| the other eleven | 6 to 210 each | **Link, and nothing else** |

The three `_EC3` and `-2022` files record their errata as review markup and apply them to nothing:
**360 distinct `Issue #NNN` subjects**, **434 strikeouts over 4038 words** across **252 sections**,
each with a `Caret` carrying the replacement, and 1752 §12.5.6.4 state annotations whose `/State`
says how far the change got. The body text underneath is unamended.

`/StructTreeRoot` in all fourteen, `/MarkInfo /Marked` in twelve — but `Tree::walk`'s
`MAX_CHILDREN` bound stops at 65 536 and ISO 32000-2's tree is larger, so `logical_order` and
everything on it sees only the front of that document. 1021 of its 1023 pages state
`/StructParents`, so `ParentTree::for_page` is the route that works. Any future substrate goes page
by page.

## What is built

`tools/spec-errata`, with `census`, `emit` and `check`. Six seconds over all fourteen documents. Not
a gate and not a test, and `conformance` does not know it exists — the gate must keep checking
quotations against a conversion this project did not make, or a defect in our extractor becomes a
defect in the standard. ADR 0252 has the argument.

`check` reports two things:

- **79 struck passages of four words or more that `doc/md/` still carries as current text.**
- **Rustdoc quotations that quote a passage struck out of the clause they cite.** Three, all fixed
  in the four-hundred-and-sixteenth session: §7.9.4 (`date.rs`), §7.5.5 (`write.rs`), §14.7.6.1
  (`structure.rs`). Seven more matched a phrase struck out somewhere else and none is a finding,
  which is what `Landing::in_clause` is for.

## What is still owed

1. **Read the other 76.** `spec-errata check | head -80` names each one by document, page and
   section; the sections span §3 to Annex H and include §8.11.4.3, §9.4.4, §12.7.5.5, §14.7.5.4 and
   §14.8.4. Each is a passage the ledger may have been written against. This is the step with the
   findings in it, and it is a reading job rather than an engineering one — the same shape as
   `doc/todo/01`'s sweeps, and the same reason it is not a gate.
2. **Sweep the ledger's own 977 quoted spans**, which `spec-errata` does not see: it scans rustdoc
   blockquotes through `conformance::citation::scan`, and `ledger.toml`'s notes are prose. ADR 0249
   established that those spans are unchecked by anything; they are now unchecked against the
   errata too.
3. **The disagreement sweep** (the old step 3), unchanged: compare our extraction against `doc/md/`
   and report every span where they differ. The annotations were one loss of four; three others are
   recorded in this file's history and none has been swept for.
4. **Only then** the substrate question (the old step 4), and the migration cost is unchanged: a new
   conversion moves every line number and every quotation's whitespace, so switching wholesale means
   re-verifying 6051 citations and 577 quotations. Session 413 declined a 417-span migration.

## What would make this item wrong

It was written with the closing condition "if the annotations are links and bookmarks, steps 2 to 4
buy nothing". Eleven of the fourteen documents met it and the three that matter did not, so the
condition is spent. **The replacement, and it is a harder one: if reading the remaining 76 passages
turns up no clause this tree implements differently, then the errata are a documentation concern
and not a correctness one, and this item drops out of the instruments band into the notes.** Say so
plainly in that case — an item that survives its own evidence unchanged is one nobody re-checked.
