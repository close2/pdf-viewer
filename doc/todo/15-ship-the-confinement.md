# Road B — ship the confinement and let the OS hold the bounds

Status: **open, one of its two defects carried out (ADR 0597), the tier change decided (ADR 0607),
its codec built (ADR 0626), wired into the frame path (ADR 0633), paid for (ADR 0640), the
host's own draw made stoppable (ADR 0650), the stopping decided (ADR 0657), the boundary has
its first host (ADR 0713), that host draws on the graphics device (ADR 0725), the owner's
warn-and-abort has reached the three established windows (ADR 0729), a breach is a refusal
of the page rather than of the document (ADR 0734) — and the reader's own view survives that
refusal (ADR 0737)**:
`pdf-viewer-confined`, a window whose every page comes out of `pdf-view-worker`, on both payload
arms, presented through `render-quorra` — which is what the marks cross the pipe *for* — with
Escape ending the worker *and* taking back the drawing thread, and `--cpu` the window with no
device. The
machinery exists and is verified
against the kernel (ADRs 0218, 0223, 0235, 0241); a ceiling breach is no longer a crash; a
frame carries either the pixels or the marks, chosen per page by comparing two byte counts the
confined process can both compute; and a page shipped as marks is **not drawn at all**, which is
what the choice was for. The tier change is complete. What it left behind was one *host-side* debt
whose mechanism, policy and input are now all built, in all four windows.
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

## What the seven-hundred-and-seventy-fifth session built, so the next round starts from it

**A window on this boundary exists**: `pdf-viewer-confined` (in `viewer-ui`, ADR 0713 for why
there), the smallest complete host — open, arrange, turn, scroll, zoom, report, abort — with
everything outside that scope refused by name. **Since the seven-hundred-and-eighty-first
session that scope includes §7.6.4.1's prompt** (ADR 0718): an encrypted document was the one
refusal standing in front of *open* itself, and the prompt is the shared card and the shared
policy, with the password crossing into the confinement inside `Command::Open` — the direction
argued in that ADR, not an accident. Both payload arms reach its screen: rasters are
placed as they arrive, marks are drawn by `render-cpu` on `viewer_host::drawing`'s thread, which
became generic over its request (`DrawRequest`) because a confined host cannot hold a
`RenderToken`. **The cancel path is proven from a host**, which this file used to list as owed:
Escape kills the worker and interrupts the in-flight draw, without blocking, driven under `Xvfb`
on the amplification fixture.

What that sharpens rather than closes: the three *established* windows still interpret in
process, so "make `viewer-confined` the viewer's actual path" now means moving them — their
panels already have owned `Reply` counterparts for every answer, and the page path has a working
model to copy. ~~And the confined window presents through the processor~~ — **it presents
through the device since the seven-hundred-and-ninetieth** (ADR 0725): the marks as they
crossed, the worker's rasters wrapped as one-image lists, the interruptible drawing thread kept
for the frames the device refuses, and the `Arc` identity of an unchanged page now surviving the
pipe, without which every host-side scene and drawing cache missed on every scroll. What that
round could not measure is owed rather than guessed: the window's bring-up and present cadence
on the **real adapter** need the owner's session — Xvfb's llvmpipe numbers are in ADR 0725 as
illustration only — and the device lane's coverage choice stays quorra's default until such a
measurement asks for more.

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

  **The policy is decided and built** (ADR 0657), and the two sentences this entry used to carry
  here were both wrong in the same way — they assumed the interrupt's question belongs to *this*
  boundary. It does not. It is asked wherever a display list this program did not write is drawn
  by `render-cpu`, and `viewer-ui`'s `--cpu` window has been such a host since ADR 0461 gave it a
  composing thread. So the policy has a host today and is wired into that one.

  - **The rule reads no clock**: a draw is interrupted where finishing it would produce a picture
    the program has already decided it will never show — `crate::stale::could_stand_in`, which is
    `doc/todo/37`'s own question asked of a frame not yet drawn. A page turn, a resize, a
    re-interpretation and a zoom of a column are interrupted; a scroll and a zoom of a single page
    are **not**, because their frame is the next stand-in's base.
  - **A deadline was priced and refused, on a stronger measurement than ADR 0650's.** That one
    says the tree cannot predict a draw's cost; this one says the cost is not the question. At
    twice device scale, 6.1% of `doc/pdf.js`'s first pages take longer than one 60 Hz period and
    the slowest of 957 takes 252 ms, while the amplification fixture takes 27.6 s — and a
    document's author picks where in that gap to sit.
  - **The message this entry said was owed is not owed**, and the reading that found it is in ADR
    0657 §3: `viewer_core::Rendered::Failed` records a page as *answered* and stops the scheduler,
    deliberately, so a host saying it about a draw it abandoned itself would freeze that page. An
    abandoned draw produces no `Rendered` at all, and `viewer-core`'s
    `a_refusal_is_final_for_this_view_and_a_token_never_answered_is_not_re_asked` pins both halves.

  ~~**What is left of it is the owner's callback in the three established windows**~~ — **carried
  out in the seven-hundred-and-ninety-fifth** (ADR 0729). *"Warn the user and allow the user to
  abort, however don't block"* is `viewer_host::drawing::WARN` (a second, measured against
  `doc/pdf.js`'s first pages, and a clock that raises nothing), `viewer_host::still_drawing` and
  its two companions, and `viewer_host::keys::Waiting` giving Escape a third row — offered only
  while the window is saying the sentence that names the key, and never in a presentation, where
  Table 29 forbids the sentence. All three established windows have it and the confined one has
  had its own since ADR 0713, so four windows now mean the same thing by the key.

  Two findings came with it and are the reason it was not three lines. **`viewer-qt` forwarded
  Escape to the shared table only in full screen**, so §12.4.2's clear-the-selection row had never
  reached that host at all since ADR 0526 — a shared key table is only as level as the narrowest
  path a key takes to reach it, and nothing looks at that path. And **on `viewer-ui`'s composing
  surface an abort without a record is a loop**: that host asks for its own frame once a tick, so
  the abandoned frame was re-asked at the next tick, warned about again and stopped again, until
  `Composer::declined` recorded the arrangement the person had stopped. A native window needs no
  such field, because a viewer's token never answered is never re-issued.
