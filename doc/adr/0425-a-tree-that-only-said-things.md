# ADR 0425 — A tree that only said things, and the queue nobody could drain

Status: accepted, 2026-08-18. Session 590. Takes `doc/todo/31`'s last AccessKit-surface entry —
**actions** — which ADR 0394 called "the sharpest entry left on the file". Amends §14.7's,
§14.7.5.3's, §14.8.4.7.2's and §12.5.1's ledger rows; corrects four places in `crates/` that
quoted a sentence Errata Collection 3 struck out; and closes `doc/todo/05`'s third instrument by
ratcheting the accessibility census. Extends ADRs 0214, 0338 and 0394; changes nothing ADRs 0300,
0301, 0312 or 0325 decided.

## What was there, and what was missing

Since ADR 0214 a screen reader can walk this program's page: §14.8.4's types as `accesskit::Role`,
§14.9.3's `/Alt`, a `TH`'s axis, a cell's headers, a `Form`'s control, and since ADR 0394 a caret
that moves through the text by character, word and line. All of it is the tree **saying** things.

Not one node declared an `accesskit::Action`, so a conforming client had nothing to request, and
`doc/todo/31` had recorded the consequence twice over: a check box that announces itself as a check
box "invites exactly the request this tree declines" (ADR 0338), and a page that says a caret may
move through it "invites `SetCaretOffset` as surely as a check box invites a click" (ADR 0394). A
person using only a screen reader could hear this document and could not tick a box in it.

## Decision 1 — three actions, each on the condition that makes it answerable

Not on the role that suggests it, which is the mistake available here: `Role::Link` looks
clickable, and an element mapped to it may name no annotation at all.

| action | declared on | the clause |
|---|---|---|
| `ScrollIntoView` | an element that has a place | a scroll takes a rectangle; an element with no place names none |
| `Click` | an element whose content **is** an annotation | §12.5.1: "[w]hen the user activates the annotation by clicking it, it exhibits its associated object" |
| `SetTextSelection` | the page node, where it has text runs | the interface a caret moves through is on that node and nowhere else (ADR 0394) |

**The second needed a new fact to cross**, and that is the interesting half. `AccessibilityNode`
already carried *where* such an element is (ADR 0301, ADR 0338) and *what control* it is (ADR
0338); neither says **that it is an annotation**. A rectangle cannot: a `Figure` has one from Table
379 and a caption has one from its own glyphs. So `AccessibilityNode::annotation` is the first
annotation of this page that the element's own §14.7.5.3 object references name — one object rather
than the union `bounds` takes, because an action needs one thing to act on.

Table 368 is why the population is all three of its annotation types rather than `Form` alone, and
**it is the erratum's wording that makes it obvious**: `Annot` "[e]ncloses one or more PDF
annotations and associated content, if any" and `Form` "[e]ncloses a PDF widget annotation and
associated content, if any" (Issue #437), with §14.8.4.7.2 giving a `Link` element "[o]ne object
reference (see 14.7.5.3, "PDF objects as content items") to one link annotation associated with the
content". The *association* wording this tree had been quoting says the same thing the other way up
and reads as though the content were the point.

## Decision 2 — what an action means is a **place**, and the boundary gains no message

`viewer_accessibility::Act` is `Show { at: [f32; 4] }`, `Click { at: (f32, f32) }` and
`Caret { from, to }`, all in the viewport's device pixels — the space
`Query::Selection` and `AccessibilityNode::quads` are already in. The host sends
`Command::Scroll` and `Command::Pointer`, which it already had.

That is `doc/ui-boundary.md`'s test passed rather than dodged: a message is added for *a question a
host cannot answer for itself*, and "where is this node" is a question the tree has already
answered. Seven consumers, still no host has asked for a message.

**A click is a point and not an object**, and the alternative was considered and is wrong.
`Command::Activate(ObjectId)` performs an annotation's `/A` and `/Dest`; a check box states
neither, so activating a widget that way does nothing at all. What ticks a box in this program is
§12.7.5.2.3's appearance-state name sent as `Edit::SetField`, which the host works out from the
*point* (ADR 0235). Routing the request through the point therefore gets §12.6.3's triggers,
§12.5.5's appearance state, §12.7.5.2's toggling and §12.5.6.5's link — the whole of what a click
means — from one definition.

**One definition, literally**: `App::click_page` is what the mouse handler now calls and what an
action request calls, so a screen reader's click cannot drift into being a different click from a
person's.

## Decision 3 — the request is resolved against the tree the client walked

`Bridge::requested` looks the node up in the published `TreeUpdate` rather than in a second copy of
the page built beside it. One model instead of two, and it cannot disagree with what the assistive
technology was told — which a second copy could, in exactly the window between a page turn and the
request that followed it. A caret position is the inverse of the arithmetic `tree::along` wrote:
the run's rectangle plus the character's offset along its own reading direction, taken at the
character's **leading** edge because that is what `viewer_core::select::position_at` reads as the
offset in front of it.

