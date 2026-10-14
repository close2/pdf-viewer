# 1936 — The processor window's frames wake it too

Session 1550, slot 5 of batch eighty-seven. Status: **accepted, built**. Context: ADR 1926 (the
device lane's wake and the cadence's two instants), ADR 0461 (the `--cpu` composing thread), ADR
1831 (its looks). It cites no clause and moves no ledger row.

## 1. The question

ADR 1926 §5 left one thing open. The `--cpu` window's composing thread sends a finished frame and a
look down its channel and wakes nothing. So the event thread found them only at the next tick of the
clock it keeps while a frame is out, up to one period after they landed.

## 2. What is built

- **The composing thread wakes the loop after `Finished::Frame` and after `Finished::Partial`**
  (`composer.rs`). These are the same two kinds of news the render thread wakes for. A retained page
  wakes nothing, as on the other surface: it is a picture for a view nobody has asked for yet.
- **The thread is handed the wake as a closure** over the event loop's proxy, which the window gives
  `Composer::new`. That lets a unit test run the thread's loop with a counting wake.
  `a_finished_frame_wakes_the_loop_after_it_is_sent` holds the order: the frame is in the channel
  before the wake comes. With the frame's wake taken out, it fails at its 60 s bound.
- **The cadence rule needed nothing new.** The window's one `Cadence` serves both surfaces, so
  `redraw_requested` already asks `may_present`, and a look put up as built already counts as a
  present (ADR 1926 §3). The brief's "the same cadence rule" was already in force for `--cpu`.

## 3. What it measured

One `--clock` hold under Xvfb at the 60 Hz floor, with `--cpu`, at load 0.5–3.3. HEAD's export was run
against this round's export, alternating, three runs an arm. Each run pressed four `Next` keys 700 ms
apart, and each key was timed to the next frame line that said `presented`. AN001's fourth key is on
a page its column already shows, and its match is Escape's frame about 740 ms later, so it is left
out, as in ADR 1926 §4.

| document | HEAD, key to frame | this ADR | first present, HEAD → this ADR (medians) |
|---|---|---|---|
| AN001 | 30–49 ms, median 37 | 27–40 ms, median 30 | 46.4 → 40.0 ms |
| WTPDF | 16–25 ms, median 21 | 13–23 ms, median 17 | 46.6 → 38.0 ms |
| ISO 32000-2 | 23–46 ms, median 38.5 | 18–51 ms, median 36.5 | 65.7 → 60.8 ms |

The gain is at most one period, and that is what the mechanism can give. A turn on this surface is
the composed frame plus the copy to the window: WTPDF's page 2 composed in 6.5 ms and presented in
8.6 ms, 18 ms after its key. What the wake removes is the wait for the tick after the frame landed.
The first present moved as well: every run was 3–11 ms earlier than the HEAD run beside it, because
page one's frame is also a composed frame that used to wait for the tick.

**On the owner's display**, presenting at the 120 Hz its trace reports, the same script ran through
their loop inside a second clock hold, at load 0.2–0.6, with the window at that display's size:

| document | HEAD, key to frame | this ADR |
|---|---|---|
| AN001 | 43–51 ms, median 48 | 40–53 ms, median 44 |
| WTPDF | 23–38 ms, median 30.5 | 22–30 ms, median 24 |
| ISO 32000-2 | 37–54 ms, median 46 | 34–59 ms, median 50.5 |

AN001 and WTPDF moved by about one 8.3 ms period, as under Xvfb. ISO 32000-2's frames take longer
than five periods there, and its two ranges overlap almost entirely, so it is not judged.

## 4. What this does not reach

- **`quorra-confined` is untouched.** Its release binary has the same digest at HEAD and here.
- **`launch_path` and `turn_path` cannot see the change**, for ADR 1926 §5's reason, and their
  bands stand.
- **A count of visible windows does not say whether the owner's session is open.** On the open
  session, `xdotool search --onlyvisible --name .` counted 0 before every script. A quorra window
  the loop script opened under X11 counted 1. On the locked session earlier the same day, both
  counted 0. So the loop script decides "locked" only when its own window is not visible either.
