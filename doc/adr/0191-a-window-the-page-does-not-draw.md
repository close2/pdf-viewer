# ADR 0191 — A window the page does not draw

Status: accepted, 2026-08-05 (session 312).

## Context

`doc/todo/01`'s capability sweep produced §12.5.1, whose `partial` note ended:

> What is left is one piece of interaction this program does not have: opening an annotation to
> exhibit its object.

That is the sweep's own tell — a row explaining itself by naming something the *program* lacks
rather than something the *standard* leaves open — and it had the usual fuse. The clause's sentence
is:

> When the user activates the annotation by clicking it, it exhibits its associated object, such as
> by opening a popup window displaying a text note ("Figure 77 -Open annotation") or by playing a
> sound or a movie.

A press and a release on one annotation is what this program has had since the
hundred-and-thirty-second session. §12.5.6.14's row said the same thing from the other end — "/Open
and /Parent are the interactive half" — and named neither a reader for those two entries nor a
caller for one, which is the shape ADR 0177 found for §12.5.6.19's `/H`.

**The corpus was measured before anything was built**, over all 974 documents' first fifty pages:

| | |
|---|---|
| documents stating a popup | **45** |
| popups | 128 |
| popups Table 186's `/Open` opens with the page | **7**, on 2 documents |
| popups with text to show | 79 |
| popups with no `/Parent` (the clause's NOTE 3) | 4 |
| `/Popup` references naming an annotation `/Annots` does not list | **0** |

The last row is why `popup::popups` walks `/Annots` and does not chase Table 172's entry from the
markup annotation's end: the two routes reach the same 128, and the page's own array is where
§12.5.2 puts an annotation.

## Decision

**Read the popup in `pdf-model`, place it in `viewer-core`, draw it in `viewer-ui` — because
§12.5.6.14 says a popup "shall have no appearance stream".**

A clause that refuses an annotation an appearance stream has said the thing is not page content.
Nothing in `crate::appearance` can construct one, neither backend can be compared on one, and the
oracle's 1794 pages cannot see it — so the division is the one §12.3.3's outline already has: the
model reads what the document says, the core says where it is on the screen, and the host decides
what a window *looks* like. A native host draws a real window with the platform's furniture; this
one draws a card with a title bar, and every number in it is written down as this host's choice.

Three readings are worth naming because each could have gone the other way.

**Table 186's override is a `shall` and is obeyed at the reading, not at the drawing.** "[T]he
parent annotation's Contents , M , C , and T entries … shall override those of the popup annotation
itself", so `popup::read` takes all four from the parent where there is one and from the popup where
there is not — which is exactly the clause's NOTE 3, "[t]he Contents entry for a popup annotation is
relevant only if it has no parent". Four of the corpus's popups are that case.

**§12.5.3's flags are the popup's own, and one corpus document exists to say otherwise.**
`pr7352.pdf` carries a note in its own `/Contents`: "This Popup annotation, which by itself isn't
viewable, should fallback to inherit the annotation flags of the parent annotation." That is another
reader's rule, and principle 5 decides it: Table 186 enumerates exactly four entries the parent
overrides and `/F` is not among them, so a table that names four has said something about the fifth.
(The file's own flags make the question moot — `/F 25` is `Invisible`, `NoZoom` and `NoRotate`, and
`Invisible` "applies only to annotations which do not belong to one of the standard annotation
types", which `Popup` is.)

**Table 166's `/M` crosses as the string the file wrote.** "The format should be a date string as
described in 7.9.4, "Dates" but interactive PDF processors shall accept and display a string in any
format" — a `shall` about *displaying*, which a reader that parsed the entry and dropped what would
not parse would break. `Popup::modified` is the bytes and `Popup::modified_date` is the parse, so
both halves are available and neither is imposed.

**What a second click does is a choice.** The clause says *exhibits*; it says nothing about closing.
Closing is what every reader of a sticky note expects, and `Open::toggle_popup` is where it is
written down. The state lives in `viewer-core` and not in the document, because Table 186's word is
"initially": the file states the first frame, and `CLAUDE.md`'s rule 1 puts everything after that in
a log beside the file.

## Consequences

- **§12.5.6.14's window opens.** `Query::Popups` answers with a rectangle in device pixels and the
  strings the document states; `Command::Activate` on the markup annotation or on the popup toggles
  it, so a host with a keyboard or a comments panel needs no pointer.
- **Four ledger rows move**: §12.5.1's expired sentence is replaced by what actually keeps it
  `partial` (Table 171 has subtypes `CLAUDE.md` excludes), §12.5.6.14's "interactive half" names its
  reader and its callers, §12.5.6.4 loses one of its three `partial` reasons, and §12.5.6.2's `/T`
  and `/Contents` reach the window the clause says they are for.
- **A defect in §12.6.3 came out of the same arm.** Table 197's `/U` is "an action that shall be
  performed when the mouse button is released inside the annotation's active area", and this crate
  raised it only where the release *also activated a link*: the release arm returned early whenever
  the press had not landed on one. A click on a stamp, a widget or a markup annotation raised
  nothing. The table conditions the event on the release position and on nothing else, so it is
  raised before the link question is asked now, with the one exclusion the table itself states.
- **One arithmetic is shared rather than copied.** `Viewer::device_quad` is the mapping
  `Query::Focus` already used, now called by both — the origin, the magnification and the y flip
  that were wrong for seventy-five sessions (ADR 0118) exist once.
- **A silence in this host's own chrome is now loud.** `Chrome::text` draws nothing for a character
  its face states no code for, which is right for a title being elided and wrong for a document's
  note: six of the corpus's seven open popups are in Chinese. `Chrome::without_a_code` counts them
  and the window says how many it could not set. **The same silence is still there for outline
  titles, layer names and attachment descriptions** — `doc/todo/27` carries it, with the three answers and
  what each costs.
