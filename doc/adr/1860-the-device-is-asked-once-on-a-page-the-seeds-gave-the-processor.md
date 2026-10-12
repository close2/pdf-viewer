# 1860 — The device is asked once on a page the seeds gave the processor, and the seeds are re-measured

Session 1511. Status: **accepted** and **built**. Amends ADR 1831's lane rule (its section 4 named
the gap) and replaces its seeds, which ADR 1845 section 6 found stale; `doc/questions/A393`'s answer
stands — page one may go to the processor where it is predicted the faster lane. Context: ADRs 1823,
1831, 1841, 1845. Code: `crates/viewer-ui/src/bin/quorra/renderer.rs` (`Costs`, `Measured`).
Instruments: `render-raster`'s `zoom_frame`; `quorra --trace=frames` on the witness. No ledger row
moves.

## 1. What the window did

`Costs` priced a lane that had not drawn an arrangement's pages by a seed: the device at 1 788 ns a
command, the processor at 667. The processor's seed was replaced by a measured price the moment it
drew a frame. The device's could be replaced only by a frame the device drew, and on a page the seeds
gave the processor the device was never asked again, so its seed stood for as long as the window was
open. On the witness the window kept predicting 7.9 s on the device while the device drew it in about
1.1 s. A machine whose device was faster than the processor on such a page would never have used it.

## 2. Decision

**Once the processor has drawn an arrangement, the next frame of it is the device's, unless the
device has already been asked.** That frame is the one the device is measured on, and from then on
both prices are measured and the smaller prediction draws (ADR 1831's rule, unchanged). A refusal
counts as the device's answer: a device that refused the pages at one magnification is not asked again
at the next on its seed, because each attempt is paid in full (ADR 1831 section 1 measured 2.2 s of it
on `llvmpipe`). `Measured::device_asked` holds the fact; `Costs::refused` records it beside
`Refusals::remember`.

**The seeds are re-measured on this machine**, on the witness:

- device, **253 ns** a command: the fit frame headless on the Radeon 890M's `Cpu` lane, `zoom_frame`,
  1 113.7 and 1 212.2 ms over 4 408 406 commands, the quicker taken;
- processor, **120 ns**: the first frame in `quorra` on the owner's display, global pool, 530.0,
  545.9, 554.0 and 659.1 ms over 4 408 405 commands, the quickest taken.

At these seeds the device keeps any frame under about 3.95 million commands (it was about 560 000),
and the witness's first frame still goes to the processor, 0.53 s against 1.12 s.

## 3. Evidence

One clock hold per display, HEAD `4c0c04f9`'s `quorra` (`63d2aaa6…`) against HEAD plus this and ADR
1859 (`ebb8d1ba…`, built with the old seeds, which decide the witness the same way). The witness, its
first frame and ten `plus` notches 300 ms apart, two runs per arm alternating.

- **Owner's display**, load 3.9–6.4. Mine: the first notch went to the device, which drew it in about
  1.1 s; every later frame said "1.1 s on the graphics device (as this window measured it)" and took
  the processor at 0.5 then 0.2 s. HEAD said 7.9 s on the device, not measured, every frame. Last notch
  to the real frame 0.154, 0.148 → 0.142, 0.163 s; first frame drawn on the processor in 554.0, 659.1 →
  545.9, 530.0 ms.
- **Xvfb**, `llvmpipe`, four rayon threads, load 10–18 (not judged, trap 149). Mine: the first notch
  went to the device, which refused it in 629.8 and 1 149.9 ms (its 128 MiB budget). Nothing was
  asked of it after that. Last notch to the real frame 0.265, 0.237 → 0.171, 0.333 s.

What it costs: the frame after the first is the device's. On the owner's display that frame took
about 1.1 s where the processor's take 0.2 s. On a device that refuses, it is one refused attempt.
Both are paid once per arrangement and never again.

## 4. What it leaves

- The price is per command of the whole list, so a zoomed frame's culled draw (ADR 1841) makes a
  lane look cheaper per command than its fit frame was. Both lanes cull, so the comparison holds,
  but a price read at one magnification predicts another only roughly.
- The ask is a whole frame. A device whose host encode can be timed on a sample of the list would
  not need one; raster draws a frame in one call (ADR 0725).
