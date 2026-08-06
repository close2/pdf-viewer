# What a native host would not draw itself, and whether the API lets it delegate

Status: **audited in the three-hundred-and-fifty-seventh session, at the project owner's request.**
Five of six populations already cross as data; **form fields are the gap** and this file is what
closing it means.
Priority: 37 — capability, and a prerequisite for `doc/todo/30`'s hosts being any good
Clauses: §12.5.6.14 (popups), §12.7.4 and §12.7.5 (fields), §12.3.3, §12.3.4, §8.11.4.3, §7.11.4,
§14.3.3, §12.5.1
Code: `crates/viewer-core/src/query.rs`, `crates/viewer-ui/src/chrome.rs`

## Why it was asked

The project owner:

> I did note, that we implemented a lot of UI-interaction functionality (mostly annotations) by
> rendering it ourself. I might want to switch displaying for instance pop-ups from rendering
> ourself to native gui frameworks. Make sure that we have a clear-cut API for this.

`doc/HANDOVER.md` §0 already states the rule this is measuring against:

> **Interactive chrome crosses as geometry, not pixels.** Selection highlights, an in-progress
> annotation rubber-band, resize handles, a caret — these change at pointer speed and must not
> force a page re-render. Emitting them as quads and points lets a native host draw selection in
> macOS's selection colour, KDE's accent, the Windows highlight brush …

So the question is not whether `viewer-ui` draws these — it does, and that is what a tier-2 host
*is* — but whether a host that wanted to use a `QTreeView`, an `NSPopover` or a `GtkText` could,
without reaching into `viewer-ui` or re-deriving anything.

## The audit: five of six are clean

| what `viewer-ui` draws | what a native host would use instead | crosses as |
|---|---|---|
| §12.5.6.14's popup window | `NSPopover`, `QToolTip`, `GtkPopover` | **`Query::Popups`** → `PopupWindow { annotation, parent, quad, title, text, modified, colour }` |
| §12.3.3's outline, §8.11.4.3's layers, §7.11.4's files, §12.3.5's collection, §12.4.3's articles, §14.3.3's `/Info` | `QTreeView`, `NSOutlineView`, `GtkTreeView` | `Query::Outline`, `Layers`, `Attachments`, `Collection`, `Articles`, `Properties` — all data |
| §12.3.4's thumbnails | an icon view | `Query::Thumbnail(index)` → a decoded `Image`, one page at a time |
| a text selection | the platform's highlight brush | `Query::Selection` → quads in device pixels |
| §12.5.1's focus ring | the platform's focus ring | `Query::Focus` → one quad |
| **§12.7's form fields** | a real `QLineEdit` / `NSTextField` / `GtkEntry` | **nothing** |

**The popup case the owner named is already clear-cut**, and its own doc comment says so: "a
window belongs to the host's platform. What this crate owns is which windows are open, where the
page puts them and what the document says goes in them; what a title bar looks like is the host's,
and a native one would draw a real window rather than a rectangle." `Query::Popups` answers **only
the open ones**, in `/Annots` order, with the quad already through the centring, the magnification,
the crop box's origin and §7.7.3.3's rotation — which is exactly the arithmetic ADR 0118 found
wrong for seventy-five sessions and which no host should repeat.

## The gap: a page's form fields

`Query::FieldAt((x, y))` answers for **one point**. A native host does not have a point; it has a
page, and it wants to put a widget over every field on it before anybody clicks. There is no query
that enumerates them.

What such a query has to carry, and each has a clause behind it:

- **the field's identity** — §12.7.4.2's fully qualified name, which is what `Edit::SetField`
  addresses;
- **the name to show** — Table 226's `/TU`, because §14.9.3 makes showing it a `shall` and ADR
  0167 is the round that learned one string cannot be both;
- **the quadrilateral**, in device pixels, in the same form `Selected::quads`, `Answer::Focus` and
  `PopupWindow::quad` take — one arithmetic, one place;
- **what kind of control it is** — §12.7.5's four types plus the flags that change the control
  rather than the drawing: Table 231's `Multiline` (a text view, not a line), `Password` (a secure
  entry), `Comb` and `MaxLen` (a fixed-cell entry), `DoNotScroll` (ADR 0197's acceptance limit),
  Table 233's `Combo` and `Edit` (a combo box, editable or not), Table 227's `ReadOnly`;
- **the value**, as `Answer::Field` already carries it since ADR 0201, with `None` for a field
  whose value is not text and `Some("")` for an empty one;
- **§12.7.5.4's `/Opt`**, because a combo box or list box needs its items and nothing answers with
  them today.

**And the page must then not draw them.** A host placing native widgets needs the page rendered
*without* the widget appearances, or it gets both. That is a second decision and probably a flag on
the render request rather than a query — §12.5.5's appearance streams are page content, and leaving
them out is a departure a host asks for rather than one this crate takes.

## What is deliberately not here

- **Annotation *appearances* other than widgets.** §12.5.6.10's markups, §12.5.6.4's icons and the
  rest are page content by §12.5.2 — "the annotations shall be drawn in the order in which they
  appear in the array" — and a native host has no widget for a highlight. Those stay in the page.
- **A caret.** `doc/todo/33` owns it, and a host with a native text field gets the platform's caret
  for free — which is an argument for doing this before that one.

## Order

After `doc/todo/30`'s first host exists, not before: the shape of this query is a guess until
something real tries to place a widget with it, and ADR 0166 and 0167 are two rounds' evidence that
a message's shape is settled by the second consumer rather than by the first.
