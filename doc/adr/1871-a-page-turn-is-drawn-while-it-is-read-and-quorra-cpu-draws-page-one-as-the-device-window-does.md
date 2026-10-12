# 1871 — A page turn is drawn while it is read, and `quorra --cpu` draws page one as the device window does

Session 1517, on the project owner's target of 2026-10-10, "improve performance". Status:
**accepted** and **built** in `quorra`, `quorra --cpu` and the confined worker's turns; the confined
window's launch is priced and not built (§4). Context: ADRs 1811, 1827, 1831, 1833 §5, 1855; trap
154. Code: `Viewer::{handle_unread, read_the_rest}` (`viewer-core`); `App::turn` (`quorra/dispatch.rs`),
`drawing_ahead` (`quorra.rs`); `Composer::{draw_ahead, show_read, shows_as_built,
put_up_as_built}`, `Finished::Partial`, `Job::ahead` (`quorra/composer.rs`);
`software::over_the_ground`; `Turning` and `draw_beside` (`viewer-confined`'s `worker.rs`). Tests:
`viewer-core`'s `a_turn_read_by_the_host_asks_for_what_a_turn_handled_whole_asks_for`, `viewer-ui`'s
`a_picture_part_way_shows_the_ground_where_nothing_is_drawn_yet`, `viewer-confined`'s
`a_turn_drawn_while_it_is_read_is_the_page_drawn_whole` and, behind the filter,
`a_turn_drawn_while_it_is_read_behind_the_filter_is_the_page_drawn_here`. Witness:
`/tmp/demo_heavy_engineering_A1.pdf`, and for a turn the same page behind a light one. Cites no
clause and moves no ledger row.

## 1. The premise and the hypothesis, checked

**The premise held.** ADR 1855 §5 left three paths reading and drawing as before: `--cpu`'s composer
drew through `compose_pages_interruptibly` with no look and no drawing ahead (`ahead` was `None`
under `--cpu`), and `compose_pages_watched` had no caller there; a page turn ran its read inside
`Viewer::handle` on the toolkit thread, 1.86–2.42 s on the witness, and then drew the page whole,
0.55–0.62 s more; the confined worker read and drew inside one command.

**The hypothesis held for `quorra` in its shape, and needed the core split.** The core read a turn's
page inside `handle`, so no host could hand that read a drawer. `handle_unread` performs a command
with the arrangement's unread pages left unread and nothing asked of them; the page is then handed
out by the same `anticipating` that hands out page one (its precondition is already "this page
unread, every other read"), taken in by `anticipated`, and `read_the_rest` settles. The read keeps
the toolkit thread (trap 154) and `viewer_host::Ahead` spawns the drawer, unchanged. **What it cannot
do is show a look during the turn's read**: the thread that would put one up is the reader. What
moves is the time from the read's end to the frame.

**The confined half held for a turn and not for the launch.** The worker's turn is the same split
with the worker as the host; its launch reads page one inside `Open`, before the window has a size,
and the brief's wire message carrying list parts is the larger payload on the witness, whose list is
many times its raster — §4.

## 2. Decision

1. **The core: `Viewer::handle_unread` and `Viewer::read_the_rest`.** Everything `handle` does, in
   its order, except `arrange`'s interpretation; the host must end with `read_the_rest`. The core
   still spawns nothing (rule 4), and the read is `first_read`, which `anticipating` hands out only
   for a page holding no replacement — the same read `interpret` makes of it.
2. **`quorra` turns a page through `App::turn`**: every `Command::GoTo` is handled unread, read on
   the toolkit thread beside a drawer, and the frame is finished from the drawing where the request
   is for that list at that target, on either surface (`renderer::Window::draw_ahead`,
   `Composer::draw_ahead`). Nothing is drawn until the read has run `GLIMPSE_AFTER`, so a quick page
   costs a thread spawn.
3. **`quorra --cpu` takes page one's drawing too** (`drawing_ahead`, one constructor for both
   windows): each look is put on the window while the document opens (`Composer::show_read`), the
   first frame is finished from it, and its cost predicts nothing (trap 143, `settled(…, false)`).
4. **`--cpu`'s composer draws under `compose_pages_watched`'s looks** (ADR 1831's half): each look
   crosses as `Finished::Partial` and goes up over the window's ground (`over_the_ground`) only where
   the window holds no picture of those pages, the device window's rule; "first frame" stays the
   finished page (trap 144).
5. **The confined worker draws a turn beside its read** (`Turning`): it cannot link `viewer-host`,
   which brings `ureq`, `rustls` and `ring` into the confined process, so the arrangement's two types
   are used directly — a scoped drawer over `Parts` into an `Underway`, on the pool's stack, unnamed
   (naming a thread is a `prctl` the filter kills for, which `tests/confined.rs` found), beginning at
   the first part handed over, because the worker reads no clock it was not handed (ADR 1811).

## 3. Measured

One `--clock` hold, HEAD `c6fb18fb` against an export of HEAD plus these hunks, alternating, three
runs an arm under Xvfb and two on the owner's display; seconds; a turn is the key's `GoTo` to the
turned page's first frame.

| | Xvfb, HEAD | Xvfb, this | owner's, HEAD | owner's, this |
|---|---|---|---|---|
| `--cpu` first look | never | **0.536–0.537** | never | **0.537, 0.538** |
| `--cpu` first frame | 2.52, 2.64, 3.17 | **2.27, 2.58, 2.72** | 3.26, 2.57 | **2.17, 2.16** |
| turn, `quorra` | 2.42, 2.42, 2.96 | **2.10, 2.33, 2.35** | 3.17, 3.53 | **2.94, 3.03** |
| turn, `--cpu` | 2.40, 2.52, 2.55 | **2.06, 2.11, 3.11** | 2.63, 2.73 | **2.15, 2.34** |
| read's end to frame, `quorra` | 0.547–0.553 | **0.086–0.091** | | |
| read's end to frame, `--cpu` | 0.527–0.622 | **0.062–0.095** | | |

The read beside the drawer is up to 0.6 s slower in a run (`--cpu` turn's 3.11 is a 3.01 s read), as
ADR 1855 measured; the gain is net of it. With `MALLOC_ARENA_MAX=1`, the confined worker's single
arena, set on `quorra`, the turn still
reads 1.91–2.28 s against HEAD's 1.86–2.23 and lands 2.01–2.37 s against 2.42–2.85. The photographs
are the page; against HEAD's, 164 pixels of 452 800 differ, by at most two levels, where the one-strip
finish differs from a frame drawn in strips (ADRs 0219, 1758).

## 4. The confined window: its turn measured, its launch priced

**The turn.** End to end, from the host, the read's own spread hides the change: 3.39, 3.98, 3.91 s
against HEAD's 4.04, 3.15, 3.14 under Xvfb, 3.58, 3.78 against 3.38, 4.08 on the owner's display.
So the worker's stages were stamped into the frame it sends (a probe build, four runs an arm, the
drawn-ahead frame marked so that its use is seen): **the draw after the read is 504–657 ms in HEAD
and the finish 53–83 ms here**, every turn marked; the read is 2.05–2.89 s against 2.22–2.83, no
slower beside the drawer; the turn 3.42–4.24 s against 2.85–3.75. Behind the filter the page is the
one-strip draw byte for byte, and `awkward_classes` reads 0 deaths over its 225 documents.

**The launch, not built.** The worker reads page one inside `Open`, before any window exists, so a
drawing ahead has no target: a foresight from the size the window asks for misses on the owner's
display (KWin gave 992 and 1 001 rows for a requested 1 000, round 1509's logs), and a `Resize` the
worker could see mid-read needs a reader beside its loop and a writer beside the host's exchange —
ADR 1827 §3's larger change. A look across the wire is a pixel notice, and `frame::MAX_NOTICE` is
256 bytes on the trusted side; raising it is a bound of its own.
