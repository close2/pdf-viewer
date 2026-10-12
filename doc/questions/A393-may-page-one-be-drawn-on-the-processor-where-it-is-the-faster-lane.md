Status: complete
Given: 2026-10-12, in conversation — transcribed by the round
Owes: none

> yes, you are allowed to print the first frame on the CPU instead of the GPU. (The limitation was
> just to avoid 'unnecessary' work and assumed that the only reason to draw on the CPU was to avoid
> the cost of setting up the GPU pipeline)

Reading: **the recommendation is taken — page one may be drawn on the processor where the processor
is predicted the faster lane, and `CLAUDE.md` principle 2 says so.** The owner's parenthesis is the
reason, and it narrows the old sentence rather than reversing it: "Page one goes to the GPU" was
written against one case, a processor frame drawn only to hide the device's bring-up, which is
unnecessary work because the device is coming up regardless. That case is still forbidden — the
device stays the default lane, its bring-up stays on the launch path, and nothing waits for warmth.
What the answer admits is the other case `Q393` described: the device is up and the *page* is past
what a frame was budgeted to be, so the processor is not duplicating work but doing it faster.
ADR 1831's rule is the one that decides it, unchanged: the device draws every frame it is predicted
to draw inside `WARN`, and past that the lane with the smaller prediction draws, said out loud in
the title and on the terminal. ADR 1831's status is ratified by this answer.
