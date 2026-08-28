# ADR 0737 — A view is a question, and the answer has to go back

Status: accepted, 2026-08-28. Session 805. Takes the piece `doc/todo/15` named after ADR 0734: the
exact restore of the reader's view across a confined worker's death. Cites no clause — this is
`CLAUDE.md` principle 3's boundary reaching a person, as ADRs 0713, 0725, 0729 and 0734 were — and
the ledger is untouched.

## The subject: what a boundary can and cannot say

ADR 0734 made a worker's death a refusal of the *page* rather than of the document, and named its
own limit precisely:

> **The magnification and the position on the page are not restored.** Nothing on this boundary asks
> the viewer what they are — no `Query` asks for the magnification or the offset — so a host can
> only replay what it issued, and `Zoom::In`/`Out` and `Scroll` are relative commands the viewer
> clamps.

That sentence contains two defects and names one of them. The named one is that the boundary has no
*question* about the view. The unnamed one is that it has no way to **state** one either, and the
second is the harder half: `GoTo(Index)` is absolute and `Zoom::Scale` is absolute, but the third
part of a view — how far the page is scrolled under the viewport — has only `Scroll { dx, dy }`,
which is a delta.

## Decision 1: `Query::View` → `Answer::View(Viewing)`

`Viewing` is the page, the magnification and the scroll, as one value. One value rather than three
because the scroll is measured from the top of the *current page's row*, so a scroll paired with a
different page is not a smaller truth but a wrong one, and the magnification is what the scroll's
device pixels are pixels *of*.

**It passes `doc/ui-boundary.md`'s test for a question — a host cannot answer it for itself — and
the interesting part is that its nearest neighbour looks as though it could.**
`Query::PageGeometry` already answers with a `scale` and an `origin`, and neither is this:

- `scale` is device pixels per user space unit — the magnification and the display's scale
  multiplied together — so recovering the first means dividing by the second, and `Open::magnification`
  refuses that round trip in its own comment: it "is not the identity in `f32`, and this is the one
  place where a pixel of error becomes a scrollbar". It also cannot say `FitWidth`: a *mode* survives
  a resize and the number it resolves to does not, so a host handed the number restores this window's
  picture and loses the next window's.
- `origin` is where the raster *ended up*. For a page smaller than the viewport it is the centre and
  says nothing about the scroll at all; for a continuous arrangement it is laid out from the anchor
  row's top. Inverting it would be a host holding a second opinion about `viewer_core::layout`'s
  arithmetic, which is the one thing that module refuses to have two of (ADR 0118).

## Decision 2: `Command::View(Viewing)`, and why the three commands a host already has do not do it

The alternative is a replay: `GoTo(Index(page))`, then `Zoom { zoom }`, then a `Scroll` of the
difference between where the viewer now is and where it should be — which needs `Query::View` asked a
second time and nothing else new. It was priced rather than dismissed, and it loses on three counts:

- **It is not exact.** The delta is `want - have` and the viewer computes `have + delta`, and
  `a + (b - a) == b` is not an identity in `f32`. Over two million uniform pairs in a device pixel's
  range (0 to 60 000), **16.5% do not round-trip**, the worst by 0.0039 px — which is a fraction of a
  pixel of placement most of the time and a whole one when the value sits near a rounding boundary.
  A restore that is *nearly* where the reader was is exactly the class of answer this project does
  not ship silently.
- **It makes a host know this crate's internals.** A page turn zeroes the scroll and a zoom moves it,
  so the three have exactly one correct order, and a host that got it wrong would work for every
  document whose page it did not have to change. Rule 5 keeps a toolkit out of `viewer-core`; this
  keeps `viewer-core` out of a toolkit.
- **It passes through states that are not the reader's.** Three commands are three `settle`s, three
  damage events and a magnification applied to a page nobody asked to see.

