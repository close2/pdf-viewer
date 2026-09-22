# The Windows DX12 retest: the first traces arrived, and half the file is answered

Status: **open — the owner's machine has spoken twice** (2026-08-31, `tmp/win/`, untracked; every
number below is copied from those four files so this one survives their deletion). What was
blocked is now measured; what is owed is the re-run against the fix the first reading produced
(ADR 0761), and the two A/Bs only that machine can take.
Priority: 50 — the remaining runs still need hardware only the project owner has.
Corpus: the owner's own windowed runs of `Entwurf Küchenrückwand.x.pdf` (the 58 009-command page
`doc/todo/44` is about); `tmp/win/entwurf.2.trace.txt` is the full session, `output*.txt` the two
`--backend gl` runs, `entwurf.3.trace.txt` a three-line fragment.
Code: ADR 0761's gate in `crates/viewer-ui/src/bin/quorra/renderer.rs` is the one change the
first reading justified from Linux; the rest is upstream's (`doc/QUORRA_FEEDBACK.md` §41).
Instrument: a Windows build of `quorra` with `--trace=frames` on the owner's Intel UHD /
DX12 machine.

## What the first traces answered

- **The 6.4 s atlas-flush pathology is gone on DX12** — the retest this file was opened for.
  quorra ADR 0078's batched flush was taken on Linux evidence alone; on the same page whose first
  present once spent 6.4 s in `queue.write_texture`, the first frame's `transfer` is now
  **2.6 ms** (device 256.2 = encode 228.2 + transfer 2.6 + execute 6.2 + elsewhere 19.2).
- **The launch path holds its shape there**: device up 181.6 ms, surface configured at 558.7 ms,
  first present 2300.1 ms of which **1303.0 ms is interpretation** — `doc/todo/44`'s largest step
  on a slower processor (670–719 ms on this machine), not a Windows pathology. The 1.3 s
  `resize … in 1.3014148s` line at t=1.862 *is* that interpretation step (same instant, same
  duration in the launch table), **not** `doc/todo/47`'s 9–19 ms resize-frames item. Pipelines
  compiled in 244.1 ms in the background; nothing on the launch path waited for them.
- **ADR 0699's unbounded sharp pass was the worst item in the trace**: 8 867.6 ms on the render
  thread for the settled view at 2×, during which the person's zoom waited ~8 s to start
  rendering, presents blocked for up to 5.25 s (a DX12 present executes after whatever was
  submitted before it on the queue), and not one idle-thread proxy was drawn. ADR 0761 gates the
  pass on a prediction from the last built frame; the decline on this machine is certain (431.6 ms
  first frame → predicted 1.7 s against a 400 ms budget).
- **The compute lane is priced very differently on this adapter**: a moved-view frame's `device`
  was 2161.3 ms of which `elsewhere` — the frame's own end-wait for the compute chain — was
  2123.6, against 53–66 ms per zoom step on the 890M. ~35–150× where raw throughput predicts
  12–20×; quorra ADR 0091's occupancy suspicion fits. Upstream's to attribute:
  `doc/QUORRA_FEEDBACK.md` §41.
