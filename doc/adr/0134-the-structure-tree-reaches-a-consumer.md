# ADR 0134 — The structure tree reaches a consumer

Status: accepted, 2026-08-02. Session 149. The last of the five items `doc/HANDOVER.md` section 0
listed as blocked on the `viewer-core` boundary.

## Context

`pdf-model` has read ISO 32000-2 §14.7's logical structure since the seventy-eighth session and
§14.9's `/Alt`, `/E`, `/Lang` and `/ActualText` since the sixtieth. `structure.rs`'s own module
comment recorded what that was worth:

> Like Table 99's `/Order`, the data is this crate's and the consumer is not: nothing in this
> program yet hands a structure tree to anybody.

Six sessions of clause reading with nothing on the other end. This is the other end.

## Decision

`Query::AccessibilityTree` answers with `Vec<AccessibilityNode>`: the page's structure elements,
**parent-first**, each carrying

- **`role`** — §14.7.4's `/S`, as the document states it;
- **`name`** — what §14.9 says the element should be spoken as;
- **`language`** — §14.9.2's tag, resolved through §14.9.2.3's hierarchy;
- **`quads`** — where the element is, in the same device pixels `Query::Selection` answers in.

Four decisions in that, each of which could have gone another way.

**The role is not mapped.** §14.7.4 has a role map and §14.8.4 a standard set, and neither is
applied here. A host that knows its platform's vocabulary is better placed to turn `H1` into
`AccessKit`'s `Role::Heading` or AT-SPI's `ROLE_HEADING` than this crate is, and a mapping here
would be a second opinion nobody asked for. What the document says it is, is what crosses.

**Parent-first, as a flat list with indices.** That is the shape both `AccessKit` and AT-SPI want,
and a flat list has no recursion for a host to bound. The test asserts the ordering directly — a
node's parent is always at a lower index — because that is what makes an index usable as a link.

**The name goes through §14.9 rather than being the raw text.** The element's own `/Alt` or `/E`
wins where it states one, because that is a substitution for the whole element. Where it does
not, the element's span of the readback goes through `pdf_model::accessibility::speech`, so an
`/Alt` on a `BDC` *inside* the element applies. Reading the raw text there would speak the letters
an abbreviation is written with, which is precisely what §14.9.5's `/E` exists to prevent.

**The quads are in `Query::Selection`'s space, and reuse its code.** `Viewer::device_quads` was
written for a drag and does the magnification, the centring and the y flip ADR 0118 found wrong
in the one place it existed. A second mapping here would be a second place to get it wrong; a
host that can draw a selection can draw a focus ring with no new arithmetic.

## What an untagged document answers

An empty list, and that is an answer rather than a failure. 885 of the corpus's 974 documents are
untagged, and §14.7 leaves a producer free to say nothing about its own structure. A host learns
that the page says nothing, which is different from the query failing.

## Cost, and where it is paid

`Interpreted` gains two vectors and an `Option<String>` per open page: §14.7.5.2's marked-content
spans, §14.9's described spans, and the catalog's `/Lang`. They are kept for the same reason the
text layer is — the map from an `/MCID` to the text it produced is knowable only during
interpretation — and at a far smaller cost, since the great majority of pages have none.

The tree itself is built **on demand**. A screen reader asks when it attaches and again on a page
change; a drag asks `Query::Selection` sixty times a second, which is why that one's inputs are
cached and this one's answer is not.

One walk of the page tree is paid per query, to invert §14.7's `/Pg` — which names a page
*object* — into the index this crate holds. `Pages::indices` is what makes that one walk;
`Pages::index_of` in a loop is exactly the defect ADR 0124 was about, and the comment beside the
call says so.

## What is still owed

- **A host that uses it.** `viewer-ui` does not, and `AccessKit` is not a dependency of anything
  yet. The interface is the part that was blocked; wiring AT-SPI to it is a host job.
- **§14.8.2.5's logical order for a *selection*.** This answers in the structure tree's order,
  which is the logical one. A selection is still taken in content order, and the map between the
  two offsets is what `HANDOVER.md` has listed as owed since the hundred-and-thirty-third session.
  The `marked` spans this ADR made resident are half of that map.
- **§14.7.5.3's object references** — an annotation reached through `/OBJR` — contribute no
  `/MCID` and so no text and no quads. Their element is answered with whatever `/Alt` it states,
  which is a true statement about what this program can locate rather than a silent drop.
