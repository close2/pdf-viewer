# The resize frames: 9–19 ms per step of a drag, parked by the owner

Status: **open — parked.** This was point 8 of the seven-point GPU round; the owner
excluded it from that round's scope and nothing has been built or priced since.
Priority: 47 — performance, measured in passing, never attributed.
Corpus: any document; the numbers were taken on `tmp/Entwurf.pdf` and dense text pages
alike during the ADR 0704 round.
Code: `crates/viewer-ui/src/bin/pdf-viewer/surface.rs` (the resize path),
`crates/viewer-ui/src/bin/pdf-viewer/stale.rs` (`Refusal::Resized` and the retained-page
answer to it).
Instrument: the frame trace during a window drag — resize frames print as ordinary frames
with their own totals; drive with `xdotool` window resizes on the measurement loop, which
is the one input that provably reaches the owner's session (ADR 0700's lesson).

## What is known, which is little

A resize costs 9–19 ms per step on the 890M (recorded in passing during ADR 0704's
measurements). **The 1.3 s `resize` line in the owner's first Windows trace is not this
item**: `tmp/win/entwurf.2.trace.txt`'s `resize 1200x1500 … in 1.3014148s` at t=1.862 is
the same instant and the same duration as the launch table's own `interpreted, 58009 cmd
… (+1302.964)` — the first resize is where page one is interpreted, which is the launch
path's known largest step (`doc/todo/44` §6), not a per-step drag cost (ADR 0761 §1). What has never been done is the attribution: how much is the surface
reconfigure, how much the chrome rebuild, how much the re-render at the new extent — the
trace's columns exist and nobody has read them for a drag.

What is already decided and not owed here: the *base* stand-in is impossible on a resize
by design — the held picture is the old window's own pixels, chrome included
(`Refusal::Resized`'s doc comment carries the argument) — and the retained pages already
answer a resize, so a drag is not frozen. A per-page placement in the presenter, which
would let the base carry across a resize, was priced and declined in `doc/todo/37`'s
close-out. This item is therefore about the *cost of the real frame*, not about standing
in.

## What taking it would look like

One measurement round first: a driven drag under the trace, three samples, the columns
read. If the render at the new extent dominates, the question becomes quorra's (a resize
is a viewport change over an unchanged scene — page-space scenes should make the scene
free and the encode the term, same decomposition as the zoom step). If the reconfigure
dominates, it is the host's swapchain handling. Do not build before attributing; the
seven-point round's lesson was that the attribution changes the plan every time.
