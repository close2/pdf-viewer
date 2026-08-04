# A selection costs a compositor layer per quad, and the window stops answering

Status: **reported by the project owner, two-hundred-and-fifty-first session; reproduced,
measured, the arithmetic is exact, and the design question is settled. Not fixed.**
Priority: 13 — the only defect on this list a person meets by using the program normally
Corpus: every document with text; reproduced on `issue14821.pdf` and `tracemonkey.pdf`
Clauses: none. This is ours, in the host.
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs` (`highlight_list`, `present_page`)

## What was reported

> If I open `issue14821.pdf` and select text randomly (by dragging the mouse over the page), the
> application becomes unresponsive. I sometimes were able to "reset" it by changing the window
> size, but not always (or I didn't wait long enough).

## Reproduced, in a window, with the numbers

ADR 0126's recipe, an 800 × 1000 window on `Xvfb` through `lavapipe`, `xdotool mousedown 1`
then a walk down the page. The trace is unambiguous:

```text
SELECTION quads 37   present -> presented in 105.6 ms
SELECTION quads 63   present -> failed: frame needs 409600000 bytes of instance data,
                                over the stated budget of 268435456
SELECTION quads 63   present -> failed: … the surface is not renderable right now: Timeout
                                in 1.008 s
SELECTION quads 50   present -> nothing to show in 1.001 s
```

and from then on every present blocks for **exactly one second** — the process sits at 4% CPU,
so it is *blocked*, not spinning. That is the unresponsiveness. A resize reconfigures the
surface, which is why it sometimes recovers.

**Not this document, and not this backend.** `tracemonkey.pdf` fails at the same 63 quads with
the same number, and `--cpu` fails too — that flag skips the graphics *device*, and the
processor's frame is still presented through `render-quorra`, which prices the same layers:

```text
--cpu, 63 quads: frame needs 409600000 bytes    --cpu, 91 quads: frame needs 588800000 bytes
```

## The arithmetic, and it closes

`highlight_list` draws **one fill per selection quad**, each with `BlendMode::Multiply` — a
deliberate choice, and its comment says why: "overlapping quads darken twice, which is what
overlapping selections look like."

quorra gives every non-`Over` blend its own compositor layer, and prices the compositor's
internal textures before allocating any of them (`compose::internal_texture_bytes`):

```text
internal_texture_bytes = (layers + 1) × 2 × width × height × 4
```

At 800 × 1000 that is 3 200 000 bytes a layer, and

```text
(63 + 1) × 2 × 3 200 000 = 409 600 000        (91 + 1) × 2 × 3 200 000 = 588 800 000
```

— both observed numbers, to the byte. **A selection quad costs 6.4 MB of frame budget at this
window size**, so the 256 MiB budget is spent at 63 quads, which is one short paragraph of text.

And the cost is visible long before the refusal: present time is about 35 ms of baseline plus
**1.9 ms per quad** — 5 quads 42 ms, 19 quads 63 ms, 27 quads 80 ms, 37 quads 106 ms. Selection
was meant to be the one thing that never waits on a render.

## The blend mode is load-bearing; the *per-quad* part of it is not

The project owner asked the obvious question of the paragraph above — why would anyone want
overlapping selection quads to darken twice, and is that not counter-intuitive? — and the answer
is that nobody would, that it is, and that the comment claiming otherwise was an assertion nobody
had checked.

**Measured.** A selection of three lines of `tracemonkey.pdf` gives 19 quads and **two
overlapping pairs out of 171**, overlapping by **0.28 and 0.17 device pixels** horizontally:
sub-pixel slivers where two runs of one line abut, not anything a person would call an overlap.
`Query::Selection` answers with one quad per run of text, and runs tile rather than overlap. So
the behaviour the per-quad fill preserves does not arise, and where it does it is invisible — and
if it *were* visible it would read as a defect, because no reader draws a darker patch inside one
selection.

**What `Multiply` is actually for is the text underneath**, and that is not negotiable. §11.3.5.2
makes it the one blend mode whose "result colour is always at least as dark as either of the two
constituent colours" — the standard's own guarantee that what is under the wash survives it. This
tree already cites exactly that for §12.5.6.10's highlight annotations, in the ledger. A
photograph of a live selection confirms it: every character stays legible through the blue.

So the fix is neither "drop the blend" nor "keep a layer per quad". It is **one fill of one path
with one subpath per quad, under one `Multiply` layer**: the text still shows through, the
sub-pixel slivers stop darkening twice because a single path under the non-zero rule is one
shape, and the compositor's cost stops depending on the quad count at all —
`(1 + 1) × 2 × width × height × 4`, once, whatever is selected.

## What is left to settle

1. **Whether `viewer-core` should coalesce the quads as well.** One rectangle per selected line
   rather than one per run would shrink the path too, and is a change to what `Answer::Selected`
   means that `headless.rs` would have to pin. Not required by the fix above — the layer count is
   what the budget turns on — so it is an optimisation with a measurement owed, not a repair.
2. **What the host does when a frame is refused.** Today the refusal is followed by a surface in
   `Timeout` and a one-second block per frame for ever. That is a separate defect and the one the
   *report* is actually about: whatever the selection costs, a refused frame must leave the window
   answering. It would still be reachable by a page the device cannot draw in one pass, which is
   what `render-gpu`'s banding exists for (ADR 0127) and what quorra has no equivalent of.

## Why it is not fixed in the session that found it

The two items above are different repairs — one is what a selection *costs*, the other is what a
host does when the device says no — and the second is reachable without the first. Fixing only
the cost would leave the next over-budget frame hanging the window again.

**What is *done* is the measurement and the design decision**: the reproduction, the exact
arithmetic, that it is neither document-specific nor backend-specific, that the per-quad cost is
already 1.9 ms of present time before anything is refused, and — settled by the project owner in
the session after the report — that the per-quad blend was preserving a behaviour nobody wants
and which 169 of 171 quad pairs never exhibit.

## The one line that makes this visible again

`present_page` queries `Query::Selection` and has the quads in hand; the trace used above was

```rust
if self.trace {
    eprintln!("trace: SELECTION quads {}", highlight.len());
}
```

immediately after that query. It is not in the tree — the session that fixes this should add it
and keep it, because the count is the number every part of this file turns on.
