# Q13 — Are 150 and 300 dots per inch the right set for rendered pages?

Source: RFC 0003 §9 question 7.
Status: **open** — answered when `A13-the-resolution-set.md` exists beside this file.

## Why it needs the owner

Every resolution is a directory a search or a thumbnailer will walk, and each entry generates when it is stat'd.

## What the tree does meanwhile

Two are offered, as the RFC proposed.

## Recommendation

Keep two. If a third is wanted, a low one for a quick look is more useful than a higher one, since large pages at higher resolutions are already past the pixel ceiling.
