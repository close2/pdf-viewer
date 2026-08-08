# ADR 0227 — A frame is four stages, and a flag is a set

Status: accepted, 2026-08-08 (session 390).

Supersedes nothing. Closes `doc/todo/44-what-trace-cannot-see.md`, raised by the project owner on
2026-08-08 from a Windows trace of a 30 MB, 65-page document that felt slow. Its complaint was
exact: the trace says *where* a slow frame is and not *what* it spent its time on.

## What was wrong

`--trace` printed `present -> presented in 126.6549ms` and that was the whole of what a person got.
The number was one figure over four different things — this host's own queries, translating a
display list into a GPU scene, quorra's encode-transfer-execute, and, where the device refused, a
whole `render-cpu` rasterisation — and it *also* included two things that are not the frame at all:
the launch timeline's own printing and the accessibility publication. Six of the seven items on
that todo were about this; the seventh was that no run of the document existed on the machine this
project develops on.

## The four decisions

### 1. The stages are read, not manufactured

The obvious way to time a GPU submit is to put a fence after it and wait. That would report a
program that only exists under `--trace`, which the item forbids and which is the right rule.

It turned out to be unnecessary. `quorra_gpu::Device::render` **already** measures `encode`,
`upload` and `execute`, already says whether `execute` came from the adapter's timestamp queries or
from a wall clock, already counts commands, culled commands and bytes transferred — and already
blocks on the device (`poll` with an indefinite wait) before it returns. `render-quorra` was
throwing the whole `Frame` away. So the boundary is real, the wait was always there, and this round
*reports* numbers rather than causing them. `FrameCost::execute_measured` carries quorra's own
distinction between evidence and context into our output.

The one stage this project had to time itself is `scene` — the display-list walk in
`render_quorra::present::build`, uploads included — plus `settle`, `host` and `fallback` on our own
side of the boundary. Four `Instant`s.

Uploads are **counted, not timed**. Every upload in that walk is followed by either a
`transient.push` or a `ResourceCaches::store_*`, so the count is `transient.len() + stored`, which
is three `+= 1`s in code that was already being read rather than nine `Instant` pairs in code that
was not. The count is also the more diagnostic of the two: the caches are keyed by `Arc` identity,
so a display list rebuilt from scratch every frame would re-upload everything, and a number that
stayed high on an unchanged page is what would say so. (It does not: a page's first frame uploads
90 to 793 resources and its second uploads 2.)

### 2. The frame's number is the frame's

`redraw_requested` started a timer, called `present`, called `Launch::arrived`, called `App::attend`
— the accessibility publication — and only then read the timer. The todo called that a measurement
defect rather than a design choice, and it was exactly that.

The timer now closes after `present` returns. `attend` is measured separately and named separately,
in the same line, so nothing is hidden and nothing is misattributed. On the local run below it is
**81.7 ms over 40 frames — 6.7% of what the old figure would have reported as the graphics
device's.**

### 3. The verbosity is a set of topics, argued rather than picked

The item asked for this explicitly, so here is the argument.

**A level was rejected because the topics are not ordered.** `--trace` covers seven things: the
launch path, frames, core commands and events, window events, pointer movement, the accessibility
bridge, and the selection's shape count. A person chasing a slow frame wants frames and nothing
else. A person chasing a window that never appears wants launch and window. A level has to decide
which of those two is "more verbose" than the other, and there is no such fact — any ordering would
be this project asserting one, and the wrong one for half the people who type it.

**A threshold — print only frames over N milliseconds — was rejected on two grounds.** It answers
only the frames question, leaving the other 285-line problem untouched; and the summary at exit
already gives the distribution a threshold approximates, without the risk that the one frame a
person cared about fell a millisecond under the bar. A threshold discards data to save lines; a
topic set discards *lines a person said they did not want*.

**So: `--trace=<topics>`, comma-separated, with `-` to subtract.** `--trace` alone still means
everything, because that is what it has meant for a hundred sessions and it is what the project
owner's own invocation types — the flag may gain a value but may not change meaning without one.
The equals form rather than a following word, because `--trace document.pdf` must keep working and
only the equals form can promise that. A list that *starts* with a subtraction means "everything
except", so the one word that answers the complaint that raised this item is `--trace=-pointer`;
the one that answers a slow page is `--trace=frames`, which is 64 lines where `--trace` is 453.

### 4. One line per frame, and the summary carries the distribution

The item's constraint is the owner's: enough to debug this, without flooding the output. So the two
lines a frame used to print became one, with the stages in it:

```
frame p3 2822cmd presented 75.3 | host 0.0 scene 2.3 device 73.0 settle 0.0 attend 2.9 | 793 up, 12 culled
```

`fallback` and `attend` appear only when they are not zero, so a frame that did nothing unusual is
no longer than the two lines it replaced. Seven lines of legend are printed once, before the first
frame, because a column called `settle` teaches nothing on its own.

