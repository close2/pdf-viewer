# 2016 — The confined window draws a window where the toolkit windows do

Session 1590, slot 6 of batch ninety-three, ADR 1994 §3's residue under the project owner's
"switch to code improvements … until I say otherwise" of 2026-10-14. Status: **accepted** and
**built** in `viewer-confined` (`protocol::windowed::worth_a_window`, `Turning::draws_a_window_of`,
`decode_window_payload`, greeting `PDFVCF16`) and `viewer-host` (`oversize::worth_a_window`).
Amends ADR 1986 §2.3. Context: ADRs 1871, 1930, 1944, 1986, 1994. Tests: `windowed`'s
`the_window_this_process_draws_is_the_one_the_toolkit_windows_draw` and
`this_process_draws_as_a_window_the_pages_the_toolkit_windows_do`, the protocol's
`a_window_no_worker_could_have_drawn_is_refused`, `tests/confined.rs`'s
`a_page_under_the_bound_whose_window_is_a_tenth_of_it_crosses_as_its_window`. Drive: step
`82-zoomed-window`, now judged in `quorra-confined`. Cites no clause and moves no ledger row.

## 1. The premise, and the hypothesis

**The premise held.** ADR 1994's rule lived in `viewer_host::oversize::oversized` and nowhere on the
confined side: the worker held a window only for a target past `MAX_PIXELS`, and
`decode_window_payload` refused any window under it as "nothing an honest worker sends".

**The hypothesis — "the host's answer crosses as one field of `Oversize`" — did not hold.** The rule
asks what share of a page's commands its window keeps, and for a page that crosses as pixels the
host holds none of its list: the worker holds it. So only the worker can answer, and
`viewer_core::Oversize` is not where an answer would go. The page that needs the rule is one that
crosses as pixels — a list the format refuses, or one larger than its raster, as the drive's own
`drive-band.pdf` of 2 400 000 fills is at every notch. A page whose marks cross is drawn by the
host's device at the window's size already, and is ADR 2017's.

## 2. Decision

1. **The worker asks the toolkits' question of a page that crosses as pixels**, for a host that said
   `Oversize::Listed`, behind a viewport: the largest window, centred, is part of the page and is at
   most half its area or keeps at most seven eighths of its commands. Where it holds, the page is
   held as windows (`Marks::hold_window`) and answered `Rendered::Listed`, as a page past the bound
   is; elsewhere it is drawn whole as before. The question is asked once per request, at the
   viewport it was asked under, as the toolkits ask it.
2. **The rule is written twice, and a test holds the two equal.** The worker links no host crate
   (ADR 1871), so `windowed` states the largest window and the bound again beside the shape ADR 1986
   already stated twice. `viewer-confined` takes `viewer-host` as a dev-dependency so that its tests
   compare the two statements: every corner, margin and largest window over four pages and four
   viewports, and the rule's answer over 361 magnifications of a row of squares, among them windows
   kept for their commands and windows past seven eighths drawn whole. The worker's binary still
   links no host crate. The cost is that `viewer-confined`'s tests build `viewer-host`'s graph.
3. **The decoder admits a window of any page for a host that said it places one**, and still only
   inside its page and holding a pixel. The share of commands is not asked again on this side: it is
   a question of a list this side does not hold, and what a window can cost this side is its pixels,
   which are checked against the bytes sent as every raster's are. The greeting moves to `PDFVCF16`,
   because an older host would refuse a window under the bound.
4. **The confined window says each window its worker sent** as `page N crossed as the window shows
   it, W×H at X,Y of W×H`, and step 82's third part judges it as it judges the toolkits.

**Driven.** Step `82-zoomed-window` in `quorra-confined` from the round's export: the window crossed
at the third notch, 1 201 × 1 501 at 154,226 of 1 510 × 1 954, its red block 147 pixels a side where
60 points of that page are 148. HEAD's binaries, the control, say no window after eight notches. On
the witness the change reaches every notch under the bound, which ADR 2017 §3 measures with it.

## 3. What this does not do

- **The confined processor screen** (`--cpu`, or a frame the device refused) still windows a page
  whose marks cross only past the bound. It holds the list there and could ask the toolkits'
  question itself; nothing measured that screen, so it is left as it is.
- **A page turn draws its page whole beside the read** whatever the rule then says, as the toolkit
  windows' `Ahead` does: the rule needs the list, which the read is still producing.
