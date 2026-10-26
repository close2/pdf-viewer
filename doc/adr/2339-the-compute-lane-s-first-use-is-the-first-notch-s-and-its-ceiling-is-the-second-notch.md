# 2339 — The compute lane's first use is the first notch's, and its ceiling is the second notch

Status: rejected — ADR 2323's first remainder, priced at its ceiling (habit 164) and not built.
Session 1752, 2026-10-26, the round of ADR 2338 and its two holds. Context: ADRs 2322, 2323, 0700;
raster's ADRs 0080 (the lane's pipelines compiled on first use) and 0095 (the persistent edges
capacity); CLAUDE.md principle 2 (nothing on the launch path waits for warmth). Code read:
`raster_gpu::compute::{dispatch_chain, Pipelines::compile, ComputePersist::grow}`, the overflow loop
in `raster_gpu::device::render`, `viewer_device::lane::lane_for`.

## 1. What the first use is

Read in an export of HEAD whose raster printed two lines to the terminal, deleted with it:

- **the four compute pipelines**, compiled inside the first `Compute` frame: 1.99 to 6.12 ms over
  54 devices, the driver's shader cache warm;
- **the edges capacity's first guess** — 32 edges a tile, at least 65 536, so 99 200 for the
  window's 3 100 tiles — against the 246 217 the scan counted: every first window overflowed and
  ran again whole, which ADR 0095 accepts as "the first frame's ordinary cost";
- **segment residency**, each outline uploaded once.

## 2. The ceiling, against the bar

The same second hold as ADR 2338, gated arms. The first window's device job, ms: as shipped 8.6 to
13.9; with the capacity sized before it, so no second run, 7.0 to 8.6; with every first-use cost
paid before it — a `Compute` frame of the launch's window drawn first — 5.1 to 6.3, which is the
second window's 4.7 to 5.9. So the whole first use is 3 to 8 ms of the first notch's window, and
its ceiling is a later notch's window.

**That does not clear the bar.** On the owner's adapter the second notch settles at 9.9 to 11.2 ms
(ADR 2323), and the first notch draws pages 20 and 21: at its ceiling it is the second notch with
a second window beside it, past 8.33 ms on every run. A lever whose ceiling leaves the notch past
the frame is not built (habit 164).

## 3. Decision

1. **Not built.** `lane_for`'s comment names the first use's three parts and cites this ADR.
2. **The two parts, priced, for a round whose bar they clear**: the four pipelines added to
   raster's warm-up thread on a device whose host asks for them (an option, because a corpus
   device never takes the lane), worth the 2 to 6 ms compile; and the first `Compute` frame sizing
   its edges from its own scanned total, one read of the count before the emit, in place of a
   second run — worth 1.5 to 5 ms here, unmeasured on `quorra`'s worst page. Neither may put work on
   the launch path: the first is a thread nobody waits on, the second runs on the first moved view.

## Costs

None in code; the first notch keeps paying 3 to 8 ms of first use on this page.
