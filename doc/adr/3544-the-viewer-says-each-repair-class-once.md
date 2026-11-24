# 3544 — The viewer says each class of repair once per document

Session 2210. Status: **accepted**. Context: ADR 3477 ("the viewer keeps repairing, but never
silently"), ADR 3541, ADR 0366 (`losses_said`, the precedent); trap 5.

## Context

The viewer reports what a page could not draw (`Unsupported`), and nothing of what the reader read
in place of what the file says: a page drawn on this reader's own box, a `Q` ignored, a string read
to the end of the stream. The log on the document holds all of them once ADR 3541 is built.

## Decision

- `Document::repair_classes` lists each class the log holds, once, without walking every site.
- `viewer-core`'s `notes::repairs` words each class not yet said — the clause the file broke, what
  was read instead, the class's name — and `Open::repairs_said` keeps which were, so a class is
  said once per document, at the install of the first page whose reading made it, beside
  `notes::losses`. It is kept with that page's notes, so `Query::Reports` says it again.
- The sentence is the class's own `read_instead`, not a second wording: a host reads the same words
  a validator's finding carries.

## Consequences

- A file the reader repaired says so on the first page that met the repair, including the syntax
  layer's classes (a table rebuilt, a string unterminated). A page whose ancestry states no
  `/Resources` says that too, which a fixture written without the entry now hears.
