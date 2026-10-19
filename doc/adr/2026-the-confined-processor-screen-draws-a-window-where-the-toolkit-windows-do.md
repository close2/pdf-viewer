# 2026 — The confined processor screen draws a window where the toolkit windows do

Session 1595, slot 4 of batch ninety-four, ADR 2016 §3's first residue under the project owner's
"switch to code improvements … until I say otherwise" of 2026-10-14. Status: **accepted** and
**built** in `quorra-confined` (`screen::drawn_as_windows`, the trace's whole-page line and the
window line's `of W×H`) and `tools/drive-windows.sh` (step `82-zoomed-window-processor`). Amends
ADR 1944 §2 (the processor screen windowed a page only past `MAX_PIXELS`) and ends ADR 2016 §3's
first bullet. Context: ADRs 0725, 1944, 1994, 2016, 2017. Tests: the screen's
`a_page_under_the_bound_the_toolkits_window_is_drawn_as_its_window_and_composes_alike`,
`a_page_under_the_bound_behind_no_window_is_drawn_whole` and
`a_refused_page_under_the_bound_falls_back_to_a_window_where_the_rule_holds`. Cites no clause and
moves no ledger row.

## 1. The premise, and the hypothesis

**The premise held.** `Screen::take` sent a list payload to `windowed` only where `past_the_bound`,
and `Screen::fall_back` the same, so under `--cpu`, or for a frame the device refused, a page whose
marks crossed was drawn whole however little of it the window showed.

**The hypothesis — "the screen's `Windowed` arm reads the same `Turning::draws_a_window_of`
decision the device path reads" — did not hold, and neither did the brief's "the worker computes
the share".** `draws_a_window_of` is the worker's question about a page that crosses as *pixels*;
of a page whose marks cross it records `Carries::Marks` and asks nothing. And it cannot be the one
to ask: whether those marks are drawn on the processor is this side's to know — `--cpu`, and a frame
the device refused, are both decided in the window, and the worker is told `Oversize::Listed`
either way. The marks are on this side, and this binary links `viewer_host::oversize` already, so
the screen asks the toolkits' own `worth_a_window` of them: no frame, no greeting, and no third
statement of the rule. The two statements ADR 2016 §2.2 holds equal by its tests stay the only two.

## 2. Decision

1. **The processor screen draws a list payload as its windows where `drawn_as_windows` holds**:
   past the bound always, and under it where `viewer_host::oversize::worth_a_window` holds at the
   window's extent — the largest centred window part of the page and at most half its area or
   keeping at most seven eighths of its commands (ADR 1994). A window with no extent shows no
   window, so there the page is drawn whole, as the worker draws one behind no viewport.
2. **It is asked once for a drawing**, as the toolkits ask it once for a request: a list and target
   held whole, on the thread, or refused keep that answer, and one held as windows stays windows, so
   a scroll asks the list nothing and no target is drawn both ways. A new list or target is asked.
3. **A frame the device refused falls back by the same question**, page by page.
4. **The question is one stride of the list on the window's thread** (`share_reaching`, at most
   4 096 commands, none where the window is half its page or less), bounded by the bytes that
   crossed, which decoding the frame has paid once already.
5. **The window says each page its processor drew whole in the toolkit windows' words**, `page N
   rasterised W×H in …`, and a window's line ends `, of W×H`, the page it is of, so a drive tells a
   window of a page under the bound from one past it.

## 3. Measured

**Drive.** Step `82-zoomed-window-processor`, `quorra-confined --cpu` on the drive's A1 page under
the round's own Xvfb: this round's export drew a window 1 201 × 1 501 of a page 1 725 × 2 442 at the
fourth notch, its red square 204 pixels a side where 200 points of that page are 204.9. HEAD's
export, the control, drew its first window at the fourteenth notch, past the bound: wrong.

**One `--clock` hold** (282 s, load 8.6 → 6.9), twenty `=` notches 0.3 s apart, HEAD's release
binaries against the export's (worker identical by digest, `9435af31…`), alternated HEAD, this,
this, HEAD; llvmpipe is not used, the screen is the processor's (habit 113).

| | HEAD | this round |
|---|---|---|
| the witness: settled after the first notch | 37.0, 33.5 s | 35.9, 22.9 s |
| the witness: windows of a page under the bound | 0, 0 | 2, 2 (149.6 and 233.7 Mpx pages) |
| the A1 page: settled; windows under the bound | 6.1, 6.0 s; 0, 0 | 6.0, 6.0 s; 10, 10 |
| final photographs against HEAD's first | 0, 0 pixels | 0, 0 pixels |

**The witness reaches the rule**, which the brief did not expect: its 432 MB list crosses as pixels
only while its raster is smaller, so at the two notches before the bound its marks cross and HEAD
drew 600 MB and 935 MB rasters of it on the processor. The settled figures are within the hold's
noise; HEAD's whole draws said nothing, so its longest is not in its trace. **What moves is the
window's resident high-water** (`VmHWM`, a run of its own behind the lock and not a clock figure):
the A1 page at fourteen notches, the last under the bound, 934 980 kB at HEAD and 32 016 kB here;
the witness at twenty, 3 663 588 kB and 2 479 604 kB; both final photographs 0 pixels apart.

## 4. What this does not do

- **A device screen's own pages keep their marks**: the device draws them at the window's size
  (ADR 2017 §4). Only `--cpu` and a refused frame reach this rule.
- **A resize that changes no target keeps the drawing's answer**, as the toolkits keep theirs until
  the viewer asks for the page again.
