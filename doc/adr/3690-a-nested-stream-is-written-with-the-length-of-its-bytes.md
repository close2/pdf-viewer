# 3690 — A nested stream is written with the length of its bytes

Session 2247. Status: **accepted**. Context: ADR 3681 (an incremental update writes each stream's
length), ADR 3689 (`syntax/stream-not-indirect`); ISO 32000-2 §7.3.8.1, §7.3.8.2; audit 2243-B02's
writer half.

## Context

A stream read where §7.3.8.1 forbids it — as an array element or a dictionary's value — reaches the
writers as an `Object::Stream` inside another value. `serialize` and `incremental_update` re-derive
the `/Length` of each stream they write *as an object*; a nested one went through `write::object`,
which wrote its dictionary as handed. A stream read with `/Length 3` over 8 bytes went out the same.

Two answers were open: re-derive the length, or refuse to write the stream.

## Decision

**`write::object` states the count of the bytes it writes for every stream it writes**, nested or
not. Refusing would have meant a fallible `write::object` — a signature every writer of the tree
calls — for a value the reader now logs as a repair and that a viewer's save should not lose. A
nested stream written with a true `/Length` reads back the same through this reader and any other
that tolerates the nesting; one written with the stale length does not.

The nesting itself is kept: the writer writes the value it is handed. Moving the stream into an
indirect object of its own would need a new object number before the cross-reference section is
laid out, which `serialize`'s `Assembly` allocates only for objects a caller places.

## Consequences

- A save or a rewrite never states a `/Length` that does not count the bytes after it.
- An archival conversion of a file with a nested stream is refused by the output's own check, which
  names `syntax/stream-not-indirect` (ADR 3689). Hoisting the stream in the assembly is what would
  answer it; that is the next step if a corpus file needs it.
