# Q06 — Confirm the deterministic default: no dates written unless asked.

Source: RFC 0002 §13 question 7.
Status: **open** — answered when `A06-the-metadata-stance.md` exists beside this file.

## Why it needs the owner

A writer that stamps the clock cannot be byte-compared, and every gate this suite has rests on running a verb twice and getting the same bytes.

## What the tree does meanwhile

Implemented: output is deterministic unless `--date` is given. Session 870 built it, and the determinism and idempotence columns of five corpus walks depend on it.

## Recommendation

Confirm as built.
