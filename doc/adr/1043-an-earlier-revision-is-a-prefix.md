# 1043 — An earlier revision is a prefix, so §12.8.2.2.2's comparison needs no mutation

Session 1026. Status: **accepted**. Adds `FileBytes::prefix` to `pdf-syntax` and
`pdf_signature::revision` to `pdf-signature`: the signed revision of a document, opened as a second
`Document` over the same bytes, and diffed against the current one object by object. Moves five
§12.8.2 ledger rows' notes. **Does not** rank a change against Table 257's `/P`, and refuses that
by name.

`§N` is ISO 32000-2 and nothing else.

## 1. The obstacle that was not one

§12.8.2.2.2's row has said, since session 545, that step two "compares the signed revision with the
current one object by object, which this reader does not reconstruct", and named the reason:
"`Document` holds one view of the file". `CLAUDE.md` makes that view immutable and makes
`interpret` a pure function of the bytes, which is the oracle's whole premise, so the debt looked
architectural.

It was not. The clause asks for a *second* document, never a mutated one, and §7.5.6 hands it over:

> When updating a PDF file incrementally, changes shall be appended to the end of the file, leaving
> its original contents intact.

with "[e]ach trailer shall be terminated by its own end-of-file (%%EOF) marker". **An earlier
revision is therefore a prefix of the file**, and opening a prefix needs nothing but a shorter
length. `FileBytes::prefix` is that: on disk a duplicated descriptor and not one byte read; in
memory one allocation, because neither `Arc<Vec<u8>>` nor `Arc<[u8]>` sub-slices without a copy.

Immutability turns out to be what makes the comparison *exact* rather than what forbade it: both
states are functions of the same bytes, and neither can move under the other.

## 2. Identity by placement, on the digest's warrant

Two states agree about an object when both cross-reference tables place it at the same offset — or
at the same index of the same object stream — **and** that offset is below the signed boundary.
The soundness is the clause's own sentence: once the byte range digest is validated the signed
portion "is known to correspond to the state of the document at the time of signing", so identical
offsets below the boundary name identical bytes. Reparsing both objects would answer the same
question at the cost of reading the file twice. One level of resolution is the whole of it, because
§7.5.7 excludes "[s]tream objects" from an object stream and an object stream is one.

`/Root` is compared separately: an update may point Table 15's entry at a catalog the signed
revision already held, which changes the document and moves no object.

## 3. What is refused, and why refusal is the design

A `/DocMDP` answer that says *permitted* without comparing is worse than one that refuses, because
the transform exists to be believed. So:

- **`NotComparable`**, one variant per way two states cannot be put beside each other — a range
  that is not a revision boundary, a hole holding more than the signature value, a table recovered
  by scanning, a prefix that will not open. Four of the corpus's ten signatures take this route,
  including its one certification (`xfa_filled_imm1344e.pdf`, whose hole holds 32 578 bytes that
  are not its value). Nothing is compared for them and each is named.
- **`Changes::unplaceable`**, a per-object refusal, so an object no table can place is never
  counted as unchanged.
- **`Judgement::NotClassified`**, which is the ranking against Table 257 and is not done. It names
  the level, counts the objects, and states what deciding needs — each changed object resolved to
  what it *is*, and the table's carve-out for a DSS or document-timestamp update, which is a
  judgement about a whole revision and is why `Comparison::updates_after` is counted first.

The one thing asserted is `Judgement::NoChangeToRank`: the same objects in the same places under
the same catalog. Even that is a statement about objects, not a verdict on a signature — step one
is the digest and step three has no trust store.

## 4. What this does not reach

Nothing in the *program* reports any of it. The comparison is reached by `pdf-signature`'s tests
and by no host, which is the debt §12.8.2.2.2's row now carries in place of the old one — and it is
the shape `doc/habits/the-ledger-and-claims-about-this-tree.md` warns of, named here so a later
round finds it rather than rediscovers it. §12.8.2.1's `shall` — transform parameters "shall
determine which objects are included and excluded in revision comparison" — is likewise narrower
now and still unmet: the comparison is over every object, and Table 259's `/Fields` and Table 256's
`/Data` select none of them.
