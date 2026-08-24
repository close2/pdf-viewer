# 0573 — The place a refusal took away, and the census that could not say so

Status: accepted, in the seven-hundred-and-seventh session.
Supersedes nothing; it pays one of the two residues ADR 0557 recorded as owed.
Touches `crates/pdf-model/src/content/` (the interpreter, one counter and one struct field),
`crates/viewer-core/src/accessibility.rs` and `crates/viewer-confined/src/protocol/panels.rs` (one
boundary field), and `crates/viewer-core/tests/accessibility_census.rs` (two printed counts and a
witness list). **No pixel, no verdict, and no ratchet moved** — measured, below.

## The problem, in the words of the round that found it

The seven-hundredth session closed the largest instrument defect this project has found: six of
eight corpus gates ran without `pdf-sandbox-worker`, decoded no `CCITTFaxDecode`, `JBIG2Decode` or
`JPXDecode` image, and said nothing about it. What it left behind, in its own words:

> The census counts no reports. A refused image is loud in the interpreter's `Unsupported::Image`
> and silent in the census's own output.

That is not a cosmetic gap, and the same round measured why. ISO 32000-2 §14.8.3.3:

> The content rectangle shall be derived from the shape of the enclosed content and defines the
> bounds used for the layout of any included child elements.

*Derived from the enclosed content* — so an element whose enclosed content was never drawn has no
content rectangle, and for elements whose only place it was, no place at all.
`issue5481.pdf` carries a `JPXDecode` image; with the worker absent the image is refused, and nine
of that document's structure elements move from *placed by their own marks* to *no place by any
route*. The accessibility census counted both ends of that move and could name no cause, which is
what left four rounds diagnosing a deterministic nine-element shift as a build directory, as
staleness and as Cargo feature unification (trap 16).

## The decision

**A refusal is attributed to the marked-content sequence that enclosed it, carried to the
accessibility boundary, and counted by the census beside the places it took away.**

Three things, one per layer:

- **`content::MarkedSpan::enclosed_a_refusal`** — whether any content the sequence enclosed
  produced an `Unsupported` report. The interpreter keeps a monotonic count of calls to
  `Interpreter::note`; `open_marking` records it and `close_marking` compares. A note raised
  between a sequence's `BDC` and its `EMC` was raised inside it, and an enclosing sequence opened
  earlier still, so nesting needs no propagation of its own — the same enclosure §14.8.3.3 gives
  the rectangle.
- **`viewer_core::AccessibilityNode::enclosed_a_refusal`** — the union over the element's own
  sequences *and its descendants'*, which is what `Gathered::mcids` already holds and what
  `marked_extent` already takes.
- **Two printed counts and a witness list** in the census: how many elements enclose a refusal, and
  — per page, with the page's own `Query::Reports` sentence — how many elements have **both** no
  place and a refusal inside them.

### Why the counter and not the map's length

`Interpreter::unsupported` is a `BTreeMap` keyed by the report, so that a page refusing the same
image a thousand times reports it once. Its *length* is therefore the wrong instrument for this
question: a page whose second refusal of `/Im0` falls inside a different structure element than its
first must attribute one to each, and the map would have moved only for the first. The counter
counts calls, is never printed, and exists to be bracketed.

### Why a field on the boundary rather than a join in the census

`doc/ui-boundary.md`'s test is whether a host can work the answer out for itself. It cannot: it has
`Query::Reports`, which says what a *page* could not draw, and `Query::AccessibilityTree`, which
says what each element is, and the join between them runs through §14.7.5.2's marked-content
identifiers, which do not cross the boundary and are not meant to. The census is one consumer of
that field; a screen-reader host is the other, and the thing a person needs is to be told that an
element nothing can point at is one whose picture is missing.

## The condition, which is trap 11's question

**The reflex condition is wrong and was not taken.** It is *an element with no place, on a page
that reported something* — and it fires on every placeless element of a page whose report is about
a font, a transparency group or an annotation somewhere else on the sheet. That is a condition the
clause does not state. What §14.8.3.3 states is **enclosure**, so the refusal is attributed to the
sequences the element encloses and to no others.

**And it claims enclosure rather than cause**, which is stated in the field's own documentation
rather than left for a reader to assume. An element may enclose a refusal and still have a place —
`issue8702.pdf`'s two do, because they drew text as well — and an element may be placeless for
reasons that have nothing to do with a report. What the class says is that this program refused
part of what this element holds, which is the fact that could not be read off the old output at
all.

## What it says now, measured both ways

The conditions, because that is trap 16's own rule: worktree `r707`, one build, one test binary,
`--profile gates`, the corpus's 988 documents. The two runs differ in **one variable** —
`PDF_SANDBOX_WORKER` pointing at a path that does not exist — and the second run needed
`require_the_sandbox()` taken out, which was done for the measurement and put back.

| | placed by their own marks | with no place | enclosing a refusal | no place **and** a refusal |
|---|---|---|---|---|
| the worker beside it | 93 267 | 1336 | **2** | **0** |
| no worker | 93 258 | 1345 | **11** | **9** |

The nine are named, three pages of `issue5481.pdf` at three apiece, and each line carries the
page's own sentence:

```text
issue5481.pdf p1: 3 element(s) with no place enclose content the page could not draw:
  an image (Im0: starting the sandbox worker failed: No such file or directory (os error 2))
  was not drawn
```

**Every other count is identical between the two runs and to the ratchet's floors.** The +9 in the
new class accounts for the whole of the −9 in `placed by their own marks` and the whole of the +9
in `with no place`; the +9 in `enclosing a refusal` is the same nine. That is trap 13's requirement
met the only way it can be: the instrument was run against the defect it was built for, and it
names it exactly.

## What is not done, and why

**Nothing is ratcheted.** ADR 0323's rule is that an instrument's counts enter a gate only once
they have held across rounds, and these are one round old. `placeless_and_refused` is a defect
class and will want a **ceiling**; `refused` is neither a capability nor a defect — it is a
denominator — and probably wants no bound at all. A later round decides both, with two runs behind
it.

## The other two instruments, asked the same question

The seven-hundredth session measured that `selection_census` and `text_extraction` move nothing
when the worker is absent, and asked whether they carry the same silence. Read against the code
rather than assumed:

- **`text_extraction` already counts reports** and prints them: `{incomplete} incomplete and not
  gated`. Its verdict excludes a page that reports, by name and with the reason beside it, and the
  count of excluded pages is on its summary line. There is no silence to break.
- **`selection_census` does not, and the shape means it does not need one in this form.** Its
  denominator is poppler's word boxes, so a refusal that costs us text arrives as a *missed drag*
  rather than as a smaller number — the loud direction. What is owed there is narrower and is
  recorded as owed rather than built: ADR 0421 names eleven drags that still miss in four classes,
  and none of the four is "the page reported something", which is a class that instrument cannot
  currently distinguish.

## The cost

One `usize` on the interpreter, incremented once per `Interpreter::note` — a call that happens at
most a few dozen times on a page and not at all on most — and one `usize` per open marked-content
sequence, which is the stack `ADR 0486` already allocates on a tagged page. Nothing in the
per-command path changed; `Interpreter::draw`'s body is the same union it was, reaching through one
field name instead of a bare `Option`.

One `bool` per node across `viewer-confined`'s pipe, beside the twelve fields already there.
