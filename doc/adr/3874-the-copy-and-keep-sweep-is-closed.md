# 3874 — The copy-and-keep sweep is closed

Session 2293. Status: **accepted**. Context: `doc/questions/A834`, ADRs 1196, 3861, 3862, 3863,
3869, 3873; the habits of rounds 2287 ("ask what still reaches the producer's") and 2290 ("ask who
*draws* it, not who references it").

## The question

Where the redaction keeps a producer's object that holds covered marks, does anything in the output
still reach it when nothing draws it?

## The shapes, each with its answer

A page's walk keeps a producer's object in four ways, and each is answered:

- **A form or an image copied for the page** (ADR 1196), its soft mask included, which is one of
  the page's cleared images: the original takes the edit in its own place where nothing draws it
  (ADR 3861); a form the page owns holding one another page draws is rebuilt from its slot (ADR 3863).
- **A shading or pattern named by reference** whose located data the page destroyed (ADR 1363):
  the same rule (ADR 3861).
- **A shading or pattern written directly** in a resource dictionary: in inherited resources (ADR
  3869); in resources the page or a form names by reference and shares, or the page's own resources
  a form without its own falls back on (ADR 3873). §7.3.8.1 makes every stream indirect, so a
  form or an image is never a direct entry, and a direct font's glyph descriptions are streams
  (§9.6.4's `/CharProcs`). A font's program holds no page marks, since a removed glyph is removed
  from the content stream that shows it.
- **References that draw nothing**: a structure element's `/Stm` reaches the edited object (round
  2287, ADR 3861); an element's descriptions are restated whole (ADR 3862), which is text about the
  content and not a mark.

## Decision

No shape is left. What the sweep keeps, it keeps on purpose. An original the search finds drawn
keeps its marks. The search reads the original as drawn when it cannot tell: a stream that does not
decode, a resource reached without a `Do` naming it, an equal dictionary reached elsewhere, or a
search past `MAX_TOKENS`. That error is the direction ADR 3861 allows, since the other direction
would change another page's drawing. A new construction that copies an object reopens the sweep,
and the habit is to run the probe first.
