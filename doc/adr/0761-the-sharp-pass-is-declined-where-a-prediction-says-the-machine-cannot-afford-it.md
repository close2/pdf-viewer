# 0761 — The sharp pass is declined where a prediction says the machine cannot afford it

Status: accepted.
Context: the project owner's first Windows DX12 traces (2026-08-31, `tmp/win/` — untracked, the
numbers copied here so this file survives their deletion), `doc/todo/50`'s standing retest, ADR
0699's settled-view sharp pass, ADR 0384's rule-5 prediction, quorra ADR 0078 (the atlas-flush
batch) and ADR 0095 (the compute lane as one submission).
Code: `crates/viewer-ui/src/bin/pdf-viewer/renderer.rs` (`SHARP_STALL_BUDGET`,
`sharp_pass_affordable`, and the gate in `draw_until_told_to_stop`).

## The traces, read

The owner ran the Windows build on their Intel UHD machine — DX12, a 120 Hz surface — on
`Entwurf Küchenrückwand.x.pdf` (the 58 009-command page `doc/todo/44` is about), and on the same
document under `--backend gl`. Four findings, in the order of their importance:

### 1. ADR 0078's fix is confirmed on DX12 — the retest `doc/todo/50` existed for

The one earlier Windows trace measured **6.4 s** of `queue.write_texture` calls flushing the
atlas on this page's first present. In this trace the same page's first frame moves **2.6 ms of
`transfer`** (device 256.2 = encode 228.2 + transfer 2.6 + execute 6.2 + elsewhere 19.2). The
batched flush holds on the backend it was written for and never measured on. First present is
2300.1 ms end to end, of which 1303.0 ms is interpretation — the same largest-step shape as the
Linux launch (`doc/todo/44` §6), scaled by a slower processor — and nothing on the launch path
waited for the pipelines (compiled in 244.1 ms, in the background, noticed after the first frame).

### 2. The unbounded sharp pass is the worst thing in the trace

