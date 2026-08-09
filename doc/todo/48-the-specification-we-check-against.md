# The copy of the standard this project checks itself against is lossy

Status: **census run, step 2 built, step 1 of what was owed read** — sessions 416 (ADR 0252) and
417 (ADR 0253). The census answered the question the item turned on and answered it the bad way:
the annotations are the **errata**, and `doc/md/` presents struck-out passages as the standard's
current text. **All 79 that were known have been read** — `doc/errata-read.md`, one line each with
a verdict — and reading them found **two clauses this tree implemented differently**, which settles
this file's own closing test against closing it.
Priority: 48 — kept, by the convention that the `40`–`49` band is *the project's own instruments*.
**Its real weight is higher than the number**: this is a list of known-wrong passages in the file
principle 5 rests on, and the list grew when the instrument was corrected — **151 now, not 79**.
Corpus: —, the subject is `doc/md/` and the fourteen documents under `doc/`
Code: `tools/spec-errata` (built), `crates/pdf-model/examples/spec_annotation_census.rs` (the census)
Read: `doc/errata-read.md` — the 79, their verdicts, and what is owed

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

`check` reports two things, and **its comparison changed in the four-hundred-and-seventeenth
session**: it drops every space before comparing, because both sides are extractions of the same
glyphs by different programs and neither can recover a space the file does not state. That took the
first number from 79 to 151 and found the session's largest finding (ADR 0253).

- **151 struck passages of four words or more that `doc/md/` still carries as current text**, 120
  of them distinct, over 92 sections. 79 were known before the correction; all of those are read,
  in `doc/errata-read.md`.
- **Rustdoc quotations that quote a passage struck out of the clause they cite.** Four now. Three
  were fixed in the four-hundred-and-sixteenth session — §7.9.4 (`date.rs`), §7.5.5 (`write.rs`),
  §14.7.6.1 (`structure.rs`) — and each keeps its retired blockquote deliberately, with the erratum
  stated beside it, because the blockquote is what the gate verifies against `doc/md/`. The fourth
  is `date.rs`'s second struck sentence, which the coarser comparison found. Ten more match a
  phrase struck out of another clause; all ten have been read and none is a finding, which is what
  `Landing::in_clause` is for.

## What is still owed

1. **Read the 55 newly visible passages.** The four-hundred-and-seventeenth session found the
   checker's comparison blind to a space — both sides are extractions of the same glyphs by
   different programs, and one writes "inthe" where the other writes "in the" — so
   `still_in_conversion` reported 79 where the answer is **151** (ADR 0253). 55 further *distinct*
   passages appeared, none of them read; among them §7.2.3, §7.8.3, §8.4.5, §8.6.6.5, §8.9.6.3,
   §12.5.6.19, §12.7.4.3, §14.5, §14.8.6.2 and Annex F.3.5. Same shape as step 1 was: a reading job
   rather than an engineering one, and not a gate.
2. **The specific corrections the first reading left owed**, each with its erratum number and the
   replacement text, in `doc/errata-read.md`'s "Owed" section — §8.9.5.4's rewritten algorithm
   above all, which is the one clause this tree still implements a retired version of.
3. **Sweep the ledger's own 977 quoted spans**, which `spec-errata` does not see: it scans rustdoc
   blockquotes through `conformance::citation::scan`, and `ledger.toml`'s notes are prose. Two of
   them were found stale by hand in the four-hundred-and-seventeenth session — §12.5.2's and
   §12.7.5.4's — which is two out of two attempts and says the population is worth a pass. ADR 0249
   established that those spans are unchecked by anything; they are now unchecked against the
   errata too.
4. **The disagreement sweep** (the old step 3), unchanged: compare our extraction against `doc/md/`
   and report every span where they differ. The annotations were one loss of four; three others are
   recorded in this file's history and none has been swept for.
5. **Only then** the substrate question (the old step 4), and the migration cost is unchanged: a new
   conversion moves every line number and every quotation's whitespace, so switching wholesale means
   re-verifying 6070 citations and 575 quotations. Session 413 declined a 417-span migration.

## What would make this item wrong

It was written with the closing condition "if the annotations are links and bookmarks, steps 2 to 4
buy nothing". Eleven of the fourteen documents met it and the three that matter did not, so the
condition was spent, and session 416 replaced it: **if reading the remaining passages turns up no
clause this tree implements differently, the errata are a documentation concern and not a
correctness one, and this item drops into the notes.**

**That test has been run and the item passes it.** Two clauses were implemented differently —
§12.5.2's `/BM`, which was being ignored on every stored appearance stream, and §14.13.5's
property-list key — and a third, §8.9.5.4, still is. So the item stays where it is.

The replacement condition, for whoever reads the 55: **if none of them touches a clause this tree
implements, and the ledger's 977 spans come back clean, then what is left is a documentation debt
with a known size and it belongs in the notes.** Say so plainly in that case. Note that the first
reading's own rate was two findings in sixty-two passages, so a clean sweep of fifty-five is a
result rather than a formality.
