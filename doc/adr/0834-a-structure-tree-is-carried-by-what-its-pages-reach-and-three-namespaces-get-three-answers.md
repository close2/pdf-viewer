# 0834 — A structure tree is carried by what its pages reach, and three namespaces get three answers

Session 897. Status: **accepted**. The twelfth decision record of RFC 0002's implementation, and
the one that pays down the debt sessions 888, 891 and 893 all named as the suite's largest.

## Context

ADR 0831 §2 recorded, honestly and deliberately, that no verb of this suite carried §14.7's
logical structure: the catalog's `/StructTreeRoot` was in `split`'s and `merge`'s not-carried
lists, and every carried page kept the `/StructParents` integer its producer wrote, which then
named nothing. Its argument for leaving the debt whole rather than paying it a fragment at a time
was that **half a structure tree is worse than none** — a key into a *partial* tree would name
another page's structure element and tell an assistive reader something false with no way to see
it.

That argument is about a *half* carry. It is not an argument against a whole one, and this round
is the whole one.

### The population, measured before the change

`crates/pdf-model/examples/structure_tree_census.rs` over the 964 corpus documents this tree
opens, with `pdf_syntax` alone rather than through `pdf_model::structure` (trap 8):

| | |
|---|---|
| documents stating `/StructTreeRoot` | 90 |
| of those, whose page 1 states `/StructParents` | 90 — all of them |
| documents stating `/MarkInfo << /Marked true >>` | 78 |
| pages stating `/StructParents` | 137 of 1760 |
| annotations stating `/StructParent` | 32 880 of 34 832 |
| **streams** stating `/StructParent` | **0** |
| structure elements reached from the roots | 37 000, of which 1 972 state `/Pg` |
| elements with a marked-content child and no `/Pg` anywhere | 379 |
| elements stating `/ID` | 632 |
| `/RoleMap` keys across the tagged documents | 124, of which **12 are mapped to two different names** |
| `/IDTree` keys | 568, of which 35 are stated by more than one document |

Three of those numbers decided the shape of the work. **Every tagged document's page 1 is keyed**,
so `split` and `merge` — which carry page 1 — reach the construct on every one of them rather than
on a handful. **No stream states the key**, so Table 359's third home has no corpus witness and its
correctness rests on a fixture rather than on the walk. And **twelve role-map keys collide across
the corpus**, so a merge of two ordinary tagged documents meets the collision — which is what
turned the first implementation's blanket refusal from a principle into a defect, below.

## Decision

### 1. What is carried: the elements the output's pages reach, and the ancestors that hold them

`crates/pdf-transform/src/structure.rs` reads every contributing document's `/StructTreeRoot` and
decides, depth-first and bottom-up, which elements survive. §14.7.2 makes `/K` "[t]he children of
this structure element", so an element is kept when a content item of its own is on a page the
output holds **or** when a descendant is kept — an ancestor of a kept child is what holds it in the
hierarchy. An element that reaches nothing is dropped, and is *blocked* from the closure walk so
that a `/Ref` or an attribute naming it cannot drag its subtree back in behind it.

A kept element is rebuilt rather than copied: `/K`, `/P` and `/Pg` come from the pruned hierarchy
and every other entry the producer wrote crosses. Its content items are filtered the same way —
§14.7.5.2's integer identifier against Table 355's `/Pg`, Table 357's `/MCR` and Table 358's
`/OBJR` against their own `/Pg`, which "overrides any Pg entry in the structure element containing
the marked-content reference".

Table 354's root is then written whole: `/Type`, the kept `/K`, `/ParentTree`,
`/ParentTreeNextKey`, the merged `/RoleMap` and `/ClassMap`, an `/IDTree` over the kept elements'
identifiers, and `/Namespaces`, `/PronunciationLexicon` and `/AF` concatenated across the sources
that state them — three lists, no namespace two sources can collide in, so the union is the whole
reconciliation. Table 353's `/MarkInfo` follows, with `/Marked` a **conjunction**: the row is "[a]
flag indicating whether the document conforms to tagged PDF conventions", and a document some of
whose pages came out of a source that stated no structure does not conform to them, whatever its
other sources said. `/UserProperties` and `/Suspects` state that something is *present*, so they
are unions.

### 2. §14.7.5.4's keys are the output's, and both ends of the index move together

The parent tree is rebuilt with keys this file assigns — in output page order first, then in the
order the closure walk reaches the objects, so the numbering is a function of the plan and RFC 0002
section 9's first layer holds. Every carried page's `/StructParents` and every carried annotation's
or XObject's `/StructParent` is restated to match.

