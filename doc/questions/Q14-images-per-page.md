# Q14 — Ratify the departure: images are a directory per page, not one flat directory.

Source: ADR 0841 §3, session 899, a departure from the approved RFC 0003 §4.
Status: **open** — answered when `A14-images-per-page.md` exists beside this file.

## Why it needs the owner

A flat directory cannot be listed without extracting every image first, because a file's name depends on its codec and on whether a mask travels beside it. Predicting names would make a listing name files that a read cannot produce.

## What the tree does meanwhile

Built per page, with the argument recorded.

## Recommendation

Ratify. The alternative makes a plain listing cost a full extraction of the document.
