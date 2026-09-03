# 0835 — What a corpus walk can ask a carried structure tree

Session 897. Status: **accepted**. The thirteenth decision record of RFC 0002's implementation, and
the instrument beside ADR 0834's carry.

## Context

RFC 0002 §9's four layers of correctness are about *appearance*: bytes, structure, raster, and the
foreign readback. §14.7's logical structure is invisible on the page — that is what §14.7.1 means
by "stored separately from its visible content" — so **not one of the four layers can see whether
the carry is right**. A merge that dropped every structure element would draw bit-identically and
pass every assertion the three walks had before this round.

ADR 0834's carry is therefore the first thing this suite writes that its existing instruments are
blind to, and the walks needed a fifth question. The one thing they must not ask is whether the
output matches what `structure.rs` intended: that is the code measuring itself, and the corpus
would agree with any consistent mistake.

## Decision

### 1. The walk asks the output whether it is a conforming tagged document

`crates/pdf-transform/tests/support/mod.rs::check_structure` reads a derived document with
`pdf_syntax` and `pdf_model::Pages` and checks four properties, each a clause's rather than the
writer's. The module it lives in is the one whose own comment says its walks are "written
independently of the crate, so that a gate's expected value is derived from the document rather
than read off what the tool printed" — trap 8, and the reason the check lives there rather than
in `structure.rs`'s own tests.

1. **Every page stating Table 359's `/StructParents` has an entry in the parent tree, and it is an
   array.** §14.7.5.4: "[f]or a content stream containing marked-content sequences that are content
   items, the value shall be an array of indirect references to the sequences' parent structure
   elements." A key with no entry, or one whose value is not an array, is a fault named with the
   page.
2. **Every object stating Table 359's `/StructParent` has one, and it is a reference.** The clause's
   other bullet: "[f]or an object identified as a content item by means of an object reference …
   the value shall be an indirect reference to the parent structure element." Asked of every object
   of the file, so an annotation and a stream are asked the same question.
3. **Every structure element's `/Pg` names a page this document holds.** Table 355 makes it "[a]
   page object representing a page on which some or all of the content items designated by the K
   entry shall be rendered", and this is the property that separates a *pruned* tree from ADR 0831
   §2's half-carried one — the failure that record was written to prevent, asserted rather than
   argued. An element is recognised by Table 355's two required entries, `/S` and `/P`, rather than
   by `/Type`, which the table makes optional.
4. **`/ParentTreeNextKey` is greater than every key in use.** §14.7.5.4: it "shall hold an integer
   value greater than any that is currently in use as a key in the structural parent tree."

Table 354's `/Type` is checked too — "shall be StructTreeRoot" — because a root that says nothing
about itself is one a reader cannot recognise.

### 2. All three walks ask it, and the counts are printed rather than ratcheted

`split_corpus`, `merge_corpus` and `pages_corpus` each report how many outputs carried a tree, how
many elements they hold and how many parent-tree keys resolved, and **fail on any fault**. The
counts are printed and not ratcheted, on `doc/todo/05`'s standing rule: they are a function of the
corpus and of what each walk asks of it, and a walk whose plan changed would move them for a reason
that is not a regression.

What the three ask is deliberately different, and together they cover the carry's three shapes:

- **`split_corpus`** takes page 1 out of one document, so it exercises *pruning*: on a tagged
  document with several pages most of the hierarchy is dropped and what survives has to be a
  conforming tree on its own.
- **`merge_corpus`** merges each document's page 1 with a fixed second document that is **itself
  tagged** — `doc/PDF20_AN001-BPC.pdf`'s first page, with a `/RoleMap` of 23 keys and a `/ClassMap`
  of 8 — so it exercises *reconciliation* on every corpus document, and the role-map and class-map
  paths of ADR 0834 §4 are walked rather than only unit-tested.
- **`pages_corpus`** deletes a page from one document, so it exercises the carry beside §7.7.3.3's
  rotation and §12.4.2's labels — the case where a reader is least likely to expect the tagging to
  change at all.

### 3. What the corpus cannot exercise is a fixture, and the fixture says why it is one

The census (ADR 0834) found no corpus document with two structure trees stating one `/ID`, no
document pair the walk merges with a class defined two ways, and **no stream anywhere stating
`/StructParent`**. A corpus finds what documents contain, not what the specification says (trap 8),
so four fixture tests in `tests/merge.rs` carry those paths, over a built one-page tagged document
whose doc comment records the same reason `form_document`'s does — the collision cannot be made to
happen out of the tree's own files.

They are: the `/ID` refusal by name and its exit status, with a control merge of two documents whose
identifiers differ so that the refusal is about the collision rather than about tagging; the class
rename with both definitions surviving under distinct names and each source's element naming its
own; the role-map disagreement keeping the first source's mapping; and the marked-content
identifier assertion of ADR 0834 §2.

## Consequences

- The three walks' figures on the run that landed this: **90 trees carried and 0 faults** over
  `split`, **966 and 0** over `merge` — every output, because the fixed second document is tagged —
  and **90 and 0** over `pages`; 3 090, 15 654 and 3 161 elements, with 182, 2 114 and 219
  parent-tree keys resolving. No walk lost a bit-identical raster or a byte-identical content
  stream to the change.
- **A walk that reports zero carried trees is the signal to look at**, not a pass. It is what
  `split_corpus` printed while the first implementation was dropping every hierarchy behind an
  indirect `/K`, and only the *element count* would have said so — which is why the count is printed
  beside the fault list rather than only the faults.
- The foreign readback ADR 0334 priced and `doc/todo/57` §5 still owes is the instrument that would
  judge these trees from outside this tree. It has now inherited a fifth reason: a structure tree is
  the part of a derived document that only an assistive processor reads, so it is the part where a
  file only this tree can make sense of would be least likely to be noticed.
