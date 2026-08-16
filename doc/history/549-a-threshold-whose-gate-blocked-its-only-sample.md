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

## The round was taken twice

The first fix was shipped and the owner ran it (`tmp/trace3.entwurf.txt`). Much better — the
cadence read **120 Hz stated by the surface**, and **7 of 24 presents were reprojections** against
0 of 15 — and still *"it still doesn't work"*, because six view changes showed nothing and they
were the quick ones a person notices most. Rule 4's bar was a *tenth* of the frame; `measured` was
16.2 ms, so the bar was 162 ms, and reprojections of 6 to 16 ms were refused against frames of
57.7, 71.0, 90.2, 104.5, 155.5 and 156.3. **A tenth of a real device's frame is less than what a
readback costs on it.** Same defect, one layer down: a number the project chose rather than
measured, in the way of a decision about two things that are both measured.

`SHARE` is gone. Standing in must **buy at least one refresh** — `reprojection + period ≤ frame` —
because a period is the smallest difference the display can show. There is now no constant in this
design that was not measured, and the two remaining bounds are both the surface's own.

And the other half of why the owner had to write twice: **that refusal was silent.** `Stale::plan`
returned `Option<Transform>` and every refusal was `None`. It now returns a `Plan`, and the one
refusal that is a judgement rather than an impossibility prints both numbers it judged.

## What was built

Four changes from the first pass and two from the second, all inside
`crates/viewer-ui/src/bin/pdf-viewer/`. ADR 0384 has the argument.

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

**The second bound is what the harness could not see**, and the reason is the lesson of the round:
with `SHARE` in place llvmpipe's own reprojection cost put the bar at 300–372 ms, above every
document in this repository, so the change looked like a no-op here. Under the refresh bound, the
same script on the owner's own witness (`tmp/Entwurf.pdf`):

| | presents | reprojections | median interval |
|---|---:|---:|---|
| before the round | 18 | **1** | 259.1 ms |
| after the first pass | 18 | **1** | 259.1 ms |
| after the second | **32** | **15** | **142.6 ms** |

Fifteen of sixteen view changes; the sixteenth is an atlas repack. Churn is still refused and now
says so — `doc/PDF20_AN001-BPC.pdf` at deep magnification prints `no reprojection: one costs
17.9 ms here and this frame is expected to take 27.3, so standing in would not gain the 16.7 ms
refresh it delays the real frame by`.

Nothing about Wayland is exercised here: `Xvfb` is X11. What settled that is the owner's own trace.
And a frame every refresh is still not reached — median interval 142.6 ms — for `doc/todo/36`'s
named reason rather than anything here: the render runs to completion on the event thread.

## The three secondary questions

- **The refresh rate on Wayland**: fixable, fixed, and **confirmed on the owner's own machine** —
  their second trace opens `120.0 Hz — no output claims this window yet, so the slowest display
  attached states it` and closes `120.0 Hz, stated by the surface`. Both routes worked, in order,
  and `doc/todo/36`'s target rate is reached for the first time. §5 of ADR 0384 has winit file and
  line for every claim; `primary_monitor` is `None` on Wayland by construction and is no route.
- **The atlas repacked after 3 of 15 frames**: ordinary, and *not* the explanation for 1 of 15
  replays. Thirteen of fifteen frames were zoom steps and a placement is part of the scene key, so
  they must re-encode; the single replay is the focus redraw. The repacks are a new magnification
  orphaning the previous one's glyph tiles, and the working set — 3 643 223 bytes against an 8 MiB
  budget — says the page fits, so this is not the thrashing pathology the counter exists for and
  raising the budget would change nothing. `Counters::atlas_repacked`'s first witness on real
  hardware, and its answer is "nothing to do". It *does* cost three reprojections, which now say
  so — two of the seven refusals in the second trace are this, by name — and it cannot be worked
  around from here: the repack is reported by the call that presented the frame, so by the time the
  host could react the encode is already invalid and capturing would re-encode, which rule 4
  refuses. No ask for `doc/QUORRA_FEEDBACK.md`.
- **Encode threads**: the number reaching quorra is the one we think. Both window constructors
  spread `&crate::options()`, so the window path is not one that quietly took the default of 1, and
  128.9 ms of encode at 58 009 commands is ADR 0377's multi-threaded column rather than its serial
  one. The gap worth naming is that the number is never *printed*, so this is an inference from the
  shape rather than a reading; recorded in `doc/todo/36` as a one-line trace item.

## Gates

Every one run, all green after both passes, and nothing on a judged path moved. `fmt` clean;
`clippy --workspace --all-targets` silent of Rust lints; **2036 tests run, 2036 passed, 15
skipped**; doctests pass;
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
