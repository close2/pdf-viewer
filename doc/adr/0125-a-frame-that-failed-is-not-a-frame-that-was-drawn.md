# ADR 0125 — A frame that failed is not a frame that was drawn

Status: accepted, 2026-08-01.

## The report

> I can't view page 6, 7, 1010 or 1011 of `ISO_32000-2_sponsored_EC3.pdf`. The title says that I
> am looking at it, but it's not updated.

**Not reproduced in the current tree.** All 1023 pages interpret, all 1023 produce a render
request, and all 1023 render through the exact path the binary uses — `evaluate_soft_masks`,
`build_scene`, `render_to_texture` at the *window's* target with the centring transform composed
in — against RADV, headless, checked one by one. The binary the report came from was built at
16:26 and the tree had moved six commits past it, so what it was is not knowable from here.

What *is* knowable is why the symptom was a silent stale window rather than a sentence, and that
is a defect whatever caused it.

## What was wrong

```rust
if let Err(problem) = draw(…) {
    println!("note: page {}: {problem}", …);
    return true;          // ← "yes, that was presented"
}
```

`true` meant `Rendered::Presented`. The core recorded the page as shown, never asked for it
again, and the window kept the previous page's pixels under a title bar naming the new one. The
one line that could have explained it went to stdout, which a person looking at a window is not.

That is trap 5 — *unsupported input must stay loud* — in the one place the trap had never been
applied, because `viewer-ui` had only been a consumer of `viewer-core` for ten sessions and this
path had never failed on this machine.

## Three changes

**A failure is answered as one.** `present` returns `Option<Rendered>` rather than a `bool`:
`None` where the swapchain gave the frame back (occluded, outdated — the next redraw retries),
`Some(Failed(reason))` where the drawing itself refused. The core turns that into
`Event::Reported`, which the binary prints and which a host with a status bar would show.

**The CPU backend draws it instead.** `CLAUDE.md` keeps `render-cpu` as the correctness oracle
*and* the startup path; a page the graphics device refuses is a page this program can still show,
more slowly. The raster crosses into the surface as a Vello image — one copy, which is exactly
what tier 1 costs everywhere, paid only on a page the device refused. So "I cannot view page 6"
becomes "page 6, drawn on the processor", said out loud.

**A refusal is recorded as an answer.** With the first change alone the scheduler asked again
immediately — its question is "is what is on the screen what should be", and a refused page is not
— so a host that always refused and a core that always asked would spin a processor for ever.
`Rendered::Failed` now marks the page shown at that resolution: the answer does not change until
the question does. `a_render_that_cannot_be_drawn_is_asked_for_once` pins both halves, and it
failed against the intermediate version, which is how the spin was found before anybody met it.

## What the reporter gets

The release binary is rebuilt, which it had not been since the second of the ten sessions. If
those four pages still fail, the program now says which of the two backends refused and why —
in the title bar, on stdout, and with the page number — which is the sentence this report needed
and did not have.

## The lesson

**A host is a place where a report can be swallowed, and it is the last one.** Every layer under
it reports: `pdf-syntax` refuses, `pdf-model` names what it could not draw, `viewer-core` carries
the note through as an event. The binary printed it to a stream nobody was watching and told the
core everything was fine. **Ask of every `return true` on an error path: who is this lying to?**
