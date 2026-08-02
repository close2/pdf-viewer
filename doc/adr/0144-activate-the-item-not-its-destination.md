# ADR 0144 — Activate the item, not its destination

Status: accepted, 2026-08-02. Session 168. Closes §12.3.3, and removes a command variant added
two sessions ago.

## The half-sentence

§12.3.3, on what a click on an outline row does:

> Clicking the text of any visible item activates the item, causing the interactive PDF processor
> to jump to a destination or trigger an action associated with the item.

ADR 0142 built the jump. `Item::destination` reads Table 151's `/Dest`, or the `/D` of a go-to
action inside `/A`, and `Command::GoTo(PageTarget::Destination)` carried it across the boundary.
An item whose `/A` is a URI, a named page command or a set-OCG-state action did **nothing at
all** — not a refusal, not a note, nothing.

Counted before it was built, over the pdf.js corpus's 281 outline items carrying an `/A`:

| `/S` | items | documents |
|---|---|---|
| `GoTo` | 249 | 129 |
| `JavaScript` | 18 | 1 |
| `URI` | 7 | 3 |
| `Named` | 5 | 2 |
| `GoToR` | 1 | 1 |
| `SetOCGState` | 1 | 1 |

Small, and the point is not the size: `viewer-core` already performs every one of those for a
*link*, so what was missing was a path and not a feature.

## The decision: name the object, not the payload

`Command::Activate(ObjectId)`. A host hands over the outline item's own object and this crate
reads `/Dest` and `/A` from it.

Three candidates were weighed:

- **Hand back the destination** — what `PageTarget::Destination` did. Expresses half the
  sentence and cannot express the other half.
- **Hand back the actions** — `Command::Perform(Vec<Action>)`. Works, and moves the reading of
  the document to the host's side of a round trip: the host would be submitting a payload it
  could have invented. Nothing about a viewer is improved by letting a panel say what a document
  asked for.
- **Hand back the object.** The *document* decides what activation means, which is
  `CLAUDE.md`'s rule 1 for the interaction path — `interpret` is a pure function of the file and
  the view state, and so is this.

The third also generalises without any new vocabulary: §12.5.6.15's file attachment annotation,
a widget in a form panel, a §12.3.5 collection row are all *objects*, and a host showing any of
them outside the page has the same problem and now the same answer.

## What was removed, and why that is not thrash

`PageTarget::Destination` is gone, one session after it was added. Once an outline row sends
`Activate`, nothing in the tree constructs a `Destination` variant — `Item::id` is always known,
because Table 151 makes `/First`, `/Next` and `/Last` indirect references — so it became a path
nobody takes. ADR 0138's precedent is explicit about those: the strip driver was reverted whole
because `CLAUDE.md` forbids shipping code no caller reaches.

The honest summary is that ADR 0142 read one half of a sentence and built a command for it. The
remedy is not to keep both commands; it is to keep the one that expresses the whole sentence.
`PageTarget`'s own doc comment records the removal, so the next person to reach for the variant
finds the reason rather than the absence.

## The refactor

`interact::activate(open, x, y)` was one function doing three things: find the link under a
point, perform §12.6.2's action sequence, and resolve whatever page the sequence named. The
middle and last are now `interact::perform`, and both callers use it.

The one parameter that differs is the click position, and the clause decides what happens without
it. §12.6.4.8 on Table 210's `/IsMap`:

> This entry applies only to actions triggered by the user's clicking an annotation; it shall be
> ignored for actions associated with outline items or with a document's `OpenAction` entry.

So `perform` takes `Option<((f32, f32), [f32; 4])>` and the `None` case is not a default — it is
the clause naming outline items in as many words.

## §12.3.3 closes

The row moves from `partial` to `implemented`, and it is the first clause-12 row to close on a
*surface* rather than on a reader. What the clause asks of a screen is now all done: the linked
list's order, the disclosure and its `/Count`, hiding a closed item's descendants, Table 151's
`/C` and Table 152's two style bits, and both halves of the activation sentence. `/SE` stays
unread with the clause's own reason beside it — "[t]his value is not intended for navigation".

Verified in the window as well as in a test: `issue3214.pdf`, whose outline carries a `/URI`,
prints `link: http://google.com` when the row is clicked on `Xvfb`.
