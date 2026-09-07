# Q50 — May a converter replace the deprecated `F` operator with `f`?

Source: `doc/pdf-a-conversion-limits.md` §5.2, from ISO 19005-4 §5.1's rule that features
ISO 32000-2 deprecates shall not be used.
Status: **open** — answered when `A50-the-deprecated-f-operator-and-the-fence.md` exists beside this file.

## Why it needs the owner

It is the smallest possible test of where ADR 0816's fence is drawn, and it has a real target:
PDF/A-4 forbids deprecated features, ISO 32000-2's Table 59 deprecates `F` — documenting it as
"Equivalent to `f`; deprecated in PDF 2.0 and included only for compatibility" — and legacy content
streams contain it. The substitution changes one byte and provably cannot change a mark.

If the fence is drawn at **marks**, it is allowed. If it is drawn at **content streams**, it is
not, and a document using `F` is unconvertible to PDF/A-4 for the sake of a synonym.

## What the tree does meanwhile

`doc/pdf-a-conversion-limits.md` §5.2 lists it as refused, and says the refusal is undecided.

## Recommendation

Allow it, narrowly and by name: a closed list of operator spellings the standard itself documents
as equivalent, applied only where a target's deprecation rule requires it, and reported. The fence
was drawn to keep this project from *composing* content; an equivalence the standard states in its
own table is not composition. A broader licence to rewrite content streams is not sought and
should not be granted.