**A page's value is carried at the source's own length and in its own order**, and that is the
whole reason the marked-content identifiers inside the content streams need not be rewritten. The
claim was checked rather than assumed. §14.7.5.2 makes an `/MCID` "an integer marked-content
identifier that uniquely identifies the marked-content sequence within its content stream", and
§14.7.5.4 says what the number is then for:

> The array element corresponding to each sequence shall be found by using the sequence's marked
> -content identifier as a zero-based index into the array.

So an identifier means nothing outside the stream that holds it and the one array that stream's key
names. RFC 0002 §11.1 makes every content stream cross byte for byte; renumbering the *key* and
leaving the *array* alone moves both ends of the index together, and no byte of any content stream
has to change. **The assumption holds**, and `merge.rs`'s
`the_marked_content_identifiers_are_not_rewritten_and_the_array_still_indexes_them` asserts it on
both ends: the stream still contains its `/MCID`, the array is still one element long, and index 0
still names an element whose `/Pg` is that page.

**An object whose key has nothing to point at loses the key**, rather than keeping one. If the
source's parent tree has no entry for it, or the entry names an element the output does not hold,
`/StructParent` is removed. This is ADR 0831 §2's own distinction applied in the other direction: a
key into a tree the output *does* state, naming nothing, tells an assistive processor that the
content has a parent element and then hands it none.

**Table 359's third home is carried by rebuilding a stream's dictionary.** The key may sit "in the
stream dictionary of a form or image XObject", and a stream is the one object in this writer that
crosses rebuilt while keeping its bytes: the dictionary is rewritten and the encoded data is the
same `Arc` the source holds, never decoded and never re-encoded. No corpus document states one, so
the witness is the code path and a fixture rather than a file.

### 3. ADR 0831 §2's dangling key is superseded on its own terms

That record kept the producer's `/StructParents` on a carried page, on the ground that with no tree
in the output the integer named nothing rather than the wrong element. Now that the output can
state a tree, **the same argument requires the opposite**: a page or an annotation out of an
*untagged* source that kept its producer's key would be a key into the *other* source's renumbered
tree, naming an element it has nothing to do with. That is exactly the harm ADR 0831 was protecting
against, arriving from the direction it did not have to consider.

So the rule is stated once and covers both cases:

> Where the output states a structure tree, every §14.7.5.4 key in it is the output's own or
> absent. Where it states none, both are left as the producer wrote them.

The second half is ADR 0831's choice, unchanged, for the case its argument was about.

### 4. Three namespaces two sources can collide in, and three different answers

The first implementation refused all three collisions, on §12.7.4.2's precedent (ADR 0821 §2), and
**that was wrong for two of them** — the merge fixtures failed on their first run, and the census
says why: 12 of 124 role-map keys collide across the corpus, so refusing would block a merge of two
ordinary tagged documents. The clauses distinguish the three, and the distinction is not about how
often they happen:

**`/RoleMap` — the first source's mapping wins, warned by name.** §14.7.3 makes the value "a single
name identifying the target structure element type", so two cannot be written; but its NOTE 1 says
what the value *is*:

> The equivalence need not be exact; the role map merely indicates an approximate analogy between
> types, allowing PDF processors to share nonstandard structure elements in a reasonable way.

An approximation is not an assertion two documents can contradict each other about, and a type left
mapped to the first source's nearest equivalent is still shared in a reasonable way — where
refusing leaves the reader with no structure at all. §12.7.4.2's collision is a different thing: two
fields with one fully qualified name **are one field**, and the clause says they "shall have the
same field type ( FT ), value ( V ), and default value ( DV )". Identity against approximation is
the line, and it is the clauses' rather than this project's.

**`/ClassMap` — the colliding class is renamed and every carried element's `/C` follows.** An
attribute class is not an approximation: §14.7.6.2 makes the attributes "considered to be attached
to the given structure element", so an element given another source's `/Pa5` would be laid out by
properties it never had. The rename is safe because the clause **closes the set of referrers** —
"[s]tructure elements shall refer to the class by name", and "[t]he C entry in a structure element
dictionary … shall contain a class name or an array of class names" — so rewriting `/C` is the
whole of it. That is ADR 0821 §3's rule for `/Dests` applied to a second namespace: chase the
references where the standard says what states them. The new name is `merge::free_key`'s, so a
merge renames the same way every time.

