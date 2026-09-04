# Q09 — Is reordering pages by `mv` wanted, and with what semantics?

Source: RFC 0003 §9 question 3.
Status: **open** — answered when `A09-reorder-by-mv.md` exists beside this file.

## Why it needs the owner

Ordinal names are positions, so a rename could mean reorder. But rename has no atomic insert-and-shift, and a half-finished sequence leaves a document in an order nobody asked for.

## What the tree does meanwhile

Renaming within `pages/` refuses with a permission error and a sentence.

## Recommendation

Leave it refused. The `pages` verb reorders in one operation, and an order file would be a second grammar for the same thing.
