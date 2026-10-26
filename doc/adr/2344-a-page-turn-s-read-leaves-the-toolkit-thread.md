# 2344 — A page turn's read leaves the toolkit thread

Status: implemented. Session 1755, 2026-10-26, the host slot of batch one hundred and fifteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Cites no
clause and moves no ledger row. Context: ADRs 1855, 1871, 1886, 2328; traps 154, 259.

## 1. What was wrong

`NativeWindow::turn` read the page a turn brings on, on the toolkit's thread, beside the drawer
`viewer_host::Ahead` spawns, because trap 154's measurement keeps a launch's read on the thread
the launch placed. At a turn that thread is the toolkit's, so the key's handler did not return
until the read did: nothing was painted, the key's own release was not handled and the window
could not be closed (ADR 2328 measured 4.9 s and 9.7 s on the drive's retained-page fixture).
Trap 154 is about the first frame's critical path; a turn's read holds a window, and the window
is what a person is looking at.

## 2. Decision

1. **The core hands the read out owned.** `Viewer::read_out` is `anticipating` for a thread of
   the host's choosing: a `ReadOut` holds its own handles on the document, the font cache (now an
   `Arc` on `Open`, locked per lookup and per insertion) and the target files, a copy of the view,
   and the target the first frame is foretold at. The core spawns nothing (rule 4).
2. **No `settle` reads the page while the read is out**, so a command handled meanwhile — a frame
   answered, a zoom, a tick — neither reads it again on the host's thread nor asks for it;
   `Query::PageGeometry` places it, so a window shows what it retains of it there. `anticipated`
   takes the read in only where the page still waits for it under the view it was read under; a
   read that comes back to a changed view is dropped and `settle` reads the page as it always did.
3. **The host's thread returns at once.** `Ahead::read_out` runs the read on a thread of its own
   with the drawer beside it as `anticipate` arranges them; the window's drawing poll looks for it
   (`drawing_interval` is `POLL` while it is out) and `NativeWindow::take_the_turn` takes it in,
   answers what it raised, and hands the frame its drawing ahead.
4. **A second turn waits for the first turn's read, and every command after it**, in order
   (`WindowState::behind_the_turn`): the core cannot stop a read part-way, so two turns handed out
   would read two pages at once on the same cores, and a command let past a waiting turn would be
   handled before it. This is the order the toolkit's own queue kept while the read held it.
5. **Where it applies.** `quorra-gtk`, `quorra-qt` and the C window share `NativeWindow::turn`
   through `through_the_viewer`, the one place each window's `run` hands a command to the viewer.
   `quorra` does not share the path — `App::turn` in `viewer-ui` keeps its read on the event-loop
   thread — and `quorra-confined` reads in its worker process. An arrangement that brings on more
   than one unread page (`anticipating` answers `None`) is read on the toolkit's thread as before.

## 3. Measured

Drive step 82, the fixture's two pages of six hundred thousand fills, the key trace each window now
prints (`key … arrives` against its command's `events` line). This tree, both windows, two runs
each: the key's command handled 0.000–0.001 s after the key, the retained picture said 0.000–0.001
s after it, the read taken in 0.467–4.522 s after it, the frame 1.135–4.936 s after it; GTK's first
paint of the page after the turn is the retained picture, and in the run whose read took 4.5 s the
window painted it and handled two pointer moves within 20 ms of the key. HEAD with only the key
trace added, two runs of each window: Qt's command handled 1.92 s and 2.46 s after its key, and
GTK's not within the step's 2 s, so four of four failed the 2 s ceiling the step now holds; this
tree passed it.

In one `--clock` hold at loads 1.9 to 2.9, the two trees alternated three times: HEAD with the key
trace handled Qt's key 0.43–1.41 s after it, and GTK's past the 2 s ceiling twice in three (key to
frame 1.09 s the once); this tree handled every key within 0.001 s, key to frame 1.11–3.32 s in GTK
and 1.18–1.43 s in Qt, against HEAD's Qt 1.13–1.61 s. The 3.32 s is a read of 3.21 s on the new
thread, which is the cost trap 154 measured at launch: a read off the thread the toolkit placed may
land on one of this machine's slower cores. It is taken here because the window is answering while
it is paid. `t2-toolkit_launch` and `t2-launch_path` in the same hold: exit 0 on both trees, every
toolkit figure inside its band; this tree's one figure outside is quorra's turn on
`PDF20_AN001-BPC.pdf`, below its band and not judged.

## 4. What it leaves

- `quorra`'s own turn still reads on its event-loop thread; its surfaces draw the page ahead
  differently and the same move there is its own change.
- A second turn waits behind the first turn's read rather than turning at once; a turn handled at
  once with its read deferred would need the core to hold two pages unread.
- §12.4.4.1's `/Dur` advance turns inside `Command::Tick`, whose read stays on the toolkit thread.
