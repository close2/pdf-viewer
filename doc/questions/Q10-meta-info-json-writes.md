# Q10 — Confirm that overwriting the document information file is in the first version.

Source: RFC 0003 §9 question 4.
Status: **open** — answered when `A10-meta-info-json-writes.md` exists beside this file.

## Why it needs the owner

It is the one write that edits metadata rather than content.

## What the tree does meanwhile

Implemented, on the argument that the file is that dictionary and the write is the read's inverse, so writing back what was read changes nothing. Session 906.

## Recommendation

Confirm as built.
