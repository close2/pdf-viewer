# ADR 0316 — The mode a clause conditions its states on

Status: accepted, 2026-08-13. Session 481.

## Context

ISO 32000-2 §12.4.4.2 has been read since the seventieth session and walked by nothing.
`pdf_model::navigation::steps` reads `/PresSteps`' chain of Table 165 nodes, `ViewState::perform_all`
performs a node's actions, and the ledger row has said since the fifty-fourth session that this was
"the one presentation row whose missing piece is a control rather than a renderer". The
three-hundred-and-ninety-third session drew §12.4.4.1's transitions (ADR 0230) and left this row
`partial` with a sharper sentence: there is a presentation mode now, and the arrow keys still turn
pages where the clause says they should step through a page's states first.

The instruction for this round allowed for the possibility that §12.4.4.2 leaves the states to the
processor and there is nothing normative to build. **It does not.** The clause is a state machine
written in `shall`s, and the reading is the first half of this ADR.

## What the clause states

Read against the standard's own PDF as well as `doc/md/` — the conversion garbles Table 165's first
row and drops nothing:

> An interactive PDF processor shall maintain a current navigation node. When a user navigates to a
> page, if the page dictionary has a PresSteps entry, the node specified by that entry shall become
> the current node. (Otherwise, there is no current node.)

Then a forward request — "[t]he sequence of actions specified by NA (if present) shall be executed"
and "[t]he node specified by Next (if present) shall become the new current navigation node" — and a
backward one with `/PA` and `/Prev`. Then a page: arriving at one whose dictionary contains a
`/PresSteps`, "[t]he navigation node represented by PresSteps shall become the current node", the
request's own actions are executed and `/Next` (or `/Prev`) becomes current, and "[t]he interactive
PDF processor shall make the new page the current page and shall display it. Any page transitions
specified by the Trans entry of the page dictionary shall be performed."

Six `shall`s, one state, four transitions between states. Two NOTEs carry the rest: NOTE 2 asks a
processor to save §8.11's group states when presentation mode is entered and restore them when it
ends, and NOTE 3 says the nodes need be respected "only when in presentation mode".

**And Table 165's `/Dur` was read by nothing at all**: "[t]he maximum number of seconds before the
interactive PDF processor shall automatically advance forward to the next navigation node. If this
entry is not specified, no automatic advance shall occur." That is a `shall` inside a table this
tree already read, in the shape trap 5 calls the easiest to lose — inside a partly-implemented
feature, where the entry beside it is handled and the code path exists.

## The decision, and the one thing it cost

**`viewer-core` keeps a presentation mode, and a host is what sets it.** `Command::Present(PresentationMode)`,
two states, `Off` by default.

**This reverses half of ADR 0135**, which decided the opposite and said so:

> `viewer-core` has no presentation *state*: "is a presentation running" is answered by whether
> something is driving the clock, and a host that stops presenting stops ticking.

That deduction is not merely indirect; it is **wrong for exactly the case §12.4.4.2 is about**. A
person stepping through a slide show with an arrow key drives no clock at all — there is nothing to
advance automatically, and a page with states and no `/Dur` produces no ticks a host would send.
So the deduction answers *no* while a presentation is running, and the clause's condition would
never be met by a document that most needs it.

The rule `doc/ui-boundary.md` states for a new message is that it carry a question a host cannot
answer for itself and never a second way to say something it can. This is the first half exactly,
and the *inverse* direction of the usual one: it is a statement only a host can make. Full screen
is chrome (rule 5), and whether a window is showing a slide show is not a fact about any file.
It joins `Restrict` and `Delegate` as the third policy value in the vocabulary, applies to every
open document, and changes what is drawn — so it is pushed into `ViewState` where rule 1 says a
statement about the view belongs.

Nothing else was added. The forward and backward *requests* are `Command::GoTo(PageTarget::Next)`
and `Previous`, which already mean "the user requests to navigate forward" in this vocabulary, and
the clause's own random access — "such as by clicking on a link" — is the page change
`interact::apply` already performs. No `Event` and no `Query`: what a host draws is the page, and
the page changes for reasons it is already told about.

## What is derived, and what is chosen

Derived, with the clause's words in the code: the current node; `/PresSteps` making the primary node
current on arrival; `/NA` then `/Next`; `/PA` then `/Prev`; random access counting as forward;
Table 165's `/Dur` as a **second** clock beside §12.4.4.1's, because the standard states two maxima
and a file whose page duration is shorter than its steps take has said the page turns; NOTE 2's save
and restore, which is one `ViewState::optional_content_snapshot` and its inverse; NOTE 3's condition,
taken as the permission it is.

Two things the clause does not decide, decided here and written down as choices:

- **Running off either end of the list leaves no current node**, so the request after the last state
  turns the page. The clause says `/Next` "(if present)" becomes current and says nothing about a
  node that has none. The alternative — leaving the last node current — re-executes its `/NA` on
  every further request and no page could ever be turned, which is not a reading anybody can hold.
- **Entering the mode on a page nobody navigated to makes that page's primary node current.** The
  clause names the arrival as what sets the node, and entering the mode is not an arrival; but a
  processor that then had no current node would not be "maintain[ing] a current navigation node",
  which is the sentence the subclause opens with. Nothing is executed, because nothing was requested.

## A defect the reading found, which no gate could see

§12.4.4.1's `/Trans` was played **only** by the clock's own advance. The entry is "the style and
duration of the visual transition to use when moving from another page to the given page during a
presentation" — moving, not moving automatically — and §12.4.4.2's step (c) says it again with a
`shall`. So a person pressing an arrow key through a slide show saw every effect the file asked for
skipped, silently, since the session that drew the first frame. It is fixed on the same predicate
the nodes use, and the two are gated differently on purpose: the node walk needs the mode, because
walking it changes §8.11's groups and the obligation to put them back is discharged by entering the
mode; the transition needs the mode *or* an automatic advance, because a `/Dur` running out is a
presentation running and that is the one thing a tick alone does say.

## Population

**No document exercises any of this.** `pdf-model/examples/presentation_census` over the page tree
of every document this tree opens: 985 documents, 1978 pages, **0 stating `/Trans`, 0 stating
`/Dur`, 0 stating `/PresSteps`**. This is trap 8's own case — a corpus finds what documents contain
rather than what the standard says — so every witness is hand-built, and the two files
`viewer-core/tests/sub_page_navigation.rs` writes differ in the single entry `/PresSteps`, which is
what makes the test about the entry rather than about a page turn.

## And it reaches the program

`doc/habits.md` names "a capability that reached the crate and never reached the program" as a shape
this project keeps being caught by, so `viewer-ui`'s `p` sends the mode, and
`pdf-model/examples/presentation_fixture` writes a fourth slide with two bullets in §8.11.3.1 marked
content and a two-node `/PresSteps` chain that turns them on. Driven under `Xvfb` with `xdotool`:
pressing `p` and then `Right` twice makes one bullet appear and then the other, on the same page,
and the third `Right` turns it.
