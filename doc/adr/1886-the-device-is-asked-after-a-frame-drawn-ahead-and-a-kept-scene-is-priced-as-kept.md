# 1886 — The device is asked after a frame drawn ahead, a kept scene is priced as kept, and the toolkit windows turn a page as `quorra` does

Session 1525, on the project owner's target of 2026-10-10, "improve performance". Status:
**accepted** and **built**. Amends ADR 1860's lane rule and seeds; extends ADR 1871 §2 to the two
toolkit windows. Context: ADRs 1831, 1855, 1867, 1877; traps 143, 154; habit 88. Code: `Costs`,
`Measured`, `Prediction`, `Window::price` (`quorra/renderer.rs`); `predicted`, `say_the_lane`
(`quorra/surface.rs`); `Host::turn` in `viewer-gtk`'s and `viewer-qt`'s `host.rs`. Tests:
`the_device_is_asked_after_a_frame_drawn_ahead_and_a_kept_scene_is_priced_as_kept`,
`the_owner_s_drawing_stays_on_the_device_on_the_seeds`. Witness: `/tmp/demo_heavy_engineering_A1.pdf`,
and for a turn the same page behind a light one. Cites no clause and moves no ledger row.

## 1. The premise and the hypothesis, checked

**The premise held.** `lane_for` is `surface.rs:100` and `Costs` `renderer.rs:204`; no toolkit host
called `handle_unread`. But `lane_for` picks a coverage lane *on* the device; the lane a notch takes,
device or processor, is `Costs`'. **What sent the notches to the processor was not a stale seed.**
ADR 1860 asks the device once "after the processor has drawn" the pages, and read that as "the
processor's price is measured". On the witness it never is during a gesture: the first frame is
finished from its drawing ahead, which prices nothing (trap 143), and each of notches one to four is
taken back by the next before it lands. So the seeds decided (1.1 s against 0.5 s) until the person
stopped, and the device's first frame came at the sixth notch.

**The hypothesis held in half.** With the scene kept the device is the cheaper lane from the second
notch on, and the prediction now says so (0.32 s on the device's kept seed against the processor's
0.53). Ten notches each under 0.25 s did not hold: the device's first frame builds its scene,
0.75–0.84 s, and the kept frames at the low notches cull little, 0.26–0.36 s, so the frames shown are
those of notches six to ten.

## 2. Decision

1. **A frame the processor drew counts as drawn whether or not it priced anything**
   (`Measured::processor_drew`): a frame finished from its drawing ahead and a frame taken back both
   set it, and the device's one ask waits for it rather than for a price.
2. **The device has two prices** (`device_built`, `device_kept`), read from `FrameCost::scene_kept`,
   and the frame is priced by the one that applies: kept where the device's last frame was of these
   pages (`Costs::holds`), built otherwise. One price for both let a build past `WARN` send every
   later notch to the processor, though each is a re-encode.
3. **Every frame of a priced arrangement says its lane**: `device chosen: …` beside ADR 1831's
   `processor chosen: …`, with both predictions, which seed or measurement each came from, and
   whether the device's figure is for building its scene or drawing the one it holds.
4. **The seeds are the window's, on the owner's display** (habit 88): built 187 ns (825.4 ms, the
   first notch, the quicker of the first hold's two; the second read 797.3 and 750.5), kept 73 ns
   (320.8 ms, the notch after it, the quickest of four), over 4 408 405 commands; the processor's
   120 ns stays, for the reason its doc comment gives. The witness at the seeds is 0.82 s on the
   device, inside `WARN`; the line is now 5 347 593 commands.
5. **`quorra-gtk` and `quorra-qt` turn a page as `quorra` does** (ADR 1871 §2.2): a `GoTo` is
   handled unread, read on the toolkit thread (trap 154) beside a `viewer_host::Ahead` drawing into
   the window the core was last told (`Host::extent`), and `Drawing` finishes the frame from it.

## 3. Measured

Two `--clock` holds (the first's owner's-display turns found no window: the toolkits chose Wayland,
so they ran again under X11). HEAD `56e42ea5` against an export of HEAD plus these hunks,
alternating; seconds unless marked. A notch's figure is its key to the frame presented for it.

| | HEAD | this |
|---|---|---|
| owner's, notches with a frame of their own, of ten (four runs) | 2, 1, 2, 3 | **4, 5, 5, 5** |
| owner's, those notches' frames | 0.185–0.793 | **0.187–0.308**, every one on the device |
| owner's, last notch to its frame | 0.250, 0.272, 0.196 | **0.223, 0.203, 0.187, 0.210** |
| owner's, `quorra-gtk` turn | 2.96, 2.45, 3.27 | **2.16, 2.14, 3.12** |
| owner's, `quorra-qt` turn | 3.44, 3.41, 2.49 | **2.03, 2.09, 2.09** |
| owner's, a toolkit turn's draw after the read, ms | 509–563 | **49–113** |
| Xvfb, `quorra-gtk` turn (two holds) | 3.29, 3.29, 3.62, 3.52, 3.54, 2.94 | **2.54, 3.27, 3.43, 2.19, 3.31, 2.26** |
| Xvfb, `quorra-qt` turn (two holds) | 2.44, 2.48, 3.12, 3.41, 3.38, 2.48 | **2.43, 3.24, 3.23, 3.08, 3.20, 2.17** |

HEAD's second run delivered one notch only. On Xvfb the read beside the drawer is slower, as ADR
1855 measured, here by up to a second (Qt's reads 1.86–2.54 → 2.37–3.17 s in the first hold), and it
hides the saving: the draw after the read is 0.49–0.79 → 0.05–0.18 s. Headless `zoom_frame` at
four threads read the build at 801.9 and 1 373.6 ms and kept steps at 481–667 ms on the `Cpu` lane,
which is why the seeds are the window's.

## 4. What it leaves

- **The device's build is paid inside the gesture.** Built after the first frame while nobody waits —
  the sharp pass's rent-or-buy rule (ADR 1289), with the scene's build as the pass — it would take
  the 0.75–0.84 s out of the first notch; trap 148's rule still holds, so it is a build of its own.
- **The low notches' kept frames are 0.26–0.36 s**, the encode of a frame that culls little: the
  encode on the fan-out's threads (ADR 1878 §3) is the lever for those.
- **`Costs::holds` is the window's record**, not the device's word: an idle device draw of another
  page between two frames misprices one frame as kept.
