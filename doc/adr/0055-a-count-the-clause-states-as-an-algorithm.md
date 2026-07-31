# ADR 0055 — A count the clause states as an algorithm

Status: accepted, 2026-07-31.

## Context

§12.3.3's document outline is the second thing downstream of ADR 0054's destinations and the
third of the four rows ADR 0053's name trees unblocked. `CLAUDE.md` names it in scope — "outlines,
destinations, page labels" — and **176 of the 974 corpus documents have one**.

It is also the first item in this project's history whose entire purpose is a *panel*, in a viewer
that has none. That is the design question, and it has a good answer.

## Decision

**Read the whole hierarchy; use the one part of it a viewer without a panel can use.**

`Outline::read` produces the tree: Table 151's `/Title` as a §7.9.2.2 text string, a destination
from `/Dest` or from a go-to action's `/D`, Table 152's italic and bold bits, `/C` clamped to
`DeviceRGB`, and children. `Outline::section_at` is what `viewer-ui` calls: **the innermost item
covering the page being drawn**, shown in the title bar beside the page number and the §12.4.2
label. A person reading page 231 of a manual now sees which section they are in.

That mapping is a **documented choice, not a clause**: §12.3.3 describes a panel a person clicks
and says nothing about going the other way, from a page back to an item. Two sub-choices go with
it and are written where they are made — an item whose destination does not resolve is skipped
rather than treated as covering nothing, because a heading with a broken link still names the
section after it; and where several items land on one page the last in reading order wins, because
nothing orders them within a page except the order the file wrote them in.

**Follow `/First` and `/Next`, and nothing else.** The clause threads each level as a doubly-linked
list with `/Prev` and `/Next`, entered by `/First` and `/Last`, with every item pointing back at
`/Parent` — six indirect references per item, any of which a producer can get wrong. `/Prev`,
`/Last` and `/Parent` are redundant with what the forward walk already has; reading them could
only disagree with it. The walk is bounded in depth and item count and refuses to visit an object
twice, which is what makes a `/Next` chain that points backwards terminate.

## The finding: `/Count` is an algorithm, so the file can be checked against itself

Table 151 does not define `/Count` as a number. It defines it as three numbered steps:

> Step 1. Initialize Count to zero. Step 2. Add to Count the number of immediate children. During
> repetitions of this step, update only the Count of the original outline item. Step 3. For each of
> those immediate children whose Count is positive and non-zero, repeat steps 2 and 3.

"[T]hose immediate children whose Count is positive" are the open ones, so the number counts every
item reachable without opening anything closed — and the outline dictionary states the same number
for the whole tree. **A document therefore states one fact twice**, which is the habit this project
learned from LZW stream lengths, from ninety-six JBIG2 encodings of one image, and from a font
whose `loca` length confirmed its own `indexToLocFormat` (ADR 0052). Here it is a check on *us*: a
walk that lost a level, took a closed item's children as visible, or ran off the end of a `/Next`
chain would disagree with every producer that ran the same steps.

**144 of the 146 corpus documents that state a `/Count` over a non-empty outline agree with the
steps run over their own items.** The two that do not contradict *themselves*:

| | states | the steps give |
|---|---|---|
| `nested_outline.pdf` | 3 | **9** — its three top-level items each carry `/Count 2`, which by step 3 makes them open and their six children visible |
| `outline_goto_action.pdf` | 1 | **2** — one parent with `/Count 1`, therefore open, and one child |

Both are hand-written pdf.js fixtures whose root count was written rather than computed. Neither
is evidence about the clause, and neither is fixed by changing anything here: a number counted
from the items is the clause's, and a number written beside them is a claim.

**Twenty-six documents state an `/Outlines` and yield no items**, and that is also the file rather
than the reader: every one is `<< /Type /Outlines /Count 0 >>` with no `/First`, which Table 150
permits — the entry is "[r]equired if there are any open or closed outline entries". The test
asserts that per document rather than tolerating the count, so an outline root *with* a `/First`
that produces nothing would fail.

## Consequences

- §12.3.3 is `partial`: the hierarchy is read, the panel does not exist, and `section_at` is what
  a title bar can carry.
- 343 outline items are read across the corpus and 305 of them name a page — the same two-sided
  shape ADR 0054's named destinations have, and for the same reason.
- One row of the four ADR 0053 unblocked is left with the tree still to do: §12.7.7's named pages
  and §14.7.5.4's `/ParentTree`.