- **`--backend gl` panics in wgpu-hal 30.0.0** (`gles/device.rs:649`, "Could not lock adapter
  context"): the gles backend's WGL context guard is a one-second `try_lock_for(…).expect(…)`, so
  any GL operation over a second — plausibly the render thread's submit, which on gles executes
  its GL commands under that lock — panics whichever other thread touches the device. Checked against our own usage first: two threads on one wgpu
  device is inside wgpu's `Send + Sync` contract, so this is upstream's limitation, not our
  misuse. No host-side repair short of single-threading the GL path; DX12 stays the default and
  the answer.

## What the second trace answered

`tmp/win-prob/trace.txt` (2026-09-22, untracked; the numbers are copied here): the same Intel UHD
through DX12, a 21 907-page `out.pdf` at scale factor 1.5, `--supersample 2`, three page turns and
one zoom notch. Read against the first:

- **Bring-up is inside what the tree expects, and smaller than last time**: `device up in
  110.1726ms — … adapter 67.1639ms, device 38.8778ms, pipelines 109.3691ms` against 181.6 ms.
- **But the launch path waited about as long as the pipelines took, in a place that says it
  cannot.** `graphics device 221.926` → `surface configured 342.874` is +120.9 ms, of which the
  configure itself is `13.9558ms`; the remaining ~107 ms is `renderer::Window::split`, whose
  comment says it compiles nothing and cannot block — and the device-up line, printed after it,
  already reports the 109 ms compile finished. Under `Xvfb` the same step is ~22 ms in total
  (`doc/performance.md`). The coincidence says something in `split` (`detach_presenter`,
  `medium_texture` or `presenter.resize`) waits behind the compile on DX12. **Priced, not
  built**: time each of the four calls under `--trace=launch` on that machine; if one waits, the
  fix is raster's (which device call the compile thread holds), and `CLAUDE.md`'s rule is broken
  by ~100 ms until it is.
- **Interpretation is the launch and the page turn**: `interpreted, 1196 cmd 1084.548 ms
  (+741.199)` is 63% of `first present 1179.754`, and every turn costs the event thread the same
  — `go to Next -> 2 event(s) in 925.0303ms`, `746.1746ms`, `726.8342ms`, `732.2653ms` for 1196
  to 1766 commands. The first trace's page cost 1303 ms for 58 009 commands, so ~700 ms here is a
  fixed cost per interpretation, not per command. A synthetic flat 21 907-page file costs
  `render_at` 12 ms for page 2 on Linux against 8 ms for a five-page one, so the page tree is not
  it; what is, is a property of that file (a font parsed per interpretation is the first
  suspect). **Owed**: the file, or a run of `pdf-model --example render_at` on it. The event
  thread interpreting at all is `doc/todo/46`'s.
- **The three resizes at 1.085 are not three renders.** Each is a `needs render` in 1.2–4.1 µs
  with no interpretation; the render thread takes a job only when none is in flight, so one frame
  was drawn (`RenderToken(0..2)` all answered at 1.180 by the one `136 up` frame). Nothing to
  dedupe.
- **The frame table's 350.5 and 268.4 are sums, not maxima**: four frames drew (85.5, 92.9, 74.8,
  97.3 ms of `device`), the worst `transfer` is 78.9 ms, and the 18 360 044-byte frame is one of
  the two new-view frames (p3 at 6.813 or the zoom at 31.568). **This is not the 6.4 s
  `flush_atlas_tiles` pathology** — that was 58 k calls, and the atlas here wanted 118 662 bytes —
  it is ADR 0387's encoded scene (coverage tiles and instance streams), moving at ~230 MB/s. On
  an integrated GPU that is slow; which part of the 18 MB is which is the question, and raster's
  `bytes_uploaded` does not split it. Priced: a per-kind split of that counter, then this run.
- **`256 resource upload(s), 136 in the busiest frame`** is the first frame's 136 plus 80 and 40
  on the two turns — outlines, which is `up` and not `transfer`.
- **The first sharp pass cost `514.8 ms`**, admitted by ADR 0761's gate on an 85.5 ms first frame;
  the later ones were 112–117 ms. It ran while nothing else was asked, so nobody waited; a
  prediction four-and-a-half times short is still worth a line when the gate is next touched.
- **The icons that looked like JPEG artifacts** were the image lane filtering straight alpha: the
  colour under a soft mask's zeros — black in every reduced variant — leaked along the mask's
  edges wherever a device pixel gathered two samples or more, and not below that. Fixed and held
  by a fixture (ADR 1287); the owner's own file would confirm it is the same mechanism.

## What to run, when the machine is next available

All with a build carrying ADR 0761, `--trace=frames`, and the same document:

1. **The re-run of the same session**: cold open, let it settle ~5 s, one zoom gesture in, one
   out, close. Expected against `entwurf.2.trace.txt`: no multi-second `sharpened:` line (the
   pass declines silently), presents whose p90 interval is milliseconds rather than 2302 ms,
   proxies drawn (`N of M retained low-resolution page(s)` with M > 0), and the first real zoom
   frame in roughly the compute lane's own 2.2 s rather than twelve. The remaining seconds-long
   term is then the compute lane itself, cleanly separated.
2. **The lane A/B this machine alone can price**: the same gesture under `--coverage cpu`. On
   this adapter the atlas lane's first frame cost 432 ms total with the GPU nearly idle
   (execute 6.2 ms), so the CPU lane plausibly beats the compute lane's 2.2 s per moved view
   here — if it does, the `lane_for` moved-view rule (measured on the 890M) needs an adapter
   condition, and this run is the number it needs.
3. **The GL panic's backtrace, once**: `RUST_BACKTRACE=1 quorra.exe --backend gl …` — the
   backtrace names which main-thread call hit the lock, which is what an upstream wgpu report
   wants attached.
4. **Say what `entwurf.3.trace.txt` was**: it ends three lines in, before `backend asked for` —
   a run killed early, or a hang in adapter/device bring-up? If the latter, that is a new item.
5. Still standing from the original file: a traced cold open of the ISO specification, ten zoom
   steps, a drag — the columns against the Linux numbers beside quorra ADR 0095.