`Command::View` is one command, applied in one `settle`, in the values the viewer itself produced.
**A host does not compose a `Viewing`; it echoes one** — which is what makes the fields being public
safe: they are there so that a window can print the page in its title, not so that a host can invent
a place the clamp would never have left the reader at.

The mechanisms `doc/ui-boundary.md` prefers to a new message were checked first, and the check is
what makes this a decision rather than a habit. Nothing at all: no, the question does not exist and
cannot be derived. A field on an existing message: no, the value travels in *both* directions and no
existing message goes both ways. A variant changing shape: `Command::Scroll { dx, dy }` could have
become `Scroll(By | To)`, which is the cheaper-looking answer — and it would have put an absolute
scroll into the vocabulary that no host can construct except by echoing an answer, which is the same
value with its coherence taken out. What is one fact stays one message.

## Decision 3: the restart carries the view, and `Resuming` is where it lives

`viewer_confined::Reopen::page` becomes `Reopen::view`. ADR 0734 put "what a resume goes back to"
inside `Resuming` deliberately — it is the part two confined hosts must not answer differently — and
the change keeps that: `Resuming::showing` takes a `Viewing` instead of a page, and the host asks
`Query::View` per frame to get one. `Resuming::default` names the view it stands in with before any
frame has landed — page one, whole page, unscrolled — rather than deriving a `Default` for `Viewing`,
because a view is something the viewer answers with and a defaulted one would be a value nobody was
ever at.

`pdf-viewer-confined` loses the sentence it owed the reader. The window's own trace and title keep
the page, because that is what a person reads.

## Decision 4: the greeting moves, `PDFVCF04` → `PDFVCF05`

Nothing that crossed before means anything different — the three new discriminants are each the next
free one — so the bytes alone did not demand it. What an older worker cannot do is *answer* the new
question, and a host would find that out in the middle of putting a reader back after a death, as a
refusal of something the reader never asked for. The greeting is the cheap place to find it instead,
which is what that constant is for.

## Decision 5: `ConfinedError` stays `#[non_exhaustive]`, and the line is written down

ADR 0734 recorded the tension and left it: `doc/ui-boundary.md` says nothing on this boundary is
`#[non_exhaustive]`, and that error is. The line between them is where a value's population comes
from.

**The rule binds a vocabulary.** A `Command`, an `Event`, a `Query`, an `Answer` — and any type that
crosses *inside* one, which is why `pdf_render::RasterFormat` had to stop being `#[non_exhaustive]`
(ADR 0247) — has a population this project chooses, every member of which is something a host must
decide about. One added later and silently ignored is a feature that never reached a person.

**`ConfinedError` is a failure population, and it is the kernel's.** A seccomp filter, a Landlock
ruleset, a pipe and an address-space ceiling fail in ways this crate will learn about after a host
has shipped. And what a host must decide about a refusal is not *which* one it is — it is whether
another worker is worth starting, which is `Resume`: two arms, closed, matched exhaustively, with
`Resuming::after`'s wildcard-free match over every error variant kept inside the crate that declares
them, where the attribute does not apply. The decision is protected by exactly the mechanism the rule
asks for, so the attribute costs nothing and buys a host not being recompiled by somebody's kernel.

## Proof, driven under Xvfb on the release programs

See `doc/history/805-*.md` for the run. The instrument for a death is the same as ADR 0734's — a
`kill -9` on the worker, which from the host's side is what a ceiling breach is — and the measurement
is the one that ADR 0734 recorded as the cost of its limit: the window's pixels before the kill
against after it, with the reader magnified and scrolled away from the opening view.

## Trap-13 calibration

Every new test was run against an injected defect before being believed; the table is in the history
file, and the suite is green as committed.

## What this does not close

`doc/todo/15` keeps the rest: moving the three established windows onto the boundary, and the
real-adapter measurement ADR 0725 owes to the owner's session. The three established windows do not
use `Query::View` at all — they hold their own `Viewer` and cannot lose it — and that is the honest
statement of who this question is for today.
