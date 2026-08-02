# ADR 0142 — A panel drawn in the fourteen fonts the clause guarantees

Status: accepted, 2026-08-02. Session 166. The first panel this project has had, and the two
pieces of interface it needed that did not exist.

## What was owed, and for how long

`doc/HANDOVER.md` has called a panel "the largest single thing this project owes" for thirty
sessions. `Query::Outline`, `Query::Layers` and `Query::Attachments` have answered with
everything one needs since the hundred-and-thirty-first; no consumer asked any of them. §12.3.3
spends three sentences describing a *screen* — items in the linked list's order, opened and
closed by clicking, a closed item's descendants hidden — and the ledger row said `partial`
"because a viewer with no panel cannot show it".

The reason it stayed owed is not that a list of rows is hard. It is that this program has **no
toolkit**: winit is a window and an event loop, and everything between a display list and a
person is ours. Adding `egui` would buy a widget set for a large dependency and would prove
nothing about the architecture — winit + vello *is* the unnative UI.

## The decision

Draw it with what the project already has, and put it where a native host's equivalent would not
be: `viewer_ui::chrome`, on this side of the `viewer-core` boundary.

- **A `pdf_render::DisplayList` at an identity transform.** Not Vello calls. The consequence is
  that the panel is drawn by whichever backend drew the page — including `render-cpu` under
  `--cpu` and on a page the device refuses (ADR 0125) — and that it can be rasterised in a test
  with no display at all.
- **Device pixels, y downwards.** The raster's space, not the page's (trap 12a). The one place
  the flip happens is `Chrome::text`, because glyph outlines arrive y-up in font units.
- **The page's viewport is the window less the panel.** A panel drawn *over* the page would
  leave the core centring the page behind it; telling the core about the smaller viewport means
  a fitted page fits what is visible, and every coordinate crossing the boundary is offset by
  exactly one number.

## The two things that did not exist

**`pdf_font::LoadedFont::standard`.** Text an interface generates has no font dictionary and no
document to load one from, and every route into `pdf-font` takes both. §9.6.2.2 says the
fourteen "shall be available to the PDF processor", and since ADR 0133 they are available *as
bytes in this binary* — so an interface set in Helvetica is set in the same Helvetica on a
machine with no fonts installed at all. The constructor assembles a `/Type1` dictionary naming
the face and hands it to the ordinary `LoadedFont::load` against a new `Document::empty`, so the
encoding is §9.6.5.2's and the widths are §9.6.2.2's own metrics — the same two answers a file
naming `/Helvetica` gets. **A second path would have been a second reading of clause 9**, and
this project has three ADRs about what happens when one arithmetic gets written twice.

`Document::empty` is ten lines and no parsing: an empty `Arc`, a default `XrefTable`, three
empty maps. A reference reached through it resolves to `Object::Null`, which is what `get`
already answers for an object number the table does not name.

**`PageTarget::Destination`.** A host cannot resolve §12.3.2's destination: the target is a page
*object* (or, in §12.3.2.3's form, a structure element), and turning either into an index is a
walk of the page tree, which lives on this side with the document. So the destination crosses as
a command payload and `viewer-core` resolves it — one page-tree walk per navigation, which is
exactly what a link click has always cost. §12.3.2.2's NOTE makes a bare page *number* a page in
a remote document, so `Target::Number` moves nowhere rather than being read as an index here.

`PageTarget` loses `Eq` for it, because Table 151's coordinates are floats. That is the whole
cost.

## What the clause decides about the drawing

Almost all of it, which is the point of putting a panel behind a clause at all:

| what a person sees | where it comes from |
|---|---|
| the order of the rows | "the items at a given level shall appear in the order in which they occur in the linked list" |
| whether a subtree starts open | the *sign* of Table 151's `/Count` |
| that closing hides everything below | "[w]hen an item is closed, all of its descendants in the hierarchy shall be hidden" |
| the text | Table 151's `/Title`, a §7.9.2.2 text string |
| the colour | Table 151's `/C`, "the colour that shall be used for the outline entry's text" |
| bold and italic | Table 152's two bits |

What is *not* the clause's, and is written down as a choice: the width, the row height, the
indent, the grey, the ellipsis where a title does not fit, and the hover highlight. §12.3.3
states none of them and a native host would take all six from its platform.

## What it does not do, and the measurement that sizes it

One half of one sentence: "[c]licking the text of any visible item activates the item, causing
the interactive PDF processor to jump to a destination **or trigger an action** associated with
the item." Only the jump happens. `Item::destination` reads `/Dest` or a go-to action's `/D`, so
an item whose `/A` is anything else does nothing at all.

Counted over the corpus rather than guessed at — 281 outline items carry an `/A`:

| `/S` | items | documents |
|---|---|---|
| `GoTo` | 249 | 129 — followed |
| `JavaScript` | 18 | 1 — `CLAUDE.md`'s exclusion list |
| `URI` | 7 | 3 |
| `Named` | 5 | 2 |
| `GoToR` | 1 | 1 |
| `SetOCGState` | 1 | 1 |

`viewer-core` performs every one of the last three for a *link* already
(`interact::activate`). What is missing is the path from an outline item to the same code, and
§12.3.3's ledger row stays `partial` naming exactly that.

## The identities a toggle is keyed by

Items are numbered in **pre-order over every item**, open or closed, rather than by visible row.
The trap is one sentence: close the first chapter and, under visible-row numbering, the second
chapter inherits the first's identity, so the next click toggles something else.
`closing_a_subtree_does_not_renumber_the_items_below_it` is that, and it checks the row *below*
a closed subtree reaches the right destination rather than merely that the row count fell.

## How it is tested, with no display

The four gates cannot see chrome — the corpus interprets page one, the oracle rasterises pages
it is handed, and neither opens a viewer. So `viewer-ui/tests/panel.rs` rasterises the panel's
own display list with `render-cpu` and **counts dark pixels** in a band of rows. A test that
asserted the display list held the right number of commands would have passed with every glyph
missing; this one was checked by deleting the glyph fill, and three of its four cases fail.

The window itself was driven on `Xvfb` per ADR 0126 — `o`, a click on a title, a click on a
triangle, ten wheel notches — because that is the only way to exercise key press to command to
frame, and it is where the first defect appeared: a scrolled row drew *over* the heading. The
heading is now drawn last, over its own background, rather than the list being clipped: a clip
would cut the letters in half and say nothing.
