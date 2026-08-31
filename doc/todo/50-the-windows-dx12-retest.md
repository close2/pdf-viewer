# The Windows DX12 retest: the first traces arrived, and half the file is answered

Status: **open — the owner's machine has spoken once** (2026-08-31, `tmp/win/`, untracked; every
number below is copied from those four files so this one survives their deletion). What was
blocked is now measured; what is owed is the re-run against the fix the first reading produced
(ADR 0761), and the two A/Bs only that machine can take.
Priority: 50 — the remaining runs still need hardware only the project owner has.
Corpus: the owner's own windowed runs of `Entwurf Küchenrückwand.x.pdf` (the 58 009-command page
`doc/todo/44` is about); `tmp/win/entwurf.2.trace.txt` is the full session, `output*.txt` the two
`--backend gl` runs, `entwurf.3.trace.txt` a three-line fragment.
Code: ADR 0761's gate in `crates/viewer-ui/src/bin/pdf-viewer/renderer.rs` is the one change the
first reading justified from Linux; the rest is upstream's (`doc/QUORRA_FEEDBACK.md` §41).
Instrument: a Windows build of `pdf-viewer` with `--trace=frames` on the owner's Intel UHD /
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
3. **The GL panic's backtrace, once**: `RUST_BACKTRACE=1 pdf-viewer.exe --backend gl …` — the
   backtrace names which main-thread call hit the lock, which is what an upstream wgpu report
   wants attached.
4. **Say what `entwurf.3.trace.txt` was**: it ends three lines in, before `backend asked for` —
   a run killed early, or a hang in adapter/device bring-up? If the latter, that is a new item.
5. Still standing from the original file: a traced cold open of the ISO specification, ten zoom
   steps, a drag — the columns against the Linux numbers beside quorra ADR 0095.