- ~~**The cancel path proven from the host**, not only from a test~~ — **proven in the
  seven-hundred-and-seventy-fifth** (ADR 0713): Escape in `pdf-viewer-confined` ends the worker
  and takes the drawing thread back, without blocking, on the amplification fixture under `Xvfb`.
- **The device path warns about nothing**, which ADR 0729 states rather than guards against:
  quorra has no interrupt (ADR 0725), so the flagship's render thread cannot be taken back and a
  key offered for it would name a key that does nothing. `--cpu` is the surface with the
  interruptible thread. Whether quorra should have one at all is a question for that dependency,
  not a debt of this item.
- ~~**A breach an allocation budget cannot see** — a decode deep inside the interpreter, sized by
  the work rather than declared by a sender. It still ends the worker … making it a *refusal* needs
  a fallible allocation on a path this crate does not own~~ — **it is a refusal since the
  eight-hundred-and-first** (ADR 0734), and the entry was answering the wrong question. That
  sentence is about a refusal the **worker** makes, which is still true and still needs the fallible
  allocation. What the *reader* needs is a refusal of the **page**, and that costs nothing inside
  the confinement: `viewer_confined::Resuming` decides which errors are worth another worker and
  how many in a row, and `pdf-viewer-confined` starts one, opens the file again — it is on this
  side by rule 2 — and goes back to the page the reader was on without re-sending the command that
  killed the last one. The budget is **consecutive**, put back by every frame that reaches the
  screen, so what it bounds is a recovery that is not working rather than the length of a reading.

  ~~What that leaves is smaller and is named rather than implied: **the magnification and the
  position on the page are not restored**~~ — **they are, exactly, since the
  eight-hundred-and-fifth** (ADR 0737). The entry named the shape and got half of it: `Query::View`
  is the question, answered by the viewer and held by the host per frame, and `Viewing` is the page,
  the magnification and the scroll as one value. What it did not foresee is that the answer needs a
  way *back* — `GoTo` and `Zoom` are absolute and `Scroll` is not, so the third part of a view could
  be asked for and not stated — so `Command::View(Viewing)` is the other half, and the host echoes
  the value rather than composing one. The exactness is the point rather than a nicety: replaying
  the three commands lands within a rounding of the place, and 16% of `f32` pairs in a device
  pixel's range do not survive `have + (want - have)` at all.

  What the boundary gained is in `doc/ui-boundary.md` with its argument, and what it cost is two
  consumers failing to compile, two C entry points, one struct passed by value and `PDFVCF04` →
  `PDFVCF05`.
- `doc/todo/10` §6's four rules, which bind every road.
