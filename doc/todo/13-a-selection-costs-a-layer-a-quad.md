# A selection costs a compositor layer per quad, and the window stops answering

Status: **reported by the project owner, two-hundred-and-fifty-first session; reproduced,
measured, and the arithmetic is exact. Not fixed.**
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

## What has to be settled

1. **Whether overlapping selection quads must darken twice.** That is the only thing the
   per-quad `Multiply` buys, and it is a choice this host made rather than anything a clause
   asks for. If it goes, the whole selection is one fill of one path with many subpaths under one
   blend — one layer, whatever the quad count.
2. **If it stays, whether one layer can hold every quad.** `Multiply` over a transparent backdrop
   composited once is not the same picture as each quad multiplying the one under it, so this is a
   question about what a selection should *look* like where two quads overlap — and whether two
   quads of one selection ever do. `Query::Selection` answers with one quad per run of text on a
   line; two of them overlapping would itself be worth knowing about.
3. **Whether `viewer-core` should coalesce.** The quads are the core's; a selection of a whole
   paragraph is one rectangle per line and the core could say so. That is a change to what
   `Answer::Selected` means and would need `headless.rs` to pin it.
4. **What the host does when a frame is refused.** Today the refusal is followed by a surface in
   `Timeout` and a one-second block per frame for ever. Whatever the fix above, a refused frame
   should leave the window answering — that is a separate defect and the one the *report* is
   actually about.

## Why it is not fixed in the session that found it

Item 4 is the unresponsiveness and items 1 to 3 are the cause, and they are different repairs:
one is about what a selection looks like, the other about what a host does when the device says
no. Fixing the second without the first would leave a selection that silently draws nothing past
63 quads; fixing the first without the second would leave the next over-budget frame hanging the
window again.

**What is *done* is the measurement**: the reproduction, the exact arithmetic, that it is neither
document-specific nor backend-specific, and that the per-quad cost is already 1.9 ms of present
time before anything is refused.

## The one line that makes this visible again

`present_page` queries `Query::Selection` and has the quads in hand; the trace used above was

```rust
if self.trace {
    eprintln!("trace: SELECTION quads {}", highlight.len());
}
```

immediately after that query. It is not in the tree — the session that fixes this should add it
and keep it, because the count is the number every part of this file turns on.
