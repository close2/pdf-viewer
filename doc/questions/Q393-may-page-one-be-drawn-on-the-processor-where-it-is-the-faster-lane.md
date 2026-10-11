# Q393 — May page one be drawn on the processor where the processor is predicted the faster lane?

Source: round 1497, building ADR 1831's lane choice for `quorra`.

## What `CLAUDE.md` says, and what the brief asked

Principle 2: "**Page one goes to the GPU** … The CPU backend keeps its other two jobs and loses this
one. It is still the correctness oracle … and it is still what draws a frame the graphics device
refuses (a coverage or budget refusal, not a swapchain state), reported out loud. What it is no
longer is the startup path." The batch's brief asked for a lane choice that is cost-aware: "a page
whose device frame would cost more than its processor draw takes the processor". On the owner's
drawing that is page one itself, so the two are in conflict there, and this file says so rather
than deciding it quietly.

## The figure

`/tmp/demo_heavy_engineering_A1.pdf` on the owner's display (Radeon 890M, XWayland): the device
draws page one in 8.8–14.2 s after the read (scene 2.3–4.7 s, device 6.5–9.5 s), uninterruptibly;
the processor draws it in 2.4–3.1 s, shown as it is built and stoppable. First frame 12.5–19.0 s at
HEAD against 5.2–6.7 s with ADR 1831; the first picture part-way at 3.3–4.2 s.

## What the tree does meanwhile

ADR 1831 as built: the device draws every frame it is predicted to draw inside `WARN` (1 s) — every
ordinary page, about 560 000 commands at the seed — and past that the lane with the smaller
prediction draws, page one included, said out loud in the title and on the terminal ("drawn on the
processor, the faster of the two here: …"). GPU bring-up stays on the launch path and is unchanged;
nothing waits for warmth. Reverting page one alone to the device is one condition in
`Costs::processor_for`.

## Recommendation

**Yes, and amend principle 2's sentence to say so**: page one goes to the device unless the frame
states that the device would take past `WARN` over it and the processor would take less — a
refusal predicted from the frame's own figure, the same kind of fact as the budget refusal the
principle already sends to the processor, and reported the same way. The rule's argument — a device
that blocks the first page behind its bring-up — is not what this case is: the device is up, and it
is the page that is past what a frame was budgeted to be.
