# `--trace` localises a slow frame and cannot attribute it

Status: **raised by the project owner on 2026-08-08**, from a Windows trace of a 30 MB, 65-page
document (`tmp/windows/trace-North.txt`, `NorthAmerican.30MB.pdf`) that felt slow. The trace was
read; it says *where* and not *what*.
Priority: 44 — instrumentation, and it is the project's ability to answer a performance question
rather than the program's speed. `doc/todo/43` is its neighbour: that one is how fast a round is,
this one is how fast an answer is.
Corpus: —, the witness is outside it and is the owner's own file
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs` (`App::redraw_requested`, `App::present`,
`App::attend`, `Launch`), `crates/render-quorra/src/present.rs`

## What the existing trace did settle, so that nobody rebuilds it

It is worth keeping, because it is why this item is narrow rather than a rewrite. From 1490 lines
of one run:

- **Launch is 1044 ms to the first present, and 770 ms of that is the first frame** — `arguments`
  0.208, `chrome fonts` 1.891, `event loop` 16.296, `window` 156.908, `graphics instance` 156.915,
  `graphics device` 273.624 (adapter 52.2, device 64.1), `document joined` 273.665, first present
  1044.313. Page one is **one** display-list command.
- **Steady state is the complaint**: 63 presents, median **60.4 ms**, p90 **157 ms**, max **514 ms**.
- **Frame cost tracks the display list's command count and not the raster's size.**

  | page | commands | present min / median / max |
  |---|---|---|
  | 59 | 163 | 9.7 / 11.6 / 18.6 ms |
  | 62 | 368 | 20.5 / 39.4 / 39.4 |
  | 60 | 1387 | 47.8 / 53.2 / 53.2 |
  | 3 | 2822 | 20.1 / 87.5 / 514.1 |
  | 54 | 4477 | 57.5 / 69.3 / 77.6 |

  Against that, page 3 at 244 × 315 (0.08 Mpx) took **149 ms** and the same page at 1404 Mpx took
  **36 ms** — eighteen thousand times the area, a quarter of the time. Whatever is expensive is
  paid per command that is *visible*, every frame.
- **The eight refusals are not the slowness.** Page 3 asked for up to 411 847 032 scene-derived
  bytes against the stated 268 435 456 and fell to the processor; those frames presented in
  **20.1 to 48.5 ms**, faster than the graphics device's own frames around them. A budget refusal
  reads like a defect and was not one, which is exactly the kind of thing a trace is for.

## What it could not say, and this is the item

1. **`present -> presented in T` is one number over four different things**: translating a display
   list into a scene, uploading resources, submitting and waiting on the device, and — where the
   device refuses — a whole `render-cpu` rasterisation and upload instead. So "expensive per
   command" is measured and *which* per-command work is expensive is not.
2. **That number includes work that is not the frame.** `redraw_requested` starts its timer, calls
   `present`, then calls `Launch::arrived` and `App::attend` — the accessibility publication — and
   only then prints `started.elapsed()`. The frame's cost and the bridge's are one figure. This is
   a measurement defect, not a design choice, and it is the cheapest thing on this list to fix.
3. **Nothing says when the pipelines finished compiling.** Device bring-up prints `pipelines still
   compiling` and the timeline never closes it, so a first frame that waited on a shader and one
   that did not look identical. `CLAUDE.md` requires that nothing on the launch path wait for
   warmth; the trace cannot currently show whether the *first frame* did.
4. **Variance is invisible.** Page 3 at 1160 × 1500 took 90.4, 122.0, 152.7 and 309.9 ms for
   identical work. Nothing in the file distinguishes a slow frame from a frame that was descheduled.
5. **No image decode is timed.** The 770 ms first frame is a single image command; JPEG decode,
   colour conversion, upload and first-use allocation are one lump.
6. **No absolute time.** Every line carries a duration and none carries a clock, so the interval a
   *person* waited — key press to pixels — cannot be recovered, and idle cannot be told from busy.
7. **No baseline.** One machine, one platform, one file. Nothing says whether a 2822-command page
   costs the same on the adapter this project develops on.

## What a round taking this owes

**The constraint is the owner's: enough to debug this, without flooding the output.** 1490 lines
for one session is already at the edge, and the answer is not more lines per frame.

- **One line per frame stays one line per frame**, with the stages *in* it rather than beside it —
  translate / upload / submit / fallback, and the page and command count that are already there.
  A frame that did nothing unusual should be no longer than it is today.
- **`attend` and `Launch::arrived` move outside the timer**, or are named separately in the same
  line. Whichever, the frame's number must be the frame.
- **A summary when the program exits** is where percentiles belong — count, median, p90, max, and
  the same split by stage — because that is the shape of the question "why did it feel slow" and it
  costs nothing per frame.
- **An absolute timestamp per line**, cheap and monotonic, so a gap in the log is legible as a gap.
- **A verbosity that is not all-or-nothing.** `--trace` today is one switch over pointer moves,
  window events, commands, events and frames; 285 of that run's lines are `pointer Moved`. Whether
  the answer is a level, a comma-separated set of topics, or a threshold that only prints frames
  over some cost is a design decision — argue it rather than picking one.
- **A closing line for pipeline compilation**, which is one `Instant` and one print.

## What not to do

- **Do not put the instrument on the launch path when nobody asked for it.** `--trace` is a flag
  and the timeline it prints is already gathered unconditionally at a cost measured in nanoseconds;
  anything added here is held to that.
- **Do not measure by changing what runs.** A stage timer that forces a device wait to get a
  number would be reporting a program that only exists under `--trace`. Where a GPU submit is
  asynchronous, say so rather than fabricating a boundary.
- **Do not turn this into a profiler.** The question is which of four stages a frame spent its time
  in, not which function. `callgrind` and the existing examples answer the finer question and are
  already in `doc/HANDOVER.md`'s "Measuring".

## One thing in that trace that is not about speed

The program contradicted itself. Line 2: *"this build has no accessibility bridge — AccessKit's
macOS and Windows adapters exist and are not wired in here"*. Line 46: **`trace: accessibility
bridge up`**, followed by `accessibility: 0 element(s), 0 report(s) on page N` at every page turn
thereafter. One of the two is false on Windows, and a note that a build cannot do something is
exactly the kind of claim this project checks. Whoever takes this item should find out which,
because it is three lines away from the code being instrumented — and if the per-page query really
does run on a platform with no bridge to publish to, it is also inside every frame's figure by
item 2 above.
