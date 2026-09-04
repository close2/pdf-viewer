# Q21 — May a converter write the appearances this tree constructs?

Source: RFC 0006 §10 question 7, the closest call in that document.
Status: **open** — answered when `A21-constructed-appearances-in-a-converted-file.md` exists beside this file.

## Why it needs the owner

The standard requires an appearance where a field or annotation has one to draw. This tree constructs appearances at render time, sanctioned by §12.7.4.3. Writing them into a file is a producer act, but it invents no marks the program does not already draw.

## What the tree does meanwhile

Nothing built.

## Recommendation

Allow it, and report every one written, so the difference between the producer's file and ours is visible in the report rather than only in the bytes.
