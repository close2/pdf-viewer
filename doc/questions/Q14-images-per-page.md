# Q14 — Ratify the departure: images are a directory per page, not one flat directory.

Source: ADR 0841 §3, session 899, a departure from the approved RFC 0003 §4.
Status: **open** — answered when `A14-images-per-page.md` exists beside this file.

## Why it needs the owner

A flat directory cannot be listed without extracting every image first, because a file's name depends on its codec and on whether a mask travels beside it. Predicting names would make a listing name files that a read cannot produce.

## What the tree does meanwhile

Built per page, with the argument recorded.

**Session 923 adds a measurement, and it is for the departure rather than against it.** Session 919
found a corpus document — `corpus-cache/tika-issue-tracker/batch1/PDFBOX/PDFBOX-186-0.pdf`, 10 084
images on one page — holding a walk for twenty-five minutes, and the natural suspicion was that the
per-page directory was the shape at fault. It was not: a *listing* of `images/NNNN/` is one
extraction of one page, where the flat directory §4 proposes would have been one extraction of the
whole document. What cost the time was the core validating each name by re-running the extraction
that produced it, which is a question the layout does not reach and which ADR 0886 fixed in
`pdf-vfs`. That document's whole ten-thousand-entry directory is now listed, `stat`ed and read in a
quarter of a second. Nothing in this changes what the question asks.

## Recommendation

Ratify. The alternative makes a plain listing cost a full extraction of the document.
