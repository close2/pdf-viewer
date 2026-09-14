# Q64 — Is applying a redaction owed, now that its exclusion's reason has been overtaken?

Source: session 1024, reading §12.5.6.23's row against the clause.

## The fact

§12.5.6.23's row excluded *applying* a redaction (as against drawing the annotation's appearance,
which is done) on the argument "an incremental update cannot express it": applying a redaction
removes content from a content stream, and §7.5.6's update only appends. That was true when the
row was written. RFC 0002's serializer (`CLAUDE.md`, "Assembling documents from existing
documents") now writes a new file from an old one, and it copies stream bytes "encoded, by range,
untouched" — so the mechanism that could not express a removal now exists.

Session 1024 kept the row's conclusion on the narrower reading that the serializer copies bytes
rather than editing them, but named the change in what the row *means*: the exclusion no longer
rests on an impossibility, and `CLAUDE.md`'s own boundary line — "does the operation invent
marks?" — puts removal on the near side, because removing content composes none.

## What is asked

Whether applying a redaction is **owed** (in scope, a debt this project will build: the annotation
is read, the region's content is removed, the result is written by the serializer) or **excluded**
by a fresh argument — for instance that a redaction's *overlay text* and *fill* are marks this
program would compose, which the authoring exclusion forbids, and that a removal without the
overlay is not what §12.5.6.23 describes.

Both are defensible. What is not defensible is the row's present state: an exclusion whose stated
reason is false. This is `CLAUDE.md`'s own rule about its restrictions — "they come off step by step
as each is read against the standard rather than staying because they are written down" — and it
is the owner's call which way this one goes, because it widens what the project owes.

Recommendation: owed, with the overlay text and fill excluded by name as the marks this program
does not compose — a redaction applied without them is still a redaction, and §12.5.6.23's `/OverlayText`
is optional.
