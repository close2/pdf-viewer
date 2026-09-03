# 0818 — A piece is a page, its closure, and what the report says it lost

Session 886. Status: **accepted**. The eighth decision record of RFC 0002's implementation, on
the long-lived branch `round-867`, and the suite's first verb on ADR 0817's serializer.

## Context

RFC 0002 §14 names `split` as the first verb the serializer round lands, and §6.1 describes it:
one input, many PDF outputs, cut per page, per group of *n*, per comma-separated group of the
selection, or at the outline. Its architecture paragraph is one sentence long about the part that
is actually hard — "`pdf-transform`'s assembly walks each page's object closure … renumbers, and
hands the object list to `pdf-syntax`'s serializer" — and everything below is what that sentence
turned out to contain.

## Decision

### 1. A piece's page is the source's page, with two changes and no others

`/Parent` names the piece's own tree, which Table 30 requires ("shall be an indirect reference")
and which cannot be the source's node, since that node is not coming along. And §7.7.3.4's
inheritance is **flattened**: the clause says "[i]f such an attribute is omitted from a page
object, its value shall be inherited from an ancestor node in the page tree", the ancestors are
not in the piece, so an attribute one of them carried is written onto the page. Table 31 marks
exactly four entries inheritable and §7.7.3.3 closes the list — "[a]ttributes that are not
explicitly identified in the table as inheritable shall not be inherited" — so four is the number
and it is the standard's rather than a guess.

The value is taken **unresolved** from the nearest ancestor that states it, so a `/Resources` a
hundred pages shared through one indirect object is one object in the piece and not a hundred
copies. The walk up `/Parent` is bounded, because the entry is a reference and a file can point
it in a circle.

Everything else in the page dictionary is the producer's and is not read for meaning.

### 2. The closure stops at pages, and nowhere else

From each emitted page, every reference is followed and its object copied, transitively. It stops
at one thing: an object that is a **page** or a **page-tree node**. Without that stop a split
would copy the whole document into every piece, because `/Parent` reaches the tree and the tree
reaches every page. A reference to a page *of this piece* maps to that page's new number, which is
why `Assembly::replace` exists and why every page is numbered before any is built — an annotation
on page 3 pointing at page 4 of the same piece has to find it. A reference to any other page
becomes §7.3.10's `null`, is counted, and is reported: RFC §6.1 requires that destinations out of
the piece are "dropped with a warning (exit 3), not silently".

**What this deliberately does not do is prune.** A widget's field `/Parent` reaches the AcroForm
field tree, which reaches widgets on pages the piece does not have, whose appearance streams are
therefore copied and referenced by nothing. That is over-copying, it is stated here rather than
discovered later, and the answer to it is `optimize`'s reachability pass (RFC §6.5) rather than a
policy invented inside `split`.

### 3. The catalog carries what changes the marks, and names everything else in a warning

Carried: `/Version`, `/Lang`, `/ViewerPreferences`, `/PageLayout`, `/PageMode`, §8.11's
`/OCProperties`, §12.7.3's `/AcroForm` and §14.11.5's `/OutputIntents`. Not carried, and every one
of them named in the report where the source states it: `/Outlines`, `/Names`, `/Dests`,
`/PageLabels`, `/StructTreeRoot`, `/MarkInfo`, `/Metadata`, `/Threads`, `/SpiderInfo`,
`/Collection`, `/Perms`, `/Legal`, `/Requirements`, `/DPartRoot`. `/Info` crosses whole, because it
is one object with no page in it.

**The list of what is left behind is written out rather than derived from what is left over**, and
that is the decision: an entry nobody thought about would otherwise be an entry nobody is told
about. RFC §6.1's document-level carrying — the outline subset whose destinations survive, page
labels recomputed per piece, name-tree entries still referenced — is real work with edges of its
own, and this verb does not do it. What is not acceptable is a verb that claims a fidelity it has
not got, so `doc/todo/57` carries the work and the report carries the admission.

**`/OutputIntents` is in the first list on evidence rather than on foresight**, and that is the
second defect the corpus found this session. It was in the *second* list when the walk first ran,
and `issue17671.pdf` and `issue20513.pdf` — the only two of the corpus's 974 that state one on
page one — drew differently because of it: §14.11.5's intent is what
`pdf_model::content::colour::output_intent_space` reads to decide what a device colour means, so
dropping it changes the pixels. The clause makes carrying it right as well as observed: a piece's
pages mark with the colours the source's pages marked with. (The first defect was
`pdf_syntax::write::real`'s precision and is ADR 0817's.)

### 4. Table 22's bit 11, consumed for the first time

`Operation::Assemble` is new in `pdf_model::restriction`, and `Bit::Assemble`'s doc comment had
said "**[n]othing consumes it**: this program inserts, rotates and deletes no page, and
`doc/todo/57`'s `split`, `merge` and `pages` are the verbs that will." They do now. The bit's own
words name the operation — "[a]ssemble the document (insert, rotate, or delete pages and create
document outline items or thumbnail images)" — and a split writes a file made of pages the source
stated.

Two readings the arm needed:

- **At revision 2 the operation falls back to bit 4.** Bit 11 exists only "( Security handlers of
  revision 3 or greater )"; at revision 2 its position is inside the range Table 22 reserves and
  requires to be 1, so consulting it there would permit assembly on every conforming revision-2
  document. Bit 4's carve-out for bit 11 is exactly what does not exist at that revision, so bit 4
  is what binds. The same construction `Operation::FillInForm` already uses for bit 9.
- **§12.8.2.2's certification permits it at every level**, like printing and extracting and unlike
  attaching. Every sentence of Table 257 is about a change that "shall invalidate the signature" —
  the signature *of this document* — and a split leaves the signed bytes where they were and
  writes a different file beside them. Table 22's bit 11 is where a document says it does not want
  its pages taken apart, and that bit is read.

### 5. The gates are RFC §9's layers, and the corpus is the population

`tests/split.rs` holds the committed documents to each property one at a time, `qpdf --check`
included as foreign evidence in principle 5's register. `tests/split_corpus.rs` is the walk, in
`writer_corpus.rs`'s pattern and in `doc/todo/02` §2's sequence: every corpus document's first
page split out, the piece re-read as one page, its `/Contents` compared **encoded** with the
source page's — a comparison of decoded content would pass on a piece whose streams had been
re-encoded, which §11.1 does not permit — and page 1 of both drawn by the same backend at the
same scale and required to be **bit-identical**. Determinism is asserted per document beside them.

A refusal is not a failure: a document with no page, or one whose page is not an indirect object,
is the document's and is counted by reason. A difference nobody has diagnosed fails the run, and
`HELD` is empty.

## Consequences

- `pdf-transform` gains `split`, `Plan::Split`, `Origin::Piece` and `Selection::groups` — the last
  because RFC §6.1's `--pages 1-3,7-end` writes two files, so the grammar's own commas are piece
  boundaries and a verb that only saw the flattened list could not find them. `resolve` is now
  `groups` flattened, which is how the two cannot disagree.
- Pieces are written across rayon, because they are independent (RFC §12), and the report is
  assembled in piece order whatever order the threads finished in.
- `merge` and `pages` are next and are the same machinery: cross-file renumbering is trivial once
  renumbering exists at all, and the document-level reconciliations are the long tail RFC §6.2
  describes. `--at-bookmarks` is the one mode of `split` this round did not take; it wants
  `pdf_model::retrieval::sections`, which exists, and an outline subset, which does not.
