# ADR 0044 — The last silence

Status: accepted, 2026-07-30.

## Context

§8.11.4.4's usage and usage application dictionaries were the conformance ledger's original
`silent` row and the only one left of that kind: a layer that a document says should switch
itself off — by zoom, by language, by print state — was drawn with **nothing said**. It sat
last on the §0 list for eleven sessions with a stated method beside it: an `eprintln!` naming
the documents that carry an `/AS`, before any condition and long before any code.

The reason it was last is that the item was framed as "give it a condition, and then a report",
on the assumption that a layer panel is needed before honouring it is worth more than reporting
it. That assumption was wrong, and cheaply checkable.

## What the measurement said

Eight corpus documents carry an `/AS`. Their events and categories:

- Seven pair a `View` event with the `View` category, a `Print` event with `Print`, and an
  `Export` event with `Export` — the shape the clause's own example uses.
- One, ISO 32000-2 itself, pairs a `View` event with the `Zoom` category.
- **None uses `User` or `Language`.**

So the categories a *viewer* actually meets are `View`, `Print`, `Export` and `Zoom`, and
§8.11.4.5 says what to do with all four without a layer panel existing: after the base state and
the `/ON`/`/OFF` arrays give "the initial state used by all PDF processors", an interactive
processor "shall examine the AS array for usage application dictionaries that have an Event of
type View. For each one found, the groups listed in its OCGs array shall be adjusted".

That is a rule about what is *drawn*, not about what a panel shows. It should never have been
deferred to one.

## Decision

**Apply the `View` event, with both of the clause's ANDs.** Across the categories of one
dictionary — "if all the entries yield a recommended state of ON , the group's state shall be
set to ON ; otherwise, its state shall be set to OFF" — and across dictionaries, "if a given
optional content group appears in more than one OCGs array, its state shall be ON only if all
categories in all the usage application dictionaries it appears in have a state of ON ."

`Print` and `Export` events are not applied: §8.11.4.5 gives each a duration — "for the duration
of the print operation", and of the export — and this is neither. A viewer that applied `Print`
would hide a watermark meant for the screen, or show one meant only for paper.

**`Zoom` is answered at a magnification of 1.0, and that is a choice.** A display list carries no
magnification: it is built once and rasterised at whatever scale the caller asks for. 1.0 is the
magnification at which a page is its stated size, which is what this tree draws when nothing says
otherwise. The alternative is to thread a scale into `interpret` and rebuild the display list per
zoom, which is a viewer's design question rather than a clause's — and §8.11.4.5 already
anticipates it: "whenever there is a change to a factor that the usage application dictionaries
with event type View depend on (such as zoom level), the corresponding dictionaries shall be
reapplied."

**`User` and `Language` leave the state alone and are reported.** Both are questions about *this
processor* — "the user's identification", "the language and locale of the application" — rather
than about the document, and `pdf-font`'s `substitute` module is deliberately the only place in
this tree that reads one. Taking the clause's "otherwise OFF" would hide content on the strength
of a question nobody asked. This is the fourth place in the tree where a report accompanies
drawing rather than replacing it, and it is argued the same way as the other three (trap 5).

## Consequences

**No gate number moved**, and that is the finding rather than a disappointment. The eight
documents that carry an `/AS` pair a `View` event with the `View` category over groups whose
`/ViewState` is `ON`, so the mechanism now runs on all eight and changes nothing about any of
them. The corpus could not have tested this, which is trap 8's shape exactly, and five synthetic
tests in `tests/optional_content.rs` are what defends it — including the one that would fail if
a `Print` event were applied to a screen.

The ledger's `silent` count is 2, both of them §9.8.3's `/Style` and `/FD`, which are inputs to
choosing a substitute face rather than to drawing an embedded font. **The row this project has
called "the last silence" since the ninth session is closed**, and the two that remain arrived in
the thirtieth by reading a family rather than by anyone deciding not to build something.

The lesson is about the *shape* of the deferral rather than the clause. "It needs a layer panel
to be worth more than a report" is a claim about a feature's value, and it was never checked
against what the clause asks — which is that content be drawn or not drawn. §6.3.2.2 puts
respecting the optional content configuration second among the three obligations it places on a
rendering processor, and a configuration includes its `/AS`.
