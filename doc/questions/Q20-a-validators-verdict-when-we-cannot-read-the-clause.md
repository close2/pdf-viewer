# Q20 — What should a validator say about a requirement we have not read?

Source: RFC 0006 §10 question 6.
Status: **open** — answered when `A20-a-validators-verdict-when-we-cannot-read-the-clause.md` exists beside this file.

## Why it needs the owner

A validator that reports a pass for a rule it never checked is worse than one that reports nothing.

## What the tree does meanwhile

Nothing built.

## Recommendation

Report not-checked, by name, per requirement, and never implement a check from a secondary source. That is principle 5 applied to a feature whose whole product is citations.
