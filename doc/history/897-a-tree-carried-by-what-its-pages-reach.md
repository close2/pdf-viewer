# 897 — A tree carried by what its pages reach

2026-09-03. Argued in [ADR 0834](../adr/0834-a-structure-tree-is-carried-by-what-its-pages-reach-and-three-namespaces-get-three-answers.md)
and [ADR 0835](../adr/0835-what-a-corpus-walk-can-ask-a-carried-structure-tree.md). The ninth
implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the long-lived branch
`round-867`. `main` had moved — sessions 890 and 891 — so it was merged in first; nothing it
carried touched `pdf-transform` and the merge was clean.

The round's subject is the debt sessions 888, 891 and 893 each named as this suite's largest:
§14.7's logical structure, which no verb carried. It is carried now, by `split`, `merge` and
`pages` alike, through one module behind a trait.

Touched: **`crates/pdf-transform/src/structure.rs`** (new, the whole of §14.7's carry),
`src/merge.rs` (`Host`, the blocked set, `/StructParent` restated, `/StructParents` on the page,
the catalog's two entries, `NOT_CARRIED` shorter by two, `plan_structure` and `page_tree` split
out), `src/split.rs` (the same, plus `Piece` gaining a rebuild queue it never had),
`src/lib.rs` (`Refusal::StructureConflict`, the module); **`crates/pdf-model/examples/structure_tree_census.rs`**
(new, the population); `crates/pdf-transform/tests/support/mod.rs` (`check_structure`), `tests/merge.rs`
(four §14.7 tests and a built tagged document), `tests/pages.rs` (two tests rewritten from the
old debt to the new carry), `tests/split.rs`, `tests/split_corpus.rs`, `tests/merge_corpus.rs`,
`tests/pages_corpus.rs`; `doc/conformance/ledger.toml` (five rows), `doc/todo/57-…`; ADRs 0834
and 0835, this file.

## 1. The population came first, and it decided the shape

`structure_tree_census.rs` over the 964 corpus documents this tree opens, with `pdf_syntax` alone
rather than through `pdf_model::structure` — trap 8, because a census whose predicate is the code
under test measures the code. ADR 0834 has the table. Three of its numbers changed what was built:

- **All 90 tagged documents state `/StructParents` on page 1.** `split` and `merge` carry page 1,
  so the construct is reached on every tagged document rather than on a handful, and the corpus
  walks became a real instrument rather than a spot check.
- **No stream anywhere states `/StructParent`.** Table 359's third home — "the stream dictionary of
  a form or image XObject" — has no corpus witness, so its correctness rests on a fixture. It was
  built anyway, and cost almost nothing: `pdf_syntax::serialize` could already place a synthesised
  `Object::Stream` and writes it with the source's `Arc` and a re-derived `/Length`.
- **12 of 124 role-map keys are mapped two ways across the tagged documents** — `/Title` to `/H1`
  and to `/P`, `/Artifact` four ways. That number is why §3 below happened.

## 2. What is carried, and the one claim that had to be checked rather than assumed

An element is kept when its own content is on a page the output holds, or when a descendant is
kept. What is dropped is *blocked* from the closure walk, so a `/Ref` naming a dropped element
cannot drag its subtree back in. Table 354's root is written whole — the parent tree with the
output's own keys, `/ParentTreeNextKey` restated, the role and class maps merged, an `/IDTree` over
the kept elements, three list-valued entries concatenated — and Table 353's `/MarkInfo` beside it,
with `/Marked` a **conjunction** because a document some of whose pages came from an untagged
source does not conform to tagged PDF conventions.

**The marked-content identifiers are not rewritten, and the round was asked to check that rather
than assert it.** It holds, and the reason is worth more than the answer: §14.7.5.2 makes an
`/MCID` unique "within its content stream" and §14.7.5.4 makes it "a zero-based index into the
array" the stream's key names, so an identifier means nothing outside the one stream and the one
array. Carrying each page's parent-tree array **at its own length and in its own order** while
renumbering only the key moves both ends of the index together. `tests/merge.rs`'s
`the_marked_content_identifiers_are_not_rewritten_and_the_array_still_indexes_them` asserts both
ends — the stream still holds its `/MCID`, the array is still one long, and index 0 still names an
element whose `/Pg` is that page.

## 3. The first implementation refused too much, and the clauses said so

It treated all three colliding namespaces the way ADR 0821 §2 treats §12.7.4.2's field names:
refuse by name. The merge fixture tests failed on their first run — `/RoleMap /Title`,
`/ClassMap /Pa5` and six more — and the census explains why that is a defect rather than a strict
gate: refusing would block a merge of two ordinary tagged documents.

The clauses distinguish the three, and the distinction is not about frequency:

- **§14.7.3's role map is an approximation and says so.** NOTE 1: "[t]he equivalence need not be
  exact; the role map merely indicates an approximate analogy between types". Two documents
  approximating one name differently are not contradicting each other. First source wins, warned by
  name. §12.7.4.2's collision is a different thing — two fields with one fully qualified name *are
  one field*, and the clause requires them to agree.
- **§14.7.6.2's class is not an approximation** — the attributes are "considered to be attached to
  the given structure element" — but the clause **closes the set of referrers**: "[s]tructure
  elements shall refer to the class by name", and `/C` "shall contain a class name or an array of
  class names". So it is renamed and every carried element's `/C` follows, which is ADR 0821 §3's
  `/Dests` rule applied to a second namespace.
- **Table 355's `/ID` is refused**, because the same rule forbids the rename: §14.8.5's `/Headers`
  is "an array of byte strings, where each byte string shall be the element identifier", Annex E
  permits more, and this program does not know what else names one.

One refusal, from three readings, instead of three from one.

## 4. ADR 0831 §2 is superseded on its own terms, which is the shape worth keeping

That record kept a carried page's producer-written `/StructParents` because, with no tree in the
output, the integer named **nothing** rather than the wrong element — and half a tree would have
been worse. Now that the output states a tree, the same argument runs the other way: a page or an
annotation out of an *untagged* source that kept its key would name an element of the *other*
source's renumbered tree. So:

> Where the output states a structure tree, every §14.7.5.4 key in it is the output's own or
> absent. Where it states none, both are left as the producer wrote them.

The second half is ADR 0831's choice, unchanged, for the case its argument was actually about. A
decision superseded by its own reasoning reaching a new case is not a decision that was wrong.

## 5. The defect the writing found, which the reading could not

The first build reported `0 element(s) written, 1 dropped` on every tagged document, including one
with 82 elements. Table 354's and Table 355's `/K` may be a **single indirect child** rather than an
array, and the walk resolved the entry before asking which element the child was — which throws the
identity away, so every top-level child looked like a direct dictionary and was dropped as one
Table 355's `/P` could not name.

`pdf_model::structure` reads `/K` correctly and always has. This is a writer's mistake a reader
cannot make: a reader wants the child, a writer wants the child's *identity*. It is in the §14.7.2
ledger row for that reason.

**And the tell was a count, not a failure.** Every assertion the three walks had before this round
passed while the carry wrote nothing at all, because §14.7 is invisible on the page — RFC §9's four
layers are about appearance and not one of them can see a structure tree. That is ADR 0835's whole
subject, and it is why the walks print the element count beside the fault list: a walk reporting
zero carried trees is the signal, and zero faults is not a pass on its own.

## 6. One module, three verbs

`split` walks one document and `merge` — and `pages`, on merge's engine since session 893 — walks
several; neither's bookkeeping is §14.7's business, and a second copy of this reading would be a
second place to get the parent tree wrong. `structure::Host` is the six questions both can answer,
and §14.7 is read and written once behind it. `Piece` gained a rebuild queue it never had, because
`split` had only ever copied objects verbatim and an annotation's key has to be restated.

The ordering is stated in both callers and is the load-bearing part: the carry is planned **after**
every page has its number and **before** any page is built; the elements, the parent tree and the
root are built **after** the closure walk drains, because the object keys are assigned by it; and
the walk drains a second time, because an element's attributes reach objects nothing else did.

## 7. What the walks say

All three walked the corpus with the carry in place, and no walk lost a bit-identical raster or a
byte-identical content stream to the change. `split` carried 90 trees, `merge` 966 — every output,
because the fixed second document is itself tagged, which is what makes that walk exercise the
reconciliations rather than only the pruning — and `pages` 90. Zero faults in all three, against
the four clause-derived properties ADR 0835 lists.

## 8. What is left

`doc/todo/57` after this round: **`optimize`** and §7.5.7's producer half, `split --at-bookmarks`,
the aligned rotated comparison, a per-input password for `merge`, the RFC 0003 hand-off, the
confinement tranche, and the corpus-wide **foreign readback** — which has inherited a fifth reason
from this round, and the sharpest one: a structure tree is the part of a derived document that only
an assistive processor reads, so a tree only this tree can make sense of is the one a raster gate is
least placed to notice.

Two small things §14.7 itself still owes, both named in ADR 0834: `/Namespaces` is concatenated
without being interpreted, so two sources using one namespace name state it twice; and §14.7.4's
`/RoleMapNS` — which would let each source keep its own role map under its own namespace, and would
dissolve §3's first two collisions — is not taken, because a namespace name "should take the form of
a uniform resource identifier" and this program has no basis for inventing one per source.
