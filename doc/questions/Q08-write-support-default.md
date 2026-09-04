# Q08 — Should the faces default to read-only, with writing opt-in?

Source: RFC 0003 §9 question 2.
Status: **open** — answered when `A08-write-support-default.md` exists beside this file.

## Why it needs the owner

A file manager makes writes easy to trigger by accident: a drag lands, a rename fires, and moving a page out of the mount deletes it from the document, which session 911 found by hand.

## What the tree does meanwhile

Writes are enabled. Every write is an incremental append that destroys no bytes and leaves the producer's original in the file, which is the mitigation.

## Recommendation

Default to writable but switchable, because the append construction makes a mistake recoverable from the file itself. If the owner prefers opt-in, the flag exists to be inverted.
