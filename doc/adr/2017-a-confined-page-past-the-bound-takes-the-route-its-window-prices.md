# 2017 — A confined page past the bound takes the route its window prices, the crossing counted

Session 1590, slot 6 of batch ninety-three, the build `doc/questions/A477` owes: the project owner's
"I agree with both recommendations, please answer them" of 2026-10-19, the recommendation being
Q477's "Count the crossing." and a priced choice of route in the confined window. Status:
**accepted** and **built** in `viewer-confined` (`Route`, `Confined::route`, `FRAME_ROUTE`,
`Marks::{route, take_routed}`, `Carries::Window`, `Framed::bytes`) and `quorra-confined`
(`pricing::Pricing`, `Host::route`). Amends ADR 2001 §2, whose meanwhile it ends. Context: ADRs
1831, 1886, 1944, 1986, 2001, 2016. Tests: the protocol's
`a_route_crosses_as_its_page_and_its_route_and_nothing_else_does` and
`a_page_asked_for_as_windows_is_held_as_windows_at_its_next_render`, `tests/confined.rs`'s
`a_page_past_the_bound_whose_marks_cross_takes_the_route_its_host_asks_for`, `pricing`'s
`a_page_whose_crossing_is_past_the_warning_is_asked_for_as_its_window` and
`only_a_page_that_crossed_alone_and_new_measures_its_route`. Cites no clause and moves no ledger
row.

## 1. What the answer owes

A477's reading: a frame's prediction in the confined window includes the time its marks take to
cross, and the build is "a priced per-page choice, not a blanket switch". The witness's device frames
were all inside `WARN` while its marks' crossing put a person's wait past it (ADR 2001 §2), so a rule
that priced the device alone kept it on the slower route.

## 2. Decision

1. **The window prices both routes of each page past the bound whose marks cross**, on a device
   screen, from three rates it last measured: the crossing per byte, from the command sent to its
   frame decoded over the bytes that crossed for the page (`Framed::bytes`); the device per command,
   from each device frame over the commands of the marks it drew; and the worker's window per pixel,
   from the command sent to the frame that brought a new window, over its pixels. The device's route
   is predicted as bytes × crossing + commands × device; the window's as the largest centred window's
   pixels × its rate. A frame carrying two pages past the bound measures nothing.
2. **The window is asked for only where the device's prediction is past `WARN` and the window's is
   the smaller**; every other page keeps its marks, so `CLAUDE.md`'s rule that the device draws every
   frame predicted inside the bound stands as written. Until the device has drawn and the crossing
   has been measured nothing is priced and the page keeps its marks. Until a window is measured its
   rate is a seed, 171 ns a pixel, the slower end of ADR 1944's window on the processor, because a
   seed too quick takes a page off the device before anything is measured.
3. **The host asks, and the worker records the asking and draws nothing.** `FRAME_ROUTE` is a frame
   of its own, a page index and `Marks` or `Window`, because which form crosses is between the host
   and its worker and not `viewer-core`'s vocabulary. It answers at once whatever the page costs,
   so it is sent on the window's thread after a frame lands. The next command moves the page's held
   marks to the windows (`take_routed`) and draws its window with the others it owes; the page's
   next render is held as windows with nothing encoded (`Carries::Window`), which on the witness is
   the 0.6 s a notch ADR 1986 §1 measured. `Marks` takes effect from the page's next render. A page
   that leaves the arrangement is forgotten with the asking, so one that comes back is priced again.
4. **Every priced frame says its route** as `page N window chosen: …` or `page N device chosen: …`,
   with both predictions and whether the window's is measured or the seed — `quorra`'s two lines
   (ADR 1886), with the worker's window as the processor.

## 3. Measured

One `--clock` hold under an Xvfb of the round's own (load 1.2–4.5), `quorra-confined` on the witness,
twenty `=` notches 0.3 s apart and then twenty Down presses 0.5 s apart, figures from the window's
own trace clock. HEAD's release binaries (the batch's warm-up build, greeting `PDFVCF15`,
`quorra-confined` 045bf600…) against the round's export (HEAD and this round's paths, f098bf92…),
alternated HEAD, this, this, HEAD. **The device is llvmpipe**, Vulkan on the processor, which is what
Xvfb offers; the device's figures are not the owner's adapter's (habit 113). Both of this round's
decisions reach the witness: under the bound its 432 MB list is larger than its raster, so it
crosses as pixels and ADR 2016 draws its window; past it this ADR's pricing does.

| | HEAD | this round |
|---|---|---|
| twenty notches settled | 66.3, 47.8 s | **17.8, 18.2 s** |
| a press's next frame, median; longest | 160, 159 ms; 172, 168 ms | **4, 6 ms; 75, 71 ms** |
| the device's longest frame | 521.3, 85.3 ms | 47.3, 68.6 ms |
| frames that carried marks | 28, 28 | 3, 3 |
| windows the worker sent, under / past the bound | 0 / 0 | 9 / 25, 9 / 25 |
| the device's route as priced; the window's, measured | — | 1 135, 1 274 ms; 50–207 ms |
| final photographs, notched and pressed, against HEAD's first | 0 pixels | 0, 0 pixels |

The first notch past the bound crossed as marks and priced the device's route at 1.1–1.3 s with the
crossing, past `WARN`; the window was asked for, and the first one measured replaced the seed. The
device's route was then not measured again, because nothing of the page crossed as marks: so a page
asked for as windows keeps them while it stays past the bound, which §4 states.

## 4. What this does not do

- It prices pages past the bound only, which is Q477's population. A page under the bound whose
  marks cross is drawn by the device at the window's size and keeps its marks.
- The rates are the last measured, not a mean, as `quorra`'s are; a notch's crossing carries the
  worker's encoding and a scroll's does not, so the device's prediction moves between the two. A
  page asked for as windows sends no marks, so its device route is priced from the last ones that
  crossed until it comes back under the bound or leaves the screen.
- The figures are llvmpipe's. On the owner's adapter the device's frame is quicker; the crossing is
  the processor's and the pipe's, and this hold did not measure it there.
- The processor screen (`--cpu`) has no device route and prices nothing.
