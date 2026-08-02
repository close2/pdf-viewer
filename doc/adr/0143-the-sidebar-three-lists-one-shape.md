# ADR 0143 — The sidebar: three lists, one shape

Status: accepted, 2026-08-02. Session 167. The other two panels, and the one place the three
differ.

## What this adds

ADR 0142 drew §12.3.3's outline. This makes it a **sidebar with three tabs** and fills the other
two: §8.11.4.3's `/Order` of optional content groups, and §7.11.4's embedded files. All three
answers have existed since the hundred-and-thirty-first session and none of them had a consumer.

The reason they are one piece of code is not economy. They are one *shape* — indented rows, a
marker at the left edge, a label, and something a click does — and writing them as one makes the
places they differ explicit rather than incidental.

## Where they differ, and each difference is a clause

**An outline item discloses; a layer collection does not.** §12.3.3 says the user "may
interactively open and close individual items" and that a closed item's descendants "shall be
hidden", and gives an item a `/Count` whose sign states which it starts as. §8.11.4.3 says the
opposite kind of thing about `/Order`: a nested array is "the order of the optional content
groups", and a nested array with a leading string is a label "not to communicate actual
nesting". Neither sentence describes something a person folds away. So an outline row carries a
triangle and a layer collection carries a heading, and the panel does not decide what a person
may see.

**A layer's marker is a switch, and the clause can forbid throwing it.** Table 99's `/Locked`:

> The state of a locked group cannot be changed through the user interface of an interactive PDF
> processor.

So a locked group's switch is **drawn** — a person is entitled to see the state — and clicking it
produces nothing. Drawing no switch at all would answer a different question, and hiding the row
would answer a worse one. `ViewState::set_group` already refused the change (§8.11.4.3's row);
this is the same refusal one layer up, where a person can see it.

**An attachment row acts on nothing, and that is written down rather than left to be noticed.**
§7.11.4's list is read and shown; taking the bytes *out* is not built. `Query::Attachments` hands
over the stream undecoded and this program has no file dialogue, so a click is answered and does
nothing — answered, because a click falling through to the page underneath would start a text
selection on a page nobody can see.

## What the empty case says

Each tab, given nothing, draws a sentence: "This document states no outline.", "…no optional
content.", "…embeds no files." An empty panel and a panel this program failed to fill look
identical on a screen, and only one of them is a fact about the file. This is trap 5 applied to
an interface: the absence is reported rather than shown as blank space.

## The one caching rule, and it is not about speed

The outline and the attachment list are copied out of their queries **once, when the document
opens**; the layers are asked for on **every** call. Both halves have a reason and neither is
performance:

- The copies exist because `Answer::Outline` borrows the viewer, and a panel that is about to
  send it a `Command` cannot be holding a borrow. They are safe to copy because
  `pdf_syntax::Document` is immutable (`CLAUDE.md`'s rule 1) and no edit reaches §12.3.3 or
  §7.11.4.
- The layers are *not* safe to copy, because the whole point of the tab is that a click changes
  them. A copy taken at open would be the one thing on the panel that lies.

The same distinction decides where `Content` is built: the immutable-borrow helper is used for
drawing, and the struct is written out by hand at the three sites where `self.panel` is borrowed
mutably, because only a *field* borrow is disjoint from it.

## How it is checked

`viewer-ui/tests/panel.rs`, with no display: the switch on an unlocked group answers
`SetGroup { on: !on }`, the switch on a locked one answers `Nothing`, **and all three rows are
still drawn** — the second assertion is the one that would catch "refuse the click by hiding the
state". Ink is counted under 180 of 255, which takes black text and the 107 a dimmed row is set
in and takes neither the 240 background nor the 219 hover.

Driven on `Xvfb` as well (ADR 0126), because no test in this tree presses a key:
`issue12007_reduced.pdf` shows three groups with the first on, and clicking the second's switch
makes its image appear on the page. That is §8.11's interactive half working end to end, and it
is the first time anything in this project has done it.
