# ADR 0162 — A destination is three things and two of them were owed

Status: accepted, 2026-08-03. Session 201. Found by the capability sweep in
`doc/todo/01-ledger-partial-rows.md`, on its fourth run.

## The clause

§12.3.2.1 opens by saying what a destination is, and the list is exhaustive:

> A destination defines a particular view of a document, consisting of the following items:
>
> - The page of the document that shall be displayed
> - The location of the document window on that page
> - The magnification (zoom) factor

The page has been computed since the fifty-seventh session. `View` has carried Table 149's eight
forms since not long after. **Nothing has ever acted on the other two**, and the reason the row
and the module comment both gave was:

> properties of a window with scrolling and zoom, which this program does not have — it fits a
> page to its surface

which was true when it was written and stopped being true in the **hundred-and-thirty-second**
session, when `viewer-ui` became a host and `Command::Zoom` and `Command::Scroll` acquired a
window to act on. Sixty-nine sessions later the sentence was still there.

This is the third instance of the shape `doc/todo/01` calls *capability*: §12.6.3 said "this
crate has no events" for forty-one sessions after `Command::Pointer` landed, §14.3.3 was
`inapplicable` because "this one has no panel" for seven after one was drawn. **A row whose
blocker is a capability rather than a clause is maintained by nobody**, because the session that
adds the capability is not reading that clause.

## Where the work goes, and why not all in one crate

`pdf-model` still stops at reading Table 149. A viewport is not something a model of a file can
invent, and rule 1 of §0 — `interpret` is a pure function of the document and the view state —
would be the thing to give up if it did.

`viewer_core::Open::apply_view` is where the two items become a zoom and a scroll, and it is
called from `Viewer::settle` because that is **the first moment both of the things it needs
exist**: a laid-out viewport, and a display list. The second is not optional decoration —
`/FitB`, `/FitBH` and `/FitBV` magnify to fit "the smallest rectangle enclosing all of its
contents", which no page dictionary states and only the drawing commands can answer.

So `Open` carries a `pending_view`, set when a destination resolves — an `/OpenAction`, a link,
an outline item, a §12.6.4.2 go-to — and consumed on the next settled frame.

## What each of the eight forms becomes

Every one is two decisions: a **magnification**, and a **point of the page put at the window's
top-left corner**.

| form | magnification | corner |
|---|---|---|
| `/XYZ left top zoom` | `zoom`, where stated | `(left, top)` |
| `/Fit` | the page, both directions | the page's top-left |
| `/FitH top` | the page's width | `(—, top)` |
| `/FitV left` | the page's height | `(left, —)` |
| `/FitR` | the rectangle, both directions | the rectangle's top-left |
| `/FitB` | the **content box**, both directions | its top-left |
| `/FitBH top` | the content box's width | `(—, top)` |
| `/FitBV left` | the content box's height | `(left, —)` |

Table 149's null rule is applied **per parameter** rather than per destination, because that is
what it says: "A null value for any of the parameters left , top , or zoom specifies that the
current value of that parameter shall be retained unchanged." A `/XYZ` with a null zoom moves the
window and leaves the magnification; one with a null `left` keeps the horizontal scroll. A
reading that treated an absent parameter as a zero would fit the page instead, which is a
different picture and a plausible one.

Two edges, both stated in the code rather than hidden:

- **A `/FitR` with no extent in one direction** lets the other decide alone, and one with no
  extent at all leaves the zoom — the file has stated a point, not a rectangle.
- **A `/FitB` whose commands cannot all be bounded** falls back to the page box. The alternative
  is a magnification of infinity.

## `Zoom::FitHeight`

New, and the one addition to the public vocabulary. `/FitV` asks for "the contents of the page
magnified just enough to fit the entire height of the page within the window", and a viewer with
fit-page and fit-width and not this one would have to answer that destination with a *number* —
which stops following the window on the next resize. Nothing in `viewer-core` is
`#[non_exhaustive]`, so both consumers failed to compile until they handled it, which is the
point of that rule.

## The check

`headless.rs` builds a 600×800 page with one 200×100 rectangle of content, opens it into a
300×400 window, and reads the geometry back — so every number is checkable by hand. `/Fit` gives
0.5; `/XYZ 0 800 1` gives 1.0 and a 600×800 raster; `/FitR 100 600 300 700` gives 1.5 because the
width decides and puts the rectangle's corner at the window's; `/FitB` lands on the same 1.5,
which is the display list answering a question the page dictionary cannot. A second test pins the
null rule three ways, including "A zoom value of 0 has the same meaning as a null value".

`/FitH` is measured in a 300×200 window rather than the 300×400 one, and the reason is worth
recording: at 300×400 a 600×800 page fitted to the width fits the height too, so there is nothing
to scroll and the test would have passed with the scroll never applied. **A fixture whose
geometry makes the effect invisible is a fixture that tests nothing** — trap 2's "a scene must
fail at the defect's magnitude" one axis over.

## What did not move

Corpus 80 incomplete, oracle 893/78/788, text 98.2%, quorra 912/44/1 — every gate identical. None
of them opens a document at its `/OpenAction`'s magnification, which is exactly why this needed
its own test and why the reason for not doing it survived sixty-nine sessions.

## Alternatives rejected

- **Apply the view in `pdf-model`.** It has no window and rule 1 says it may not acquire one.
- **Apply it in `interact.rs`, where the destination resolves.** No viewport there either, and no
  display list for `/FitB`; the destination would have to be re-read later anyway.
- **Answer `/FitV` with a computed `Zoom::Scale`.** It works until the window is resized, at
  which point the page stops fitting and nothing says why.
