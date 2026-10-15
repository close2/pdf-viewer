# 1944 — The confined window draws a page past the bound from the marks its worker sends

Session 1554, slot 3 of batch eighty-eight, ADR 1921 §3's build under the project owner's "switch
to code improvements … until I say otherwise" of 2026-10-14. Status: **accepted** and **built** in
`viewer-confined`, `viewer-host` and `quorra-confined`. Amends ADR 1892 §2.1 (`Command::Oversize`
was `Uncarried` to the confined worker) and ADR 1921 §2.1 (the confined window kept the bound).
Context: ADRs 0607, 0640, 0725, 1827, 1892, 1921, 1930. Code: the wire's `OVERSIZE` command kind;
`Marks::decide`'s budget and `HeldLists::told`/`decode_list_payload`; the worker's `perform` and
`Turning::turn`; `viewer_host::oversize::{shown_at, with_margin, Window::holds}`; the confined
screen's `Content::Windowed`, `windowed`, `on_the_device`, `landed_window`; the host's `OVERSIZE`.
Tests: `viewer-confined`'s `a_page_past_the_bound_crosses_as_marks_to_a_host_that_draws_them`, the
protocol's command round trip and target refusal; the screen's two `past_the_bound` tests. Drive:
step 82 judges the confined window as the toolkit windows. Cites no clause and moves no ledger row.

## 1. What the brief supposed, and what held

The brief's hypothesis was that the witness's list for a page past the bound fits the exchange, or
else that the worker should draw the window and send its pixels — "measure which, and build the
cheaper". **The first held, and the second was not needed.** ADR 0607's choice already sends a
page's marks where they are smaller than its raster, and past the bound a raster is more than a
gibibyte; the witness's list crossed at every notch past the bound, and it already crossed at the
notches below it where its raster outgrew its list. The device side was already the shape ADR 1921
§3 asked for: the confined window's device draws every marks page into window-sized textures, so a
page past the bound costs it a window. What was missing was three refusals on the way: the worker's
viewer refused the page (the command did not cross), the host's decoder refused a list target past
`MAX_PIXELS` ("more pixels than a render request is held to, so no worker asked for it"), and the
processor screen would have drawn any such list whole.

## 2. Decision

1. **`Command::Oversize` crosses** (command kind 42, one byte for its value). The confined window
   sends `Oversize::Listed` beside the reader's policy at the open and at every restart.
2. **What a page may claim in the worker is a page's pixels at the bound, whichever form it crosses
   in.** `Marks::decide` compares the list against the smaller of the raster and `MAX_PIXELS × 4`
   bytes, the share `message_budget` already sets aside, so the ceiling's arithmetic is unchanged.
   A page past the bound whose marks do not fit is refused in the viewer's own words — "this page
   cannot be drawn at this size: target of … pixels exceeds the limit of 268435456" — once an
   answer, and a page turn past the bound is read without a drawer beside it. The worker draws no
   raster past the bound on any path.
3. **The host admits a list target past the bound only where it said it draws one** (`HeldLists::
   told`): its pixels are no allocation of the host's, so only its extent is checked. A raster
   payload, and a list target for a host that never said so, are refused as before.
4. **The processor draws the window.** On a processor screen (`--cpu`), and on a device screen for
   a frame the device refused, a page past the bound is `Content::Windowed`: the part the window
   shows, with `viewer_host::oversize`'s quarter-viewport margin, drawn through
   `Drawing::ask_window` and placed at its corner of the page; a scroll inside the margin repaints
   from it, one past it asks for the next window while the last stands in. A page under the bound
   is drawn whole as before — ADR 1930's half is not extended to this window, because a window's
   pixels differ from the page's at the tenth notch by up to 56 levels and slot 2 of this batch is
   reading why (ADR 1930 §4).

## 3. What it measured

On Xvfb (800 × 1000 window), eighteen `=` notches into the witness, exploratory: HEAD said four
refusals and showed the surround; this round said none and drew the page, on the device and under
`--cpu`, where the window of 1 201 × 1 501 pixels landed in 139–309 ms a notch. The host process's
high-water was 4.70 GB (device) and 4.38 GB (`--cpu`) against HEAD's 6.53 GB: what crosses past the
bound is the list, which the host already held below it. In one clock hold, HEAD's build against
this round's, interleaved: ten notches settled in 24.2 s at HEAD and 24.5–24.6 s here, the same
sizes drawn the same way (the ten never reach the bound, so this is the proof that nothing below it
moved); eighteen notches settled in 58.9–68.2 s at HEAD with four refusals and 63.7–67.5 s here
with none, the host's high-water 6.08–6.43 GB against 4.71–5.86 GB.

## 4. What this does not do

- A page past the bound whose list is more than a gibibyte is still refused; drawing it would need
  the list to cross in parts (ADR 1855's shape on this wire) or the worker to draw the window.
- The processor screen does not stand the last window in, scaled, for the next magnification as the
  toolkit windows do (ADR 1930 §2); the composition on the screen stays until the new window lands.