Percentiles are at exit, where they cost nothing per frame, by **nearest rank** so that every figure
printed is a frame that actually happened. The table names `elsewhere` — `Device::render` minus the
three phases it reports, which is the swapchain acquire, the present and the timestamp readback —
rather than leaving a reader to subtract, because an unnamed remainder is where a cost hides. It is
13% of the frame here.

Every line carries the seconds since `main`'s first instruction. That is the item's sixth point and
it costs one `Instant::elapsed` against a `println!`: a gap in the log is now legible as a gap, and
the interval a person waited between a key and a frame can be read off two lines.

## What it costs

- **Unconditional, whether anything asks or not**: seven clock reads and a 96-byte copy per frame,
  **0.30 µs** measured over 200 000 iterations on this machine — 0.001% of a 29 ms frame. This is
  held to the standard the item set for it (`Launch`'s timeline is gathered the same way, and it
  says nanoseconds).
- **One frame line printed**: **0.23 µs**, same instrument.
- **Memory**: nothing is *kept* unless `frames` was asked for; the summary's sample buffer is
  capped at 16 384 frames (≈2 MB) and the summary says outright when its percentiles came from a
  prefix.
- **End to end**: the viewer's own CPU time over three identical scripted sessions, in seconds —
  no flag 5.51 / 4.87 / 4.89, `--trace=frames` 5.19 / 4.94 / 4.83, `--trace` 5.17 / 4.68 / 4.90.
  The spread *within* each condition (0.64 s) is larger than any difference between them, so what
  this says honestly is that the instrument is below the noise floor of a five-second session, not
  that it is free.

## The claim that was settled

The trace contained a contradiction, and the item asked which half was false.

- Line 2: `note: this build has no accessibility bridge — AccessKit's macOS and Windows adapters
  exist and are not wired in here`.
- Line 46: `trace: accessibility bridge up`, followed by `accessibility: 0 element(s), 0 report(s)
  on page N` at every page turn thereafter.

**The note is true and the trace line was false.** `viewer_accessibility::Bridge::new` builds an
`accesskit_unix::Adapter` under `#[cfg(target_os = "linux")]` and, on every other platform, a struct
with no adapter in it at all — so what came "up" on Windows was a tree with nowhere to publish it.
The line now asks `Bridge::shortfall()`, which is the same function the note came from, so the two
cannot disagree again.

And the item's prediction was right about the consequence: the per-page query does run where there
is no bridge, and it *was* inside every frame's figure. It costs 2.0 ms on average and 3.9 ms at
worst, on every page turn, on a platform that discards the result. That is a defect, and it is
written into `doc/todo/45-where-a-frame-goes.md` rather than fixed in the round that measured it.

## What the instrument then said

The first local run of the owner's own document — `NorthAmerican.30MB.pdf`, 65 pages, 30 MB, 40
frames, `Xvfb` at 1200×1500 on `llvmpipe`. **This machine's software adapter is not the owner's
Intel UHD through DX12, so what follows is about shape and not about absolute numbers.**

```
40 frame(s), milliseconds:
                median      p90      max       sum
 frame            29.6     35.5     75.3    1225.8
 host              0.0      0.0      0.0       0.3
 scene             1.5     12.8     35.2     226.2
 device           23.7     33.3     73.0     998.9
   encode         11.4     19.2     30.8     478.6
   transfer        2.5      4.0     15.0     127.8
   execute         5.3      7.0     19.5     233.2
   elsewhere       3.4      4.8     18.2     159.3
 settle            0.0      0.0      0.0       0.3
 5355 resource upload(s), 793 in the busiest frame; execute is the device's own timestamps
```

Four things the old single number could not have said:

1. **81% of a frame is inside `quorra_gpu::Device::render`**, and the largest single part of it is
   `encode` — 39% of the whole session's frame time, and CPU work, not the GPU's. The device's own
   passes are 19%.
2. **This host's own work is nothing**: 0.3 ms of overlay lists, geometry queries, panel and
   selection over the entire session, against 1225.8 ms of frames.
3. **Our translation is 18%, and it is bimodal rather than proportional to the page.** Most pages
   cost 0.5 to 1.6 ms; pages 12 to 22 and 27 to 28 cost 5.1 to 15.9, and the worst of them
   (page 19, 15.9 ms) has **388 commands** where a 3675-command page costs 1.0. Whatever is
   expensive there is per *resource*, not per command — and the old figure could not have
   distinguished that from the device being slow.
4. **The 8 refusals the Windows trace showed did not reproduce here**: `fallback` is zero in every
   column. The budget refusal is the owner's adapter's, not the document's.

## The lesson

**An instrument that cannot be subdivided will be believed anyway.** `present -> presented in T`
was read for a hundred sessions as "the frame", and 6.7% of it was a screen-reader publication on a
platform with no screen reader attached. The fix was not more measurement — most of these numbers
were already being taken, one crate down, and discarded at a boundary that returned `Ok(())`.
