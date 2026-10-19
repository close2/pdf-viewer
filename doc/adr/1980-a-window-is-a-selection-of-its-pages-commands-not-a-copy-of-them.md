# 1980 — A window is a selection of its page's commands, not a copy of them

Session 1572, under the project owner's "switch to code improvements … until I say otherwise" of
2026-10-14: ADR 1941 §4's next cut. Status: **accepted** and **built** in `pdf-render`
(`selection.rs`, `reach.rs`, `group_cost.rs`, `strips.rs`), `render-cpu` (the readers) and
`render-raster` (the device's encoder's reader). Context: ADRs 1841, 1930, 1940, 1941. Code:
`pdf_render::{Selection, Window}`, `DisplayList::reaching`, `check_selected_blit`,
`strip_inputs`. Tests: `selection::tests`, `strips::tests::a_window_read_through_its_selection_states_what_its_copy_states`,
`render-cpu`'s `culled_window::a_window_drawn_through_its_selection_is_the_window_drawn_from_a_copy_of_its_commands`.
Instrument: `viewer-host`'s `examples/window_against_whole`. Cites no clause and moves no ledger row.

## 1. What was found

**The premise held in its mechanism and not in its address.** `grep -rn "&DisplayList"
crates/render-cpu/src raster/crates/raster-gpu/src | wc -l` is 31, every one of them in `render-cpu`:
raster's crates read a `raster_scene`, never a display list, and the device's encoder that reads one
is `render-raster`'s (`scene::Encoder::commands`, called from `present.rs` for a window's frame and
from `lib.rs` for the offscreen lane). Of the 31, the ones that read the page's *top-level* commands
are six: the strip loop (`glimpse::encode_page`), the planner (`strip_inputs` under `plan_strips`),
the soft masks' reach (`mask_consumer_reach`), the reduced images (`ReducedImages::warm`), the group
budget (`check_group_blit`) and the strip driver that hands them out; the rest read the clip and
mask tables, or a group's or a mask's own commands, which a window keeps whole. The copy also cloned
the clip table, its index and the soft masks for every window (`DisplayList::with_commands`), which
ADR 1941 did not price.

## 2. Decision

1. **`DisplayList::reaching` answers a `Window`**: the kept commands' indices into the page's list,
   32 bits each (a list past `u32::MAX` commands is drawn whole). A reader draws it through
   `Window::selection`, a `Selection` — the list and the commands it names, a whole list's being a
   run of it — and `Selection::drawn` is the one spelling of "the window's, or every command".
   `Window::copied` is the copy, for an instrument or a test that wants a list.
2. **Each command keeps both of its numbers.** Its index into the list is what the list's
   `MarkBounds` are asked by, so the bounds a window was once given a copy of (`MarkBounds::kept`,
   removed) are the page's own; its position in the selection is what the copy's index was, which
   the planner's division into runs and a glimpse's progress read. So every number a reader reads is
   the copy's, and the pixels are.
3. **The planner borrows a run of the list's bounds and states a window's.** A window's kept
   commands are not one slice of the list's bounds, so each of its runs states its extents in full
   (`Extents::Stated`), which ADR 1940 already holds equal to the borrowed form.
4. **The page's own list is what the encoder and every pass after the draw read** — the clips, the
   soft masks, the page boundary, the blending curve — which the copy carried unchanged; a list with
   a plane drawn beside it is still never culled (ADR 1841).
5. **The pooled drop stays** for a long list (ADR 1941 §2.3); a window has nothing to drop.

## 3. Measured

Two exports of HEAD `e44e0850`, one with this change; the witness at 1200 × 900, the largest window
centred, three rounds a magnification, each build's order swapped between two passes a pool; the
second of two holds of the example (load 3.8–9.2; the first, at 6.7–12.5, read the same digests and
noisier clocks). Milliseconds from the ask to the landing, whole page / window.

| × fit | window | HEAD, 4 | this, 4 | HEAD, 24 | this, 24 |
|---|---|---|---|---|---|
| 1.7 | 83 % | 511–610 / 613–787 | 461–508 / 495–517 | 346–382 / 462–483 | 318–350 / 333–381 |
| 1.85 | 70 % | 541–590 / 607–637 | 495–529 / 484–515 | 351–396 / 414–468 | 313–386 / 327–349 |
| 1.953 | 63 % | 557–598 / 563–625 | 514–565 / 461–494 | 355–420 / 396–448 | 328–362 / 315–332 |
| 2.2 | 49 % | 595–771 / 460–508 | 546–654 / 387–398 | 376–446 / 324–366 | 351–392 / 254–268 |
| 3.052 | 26 % | 763–868 / 291–297 | 686–717 / 243–289 | 426–525 / 228–249 | 405–439 / 159–181 |

**A window of 83 % is 118–270 ms quicker than at HEAD on four threads and 100–130 on twenty-four**,
more than the copy and the drop ADR 1941 priced (61–70 ms), the clip table's clone and the cull's
pairs of an index and a reference being candidates for the rest; a window of half the page is 62–121
ms quicker on four threads and 56–112 on twenty-four. The page drawn whole reads 20–150 ms quicker on
four threads too, which this change reaches only where some of the page's commands miss its own
target; that is not examined here. Every drawing, whole and window, has HEAD's digest at each of the
eight magnifications on both pools, in both holds.

## 4. The crossing, and the half

**The crossing moved**: a window now ties its page at 70–83 % of it on both pools, where it tied at
63 % and lost at 70 on four threads at HEAD (ADR 1941 §3); one of 63 % is 20–100 ms ahead on four
threads and up to 47 on twenty-four. In commands kept, the window of 83 % keeps 98 % of them
(4 333 950 of 4 408 405, ADR 1941 §1) and ties: a window now costs what it keeps.

**The half is left where it is, and that is not a finding that it is right.** A bound in commands
kept would draw nearly every culled window as a window; that is `viewer_host::oversize`'s decision,
it has to be taken before the draw from bounds the drawing thread measures, and it changes which
pixels a toolkit shows at those magnifications — the window's edge seam of a level or two that
`window_against_whole` counts (83 to 1 007 pixels at the magnifications above). It is a change with
an ADR of its own, owed to a round that owns `viewer-host/src`.