## Two defects the round found, neither on any list

**The drain could not run.** `Bridge::requested` was called from `App::speak`, which runs only when
the page or the viewport changes. A request arriving while a person reads one page waited for a
page turn; on a still window it waited for ever. And nothing woke the loop either: the queue is
filled from `accesskit_unix`'s own thread and the window rests in `ControlFlow::Wait`. Both halves
are fixed — `Bridge::new` takes something to wake the host with, `pdf-viewer` gives it winit's
`EventLoopProxy`, and the loop's user event is what drains. It is the only user event this program
has.

**A tree that went stale on an edit.** With the click working, the bus said the box was still
unticked: `App::attend` compares the page and the viewport, and an edit moves neither. So a check
box a person ticked *with the mouse* has been announced as unticked by every screen reader since
ADR 0214, and the round that added a click to the same box would have shipped it. `Event::Dirty`
looked like the condition and is not — it fires when the flag *changes*, so only the first edit of
a session raises it, which the bus showed by ticking one box correctly and the next one not at all.
The condition is the *command*: `Command::Edit`, `Undo`, `Redo` and `SetGroup` forget what was last
published.

## The erratum, which was live in the clause family and half-known

`cargo run --release -p spec-errata -- emit doc/*.pdf` over clause 14, before writing. Issue #437
strikes Table 368's `Annot` and `Form` descriptions and replaces both. **The ledger row for
§14.8.4.7.2 has recorded that since session 418 — and then quoted the struck sentence itself, two
sentences later**, and four places in `crates/` quoted it as current text, and so does ADR 0338.
`spec-errata check` sees the ADR and the ledger and not the four in `crates/`, because those are
prose broken across `//!` lines. All five that this round may touch are corrected; the ADR is left
as the record of its own session.

The lesson is narrower than "run the errata check": **a row that records an erratum is not a row
that has applied it.** The correction and the claim it corrects lived in one note, eight sentences
apart, for a hundred and seventy sessions.

## How far it was verified, and it is the bus

`doc/verify.md`'s recipe, with a client that walks `org.a11y.atspi.Accessible` from the registry
root and then asks for things.

`annotation-button-widget.pdf`, whose nine `Form` elements label their own answers:

```text
role=73 name='Check box, unchecked'   (the paragraph beside it)
role=7  name=''  ifaces=[Accessible, Action, Component]  actions=['click']
```

Nine nodes carry `org.a11y.atspi.Action`, each with one action named `click`, and `DoAction(0)` on
three of them gives the three right answers — read back off the bus as `GetState`:

| | before | after | and the program said |
|---|---|---|---|
| check box, unchecked | `Checkable` | `Checkable, Checked` | setting the field to `1` |
| check box, checked | `Checkable, Checked` | `Checkable` | setting the field to `Off` |
| check box, read-only | `Checkable` | `Checkable` | the field is read-only (Table 227) |

The third is the document's own restriction being obeyed and said out loud, from a request this
program invited.

`doc/ISO_32000-2_sponsored_EC3.pdf`: the page node answers `CharacterCount` 512,
`Text.SetCaretOffset(5)` returns true and the trace says `carried out SetTextSelection`;
`Component.ScrollTo` on a link already on the screen returns true and the extents do not move,
which is the designed answer and not an absent one; and `DoAction` on the cover's two `Link`
elements **opened both URIs** — §12.5.6.5 into §12.6.4.8, from a screen reader's click.

**What is still not verified here**: no screen reader was run, because Orca is not installed on this
machine. What a person on a desktop should do is run one and listen.

## The census, and `doc/todo/05`'s third instrument

`tools/state.sh accessibility` gains one line — the elements whose content is an annotation, 7413
of 102 853 — and **becomes a ratchet**, which is what ADR 0323 designed it as and what `doc/todo/05`
still owed. The rule it was waiting on is met: every count was unchanged from ADR 0342's round to
ADR 0394's, which added a caret to all of them without moving one. A capability count now has a
floor, a defect class a ceiling, and the population is checked before either — a tree without the
`doc/pdf.js` submodule prints why it is not ratcheted instead of failing for the one reason that is
not a regression.

## The lesson

**A tree that only says things is half an interface, and the half it is missing is invisible to
every test that reads the tree.** Nothing in this project could have found that a check box could
be heard and not clicked: the `TreeUpdate` was correct, the roles were correct, the states were
correct. What found it was asking the bus to *do* something and watching what came back — and the
same run then found two defects behind the first, one of which had been shipping since ADR 0214 and
belonged to the mouse rather than to the screen reader.
