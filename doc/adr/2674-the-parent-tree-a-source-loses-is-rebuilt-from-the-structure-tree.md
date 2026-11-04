# 2674 — The parent tree a source loses is rebuilt from the structure tree

Session 1925, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/parent_tree.rs`: `Rebuilt`, the refusals
`STRUCTURE_TOO_LARGE`, `ITEM_AND_CONTAINER`, `CLAIMED_INLINE_ONLY`; `Rewrite::ParentTreeRebuilt`,
answering `logical-structure/content-items-find-their-elements-through-the-parent-tree`).
Context: ADR 2673 (the row, and this remedy decided), ADR 2672 (the shared number-tree extender),
ADR 1163 (the appended page described in the structure tree), ADR 2669 (the census command).
Clauses: ISO 32000-2 §14.7.5.4, Table 359, Table 354, §7.9.7, §7.3.10.

## The premise, checked

`mutool show` on the four documents: `issue15340`'s root states no `/ParentTree`; `issue20516`'s
**does** — `/ParentTree 11 0 R` — but object 11 is `null`, which §7.3.10 makes the same as no entry
(the brief said "no `/ParentTree`"; the reader the row uses resolves it so, and `tagged.rs` had read
it as a tree that is not a dictionary); `pdfjs_wikipedia`'s `Link` annotations are claimed by `OBJR`
and state no `/StructParent`, its page's key right; `prefilled_f1040`'s widgets likewise, under a
parent tree of `/Kids`.

## Decision

1. **The structure tree is the authority.** §14.7.5.4 provides the parent tree so that content items
   can "refer directly back to their parent structure elements": it is derived from the elements'
   claims, and an entry naming an element that claims nothing is one that derivation cannot give.
2. **Only what the row finds lost changes.** A holder whose key leads to a claimant for every item
   it holds is left as written. A lost holder — no key, a key the tree holds no entry under, a value
   of the wrong shape, an entry naming no claimant, or Table 359's other key — takes a **new** key
   from `/ParentTreeNextKey`, moved past every `/StructParents` or `/StructParent` that any page, annotation
   or `XObject` states, claimed or not — an unclaimed object stating a key handed out would find an
   element that never claimed it — and a new entry; the
   wrong key of the other kind is taken out, as Table 359's "at most one" requires. A sequences entry
   starts from the producer's array, keeping slots that are right and slots no element claims.
   A new key rather than an edit of the producer's entry, so the entries extend the tree after its
   greatest key through ADR 2672's extender at any depth, and no producer node or `/Limits` changes.
3. **The entry names the first claimant in `/K` order**; the row accepts any claimant.
4. **One extension of the tree.** Where pages are appended, the rebuilt entries ride ahead of the
   pages' in their extension (`Planned::rebuilding`); else in their own (`Rebuilt::alone`). A rebuild
   alone leaves the root's `/K` as written.
5. **Recorded**: the report counts entries, and `xmpMM:History` says how many were written — a
   condition of the rewrite, withdrawn where the packet cannot take it.
6. **Refused by name**: a structure tree past 64 levels or 2^20 visits; an object claimed whole and
   holding claimed sequences (Table 359: "not both"); an item only elements inline in their parent's
   `/K` claim, which have no reference an entry could hold; a document past 2^20 pages,
   annotations and `XObject`s, whose stated keys the walk could not all read. The tree's own unreadable shapes keep
   ADR 2672's refusals.

Structure only: no content stream, no element, no mark.

## What moves

ADR 2669's census command against round 1924's after-run: the row refuses nothing. `issue15340`,
`issue20516` and `pdfjs_wikipedia` convert under `state-every-loss` and `no-data-loss` at 2b and
part 4 (3 more written in each of the four), and the 12 outputs pass veraPDF 1.30.3;
`prefilled_f1040` stays refused by the rows the parent tree's refusal had stood beside, the
census now listing it under `signatures/signature-assertion` at 2b too. No corpora count moves, nothing is newly refused, and the 4318
outputs both runs wrote are byte-identical once dates, `/ID` and UUIDs are masked.
