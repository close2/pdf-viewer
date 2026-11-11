# 2805 — The window keeps a history of views, and `app.goBack` walks it

Session 1997, a ledger round (§12.3.2.1, JavaScript `app`). Status: implemented. Lifts
`doc/todo/76` row 4, released by `doc/questions/A800` item 2 (ADR 2800). Builds on ADR 1736, whose
section 1 held `app.goBack`/`goForward` in `surface::REFUSED` "until a window keeps a history of
views"; ADR 1736 is a record and keeps its words.

## Context

ADR 1736 found the pair's premise missing rather than hard: no window kept a history of views, so a
person had no previous view either, and a script's request would reach nothing. Nothing else held
it — no purchase, no owner's decision — so A800's "build now" makes it owed. Adobe's *JavaScript for
Acrobat API Reference* (adobe/dc-acrobat-sdk-docs at `ab3b42a7`, "app methods") has `goBack` go to
the previous view on the view stack, the same as the toolbar's Previous View button, and `goForward`
to the next; it is cited, never quoted (owner, 2026-08-28). ISO 32000-2 keeps no history: §12.3.2
says what a destination is — "a particular view of a document", its page, the window's location on
it and the magnification — and §12.3.3, §12.5.6.5 and §12.6.4.2 say when a reader is taken to one;
no clause says what becomes of the view left. So every rule below is a documented choice under
principle 5.

## Decision

1. **The history is `viewer-core`'s** (`crate::history::History`), one per open document, so every
   host — `viewer-gtk`, `viewer-qt`, the WinUI window through `viewer-ffi`, `quorra` and the
   confined window — offers the same one. Per document, because a view names a page of a document;
   §12.6.4.4's embedded go-to replaces the document and its views with it.
2. **A view is a `Viewing`**, §12.3.2.1's three items as `Query::View` answers them — page, zoom,
   clamped scroll — and a step restores it through `Command::View`'s own path, so back is exact.
3. **A jump leaves a view, a step does not.** A destination followed (link, outline item, go-to
   action, a page's or a document's action — `Viewer::apply`), a page named outright
   (`PageTarget::Index`, `First`, `Last`, and a script's `this.pageNum`) and a script's
   `gotoNamedDest` record the view left. Next and previous, a relative turn, scroll and zoom do
   not: each is undone by its opposite step, and recording them would make back walk page by page.
   In §12.4.4's presentation nothing `apply` does is recorded, since there every turn is a
   navigation node's action and the turns are the slides' steps.
4. **Two stacks**, as a browser's: a jump clears what lay ahead; a step puts the view it leaves on
   the other stack; a view equal to the latest behind is not recorded twice. Each stack holds 256
   (`MAX_VIEWS`), the oldest forgotten past it, because a document's own `/Next` chains and timed
   scripts can jump without a person (principle 3's budgets).
5. **`Command::History(Retrace)`** is the one message; it crosses `viewer-confined`'s frame as
   command kind 52. A step with nowhere to go moves nothing and says so (`Event::Reported`).
   A step clears the destination views still waiting for `settle`, since the view it restores is
   whole: a script's `gotoNamedDest` and `app.goBack` in one run otherwise end with the
   destination's view applied over the restored one.
   `viewer_host::keys` binds it to Shift with the left and right arrows, the keys that turn pages
   and the one modifier the table reads; no clause names a key.
6. **`app.goBack()` / `app.goForward()`** leave `surface::REFUSED` for the bridge
   (`engine/window.rs`). A run's calls cross as one `ViewChange::History(steps)` — negative back —
   summed at the first call's place (wire version 15), and the window takes those steps as a
   person's, stopping at the first with nowhere to go. They run only where the reader's script
   level lets a script run at all (ADR 1616).

## Consequences

- §12.3.2.1's note names the history; its status (`implemented`) does not move — the history is
  what a reader is offered, not a requirement newly met.
- A step back across §12.6.4.3's or §12.6.4.4's change of document is not offered: the document left
  is closed. §12.6.4.11's `GoBack`-style named actions are not in Table 211, and none is added here.
- No `Query` says how deep each stack is; a host with Back and Forward buttons would need one.
