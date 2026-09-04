# 0862 — What a piece still says about its pages, and the three clauses that decide it

Session 910. Status: **accepted**. The seventeenth decision record of RFC 0002's implementation,
on the long-lived branch `round-867`. ADR 0863 is the mode of `split` this one made possible.

## Context

ADR 0818 §3 wrote out, by name, what a piece of a document does **not** carry, and said why the
list was written rather than derived: "an entry nobody thought about would otherwise be an entry
nobody is told about". Four of the fourteen entries on it were not omissions of principle but debts
— `/Outlines`, `/Names`, `/Dests` and `/PageLabels` — and RFC 0002 §6.1 had named all four in one
sentence: "the outline subset whose destinations survive, §12.4.2 page labels recomputed so every
piece starts with the label its pages had in the source, name-tree entries that are still
referenced". `doc/todo/57` §1 carried them, and pointed at `merge`'s constructions as the place to
start, because a merge and a split are the same question asked in opposite directions.

The question is one question: **what does a document say about its pages that a piece of it still
says?** ISO 32000-2 answers it three different ways, and the three verdicts are what this record
holds.

## Decision

### 1. §12.3.3's outline: **permitted**, and the shape is what binds

"A PDF document **may** contain a document outline that the interactive PDF processor may display
on the screen". So a piece with no outline conforms, and nothing in the standard requires a
derivative document to keep its source's — the only reason to carry one is that a chapter file
whose table of contents is gone is not what anybody asked for.

What binds is what a carried outline has to *be*. Table 150 makes `/First` and `/Last` "( Required
if there are any open or closed outline entries; shall be an indirect reference )". Table 151 makes
`/Parent` "( Required )" with "[t]he parent of a top-level item shall be the outline dictionary
itself", `/Prev` "( Required for all but the first item at each level )", `/Next` the same at the
other end, and `/Count` "( Required if the item has any descendants )". Every one of those is about
*this* hierarchy, so every one of them is rebuilt:

- **An item is kept when its own destination lands in the piece, or when a descendant's does.**
  The second half is Table 151's `/Parent`: dropping an ancestor would leave a kept item naming an
  object the piece does not hold. An item that resolves nowhere — §12.3.3 permits one whose `/A`
  runs a script or opens another file — is kept only as such an ancestor, which is the rule
  `pdf_model::retrieval::sections` already applies for the same reason.
- **`/Count` is recomputed by §12.3.3's own three steps** over the kept subtree, with its sign from
  the source item's. Carrying the number would be carrying a count of something else. Table 150's
  `/Count` follows the clause's *two* sentences rather than one: the total of visible items where
  any kept item is open, and **absent** where none is — "[t]his entry shall be omitted if there are
  no open outline items."
- **An item kept as an ancestor whose own destination misses loses that destination.** Table 151
  makes `/Dest` and `/A` alike optional and §12.3.3 describes activation as a jump "to a destination
  or trigger an action", so an item with neither is a heading, which the clause admits. This is
  stronger than §7.3.10's null, and the reason is in §3 below.

### 2. §12.4.2's labels: **permitted, and the source's own are forbidden**

"[A] document may optionally define page labels ( PDF 1.3 )" — permitted. But **a label is a
position**, and three sentences of the clause say so: a page index is "the page's relative position
within the document"; the `/PageLabels` number tree's keys are each "the page index of the first
page in a labelling range"; and "[t]he tree shall include a value for page index 0."

A piece is a document. Its indices run from 0. So a piece that begins at the source's page 13 and
carried the source's tree unchanged would state a tree whose lowest key is 12 — a tree with no value
for page index 0, which that `shall` forbids outright. Carrying is therefore not a choice between
fidelity and effort; the unchanged form is non-conforming.

What is written instead is one entry per page of the piece, each stating the label that page carried
in the source, which the clause makes exact: "[t]here is no default numbering style; if no S entry
is present, page labels shall consist solely of a label prefix with no numeric portion", so
`<< /P (the label) >>` is that label and nothing else. **This is `merge`'s construction and it is
`merge`'s code** — `merge::page_labels` now takes the labels and the order rather than a `Merge`,
because a merged document and a piece have the same problem seen from two sides, and two
implementations of one clause are how they come to disagree.

### 3. §12.3.2.4's named destinations: carried by whatever still names them, and §7.3.10 cannot help

This is the one where the asymmetry with everything else `split` does is the whole argument.

Every reference a piece does not hold becomes §7.3.10's null, which is the standard's own answer:
"[a]n indirect reference to an undefined object … shall be treated as a reference to the null
object". A destination "referred to indirectly by means of a name object ( PDF 1.1 ) or a byte
string ( PDF 1.2 )" is **not an indirect reference**, and §12.3.2.4 says where its meaning comes
from instead: the correspondence "shall be defined by the Dests entry in the document catalog
dictionary", or, in PDF 1.2 and later, by the `/Dests` name tree of the name dictionary. A piece
that kept a link and dropped both tables would state a destination the standard gives no meaning to
at all — not a null, not an error, nothing.

So the entries are subsetted, by two tests, because a name tree is used two ways:

- **`/Dests` is entered by name**, so an entry survives when the destination it holds resolves to a
  page the piece holds. The resolution is `Destination::read` — the *reader's*, so the tree is asked
  here exactly as it is asked when a link is followed, and the outline's keep test and this one
  cannot disagree.
- **Every other Table 33 category maps a name to an object**, and the piece reaches those objects
  through its pages rather than by name: §12.5.6.15's file attachment annotation names its own file
  specification. An entry survives where the closure walk already copied what it names. A tree
  listing objects the piece does not hold would be an index of nothing.

A key outside Table 33's ten is counted rather than passed through, because this program has no rule
for subsetting one. No key can collide: a piece has one source, so §7.9.6's "[t]he keys contained
within the various nodes' Names entries shall not overlap" is `merge`'s problem and not this one.

### 4. Where the code went, and what the report says

All of it is `split::carry_navigation`, run **after** the closure walk and after §14.7's carry,
because two of the three answers are *what the walk copied*. `NOT_CARRIED` is eight entries now
instead of twelve, and the three counts a piece can lose — name-tree entries dropped, an outline
that reaches no page of this piece, a numbering spent — are each a warning naming its clause.

## Consequences

- **The corpus walk gained four clause-derived properties and found none violated on its first
  run.** `support::check_navigation` asks the *output*, in `check_structure`'s discipline (trap 8):
  every outline item resolves to a page this document holds; Table 150's `/First`/`/Last` are
  present where there are entries and Table 151's `/Prev`/`/Next`/`/Parent` are right at every
  position; §12.4.2's tree has a value for index 0, no key twice, and no key past the last page;
  and every §12.3.2.4 entry, in either home, resolves to a page. Over the 974: 147 pieces carry an
  outline, 22 carry labels, 68 destinations are carried, **0 faults**, and page 1's label is
  unchanged in every piece that has one.
- **`merge` has one caller fewer and one function shared.** Nothing about its output changed;
  `tests/merge.rs::every_page_keeps_the_label_it_had_in_its_own_document` is the gate that says so.
- The piece's `/Names` is one root node holding one `/Names` array, which §7.9.6 permits — "[i]f
  the root node has a Names entry, it shall be the only node in the tree" — with the keys in
  `<[u8] as Ord>` order, which is what that clause's two sorting sentences define.
- What is still not carried is `/Metadata` and the seven document-level constructs beside it, each
  named in the report where the source states one. `doc/todo/57` holds them.
