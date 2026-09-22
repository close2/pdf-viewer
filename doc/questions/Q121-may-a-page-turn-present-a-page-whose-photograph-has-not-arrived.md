# Q121 — May a page turn present a page whose photograph has not arrived?

Source: round 1217, pricing the last item `doc/todo/36` left for this tree. The measurement is
ADR 1271; the design and its price are ADR 1272.

## What is being asked

A page turn onto a page that is one five-megapixel photograph costs **122.68 ms** — nothing else on
that page costs a tenth of a millisecond — against the 8.333 ms a 120 Hz refresh allows. Table 87
states the image's size in its dictionary, so the display list could carry the photograph's
rectangle without its samples: the page would appear at once with that rectangle **empty**, and be
presented again when the decode landed about a tenth of a second later.

The reason this is yours rather than a round's: it is the trade `doc/todo/36` records you deciding.
*Correct frames wherever possible, reprojection where not* — and ADR 0386 section 3.3 wrote down
the one variant of it available without an ask, declined in your own sentence, *"we should still
try to render a correct image every frame"*.

**It differs from that one in a way that may or may not matter to you.** ADR 0386's case had a
correct picture on the screen already and proposed to keep showing it; a page turn has nothing —
`stale.rs` says so in its own words, that a page turn and a `GoTo` have no base at all, because
nothing about the outgoing page's pixels is true of the incoming one. So the choice here is not
*correct frame against approximated frame*. It is **a page with a hole in it now, against the
previous page for a tenth of a second longer**.

## What the hole cannot be filled with

Not a blurred photograph. ISO/IEC 10918-1's entropy-coded data is one bit stream in which a
block is found only by decoding the blocks before it, so a decode at one eighth of the resolution
still pays every Huffman symbol — 55% of what the decode costs. Only a *progressive* codestream
offers a real preview, and whether a codestream is progressive is its producer's choice. The first
frame would show the page's own background where the picture goes.

## The three answers

1. **No** (the recommendation). A page that flashes is worse than a page that arrives, given that
   the hole cannot resemble the picture. The photograph's own decode is 73% `zune-jpeg`'s
   arithmetic after ADR 1271 and the rest of the turn is quorra's transfer
   (`doc/QUORRA_FEEDBACK.md` section 52), so the remaining road is faster decoding rather than
   earlier presenting.
2. **Yes, for a page turn only** — never during a gesture, which stays as ADR 0386 left it. Then
   `pdf-render` gains a third image state, the renderer gains a decode thread and a cache, and one
   structural cost has to be accepted with it: a display list whose contents depend on whether a
   decode has finished is no longer a pure function of the bytes and the view state, which is what
   the cross-backend oracle's comparison rests on (`CLAUDE.md`).
3. **Yes, and behind a setting**, defaulting to (1), so that a reader on a slow machine can choose
   the other trade. The cost is that both paths then have to stay correct.

## What is being done regardless

ADR 1271 took 16% out of that page's interpretation with nothing presented differently anywhere
(974 golden pages held, 0 moved). The item that makes a turn faster *without* presenting anything
incorrect — decoding a page's images in parallel across the pool — needs no answer from you and is
recorded in ADR 1272 section 5.
