# Road B — ship the confinement and let the OS hold the bounds

Status: **open, one of its two defects carried out (ADR 0597), the tier change decided (ADR 0607),
its codec built (ADR 0626), wired into the frame path (ADR 0633), paid for (ADR 0640) and the
host's own draw made stoppable (ADR 0650).** The
machinery exists and is verified
against the kernel (ADRs 0218, 0223, 0235, 0241); a ceiling breach is no longer a crash; a
frame carries either the pixels or the marks, chosen per page by comparing two byte counts the
confined process can both compute; and a page shipped as marks is **not drawn at all**, which is
what the choice was for. The tier change is complete. What it left behind was one *host-side* debt
whose mechanism is now built and whose *policy* is what remains, below.
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

- **The tier change itself is carried out** (ADR 0633), and what it still owes is one thing rather
  than the three ADR 0626 §6 listed. Those three were not independent — which host rasterises
  decided what the reply carries, which decided what breaks, which decided when `MAGIC` moves —
  and all three are settled. `viewer-confined` takes **no** rasteriser: the device is the host's
  by necessity, because a process holding one dies on its first `ioctl` under this confinement, so
  `Reply::Frame` carries a `Payload` per page and a host draws the marks with whatever backend it
  has. `MAGIC` moved once, `PDFVCF03` → `PDFVCF04`.

  **And the `viewer-core` outcome that was per viewer where it needed to be per page is
  `Rendered::Listed`** (ADR 0640). It says *the host took this request's own list*, about one page:
  the page holds its place, the viewer holds no pixels of it, and `holds_rasters` does **not** move
  — so `MAX_PIXELS` goes on bounding every request a confined process makes and `Query::Frame` goes
  on answering for the page's neighbours, which is precisely what reusing `Rendered::Presented`
  would have cost. The worker draws nothing it does not send: a sparse page's open falls from 8.7
  ms to 2.8 ms, ISO 32000-2's densest first page by about 8.5 ms, and a 1.5 kB document amplifying
  to ten thousand page-covering fills from **26.5 s to 18 ms**, with the two pixel-arm documents
  flat beside them as the control.

- **What that left was a *host-side* debt, new here rather than caused here — and the mechanism
  half of it is carried out** (ADR 0650). A cancel stops the work the worker does, and on the
  marks arm the worker does not rasterise, so the drawing of a page that will not finish is the
  host's, outside the confinement. `pdf_render::Interrupt` is what reaches it:
  `render_cpu::CpuRasterizer::interruptible` takes one and honours it between commands, which is
  the loop nothing bounds. The witness draws for **27.6 s** in the host from 990 kB of marks the
  worker shipped in 14 ms and never drew, and an interrupt returns the drawing thread in **1.3 to
  2.1 ms**.

  **What ADR 0650 also settled is that there is no budget to derive, which this entry expected
  there to be.** The only pre-draw cost estimate in the tree — `row_costs` over
  `command_extents`, which every CPU draw already computes to place its strips — correlates with
  the measured draw at **0.115** by Pearson over `doc/pdf.js`'s first pages. Two pages of the same
  size make it concrete: one painted **0.2** times over draws for **162 ms** and one painted
  **593** times over draws in **16 ms**. `render-quorra`'s budget refusals are not the answer
  either: that budget is the *device's resources*, not the frame's cost. `examples/host_draw` is
  the instrument and carries the finding.

  **What is left is the policy**, which is a host's and has no host yet: nothing decides *when* to
  raise one, because `viewer-ui` is not on this boundary (`doc/todo/34` §2's last line). The shape
  it takes is the owner's brief in [`10`](10-bounds-that-cap-size.md) — a callback that warns and
  does not block — and ADR 0650 §2 is why it has to be driven by a clock read *while* drawing.

  **And the policy owes one message besides the decision**, found by reading `doc/todo/37`'s
  machinery rather than assuming it: `Stale::plan` stands in for a view until a rendering lands,
  and an interrupted draw never lands. A host that raises one has to say the render *failed*, or
  the stand-in becomes permanent and the person is left looking at a frozen approximation with
  nothing to explain it.
- **The cancel path proven from the host**, not only from a test: the owner's brief says the
  callback may not block, and the `Canceller` is already about a millisecond.
- **A breach an allocation budget cannot see** — a decode deep inside the interpreter, sized by the
  work rather than declared by a sender. It still ends the worker; what it now does is arrive with
  the worker's own sentence attached rather than as a bare signal number. Making it a *refusal*
  needs a fallible allocation on a path this crate does not own.
- `doc/todo/10` §6's four rules, which bind every road.
