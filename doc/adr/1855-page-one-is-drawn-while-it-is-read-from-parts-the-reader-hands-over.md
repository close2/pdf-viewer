# 1855 — Page one is drawn while it is read, from parts the reader hands over, and finished from them

Session 1509, on the project owner's target of 2026-10-10, "improve performance", and the build ADR
1845 §5 chose. Status: **accepted** and **built** in `quorra`; the toolkit windows hold the same
arrangement in `viewer-host` and do not call it yet (§5). Context: ADRs 0139, 0657, 1531, 1758, 1811,
1825, 1831, 1841, 1845; `doc/todo/69` §4; `doc/questions/A393`. Code: `pdf_model::progress::Parts`,
`Interpreter::publish_progress` (`content.rs`), `pdf_render::{ListPart, Stated}` and
`DisplayList::taken_back_since_asked`, `render_cpu::Underway`, `viewer_core::{Anticipating, Reader,
Foreseen}`, `viewer_host::Ahead` and `Drawing::draw_ahead`, `quorra`'s `Window::{draw_ahead,
show_read}` and `App::show_the_read`. Tests: `pdf-model/tests/drawn_while_read.rs`,
`viewer-host`'s `a_page_read_inside_the_wait_…`. Measurement: `viewer-host`'s
`examples/drawn_while_read`. Witness: `/tmp/demo_heavy_engineering_A1.pdf`. Cites no clause and
moves no ledger row.

## 1. The premise and the hypothesis, checked

**The premise held**: `interpret_with_fonts` builds the list whole, the window's first look waited
for the read's end, and the read is three quarters of the first frame (ADR 1845 §1).

**The hypothesis held in its shape and not in its letter.** A cut between two top-level commands is
not enough to make a prefix final: the interpreter *takes commands back* after it has pushed them —
a text object's glyphs gathered into §9.3.6's clip or §9.3.8's knockout group at `ET`, a run of
marks sealed under §11.6.4.3's other `/AIS` reading (ADR 1319), a group's elements collected at the
end of its `Do`. So a part is handed over only where nothing the interpreter has open can gather
what it pushed — between two of the page's own operators, outside a text object, on the run that
draws on the device's components — and every `split_off_commands` below what was handed over is
recorded on the list (`taken_back_since_asked`) and **spoils** the hand-over: the drawer is told
and the page is drawn whole, as before. A take-back is not always a change, so this errs one way.
And **the planner is not asked per part**: the drawer paints one surface, because a page divided
into strips is the same picture only very nearly (ADR 1758, 0.36 % of a page's pixels at most) —
so the page drawn while it is read is byte for byte the page drawn as one strip, which is
`raster_golden`'s own definition, and differs from a frame divided into strips exactly where that
division does.

## 2. Decision

1. **The reader hands its list over in parts** (`Parts`), installed for one interpretation
   (`Parts::during`) and claimed by the first page run inside it, so a page that interprets another
   inside itself hands over its own list. A part is at least `FIRST_PART` (4 096) commands and grows
   to a sixteenth of what was handed: about sixty parts on the witness, nothing at all for a page
   under four thousand commands. A part is a copy of its commands' headers, its new clips and soft
   masks; geometry is shared. The finished run says how much the list states, or why the parts are
   not its prefix.
2. **The drawer paints each part into one target** (`Underway`): the parts' clips kept in a list of
   tables, each part's collapsed marks restated as the finished list restates them, the groups'
   blit added up against `MAX_GROUP_BLIT_PIXELS`, and each part drawn through the loop a group's
   elements go through. A soft mask stops it — a mask is built against its consumers' reach over the
   whole list — and so does a page this backend does not glimpse (ADR 1825's list). `finish` draws
   the rest from the finished list and runs the page's per-pixel passes.
3. **Page one is read on the opening thread, as before, and drawn on a thread of its own**
   (`Ahead`): the core hands out the read (`Viewer::anticipating`, `Reader::read`) and the target
   the first frame will be asked at (`Anticipating::foresee`, by the core's own arithmetic); the
   core spawns nothing (its rule 4), and its once-filled fields became `OnceLock`s so that its state
   may be consulted from the drawing thread. **The read keeps its thread because the other order
   was measured and lost** (§4): the read is the critical path, and a new thread may land on one of
   this machine's eight slower cores, which the drawer can afford and the read cannot. **Nothing is
   drawn ahead until the read has run `GLIMPSE_AFTER`** (500 ms, ADR 1825's wait), so no page a
   launch gate measures pays a drawing thread beside its read; the drawer is woken by the read's
   end, so a quick page is taken in the moment it is read.
4. **The first frame is finished from it**, where the request is for that list (by allocation, not
   by commands) at that target; any other request is drawn as before. **The lane is the processor's
   for that frame**: all but the last part is drawn there already, so the processor is the faster
   lane by any prediction — A393's condition — and the frame measures nothing about either lane's
   price (ADR 0657's rule; trap 143 for the cost the next view change is predicted from).
5. **`quorra` shows each look while the document opens**: it waits for the opening thread no longer
   than `GLIMPSE_AFTER` where a page can be drawn ahead, uploads each look on its render thread and
   puts it up until the first frame; the trace says `page 1 shown as it is read`. The title's bar
   is unchanged (ADR 1811).

## 3. Measured

**The corpus**: the first page of all 974 tracked documents drawn while read, in parts of 8
commands so that nearly every page is cut (`drawn_while_read`'s sweep): every one that finished is
the one-strip draw byte for byte — 910, 97 of them handed over in at least one part (a part is cut
only at a check outside a text object, every 64 operators); drawn whole instead, each by name, 27
for a soft mask, 19 composited in a space of their own (never claimed), 1 with a plane of its own.
`raster_golden` holds all 974 (moved 0). Photographed under Xvfb: the look is the page in painting
order, and the first frame is HEAD's picture.

**The witness**, `quorra`, HEAD (`63d2aaa6…`) against this build, alternating in one clock hold
each, seconds from process start, three runs an arm (load 6–8):

| | the read ends | first look | first frame |
|---|---|---|---|
| owner's display, HEAD | 2.61, 2.88, 3.21 | 3.17, 3.40, 3.75 | 3.29, 3.62, 4.01 |
| owner's display, this | 2.81, 3.46, 3.55 | **0.54, 0.54, 0.54** | **2.91, 3.58, 3.67** |
| Xvfb, HEAD | 2.74, 3.05, 3.99 | 3.30, 3.93, 4.59 | 3.43, 4.56, 4.92 |
| Xvfb, this | 2.81, 6.20, 3.47 | **0.57, 0.62, 0.57** | **2.91, 6.35, 3.58** |

Headless through the same launch path (`examples/drawn_while_read`, 4 rayon threads, 1416 × 1000):
first seen 3.38–3.74 → **0.534 s** every run, first frame 2.99–4.28 → 2.49–3.15 s. The first frame
is finished in 77–112 ms from the drawing ahead, where HEAD draws the page whole after the read in
0.62–0.80 s; the read itself is slower beside the drawer by up to 0.6 s on the owner's display (the
second Xvfb run's 6.2 s read was the sitting's, its look still at 0.62 s), which is what the gain on
the first frame is net of.

## 4. The order measured and lost

The first build read the page on a new thread and drew on the opening thread. On the owner's display
its read took 3.05–3.37 s against HEAD's 2.14–2.24 s in the same hold, three runs each, and its first
frame landed **later** than HEAD's (3.14–3.48 s against 2.73–2.88 s) — the contract's one
prohibition. The read is the critical path, the opening thread is where a launch has already placed
it, and a new thread may land on one of the machine's eight slower cores, which are worth 45–50 % of
the read (ADR 1845 §1); the drawer keeps up from either kind. So the read stays and the drawer moves.

## 5. What it does not do

- **The toolkit windows do not call it.** `viewer_host::Ahead` and `Drawing::draw_ahead` are theirs
  to call: `Ahead::page_sized` handed into the opening closure in place of `viewer.anticipate()`,
  `window_is` on the first allocation, `look` shown while the bar is up, `draw_ahead` at the join —
  four calls in each host, whose files were another slot's this batch.
- `quorra --cpu`'s composer, `quorra-confined`'s worker and a page turn after the launch read and
  draw as before.
- A page whose operators sit mostly inside text objects hands over little: a part is cut only
  outside one, and such pages read in milliseconds.
