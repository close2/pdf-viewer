# Q05 — Confirm `pdf-transform` as crate and binary, with `pdf-retrieve` separate.

Source: RFC 0002 §13 question 6.
Status: **open** — answered when `A05-naming.md` exists beside this file.

## Why it needs the owner

A public binary's name is hard to change once anyone scripts it.

## What the tree does meanwhile

Shipping as `pdf-transform`, with `pdf-retrieve` separate as the RFC proposed.

## Recommendation

Confirm. RFC 0006 proposes `pdf-transform archive` and `pdf-retrieve archive-check` on the same split, so one answer settles both.
