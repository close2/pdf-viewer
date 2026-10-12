# 1867 — The toolkit windows draw page one while it is read

Session 1515, on the project owner's target of 2026-10-10, "improve performance". Status:
**accepted** and **built** in `quorra-gtk` and `quorra-qt`. Completes ADR 1855 §5's first item,
whose four calls that ADR named. Context: ADRs 1531, 1539, 1811, 1825, 1855; trap 154. Code:
`viewer_core::Foreseen::page`, `viewer_host::loading::Look::{page, origin, placed}`, both hosts'
`Opening`/`Host::open`, `resized`, `join_the_opening`, `mind_the_opening`, `show_the_read`,
`take_the_opening` and the look's arm in `placements` (GTK) and `frames` (Qt). Cites no clause and
moves no ledger row.

## 1. The premise and the hypothesis, checked

**The premise held**: both windows opened the document on a `Loading` thread whose closure called
`Viewer::anticipate`, joined it at `WARN` and showed a bar until the read ended; ADR 1855 §5 names
the four calls and `Drawing::draw_ahead` was already waiting for them. **The hypothesis held for
three of the four calls and not for the fourth**: `Ahead::page_sized` in the closure, `window_is` on
the first allocation and `draw_ahead` at the join are one line each, but "the look shown during the
bar" is not a call — a toolkit window places page rasters where `Query::PageGeometry` puts them, and
before the join its viewer has no document and so no geometry. The look had to carry where it goes.

## 2. Decision

1. **The opening thread calls `Ahead::anticipate`** in both windows, in place of
   `Viewer::anticipate`; the read keeps that thread and the drawer is the thread spawned (trap 154).
2. **The window's size is told on every allocation while the document opens** (`window_is`, inset
   zero: a toolkit's viewport is the page area alone), so a resize during the read moves the target
   the drawer draws into; a target that then differs from the first request's is drawn whole, as
   ADR 1855 §2.4 already says.
3. **The join waits no longer than `GLIMPSE_AFTER`** where a page can be drawn ahead, as `quorra`
   does: the look is what the window shows from half a second, and the bar still appears only past
   `WARN`, its "has been read" sentence only where it did.
4. **A look carries the page and the origin the core foresaw** (`Foreseen::page`, `Look::origin`),
   and `Look::placed` makes it a `stand_in::Placed` at its own size there — the place the first
   frame then lands, by the core's own arithmetic (ADR 0118). Each window holds the newest look and
   shows it **only while the page has no frame and its frame is still coming** (the opening thread
   running, or the drawing holding a request): a read that failed leaves no part of a page on the
   screen (trap 5). The first frame on the screen drops it.
5. Each look is said on the `frames` topic as `quorra` says it, `page 1 shown as it is read, N
   commands drawn`, and the first on `launch` as `page shown as it is read at`.

## 3. Measured

The witness, one clock hold, HEAD (`c6fb18fb`) against HEAD plus this round's hunks in their own
target, two runs an arm, seconds from process start by each window's trace; the first look is the
first picture of the page (HEAD: the first part-way picture of the draw after the read).

| | first look, HEAD | first look, this | first frame, HEAD | first frame, this |
|---|---|---|---|---|
| Xvfb, `quorra-gtk` | 3.32, 3.46 | **0.565, 0.562** | 3.38, 3.55 | **2.70, 3.20** |
| Xvfb, `quorra-qt` | 3.46, 2.50 | **0.532, 0.533** | 3.60, 2.55 | **2.16, 3.11** |
| owner's display, `quorra-gtk` | 2.53, 2.60 | **0.536, 0.538** | 2.58, 2.63 | **2.19, 2.19** |
| owner's display, `quorra-qt` | 3.49, 3.23 | **0.535, 0.533** | 3.63, 3.29 | **2.22, 3.22** |

**`quorra-qt`'s first build showed no look at all**, though its trace and drive step 81's ink test
both said one: its `frames` asked the window's own viewer, which holds no document before the join
and answers no `Frame` at all, and took that for "a frame held". Photographed under Xvfb a second
into the read, by the window's id, the page was not there; the build above asks for the frames
first and the look after, and the same photograph holds the page as it stands where the first frame
lands, in both windows. So `quorra-qt`'s rows are a hold of their own. The first build's
owner's-display first frames, 2.60, 2.65 → 3.26, 3.15, sit with the rows above inside the read's own
spread, 2.0–3.1 s in both arms across the holds: **the first look moves by two to three seconds, the
first frame by less than the read varies.**

The first frame is finished from the drawing ahead in 50–95 ms, where HEAD draws the page after the
read in 524–572 ms; what it is net of is the read beside the drawer, which ADR 1855 §3 measured. The
first hold's drive steps 80 and 81 in `quorra-gtk`, one run an arm: first shown 2.91 → **0.54 s**,
the look photographed and looked at; first frame 2.70 → 3.15 s, inside the spread above.

## 4. What it does not do

A page turn after the launch reads on the toolkit's thread as before, in both windows. A look is a
processor picture at the first frame's target; nothing here changes which lane draws a toolkit
window's frames, which is the processor's in both. The read beside the drawer is the cost ADR 1855
§3 measured, and on these figures it is inside the read's own spread.
