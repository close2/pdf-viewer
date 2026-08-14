# Road B — ship the confinement and let the OS hold the bounds

Status: **chosen, second of three** — the project owner ordered the roads D → B → C, so this one
follows [`14`](14-stream-the-decompression.md). The machinery exists and is verified against the
kernel (ADRs 0218, 0223, 0235, 0241); what is missing is the *tier change* and two defects named
below. Nothing here is built.
Priority: 15 — the second road of [`10`](10-bounds-that-cap-size.md), whose §5 table prices all
four and whose §6 binds whatever lands here
Witness: Bomb B, rebuildable from `doc/todo/10` §2's description (session 519 rebuilt it to the
byte); `tmp/Entwurf.pdf` for the ordinary-document side, which is **not in the repository**
Instrument: `VmPeak` from `/proc` against `INTERPRETER_ADDRESS_SPACE_LIMIT` — the counter
`RLIMIT_AS` actually compares against, which no round reported before session 519
Clauses: none — this is an architecture decision, not a clause reading. `CLAUDE.md` principle 3 is
the standard it answers to.
Code: `crates/viewer-confined/src/lib.rs`, `crates/pdf-sandbox/src/lockdown_linux.rs`,
`crates/viewer-core/src/viewer.rs`, and whatever host takes the tier change
Blocked on: nothing technical — but see [`34`](34-sandbox-the-interpreter.md), which is written
for exactly this move, and [`35`](35-confinement-off-linux.md), which is why it is Linux-only

## Why second rather than first

`doc/todo/14`'s road removes the allocation; this one survives it by handing the bound to the
kernel. Taken in the other order, this road's ceiling would still be the thing standing between a
1.85 MB file and the machine — and a ceiling breach is a kill, which is the bluntest answer in
`doc/todo/10`. Taken after it, the ceiling becomes a backstop for what streaming does not cover
(the font, image and profile paths that are read whole by design) rather than the primary defence.

---

*What follows was `doc/todo/10` §5.1's B section and moved here whole when the owner chose the
order, so that the argument lives with the item.*

Make `viewer-confined` the viewer's actual path (`doc/todo/34` is written for it), then relax the
counting bounds hard: keep the cycle guards and the decode bounds, drop the size caps. The ceiling
becomes the memory answer, the `Canceller` the time answer, and the host offers "this is taking a
while — stop?" backed by a kill the document cannot decline.

- **For**: it is the owner's "maybe that's now up to the OS", answered *by* the OS. Already built,
  already verified against the kernel rather than the source. Cancel measured at about a
  millisecond. Bomb B is genuinely stopped — measured, not argued. Principle 3's other half
  finally reaches the program.
- **Against**: it is a **tier change**, and `doc/ui-boundary.md` calls putting `viewer-ui` on this
  boundary "a decision with a number attached rather than a switch" — page one would go through a
  pipe. Today a ceiling breach arrives as `WorkerDied { detail: "killed by signal 6" }`,
  **indistinguishable from a crash**, and the document dies with it; both need fixing (a
  `try_reserve`/`Refused` path, and worker restart plus document re-open). Linux-only
  (`doc/todo/35`).

**One of those objections has since been answered, and by a different round.** The last sentence
of the original read "and the 4 GiB ceiling is currently *smaller than* what one 2 GiB stream can
demand" — true while `filter::inflate` doubled past its own bound. ADR 0354 made the bound obey its
arithmetic: Bomb B's measured `VmPeak` is **1041 MB against a 4 GiB ceiling**, where it was
1821 MB. So this road's remaining costs are the tier change, the two defects, and Linux — the
arithmetic objection is gone.

## What a round taking this owes

- **The number the tier change costs.** `doc/ui-boundary.md` says a decision with a number
  attached; the number is page one through a pipe, measured against `doc/performance.md`'s
  launch-path figures, not estimated.
- **A ceiling breach that is not a crash.** `try_reserve` where the ceiling is reachable, a typed
  `Refused` crossing the pipe, worker restart and document re-open — a person whose document hit
  the ceiling must get a sentence, not a dead window.
- **The cancel path proven from the host**, not only from a test: the owner's brief says the
  callback may not block, and the `Canceller` is already about a millisecond.
- `doc/todo/10` §6's four rules, which bind every road.