**`/IDTree` — refused by name.** Table 355 makes an element's `/ID` "unique among all elements in
the document's structure hierarchy" and the derived document is one hierarchy, so two elements
cannot both keep it. The same rule that permitted the class rename forbids this one: §14.8.5's
`/Headers` attribute is "an array of byte strings, where each byte string shall be the element
identifier", Annex E permits further attributes, and this program does not know what else names an
identifier. `Refusal::StructureConflict`, naming the clause and every colliding identifier, at RFC
0002 section 4.4's exit status 4 — the request is well formed and this program declines to write a
document a clause forbids. **A collision inside one source is not refused**, on ADR 0821 §2's own
distinction: the clause binds the document that already held both, and carrying what the producer
wrote is RFC 0002 §11.1's premise.

§14.7.4's namespaces are the construction that would resolve the first two without a choice —
Table 356's `/RoleMapNS` maps one namespace's types to another's, so each source could keep its own
mapping under its own namespace. **Not taken**: a namespace name "should take the form of a uniform
resource identifier" and this program has no basis for inventing one per source, which is ADR 0821
§4's reason for not synthesising an outline item's `/Title`.

### 5. One implementation, three verbs, through a `Host` trait

`split` builds one piece out of one document with its own walk state; `merge` — and `pages`, on
merge's engine since session 893 — builds one file out of several with its own. Neither's
bookkeeping is §14.7's business, and a second copy of this reading would be a second place to get
the parent tree wrong. `structure::Host` is the six questions both can answer — the document at a
position, a value carried into the output's numbering, a slot reserved, a slot that stands in for a
source object, a slot filled, and an object refused to the walk — and §14.7 is read and written
once behind it.

The ordering is what makes it work and is stated in both callers: the carry is planned **after**
every page has its number and **before** any page is built, because a page's `/StructParents` is the
carry's to state and every kept element needs its slot before the closure walk can reach one by
reference. The elements, the parent tree and the root are built **after** the walk drains, because
the object keys are assigned by it — and the walk is drained a second time, because an element's
attributes reach objects nothing else did.

### 6. What is refused or reported by name, and nothing is silent

- An element that reaches no page the output holds: **dropped and counted**, in a warning that says
  how many were written and how many were not.
- A content item naming a page the output does not hold: **dropped from its element's `/K`**, and
  counted separately.
- A marked-content item with no `/Pg` anywhere — Table 355 makes `/Pg` "required if K is an integer
  object" and the element states none: **dropped and counted**, with the clause. 379 elements of
  the corpus are in this class.
- A page the output holds **twice** (`pages --insert`): the second placement states no
  `/StructParents`, warned by name, because Table 355 gives a structure element one `/Pg` and its
  content items can name only one of the two page objects.
- A source stating `/ParentTreeNextKey`: **not carried**, warned, because §14.7.5.4 makes it a
  statement about the parent tree of the file that holds it.
- An element written **directly** into its parent rather than as an indirect object: dropped and
  counted, because Table 355 makes `/P` "( Required; shall be an indirect reference )" and such an
  element cannot be named by its own children.

## Consequences

- `crates/pdf-transform/src/structure.rs` is the module; `merge.rs` and `split.rs` implement
  `Host`. `merge`'s `NOT_CARRIED` loses `/StructTreeRoot` and `/MarkInfo`; `split`'s loses both too.
- **A defect the writing found that the reading had not**: Table 354's and Table 355's `/K` may be a
  single **indirect** child rather than an array, and resolving the entry before asking which
  element a child is throws the identity away. The first implementation did, and dropped the entire
  hierarchy of `doc/PDF20_AN001-BPC.pdf` — 82 elements — while reporting "0 element(s) written, 1
  dropped". `structure::children_of` answers the items unresolved. `pdf_model::structure` reads `/K`
  correctly; this is a *writer's* mistake that a reader cannot make, because a reader wants the
  child and a writer wants the child's identity.
- **`pdf_syntax::serialize` needed no change at all.** A synthesised slot could already hold an
  `Object::Stream`, and `serialize` writes one with the source's `Arc` and a re-derived `/Length` —
  which is what made Table 359's third home a few lines rather than a serializer change.
- `Refusal::StructureConflict` is the suite's tenth refusal and its second at exit status 4.
- The three corpus walks carry the check, and ADR 0835 is what they assert.

## Alternatives not taken

- **Writing no structure tree and warning, where a collision occurs.** It would discard *both*
  sources' tagging over one entry's disagreement, and §14.7.1's separation exists precisely so that
  a reader who cannot see the page still has the document's meaning. A `--structure=drop` flag is
  the shape that would offer it; nobody has asked.
- **Renaming a colliding `/ID`.** §4 above: the set of referrers is open.
- **Carrying the tree whole and pruning nothing.** An element whose `/Pg` names a page the output
  does not hold would be a reference the serializer writes as §7.3.10's null, and a reader walking
  the hierarchy would meet elements with no content and no way to know why.
