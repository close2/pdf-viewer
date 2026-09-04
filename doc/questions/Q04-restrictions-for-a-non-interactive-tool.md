# Q04 — Confirm the restriction policy for a tool with no person at the other end.

Source: RFC 0002 §13 question 4.
Status: **open** — answered when `A04-restrictions-for-a-non-interactive-tool.md` exists beside this file.

## Why it needs the owner

Table 22's bits are addressed to a processor acting for a person, and a pipe has no person. The tree had to choose, and the owner never ratified the choice.

## What the tree does meanwhile

Implemented and shipping: the default is off, `--restrictions=on|ask|warn` selects the rest, and *ask* on a terminal asks on standard error and reads a line, while *ask* with no terminal refuses by name as unanswered. Sessions 872, 903 and 916.

## Recommendation

Confirm as built. It matches the owner's rule that restrictions are low priority and must always be switchable off.
