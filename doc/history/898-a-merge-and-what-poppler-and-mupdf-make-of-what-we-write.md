# 898 — A merge, and what poppler and mupdf make of what we write: RFC 0002 §9's fourth layer, and a parent tree that indexed nothing

Date: 2026-09-03.
ADRs: [0838](../adr/0838-a-parent-tree-value-is-an-array-whether-the-tree-holds-it-or-names-it.md),
[0839](../adr/0839-the-foreign-readback-and-what-a-second-reader-is-asked.md).
Touched: `crates/pdf-transform/src/structure.rs` (`parent_tree`, `source_entry`'s comment,
`orphaned_items` and the warning that names it), `crates/pdf-transform/tests/merge.rs`
(`ParentTreeValue`, `tagged_document_with`, one test),
**`crates/pdf-transform/tests/foreign_corpus.rs`** (new), `crates/pdf-transform/Cargo.toml`
(`pdfref` as a dev-dependency), `doc/conformance/ledger.toml` (§7.3.10, §12.8.1, §14.7.2,
§14.7.5.4), `doc/todo/02-every-round.md` (the new gate line and the map's row),
`doc/todo/57-the-transform-suite.md` (§5 rewritten, the gap closed), `tools/state.sh`; two ADRs,
this file; and the merge commit before this round's own.

## The merge

**`round-867` (969cc305) is on `main` as `a5066598`**, `--no-ff`, on top of rounds 894 and 895.
It is the whole transform stream since that branch's last merge: 886 (e1aa5adf) with RFC 0002
§10's structure-preserving serializer, `split`, and the `CLAUDE.md` authoring-exclusion amendment
the owner ratified; 888 (dafef03c) with `merge` and one document-level reconciliation per clause;
893 (97838ced) with `pages`; and 897 (969cc305) with §14.7's structure tree carried by all three.
**The branch stays open** — `optimize` is what it does next.

**Git found no textual conflict at all**, which is worth recording rather than glossing: the
branch had already merged `main` twice (db6c9e64 for round 893, 96b91192 for 897), and main's own
two sessions since are `pdf-model/src/image.rs`, its `dct_components` tests, `doc/rfc/0006`,
`doc/checks/fixed-documents.toml` and four ledger rows — none of which the branch touches.
`doc/conformance/ledger.toml` auto-merged although both sides wrote to it, and `git diff main HEAD
-- doc/conformance/ledger.toml | grep '^-'` prints nothing, which is the check that main lost no
line rather than a hope that three-way merging did the right thing.

**Three things were checked by hand, because a text merge cannot see them.** Round 893's history
names the sharpest: `open_inputs` exists only on the branch and main's round 881 had changed the
single-input body to open through `FileBytes::on_disk`, so taking the branch's function whole
would have dropped the on-disk reader silently. The merged `bin/pdf-transform.rs` opens every
merge input on disk, ADR 0809's comment above it. `pdf-model/src/image.rs` is main's, unchanged.
And main's two new files are present.

## The round

### 1. The gate that had been owed since the first writer

Four writers existed and **nothing had shown their output to anybody else**. Every walk in
`pdf-transform` judges what it wrote with this tree's own parser and this tree's own rasteriser,
which answers "did we write what we meant to" and cannot answer "did we write what the format
says" — a misreading on the way out and the matching one on the way back in agree perfectly.

`tests/foreign_corpus.rs` is RFC 0002 §9's fourth layer, and ADR 0839 has the design. The one
sentence that decides its shape: **the comparison is foreign-to-foreign**. poppler draws the
source page, poppler draws the derived page, and a difference between those two is ours. Nothing
here compares a foreign reader against our renderer — that is the oracle's question with the
oracle's tolerances, and mixing them makes a disagreement unattributable (traps 3 and 9). The
consequence is that this gate can assert **bit identity**, which the oracle cannot.

`qpdf --check` is asked as a *change of verdict* rather than a verdict: a corpus document it
already complains about says nothing about what we wrote. `mutool show` is asked about §14.7's
parent tree, which is the only part of a derived document that **nothing rendered can see**.

### 2. What it found on its first run, before it was a gate

A parent tree that indexed nothing, on most of the corpus's tagged documents (ADR 0838).
§14.7.5.4's page value is "an array of indirect references to the sequences' parent structure
elements", and §7.3.10 lets that array be an object of its own — "any object value may be a direct
or an indirect reference; the semantics are equivalent". The carry looked the key up *unresolved*,
correctly, because the array's members are identities it maps; but the value it got back for such a
document was the reference to the array rather than the array, so the arm that maps an array never
fired and the output stated a one-long `[null]`.

**70 of the 83 tagged corpus documents whose tree states a root-level `/Nums` write it that way.**
Not one instrument in this tree could see it: §14.7 is invisible on the page, so all four of RFC
§9's layers pass, and `check_structure`'s first property asks only that the value *be* an array —
which `[null]` is. `mutool show` printing `/Nums [ 0 [ null ] ]` beside the source's
`[ 12 0 R 15 0 R ]` is what said it.

The fix is one line and the reading is the sentence above it: **the array is resolved and its
members are not**. `tests/merge.rs` gained a fixture in §7.3.10's other form — the tagged fixture
had only ever had the direct one, which is why the round that wrote the carry did not catch it —
and the regression fails on the old code, checked by putting the old code back.

### 3. What was held, and the warning that came out of holding it

**Three renderings, one reading.** `bug854315.pdf`, `issue16553.pdf` and `issue17069.pdf` differ
under mupdf after `merge` and not under poppler. The cause is ADR 0821 section 2: §12.8.1's `/V`
is dropped from a signature field crossing into a merged document, because the signature is over
bytes the new file does not have — and mupdf draws an unsigned signature widget as a grey
placeholder over its `/Rect` instead of from its `/AP` `/N`. Confirmed by reading the merged
field, which has `/AP` and `/T` and no `/V`, and by the differing region being exactly that
`/Rect`. A decision of this suite showing up in a second reader is what the gate is for.

**Two structure shapes, one reading, and something owed.** `bug1365930.pdf` and
`paragraph_and_link.pdf` state parent-tree entries naming structure elements their **own**
hierarchy does not reach — in the first, `/StructTreeRoot` `/K` names one childless `/Document`
while an `/Article` → `/Story` → paragraph subtree hangs off object 20, which states no `/P` at
all; in the second, index 4 names an `/Artifact` element no element's `/K` names. §14.7.2 makes
the hierarchy what `/K` reaches and Table 355 makes `/P` required, so those elements are in the
file and not in the tree. This suite carries the hierarchy and writes null there — which is
right, and was **silent**. `Carry::report` now counts the members a source filled and the output
could not, and says so with both clause numbers. A carried tree that quietly loses a
marked-content sequence's structure is the failure this gate exists to prevent, whoever caused it.

### 4. The gate failed once, on a wall clock, and the fix is a category

It passed alone and failed inside the §2 sequence twenty minutes later, on
`issue19517.pdf`: poppler drew the source and was killed on ours. Measured quiet, that document
costs poppler **23.9 s** and mupdf **17.6 s** — on the source and on the derived file alike,
within a second of each other — against a 20 s budget. So the assertion was on which side of the
bound the machine landed on, which is exactly what `doc/todo/02` §2 warns about a gate that spawns
another program.

A reader that outruns the budget has said nothing about the file, so the document now leaves
*that reader's* comparison and is counted under its own heading — the treatment a timed-out
**source** render already got, made symmetric. Classified by elapsed time rather than by the
error's text, because `pdfref` reports a killed renderer as an ordinary failure and matching on the
message would be an assertion on a substring (trap 27).

### 5. What was looked at

`mutool show` on a source and its piece, entry by entry, which is where the defect appeared;
the differing PPM pixels of `bug854315.pdf`'s two mupdf renderings, printed with their bounding
box, which is what turned "mupdf disagrees" into "a rectangle exactly at the signature field's
`/Rect`"; the source and merged `/AcroForm` and field dictionaries side by side; `mutool show`
walked up three documents' structure hierarchies to find the element that states no `/P`; and
§14.7.5.4, §14.7.2, §7.3.10 and Table 355 read in `doc/md/` rather than cited — §7.3.10's last
sentence being the one the whole defect turns on and the one no summary of that clause carries.

### 6. What the readers said

`qpdf --check` never lost a sound file, on any of the four writers, over the whole sample. No
foreign reader that drew a source page failed to draw ours. The figures are in the round's report
and not here.

### 7. Gates

The merge is a round of its own, so the whole `doc/todo/02` §2 sequence ran on the merged `main`
before this round's work and again after it, the walking lines under `tools/bounded.sh`, one walk
on the machine at a time. The sequence has a new line — `--test foreign_corpus` — and
`tools/state.sh` a new `run`, gated so that a machine without poppler, mupdf or qpdf prints a
skip under the section's own prefix rather than reporting that it said nothing.

### 8. What the next transform round does first

`doc/todo/57`'s order is now `optimize` and §7.5.7's producer half, `split --at-bookmarks`, the
aligned rotated comparison, a per-input password for `merge`, the RFC 0003 hand-off and the
confinement tranche. The foreign readback is no longer on that list; what it still owes is in
that file's §5 — the whole corpus rather than a sample, pages past the first, an encrypted
document, and the outline, name trees and form, which no installed tool prints.
