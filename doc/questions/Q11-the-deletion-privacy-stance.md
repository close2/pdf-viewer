# Q11 — Should a face ever offer a content-destroying rewrite?

Source: RFC 0003 §9 question 5.
Status: **open** — answered when `A11-the-deletion-privacy-stance.md` exists beside this file.

## Why it needs the owner

Deleting through an append leaves the bytes in the file. Someone who deletes something in order to remove it will assume otherwise, and the standard's own construction is why they are wrong.

## What the tree does meanwhile

Every deletion warns, in those words, in both the transform layer and the faces. Session 909 found the warning was said for pages and not for attachments, and fixed it.

## Recommendation

Keep the warning and do not offer a vacuum in the faces. If a rewrite is ever wanted it belongs in the transform layer as a named verb, argued once.