Right after the first present, the render thread began ADR 0699's settled-view 2× pass and it ran
**8 867.6 ms** on this adapter (the compute lane at four times the window's pixels). During it:

- the zoom the person made at t=3.25 could not start rendering until t≈11.2 — the real frame it
  was waiting for landed at t=15.7, **twelve seconds after the wheel turned**;
- the event thread's presents blocked for **5250, 2582, 2097 and 2297 ms** — on DX12 a present is
  a queue operation and executes after whatever was submitted before it, so the reprojection
  quads that exist precisely to keep the window alive queued behind the pass. The event loop
  delivered nothing between t=3.290 and t=8.540; the summary reads intervals *median 8.7 ms,
  p90 2302.4, max 5253.9*;
- the idle-thread proxies never got a turn — every stand-in printed `0 of 0 retained
  low-resolution page(s)` — because the thread that draws them was never idle.

ADR 0699 priced the pass at ~350 ms of idle-thread time on the worst page and accepted it. This
trace shows the same term at twenty-five times its pricing, plus a consequence that pricing never
saw: on DX12 the pass starves the *presents*, not only the next job.

### 3. The compute lane itself is priced very differently on this adapter

A moved-view real frame mid-zoom: `device 2161.3` of which `elsewhere 2123.6` — the frame's own
end-wait for the compute chain (quorra ADR 0095's readback) — against 53–66 ms for the same
page's zoom step on the 890M (`doc/todo/46`). The raw compute-throughput ratio of the two
adapters is roughly 12–20×; the observed ratio is ~35–150×, which is consistent with quorra ADR
0091's occupancy suspicion (~170 scalars of per-thread state) costing disproportionately on a
small register file. One frame moved 171 400 618 bytes of encoded scene. Nothing on this side can
attribute deeper; `doc/QUORRA_FEEDBACK.md` §41 carries it upstream with the numbers.

### 4. `--backend gl` panics in wgpu-hal, and it is not our misuse

Both GL runs panic on the main thread at `wgpu-hal-30.0.0/src/gles/device.rs:649`
(`create_buffer`): the gles backend guards its WGL context with
`try_lock_for(Duration::from_secs(1)).expect("Could not lock adapter context…")`, so any single
GL operation that holds the context longer than one second — plausibly the render thread's
submit, which on gles executes its translated GL commands on the submitting thread under that
lock, here for a 58 009-command scene — panics whichever *other* thread touches the device. Two threads
sharing one wgpu device is inside wgpu's own `Send + Sync` contract; the one-second try-lock is
wgpu-hal's, documented in its source as a deadlock heuristic. This architecture (render thread
drawing, event thread presenting) will always trip it on a GL adapter slow enough, and there is
no host-side repair short of single-threading the GL path. Recorded in `doc/todo/50` and
`doc/QUORRA_FEEDBACK.md` §41.3; the practical answer on Windows is DX12, which is the default.

## The decision

**The sharp pass is declined, silently, where its predicted cost exceeds a budget.** The
prediction is four times the last frame the render thread had to *build* (the pass draws the same
commands at four times the pixels; a replayed frame's cost predicts nothing and is excluded, the
same reading rule 5's prediction makes in `surface.rs`). The budget is 400 ms — the cost ADR 0699
already priced and accepted, written down as a bound.

**The number is a choice between two measured populations, not a derivation.** The 890M builds a
settled zoom frame of the worst page in 53–66 ms → predicted ≤ 264 ms → the pass runs. The
Windows Intel UHD built its first frame in 431.6 ms → predicted 1726 ms → declined (its actual
unbounded pass was 8 867.6 ms). Any bound between those populations decides both machines
correctly, on either cadence the owner runs. Consequences accepted and written down:

- A machine near the boundary loses the pass for a heavy view and keeps it for ordinary pages —
  the decline costs a seam drawn at full width rather than half (ADR 0699's own accounting), not
  a wrong pixel.
- On llvmpipe the worst page's pass is now declined too (its 1× frame is ~640 ms); ADR 0699's
  measured scenario keeps the pass only on pages whose frames are cheap, which is most of them.
- The launch view of a heavy page on the 890M (first frame ~700 ms) no longer sharpens until the
  first interaction settles on a cheap frame. Accepted: the launch path pays for nothing it can
  defer, and this was the one place the sharp pass ran with a cold, worst-case predictor.
- The decline is **silent**, by `draw_sharp`'s standing rule (a picture nobody asked for; a
  refusal costs a softer picture and nothing else). What a person verifying this on Windows
  observes is the *absence* of a multi-second `sharpened:` span and presents that keep their
  cadence — `doc/todo/50` says exactly what to run.

## Declined

- **Banding the sharp pass into several submissions** (render in damage-rect stripes, abandoning
  between stripes when a job arrives). Structurally better — it would also bound the DX12 present
  starvation and the GL context hold — but it multiplies the encode walk per stripe, its cost
  cannot be measured on the adapter it is for from this machine, and the interruptibility it buys
  belongs at the submission level, which is quorra's. Written up as the ask in
  `doc/QUORRA_FEEDBACK.md` §41.2 rather than built speculatively here (`doc/todo/47`'s rule:
  attribute before building — and the attribution for *this* fix is in hand, while the banding's
  is not).
- **An adapter-name table** ("Intel UHD → CPU lane", or "→ no sharp pass"). A list of names
  decays and measures nothing; the cost-based gate reads the machine it is on.
- **Tracing the decline.** The render thread has no trace handle; adding a channel variant to say
  "nothing was done" inverts `draw_sharp`'s silence rule for no observable a person needs.

## What only Windows can verify

The gate's effect on the UHD — presents holding cadence through a zoom, no multi-second
`sharpened:` line, the first real zoom frame arriving in seconds rather than twelve — needs the
owner's machine. `doc/todo/50` §"What to run" is updated with the exact runs, including the
`--coverage cpu` A/B that would price finding 3's lane question on the only machine that can.
