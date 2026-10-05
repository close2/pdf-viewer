# 1539 — The three other windows anticipate page one, each in its own launch shape

Status: accepted. Session 1352. Extends ADR 1531 to `quorra-confined`, `quorra-gtk` and
`quorra-qt`; amends ADR 1531's `Viewer::anticipate` in one respect (section 3). Supersedes nothing.
Context: `CLAUDE.md` principle 2 (nothing eager; nothing on the launch path waits for warmth; an
optimisation is justified by a benchmark); ISO 32000-2 §6.3.2.2 ("unless otherwise instructed",
which `Command::Delegate` carries); ADRs 0182, 0244, 0246, 0718, 0809, 0812, 1531; trap 83.
Code: `crates/viewer-gtk/src/host.rs` (`Opening`, `opening_commands`, `standing`,
`Host::join_the_opening`), `crates/viewer-gtk/src/bin/quorra-gtk.rs`; `crates/viewer-qt/src/host.rs`
(the same three, `Host::drain`); `crates/viewer-ui/src/bin/quorra-confined.rs` (`Host::anticipate`,
`start_worker`, `Opened`); `crates/viewer-confined/src/worker.rs` (`perform` anticipates after an
open); `crates/viewer-core/src/viewer.rs` (`anticipate` carries the host's delegation).
Test: `viewer.rs`'s `a_delegating_host_keeps_the_page_it_interpreted_ahead`, written against the
defect and seen to fail without the fix (revision 2 against 1).

## 1. What each window's launch was made of

The brief named `viewer-ffi`'s session as the way the GTK and Qt windows reach the core. It is not:
both hold a `viewer_core::Viewer` themselves, so the change is in their hosts and `viewer-ffi` is
untouched. Measured with each window's own `--trace=launch` under Xvfb (1400×1100, llvmpipe), release,
warm page cache, `Well-Tagged-PDF-WTPDF-1.0.pdf`, `ISO_32000-2_sponsored_EC3.pdf` and `bug1815476.pdf`
from the launch gate's set. Before, every window opened the document only after its toolkit — or,
confined, its device and then its worker — was up: GTK's first allocation came at 125–135 ms and the
open, interpretation and raster after it; Qt's first resize at 77–106 ms; the confined window
brought the device up (35–45 ms), then started the worker, then opened, so the device was always up
before the document. In none of the three was the document the longer of the pair, so anticipation
was worth building in all three.

## 2. Decision: each window opens on a thread and joins at its first viewport

- **GTK, Qt.** The file is opened and a thread started before the toolkit is asked for anything
  (GTK: in `main`, before `Application::run`; Qt: in `Host::open`, before `viewer_qt::run`). The
  thread makes a `Viewer` with no viewport, sends the same commands every open sends —
  `Restrict`, `Delegate`, `Separations`, `Open`, now one function, `opening_commands` — and calls
  `anticipate`. The first allocation (resize) joins it, hands the viewer its `Resize` first, and then
  answers the open's events and the resize's in that order, so each reaction sees the viewer an open
  with a viewport would have left. A panicked thread is said and the open is done on the window's
  thread. A password's second open is unchanged: it goes through `opening_commands` on this thread.
- **Confined.** The worker is a process, so the thread holds a pipe: `Host::anticipate`, called from
  `main` before the event loop, starts the worker, sends `Resize` of 0×0 (the worker's own starting
  size would have page one drawn at a size nobody asked for) and `Command::Open` with the file's
  descriptor. The worker's `perform` now calls `Viewer::anticipate` after every `Open`; behind a
  viewport the arrangement has already interpreted the page and the call does nothing. Decoding
  stays inside the confined process on its one thread: the new thread is in the unconfined host,
  so trap 83's filter is not what it meets. `resumed` joins after the device and sends the real
  `Resize`. The resume after a dead worker keeps its own order.

Nothing waits for warmth: no window joins before its own first viewport exists.

## 3. Amendment: `anticipate` carries the host's delegation

The first A/B showed Qt's join-to-`opened` at 18–25 ms on `bug1815476.pdf`: the page was being
interpreted twice. `settle` carries `Command::Delegate`'s answer into the view before interpreting,
and `anticipate` did not, so a host that delegates its widgets (both native windows do; `quorra` does
not, which is why ADR 1531's gate never saw it) had the anticipated page dropped by `Open::stale` at
the first `Resize`. `anticipate` now applies the same answer first.

## 4. What it bought

Ten launches per arm and document, the two arms interleaved in one sitting (load 3–6), medians of
the first frame's trace second, and in brackets the part after the toolkit or device was up:

| window | document | before | after |
|---|---|---|---|
| `quorra-confined` | WTPDF | 104.0 (70.0) | 99.5 (58.0) |
| | ISO 32000-2 | 125.0 (92.0) | 98.0 (57.5) |
| | bug1815476 | 177.5 (139.5) | 144.5 (95.0) |
| `quorra-gtk` | WTPDF | 167.0 (32.0) | 137.5 (13.5) |
| | ISO 32000-2 | 198.0 (69.0) | 176.5 (44.0) |
| | bug1815476 | 165.5 (34.0) | 146.5 (15.0) |
| `quorra-qt` | WTPDF | 100.0 (19.0) | 88.5 (11.0) |
| | ISO 32000-2 | 148.5 (51.5) | 147.0 (39.0) |
| | bug1815476 | 136.5 (32.0) | 119.5 (14.0) |

The part after the toolkit falls on every row. The toolkit's own time rises on some (confined window
up 35 → 43 ms; Qt's first resize on ISO 32000-2 96.5 → 107 ms); moving the worker's start back onto
the main thread did not change that (39–44 ms either way), so it is contention with a second busy
thread, not the spawn, and it is why Qt's ISO 32000-2 row is level rather than lower.

## 5. Left

`bug1815476.pdf` in the confined window still spends 85–113 ms after the device: its page crosses
as pixels, rasterised on the worker's one thread (`doc/todo/34`). The open's eager `Outline::read`
on ISO 32000-2 is now on each thread rather than behind a toolkit.
