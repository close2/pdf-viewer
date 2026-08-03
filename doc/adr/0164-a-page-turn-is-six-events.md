# ADR 0164 — A page turn is six events

Status: accepted, 2026-08-03. Session 204. The second finding of the capability sweep, from the
same row family as ADR 0162.

## What was owed

`doc/todo/25-view-dependent-annotations.md` listed six of Table 197's ten trigger events as
unraised, in two groups:

- `/Fo` and `/Bl` want **keyboard focus**, which `viewer-core` has no vocabulary for.
- `/PO`, `/PC`, `/PV` and `/PI` want a **page-visibility model**, "which a one-page-at-a-time
  window does not have".

The second reason is the capability shape again, and it is wrong in the same way §12.3.2.1's was:
a window that turns pages *is* a page-visibility model — the one with exactly one member.

And beside them, Table 198's `/O` and `/C` were **read and never raised**:
`pdf_model::action::for_page` had existed since the seventy-seventh session with no caller
anywhere in the tree. That is not a capability gap at all; nobody had connected it.

## What the clause states, including an order

§12.6.3's Table 197 puts an ordering requirement on two of the four:

> The action shall be executed after the O action in the page's additional - actions dictionary
> (see "Table 198 - Entries in a page object's additional - actions dictionary") and the
> OpenAction entry in the document Catalog (see "Table 29 - Entries in the catalog dictionary"),
> if such actions are present.

and, of `/PC`, that it shall be executed *before* the page's own `/C`. So a page turn is six
things in one order:

```text
leaving page:   its annotations' /PC, then /PI      then the page's /C
arriving page:  the page's /O                       then its annotations' /PO, then /PV
```

`Viewer::page_events` is that sequence, and `interact::page_trigger` is what finally calls
`action::for_page`.

## `/PV` and `/PI` are derived, not conceded

§12.6.3 says outright why the pair exists:

> The PV and PI entries allow a distinction between pages that are open and pages that are
> visible. At any one time, while more than one page may be visible, depending on the page
> layout.

**This viewer shows one page at a time**, so "open" and "visible" are the same one-element set and
the two events coincide with `/PO` and `/PC`. That is a reading of the clause rather than a
shortcut around it, and the place a continuous-tower host would separate them is the function
that raises them.

NOTE 1 is honoured by *not* consulting Table 167 at all — "[f]or these trigger events, the values
of the flags specified by the annotation's F entry … have no bearing on whether a given trigger
event occurs" — so a Hidden annotation's `/PO` is performed. That is the opposite of the gate
`interacts` applies to the pointer's four events, and the clause states the difference.

## Nothing cascades, and that is a decision

An action performed by `/PO` may turn the page. Raising the six events again for *that* turn
would let a document whose `/PO` points at the next page walk the whole file, and §12.6.2 bounds
nothing. `Viewer::raising` is the bound. It is a flag rather than a depth counter because one
level is the only thing the clause describes: a turn a person made.

## Measured: what the corpus states

`the_corpus_states_these_page_scoped_triggers` walks all 974 documents:

| | |
|---|---|
| Table 198's `/O` and `/C` | 3 pages, all of `doc_actions.pdf` |
| `/PO` and `/PC` | 1 annotation, `issue18305.pdf` object 25 |
| **`/PV` and `/PI`** | **none** |

The last row is what the clause predicts. Those two entries exist to separate open pages from
visible ones in a multi-page layout, and a producer writing for a reader that shows one page has
no use for them — so the two events this viewer folds together are exactly the two nothing here
exercises. Held by name in both directions, because a corpus that stops exercising a feature is
as much news as one that starts.

## The check

`a_page_turn_raises_the_events_the_clause_orders` builds a three-page fixture where every one of
the six performs a `/URI` action naming itself. A URI is the one action this crate hands *out*
rather than acting on, so the events arrive in `Event::OpenUri` in the order they were raised and
the assertion is a list of six strings — which is the clause's sentence written as a test.

## What is left of §12.6.3

`/Fo` and `/Bl`. There is no focus model in `Command` at all, and adding one is a vocabulary
change rather than a clause — `doc/todo/25` keeps it.

## Alternatives rejected

- **Raise `/PV` and `/PI` only when a host says a page is visible.** That needs a `Command` the
  vocabulary does not have, for a distinction this layout cannot express and no corpus document
  states.
- **Let the cascade run with a depth bound.** A bound above one is a number nothing derives, and
  the clause describes one turn.
- **Raise the events from `announce_page`.** It is called for a page change *and* for the
  document opening, and the two need different `left` pages; making it decide would put the
  clause's ordering inside a function whose job is one event.
