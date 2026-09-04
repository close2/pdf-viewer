# Q02 — Does `pdf-transform` split into a broker and a confined worker now, or later?

Source: RFC 0002 §13 question 3; ADR 0800 §6 states the cost.
Status: **open** — answered when `A02-the-confinement-tranche.md` exists beside this file.

## Why it needs the owner

The suite runs in one process today, so the command line parses untrusted bytes unconfined. The split is a transport change on a pattern the tree now has twice, and session 902 extracted `confined-transport` for exactly this reason.

## What the tree does meanwhile

In-process, the RFC's own default. Every verb's seam was written so that the split is a transport change rather than a redesign.

## Recommendation

Take it before the first release rather than after. The machinery exists and the argument is the same one that confined the viewer.
