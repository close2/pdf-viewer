# Q58 — Does a `preserve` remedy that appends a page need its own amendment to the fence?

Source: `doc/rfc/0007` §7 question 5, raised by session 954.
Status: **open** — answered when `A58-does-appending-a-page-need-its-own-amendment.md` exists
beside this file.

## Why it needs the owner

`CLAUDE.md`'s authoring exclusion says this project does not compose pages, and its own amendment
record is explicit that a change to it is "its own argued amendment, not scope creep" — the
watermark stamp is named as the first feature on the far side of the line, deliberately excluded
despite being conventional.

A `preserve` remedy that appends an embedded image as a page, or prefixes a metadata packet as one,
is composing a page. It is composing it out of content the document already holds, which is why it
is not `derive` — but it is still marks on a page that no producer wrote.

The exclusion has been amended twice, both times by argument and both times by the owner.

## What the tree does meanwhile

No page is composed. The variable-text field appearances in `pdf-model` sit on the line already,
sanctioned by §12.7.4.3's own requirement, and `doc/questions/A21` permits constructing annotation
appearances — both cases where a *clause* states what the marks are.

That is the difference worth ruling on: an appended image page is marks **no clause specifies**,
even though the image itself is the document's own.

**The owner widened this on 2026-09-11**, and it makes the question bigger rather than narrower:
appending is available under *every* target, because what the six differ about is what may be
attached and not what may be a page. So this is not a corner used when nothing else works — under
PDF/A-2 it is the only route that keeps an embedded file's content at all, and under PDF/A-4 an
operator may reasonably prefer it to an attachment a reader has to go looking for. Whatever is
ruled here applies to the whole feature rather than to an edge of it.

One consequence to rule on with it: under a **Level A** target the appended pages need structure
elements, or they are content the logical structure does not describe (ISO 19005-2 section 6.7).
Composing a page is one amendment; composing a page *and the structure tree entries that describe
it* is arguably a second, and it is the nearest this program would come to authoring.

## Recommendation

Treat it as needing the amendment, and make it. The argument is that the alternative is losing the
content outright, and an exclusion written to stop this program becoming a layout engine was not
written to force that choice — but it is the owner's line and the record says so.
