# 549 — A self-calibrating threshold whose own gate blocked its only sample

2026-08-16. A defect round on the project owner's own report, against
`tmp/trace2.entwurf.txt` from their machine — AMD Radeon 890M under RADV, Wayland.

## The report

> I don't have the impression that reprojection works.

Fifteen presents, thirteen zoom steps, frames of 80.8 to 437.9 ms, **zero reprojections**. The
feature `doc/todo/36` and `doc/todo/37` were built for, over two sessions, did nothing on the
machine it was for.

## The diagnosis, verified against the trace frame by frame

`Stale::threshold()` was `measured.unwrap_or(ASSUMED) * SHARE`. `measured` is set only by drawing a
reprojection, and a reprojection was only drawn above the threshold, so until one existed the bar
was 51 ms × 10 = **510 ms** and nothing could bring it down. On any machine quicker than the
llvmpipe adapter `ASSUMED` was measured on, it never comes down. No value of the constant fixes
it — the fault is the direction of the dependency.

The owner's trace had a second, independent failure stacked on it. The launch frame cost 778.6 ms
and *would* have cleared the bar, but a `Focused(true)` event redrew the same view before the first
zoom, quorra replayed the retained encode in 2.1 ms, and `Stale::settled` overwrote the cost with
that. **The bootstrap sample was destroyed by a frame that drew nothing new.**

## What was built

Four changes, all inside `crates/viewer-ui/src/bin/pdf-viewer/`. ADR 0384 has the argument.

1. **Rule 5 is the cadence's own period.** A *miss* is a frame that does not land inside
   `Cadence::period` — the owner's own word, and a measurement the presenter holds before anything
   has been drawn. No calibration, no bootstrap, no first sample. `Stale::missed`.
2. **Rule 4 becomes a separate check, and unmeasured now permits.** `Stale::affordable` — `SHARE`
   times what a reprojection actually cost on this machine, against the frame it stands in for.
   Before there is a measurement it permits, because rule 5 has already established the miss and
   the first reprojection is the only thing that can produce the number.
3. **A replayed frame no longer speaks for what a render will cost.** `Settled::cost` is gone;
   `Stale::building` replaces it and is updated only by a frame that built its picture, read off
   quorra's `FrameCost::encode_source` rather than inferred from a small duration. A view change can
   never replay, so a replay is evidence about a class of frame rule 5 never asks about.
4. **Two silent refusals now say so.** `capture_presented` returning `Ok(None)` — no retained
   encode, usually because the last frame repacked the atlas — and a raster in a layout this host
   cannot read. A refusal a person cannot see is indistinguishable from a feature that does not
   work, which is the report this round was opened by.

And, from the same trace, the Wayland defect: **`Cadence::of` was asked one moment too early.**
winit's Wayland `current_monitor` is the first output in the surface's `wl_surface::enter` list and
a Wayland surface enters no output until it has been drawn to — `resumed` is strictly before the
first present, so every Wayland session took the floor and 120 Hz was unreachable in principle. The
cadence now re-asks after each present until the window's own output answers, with
`available_monitors`' *slowest* standing in meanwhile. Three sources, three sentences in the trace.

## What was measured

`Xvfb` at 900×1100, llvmpipe, release binaries of this tree before and after, driven by `xdotool`:
ten `+` and six `-`, spaced 250 ms so each waits for the frame before it.

**The A/B is on `doc/PDF20_AN001-BPC.pdf`, which is in the repository** — the case this round was
told to construct, a frame that reliably exceeds one refresh while staying far below 510 ms:

| | presents | reprojections | frames median / p90 / max |
|---|---:|---:|---|
| before | 17 | **0** | 15.0 / 21.3 / 37.6 ms |
| after | 18 | **1** | 7.3 / 30.5 / 37.7 ms |

```text
approximated: this view's frame is expected to cost 37.7 ms against a 16.7 ms refresh, so it
misses, and the last rendering's own pixels stand in (read back in 9.4 ms, whole reprojection
30.4 ms); the real frame has been asked for
```

37.7 ms is fourteen times below the bar that used to be there. ADR 0378 recorded this same document
producing "six frames and zero reprojections" and read it as rule 5 working.

**What the harness cannot say, stated because the harness is what hid the defect.** On llvmpipe a
reprojection costs 30 to 37 ms, so `SHARE × measured` lands at 300–372 ms and rule 4 refuses
everything after the first — on the owner's own witness (`tmp/Entwurf.pdf`) both binaries therefore
produce one reprojection of eighteen presents, the same one, and the harness shows no difference at
all. What decides it on his machine is what a readback costs on a real device against frames of 80
to 438 ms. **The trace now prints that number on the first reprojection of every run**, so his next
report answers it. Nothing about Wayland is exercised here either: `Xvfb` is X11.

## The three secondary questions

- **The refresh rate on Wayland**: fixable, and fixed. §5 of ADR 0384, with winit file and line for
  every claim. `primary_monitor` is `None` on Wayland by construction and is no route.
- **The atlas repacked after 3 of 15 frames**: ordinary, and *not* the explanation for 1 of 15
  replays. Thirteen of fifteen frames were zoom steps and a placement is part of the scene key, so
  they must re-encode; the single replay is the focus redraw. The repacks are a new magnification
  orphaning the previous one's glyph tiles, and the working set — 3 643 223 bytes against an 8 MiB
  budget — says the page fits, so this is not the thrashing pathology the counter exists for and
  raising the budget would change nothing. `Counters::atlas_repacked`'s first witness on real
  hardware, and its answer is "nothing to do". It *does* cost three reprojections, which now say so.
  No ask for `doc/QUORRA_FEEDBACK.md`.
- **Encode threads**: the number reaching quorra is the one we think. Both window constructors
  spread `&crate::options()`, so the window path is not one that quietly took the default of 1, and
  128.9 ms of encode at 58 009 commands is ADR 0377's multi-threaded column rather than its serial
  one. The gap worth naming is that the number is never *printed*, so this is an inference from the
  shape rather than a reading; recorded in `doc/todo/36` as a one-line trace item.

## Gates

Every one run, all green, and nothing on a judged path moved. `fmt` clean; `clippy --workspace
--all-targets` silent of Rust lints; **2035 tests run, 2035 passed, 15 skipped**; doctests pass;
sandbox and `pdfref-hayro` binaries built; corpus gate **974 documents, 0 unopenable, 8 locked, 2
encrypted beyond us, 6 pageless, 64 incomplete, 0 slow**; oracle passes over **1794 pages, 67
contradicted, 786 ambiguous**; both text gates pass (**98.26%** of matched words in bounds, 486 of
508 documents fully in bounds); dates, XMP and JPEG 2000 pass; conformance passes (5 tests, 875
subclauses); `render-quorra`'s default lane **956 pages: 931 agree, 23 differ, 2 refused, 18 not
comparable** and its gpu lane at scale 4 **951 pages: 937 agree, 10 differ, 4 refused, 23 not
comparable** — the second matching ADR 0378's record character for character, which is
`doc/todo/37` rule 2's own gate.

One gate caught a mistake in this round's own prose: `every_quotation_is_the_standards_own_words`
rejected a Markdown blockquote in `stale.rs` carrying the *owner's* words, because a blockquote in
this tree means the standard's. The house style for the owner's words is italics inside quotation
marks, and the checker is right to insist on the difference.
