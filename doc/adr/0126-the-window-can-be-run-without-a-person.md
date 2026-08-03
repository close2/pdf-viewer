# ADR 0126 — The window can be run without a person

Status: accepted, 2026-08-01.

## The claim that was retired

`CLAUDE.md` and this tree's handover have said, for dozens of sessions:

> `AI` has no X authority cookie. Anything needing a window fails at `XOpenDisplayFailed`. The GPU
> backend is headless by construction precisely so it can still be tested; **the viewer binary
> cannot be run by the agent past event-loop creation.**

The first two sentences are true. The third is false, and it was false the whole time: `Xvfb` is
installed on this machine, `lavapipe` is installed as the software Vulkan implementation, and
between them the *actual* winit window runs, renders through vello, and can be driven and
photographed.

```sh
Xvfb :77 -screen 0 900x1100x24 &
DISPLAY=:77 pdf-viewer --trace doc/ISO_32000-2_sponsored_EC3.pdf &
DISPLAY=:77 xdotool windowfocus --sync $(DISPLAY=:77 xdotool search --name ISO_32000 | tail -1)
DISPLAY=:77 xdotool key --delay 300 Right Right Right Right Right
DISPLAY=:77 xwd -root -silent -out screen.xwd && magick xwd:screen.xwd screen.png
# ^ the pipe form this line used to give — `xwd -root -silent | magick - screen.png` —
#   stopped working when this machine's ImageMagick lost its xwd *decode* delegate for
#   stdin: it still reads `xwd:<file>` and no longer sniffs the format from a pipe.
#   Re-checked in the two-hundred-and-thirteenth session; the recipe is the point of
#   this ADR, so a recipe that has stopped working is the ADR being wrong.
```

That is a **complete viewer test**: real event loop, real key events, real surface, real
presented frames, and the window's own pixels back for comparison. It was found while chasing a
report that four pages of ISO 32000-2 would not display, by asking what was actually installed
rather than what the notes said.

## What it is worth

The four gates all stop at the display list or at a raster the harness asked for. Nothing in the
tree had ever exercised **the loop**: a key press becoming a command, a command becoming a
request, a request becoming a frame, and the frame reaching a window. Every defect of the last
three sessions lived in exactly that gap — the mirrored click (ADR 0118), the tier-2 loop (ADR
0117), the page-turn walk (ADR 0124), the frame that failed and said it had not (ADR 0125) — and
each was found by a person running the program, because nothing else could.

This is how that stops being true. It is not yet a gate: `xdotool` and `Xvfb` are not build
dependencies, and a test that skips silently when they are absent is worse than no test (the
handover's own rule). What it is today is a **reproduction method**, written down here so the next
session does not conclude, as many have, that a window cannot be looked at.

## What it did not find

The report that prompted it. Under `Xvfb` with `lavapipe`, all 1023 pages of ISO 32000-2 open,
navigate, present and photograph correctly — including 6, 7, 1010 and 1011 — and a resize
redraws. Two content-based hypotheses were refuted by measurement rather than by argument:

- **geometry**: pages 6 and 7 carry no non-finite coordinate, no clip, no group, no soft mask,
  and their extents match their neighbours';
- **complexity**: they are rank **857** and **835** of 1023 by path point, near the bottom.

So the difference is the *device* — RADV against lavapipe — or the compositor, neither of which
this machine can put under a debugger. What ships instead is the ability to ask the program: a
`--trace` that prints every window event, command, event and frame with its duration, so that the
last line printed is the step that did not finish; a `--cpu` that draws with `render-cpu`, so that
a page appearing with it and not without it is the device's fault, stated; and, finally,
`Device::on_uncaptured_error`, because wgpu reports device errors to a handler and the default one
on this path said nothing at all.

## The lesson

**An environment note is a claim about the world, and it decays like any other.** "The viewer
cannot be run headlessly" was written when it was true of the tooling installed *then*, was
repeated in two documents, and was never re-checked — while the packages that falsify it sat on
the machine. The project already knows this shape: *anything deferred on an external condition
should carry the date it was last verified*. That rule was written for `PLAN.md`'s reasons about
JBIG2 decoders. It applies to the sentence describing your own machine.
