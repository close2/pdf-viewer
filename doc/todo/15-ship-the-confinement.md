# Road B — ship the confinement and let the OS hold the bounds

Status: **open, one of its two defects carried out (ADR 0597), and the tier change decided (ADR
0607).** The machinery exists and is verified against the kernel (ADRs 0218, 0223, 0235, 0241); a
ceiling breach is no longer a crash; the tier change is priced, argued and not yet made.
Priority: 15 — the second road of [`10`](10-bounds-that-cap-size.md), whose §5 table prices all
four and whose §6 binds whatever lands here
Witness: **a large ordinary document**, rebuildable — a valid one-page file padded to a stated size
with a stream nothing refers to, which is the shape `examples/confined_page` builds in memory.
Bomb B, rebuildable from `doc/todo/10` §2's description, is what the ceiling *used* to be for and
no longer is; `tmp/Entwurf.pdf` for the ordinary-document side is **not in the repository**.
Instrument: `VmPeak` from `/proc` against `INTERPRETER_ADDRESS_SPACE_LIMIT` — the counter
`RLIMIT_AS` actually compares against. `cargo run --release -p viewer-confined --example
confined_peak -- <file.pdf>…` is that instrument, and it is load-immune.
Clauses: none — this is an architecture decision, not a clause reading. `CLAUDE.md` principle 3 is
the standard it answers to.
Code: `crates/viewer-confined/src/{lib,worker,protocol}.rs`,
`crates/pdf-sandbox/src/lockdown_linux.rs`, and whatever host takes the tier change
Blocked on: nothing technical — but see [`34`](34-sandbox-the-interpreter.md) §2, which is the tier
question, and [`35`](35-confinement-off-linux.md), which is why it is Linux-only

## Why second rather than first

`doc/todo/14`'s road removes the allocation; this one survives it by handing the bound to the
kernel. Taken in the other order, this road's ceiling would still be the thing standing between a
1.85 MB file and the machine — and a ceiling breach is a kill, which is the bluntest answer in
`doc/todo/10`. Taken after it, the ceiling becomes a backstop for what streaming does not cover
(the font, image and profile paths that are read whole by design) rather than the primary defence.

**That prediction is now measured rather than argued** (ADR 0597). Bomb A and Bomb B, in the
confined worker, move `VmPeak` by **nothing at all** — the peak is reached during start-up, at 3.5%
of the ceiling, and neither bomb passes it. So what the ceiling is for has moved from *catches the
bomb* to *catches what we have not thought of*, which is a different and still-good argument, and
it is the one to make when this item is next picked up.

---

*What follows was `doc/todo/10` §5.1's B section and moved here whole when the owner chose the
order, so that the argument lives with the item.*

Make `viewer-confined` the viewer's actual path (`doc/todo/34` is written for it), then relax the
counting bounds hard: keep the cycle guards and the decode bounds, drop the size caps. The ceiling
becomes the memory answer, the `Canceller` the time answer, and the host offers "this is taking a
while — stop?" backed by a kill the document cannot decline.

- **For**: it is the owner's "maybe that's now up to the OS", answered *by* the OS. Already built,
  already verified against the kernel rather than the source. Cancel measured at about a
  millisecond. Principle 3's other half finally reaches the program.
- **Against**: it is a **tier change**, and `doc/ui-boundary.md` calls putting `viewer-ui` on this
  boundary "a decision with a number attached rather than a switch" — page one would go through a
  pipe. Linux-only (`doc/todo/35`).

**Two of those objections have since been answered, each by a different round.** The original ended
"and the 4 GiB ceiling is currently *smaller than* what one 2 GiB stream can demand" — true while
`filter::inflate` doubled past its own bound, and answered by ADR 0354. And "a ceiling breach
arrives as `WorkerDied { detail: "killed by signal 6" }`, indistinguishable from a crash" was
answered by ADR 0597, which found it was in fact *worse* than that: see below.

## What ADR 0597 carried out, so that a round taking this does not redo it

- **A ceiling breach is not a crash.** The worker derives a message budget from its own ceiling —
  every term read or measured, `doc/todo/10` §6's first rule — and refuses a document it cannot
  hold *before* the first byte is read, in a sentence naming the size, the ceiling and the budget.
  It reads past the message rather than closing, so **the worker survives the refusal with whatever
  it had open**, which is why the "worker restart plus document re-open" this item used to ask for
  is not owed for that population.
- **The open path held three copies of the document** and now holds two, because the frame buffer
  was alive across the work for no reason. `VmPeak` was the worker's start-up size plus exactly
  three times the document's length; it is now plus twice.
- **The worker's own last words reach the host.** Its standard error was *inherited*, and
  `RLIMIT_FSIZE` is 0 — so on any host that logs to a file the worker was killed by `SIGXFSZ`
  before it could print the one line explaining itself, and the host was told the wrong cause. It
  is a pipe now, drained on a thread, echoed onward and carried in `WorkerDied`.
- **The tier change has its number**: page one through a pipe, both arms in one sitting, in ADR
  0597's last table.

## What a round taking this still owes

- **The tier change itself**, whose *decision* is no longer owed: [`34`](34-sandbox-the-interpreter.md)
  §2 is settled and ADR 0607 is the argument. **Display lists cross, and the raster payload stays**,
  chosen per page by comparing the two sizes the confined process can both compute — a list for
  about 96% of first pages at a window's scale, pixels for the scanned 4% where pixels are smaller.
  The alternative, `wgpu` inside the confinement, was not rejected on price: a device confined at
  any point in the ordering dies on its first `ioctl`, and a process holding one cannot install
  Landlock under this crate's own descriptor ceiling. What is owed here now is **the codec** —
  both sides, `Arc` identity preserved, a fuzz target beside `confined_wire`, and either an
  encoding for the two deferred producers or the raster arm they already fall back to.
- **The cancel path proven from the host**, not only from a test: the owner's brief says the
  callback may not block, and the `Canceller` is already about a millisecond.
- **A breach an allocation budget cannot see** — a decode deep inside the interpreter, sized by the
  work rather than declared by a sender. It still ends the worker; what it now does is arrive with
  the worker's own sentence attached rather than as a bare signal number. Making it a *refusal*
  needs a fallible allocation on a path this crate does not own.
- `doc/todo/10` §6's four rules, which bind every road.
