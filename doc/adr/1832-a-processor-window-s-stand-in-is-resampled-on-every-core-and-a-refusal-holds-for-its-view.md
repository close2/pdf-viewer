# 1832 — A processor window's stand-in is resampled on every core, and rule 4's refusal holds for the view it refused

Session 1497. Status: **accepted** and **built**. Reported by the project owner on 2026-10-10: "the
cpu path in quorra would not show anything while zooming (it does not resize previous frames)".
Context: ADRs 0384, 0457, 0461 (rule 4 on the processor's window). Code: `paint` and `paint_row`,
`Stale::declined`, `Stale::settled` and the field `spent` in
`crates/viewer-ui/src/bin/quorra/stale.rs`. Test: `a_resample_beside_the_frame_has_to_be_up_before_it`.

## 1. What was found

Not reproduced as a blank window: under Xvfb, `quorra --cpu`'s window held a picture at every notch
of a zoom of `tracemonkey.pdf` and of the owner's drawing. Reproduced as the frames' timing, at a
2500 × 1600 window (`scratchpad/r1497/cpu-zoom.sh`, `notch.sh`):

- **Rule 4 refuses the stand-in at most notches, by its own arithmetic, and that is the picture the
  owner describes.** A resample is a window of pixels walked bilinearly on the thread that presents:
  134–185 ms on one core, against true frames of 26–40 ms. Rule 4 (ADR 0461) refuses a stand-in
  that would not be up a refresh before the true frame, so the window shows the previous picture
  unmoved until the frame lands — on a larger window or a faster refresh, at more of them.
- **And the refusal undid itself at the next tick.** `declined` cleared the sample at once, so the
  next tick of the *same* view change permitted, resampled and blocked the presenting thread for the
  stand-in it had just called unwise: every notch's true frame went up a resample late, 154–223 ms
  after the notch (mean 192 and 168 ms over two runs of ten).

## 2. Decision

1. **The resample's rows are shared among the processor's cores**, at most eight bands of whole
   rows on scoped threads. Each row samples only the layer and writes only itself, so no byte
   changes; what a band costs is a thread spawn, tens of microseconds. Rule 4 measures what a
   resample costs on the machine it runs on, so the policy follows without a constant moving.
2. **A refusal spends its sample when the view it refused lands**, not at the next tick: ADR 0461's
   reason for spending it — a sample taken while the machine was busy must not refuse every view
   change after it — is about the *next view change*, and that is when it is measured afresh now.

## 3. What it measured

Ten `plus` notches on `tracemonkey.pdf`, `--cpu`, 2500 × 1600, notch to the true frame:
**before** 192 and 168 ms mean, a stand-in at every notch with a resample of 124–185 ms; **after**
47 ms mean (28–65 ms), resamples of 34–41 ms, and five notches of ten waited for the true frame
instead of standing in, because at 36 ms a resample plus a refresh is no longer shorter than a frame
of 30–40 ms. Small walks on a loaded machine; the ratio is the finding, not the milliseconds.

## 4. What it does not do

The window still holds an unmoved picture for one frame where rule 4 refuses; on a page whose frame
is shorter than a resample plus a refresh that is the honest answer, and a resample cheaper still
(fewer pixels, or a scaled copy the toolkit makes) is where a later round would look. `quorra-qt`'s
incorrect frames during a zoom, which the owner reported in the same message, were not looked at:
they are that window's and the next batch's.
