# What a native host would not draw itself, and whether the API lets it delegate

Status: **six of six populations cross as data** since the three-hundred-and-ninety-eighth session
(ADR 0235). What is left is one decision the audit named and did not take: a page rendered
*without* its widget appearances.
Priority: 37 — capability, and a prerequisite for `doc/todo/30`'s hosts being any good
Clauses: §12.5.6.14 (popups), §12.7.4 and §12.7.5 (fields), §12.3.3, §12.3.4, §8.11.4.3, §7.11.4,
§14.3.3, §12.5.1
Code: `crates/viewer-core/src/query.rs`, `crates/pdf-model/src/form.rs`,
`crates/viewer-ui/src/chrome.rs`

## Why it was asked

The project owner:

> I did note, that we implemented a lot of UI-interaction functionality (mostly annotations) by
> rendering it ourself. I might want to switch displaying for instance pop-ups from rendering
> ourself to native gui frameworks. Make sure that we have a clear-cut API for this.

`doc/ui-boundary.md` (`doc/HANDOVER.md` §0's pointer) already states the rule this is measuring
against:

> **Interactive chrome crosses as geometry, not pixels.** Selection highlights, an in-progress
> annotation rubber-band, resize handles, a caret — these change at pointer speed and must not
> force a page re-render. Emitting them as quads and points lets a native host draw selection in
> macOS's selection colour, KDE's accent, the Windows highlight brush …

So the question is not whether `viewer-ui` draws these — it does, and that is what a tier-2 host
*is* — but whether a host that wanted to use a `QTreeView`, an `NSPopover` or a `GtkText` could,
without reaching into `viewer-ui` or re-deriving anything.

## The audit, and the sixth population closing it

| what `viewer-ui` draws | what a native host would use instead | crosses as |
|---|---|---|
| §12.5.6.14's popup window | `NSPopover`, `QToolTip`, `GtkPopover` | **`Query::Popups`** → `PopupWindow { annotation, parent, quad, title, text, modified, colour }` |
| §12.3.3's outline, §8.11.4.3's layers, §7.11.4's files, §12.3.5's collection, §12.4.3's articles, §14.3.3's `/Info` | `QTreeView`, `NSOutlineView`, `GtkTreeView` | `Query::Outline`, `Layers`, `Attachments`, `Collection`, `Articles`, `Properties` — all data |
| §12.3.4's thumbnails | an icon view | `Query::Thumbnail(index)` → a decoded `Image`, one page at a time |
| a text selection | the platform's highlight brush | `Query::Selection` → quads in device pixels |
| §12.5.1's focus ring | the platform's focus ring | `Query::Focus` → one quad |
| **§12.7's form fields** | a real `QLineEdit` / `NSTextField` / `GtkEntry` | **`Query::Fields`** → `FormField { name, partial, control, value, read_only, required, no_export, widgets }` |

**The popup case the owner named is clear-cut**, and its own doc comment says so: "a window belongs
to the host's platform. What this crate owns is which windows are open, where the page puts them and
what the document says goes in them; what a title bar looks like is the host's, and a native one
would draw a real window rather than a rectangle." `Query::Popups` answers **only the open ones**, in
`/Annots` order, with the quad already through the centring, the magnification, the crop box's origin
and §7.7.3.3's rotation — which is exactly the arithmetic ADR 0118 found wrong for seventy-five
sessions and which no host should repeat.

## The form, which closed in the three-hundred-and-ninety-eighth session

`Query::Fields` answers for the page being shown, in `/Annots` order, with everything this file said
such a query has to carry:

- the field's **identity** — §12.7.4.2's fully qualified name, which `Edit::SetField` addresses —
  and Table 226's `/T` beside it, because a label wants the field's own name rather than its
  ancestry;
- the **name to show** — Table 226's `/TU`, §14.9.3's `shall`, through the `FieldName` ADR 0167
  built;
- each widget's **quadrilateral**, in device pixels, in the same form `Selected::quads`,
  `Answer::Focus` and `PopupWindow::quad` take — one arithmetic, one place;
- **what kind of control it is** — `pdf_model::form::Control`, §12.7.5's four types with buttons
  split the three ways §12.7.5.2 splits them, plus Table 231's six flags, Table 233's five, Table
  229's two, Table 232's `/MaxLen` and the comb's cell count;
- the **value**, as `Answer::Field` already carried it since ADR 0201, already through §12.7.5.3's
  truncation;
- **Table 234's `/Opt`, `/TI` and `/I`**, so a combo box and a list box have their items and their
  selection;
- **Table 227's three flags**, so a host builds a read-only control and marks a required field;
- and the thing the audit did not know it was missing: **the appearance-state name that turns each
  widget on**, because §12.7.5.2.3 makes a check box's value a name the file invented and no host
  could guess it.

Reading the clause for that last one found that **a check box could not be checked at all** — the
page went on drawing the state the file was saved in, on both the stored and the constructed path.
ADR 0235 has the numbers and the fix. `viewer-ui` toggles a box and a radio button from a click as of
the same round, which is the first thing in this tree that gives a button field a value.

## What is left, and it is one decision

**A page rendered without its widget appearances.** A host placing native controls over the page gets
both pictures unless it can ask for one without them. This is a second decision and a larger one:
§12.5.5's appearance streams are page content, and leaving them out changes `interpret`, on which the
oracle's 1794-page comparison rests. It is probably a flag on the render request rather than a query
— a departure a host *asks for* rather than one this crate takes — and it wants its own round.

Two smaller things, both named rather than left to be found:

- **Table 229 bit 26's `RadiosInUnison` crosses and is not obeyed.** Turning on every button of a
  set that shares an on state is a decision for whatever handles the press, and this tree's own host
  has the flag rather than the behaviour.
- **§12.7.5.4's list box still draws nothing on the page**, and says so. The clause states which
  items are selected and states no highlight, so `variable_text` refuses it. A host with the items
  and the selection can now draw a real list — which is the point — but a page with a list box on it
  is still light, and the report is what says so.

## What is deliberately not here

- **Annotation *appearances* other than widgets.** §12.5.6.10's markups, §12.5.6.4's icons and the
  rest are page content by §12.5.2 — "the annotations shall be drawn in the order in which they
  appear in the array" — and a native host has no widget for a highlight. Those stay in the page.
- **A caret.** `doc/todo/33` owns it, and a host with a native text field gets the platform's caret
  for free — which was an argument for doing this before that one, and the caret went first anyway
  (ADRs 0211, 0225).
