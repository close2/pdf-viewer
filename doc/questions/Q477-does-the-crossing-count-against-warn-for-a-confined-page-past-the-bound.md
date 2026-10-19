# Q477 — For a confined page past the bound, does the time its marks take to cross count against `WARN`?

Source: ADR 2001 §2, the measurement ADR 1986 §2.4 left: should the witness, a page past the bound
whose marks cross, also cross as the worker's window?
Status: **open** — answered when `A477-…` exists beside this file.

## Why it needs the owner

`CLAUDE.md` principle 2 says: "The device draws every frame it is predicted to draw inside the
warning bound, and past it the lane with the smaller prediction draws" (ADR 1831, `doc/questions/
A393`). In `quorra-confined` the witness's marks reach the device in two steps. The worker chooses
and encodes the list's selection for the view, and the host decodes it. Then the device draws it.
Measured in two clock holds on the witness, twenty `=` notches then twenty or forty `Down` presses (ADR 2001
§2):

- **The device's own frames were all inside `WARN`**: 94 ms and 112 ms at the longest, asked to
  finished. Read without the crossing, the rule keeps every one of them on the device.
- **What a person waits for is longer**. The twenty notches settled in 46.4–48.4 s over four runs.
  A scroll's next frame came 166–180 ms after the press (median). Twenty presses 0.1 s apart stalled
  up to 3.0–3.2 s.
- **The worker's window was quicker on every count but one run.** Drawn on the processor in the
  worker and sent as pixels, three of four runs settled the notches in 40.7–41.2 s (one in 58.5 s).
  A scroll's next frame came in 9–10 ms, and a new window at a margin's exit came 44–102 ms after its
  press. The fast presses stalled at most 123–150 ms. The final photographs were identical, 0 pixels
  differing.

So the answer depends on what "predicted to draw" means for a frame whose marks must first cross a
process boundary. If the crossing is not part of the device lane's prediction, the witness stays on
the list and on the device. If it is, the notches are past `WARN` and the worker's window is the
smaller prediction. Then the next round builds a priced choice between the two routes in the
confined window.

## Recommendation

**Count the crossing.** The person waits for it exactly as for a device frame, and the rule exists
to give a waiting person the faster lane. The worker's window is the processor, which is the
oracle, so nothing drawn becomes less correct. What would be built: the confined host prices both
routes per page from what it has measured, the list's crossing per byte and the device's frame per
command, against the window's draw per pixel. It asks for the window only where that is the smaller
prediction past `WARN`, and says the lane on the trace as `quorra` does (ADR 1886).

## What the tree does meanwhile

A page past the bound whose marks cross keeps crossing as its list, and the device draws it (ADR
1986 §2.4). Nothing was built.
