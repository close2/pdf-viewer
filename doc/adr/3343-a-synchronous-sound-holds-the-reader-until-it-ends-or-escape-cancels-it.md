# 3343 — A synchronous sound holds the reader until it ends, or Escape cancels it

Session 2160. Status: accepted. Context: ISO 32000-2 §12.6.4.9's Table 212, ADRs 3301, 3329, 3341.

## Decision

Table 212's `/Synchronous`: "If this flag is true , the interactive PDF processor retains control,
allowing no further user interaction other than cancelling the sound, until the sound has been
completely played."

1. **What "retains control" means here.** The clause names the thing held: *user interaction*. So
   while a synchronous sound sounds, the window takes Escape — which already cancels a sound
   (ADRs 3301, 3329) — and refuses every other key, every press of the pointer and the wheel. A
   refused key or press says `viewer_host::sound::HELD` on the status line and the terminal; the
   wheel is refused silently, the title already saying it. The player says `Heard::Holding` for
   such a sound, `Sounds::holds_the_reader` answers for GTK and Qt, and `quorra` keeps the same
   answer in its own state.
2. **What it does not hold, as a choice where the clause is silent**: the program's own work — the
   page still draws, a server's answer is still taken — and an action chain's `/Next`, which
   `viewer_core` performs at the activation as it does for every action. The clause speaks of the
   person's interaction, not of the processor's; holding the chain would make `viewer_core` wait on
   a device it does not own, and a chain that ran on after a sound would differ only in when its
   next action's effect is seen.
3. **The menus are not held**: a toolkit's menu bar is the toolkit's, opened without passing
   through the host's key or pointer handlers. A person can still reach Quit through it, which is a
   cancellation of more than the sound.

## Proof

`viewer-host`'s `tests/play_parameters.rs`: a document's sound action with `/Volume 0.5
/Synchronous true`, activated through `viewer_core`, reaches the recording device as `±¼` and leaves
`Sounds::holds_the_reader` true until `Sounds::stop` — Escape's answer — cancels it. The windows'
key, pointer and wheel handlers each test that one answer before anything else.
