# Road C — make the unit of work small and let the host schedule everything

Status: **chosen, third of three** — the project owner ordered the roads D → B → C, so this one
follows [`14`](14-stream-the-decompression.md) and [`15`](15-ship-the-confinement.md). It is
much the largest of them, and nothing is built.
Priority: 16 — the third road of [`10`](10-bounds-that-cap-size.md), whose §5 table prices all
four and whose §6 binds whatever lands here
Witness: `tmp/Entwurf.pdf` — one page, 3 185 295 operators, **not in the repository**; the
interruptibility this road is about is what a person feels on that document and on nothing else in
the corpus
Instrument: the search pump this road generalises — `crates/viewer-core/examples/find_cost.rs` and
ADRs 0256, 0260 are the shape that already works, one page per turn of the host's event loop
Clauses: §7.8.2 (the interpreter's own unit of work), §12.6.4's actions where a pump crosses them
Code: `crates/pdf-model/src/content/run.rs` (`Interpreter::run` — the state machine to rewrite),
`crates/viewer-core/src/{viewer,event}.rs` (the pump and its messages), and every gate that
compares 1794 pages
Blocked on: nothing technical — but the honest precondition is that its cost is a rewrite against
the oracle, so it wants the other two roads' numbers in hand first

## Why third

It is the only road that is *genuinely always interruptible*, which is the owner's own phrase —
and the only one whose cost is a state-machine rewrite of the interpreter measured against an
oracle of 1794 pages. Taken after `14` and `15`, it inherits a world where the allocation is
already bounded and the kernel already holds a backstop, so what it adds is the thing neither of
those gives: a live window and partial drawing while a large document is still being read.

---

*What follows was `doc/todo/10` §5.1's C section and moved here whole when the owner chose the
order, so that the argument lives with the item.*

Generalise the search pump: interpretation becomes a resumable job the host advances one chunk per
turn of its event loop, emitting progress and taking `Stop`. Bounds become budgets *per chunk*,
and "very complicated document" becomes "many chunks" rather than a refusal.

- **For**: the only one of the three that is genuinely *always interruptible*, in the owner's
  words. The UI stays live throughout, no clock enters the core (the host decides when to pump),
  and it matches an architecture this tree has already shipped once and proved on six consumers.
  It yields **partial rendering** naturally — draw what you have and keep going — which is what a
  person actually wants from a 50 MB drawing.
- **Against**: much the largest change, for the reason in `doc/todo/10` §4 — a state-machine
  rewrite of `Interpreter::run`, against an oracle that compares 1794 pages. The cost of one chunk
  is still unbounded (one `sh` paints the page), so it needs road A's deadline anyway for the
  pathological operator. And it does nothing about the memory spent before interpretation starts —
  3.7 GB when this was written and a gibibyte now, which is a smaller number and the same
  objection.

**Road A is inside this one.** `doc/todo/10` §5.1 keeps A — a deadline and a host callback — as the
road nobody chose, and its own argument says A is a subset of C's requirements. A round taking this
takes A's pathological-operator answer with it, under A's hard rule: **off in every gate, on in
every host**, because `interpret` must stay a pure function of the bytes and the view state or the
oracle's whole comparison goes with it.

## What a round taking this owes

- **The gates' reproducibility, asserted rather than claimed.** Chunking may not change a display
  list, and something in the tree has to say so — `doc/todo/10` §6's second rule, and the reason A
  was never taken casually.
- **A chunk boundary that is a *state*, not a byte offset.** The interpreter's graphics state,
  clip chain and marked-content stack all cross a chunk; the search pump crosses nothing.
- **Partial drawing as a stated behaviour**, with what a partly-drawn page reports (trap 5: a page
  that is not finished must not look finished).
- `doc/todo/10` §6's four rules, which bind every road.
