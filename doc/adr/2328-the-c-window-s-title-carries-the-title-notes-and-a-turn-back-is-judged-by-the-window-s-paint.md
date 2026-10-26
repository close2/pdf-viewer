# 2328 — The C window's title carries the title notes, and a turn back is judged by the window's own paint

Status: implemented. Session 1747, 2026-10-25, the host slot of batch one hundred and fourteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clauses: ISO
32000-2 §12.2's Table 147 (`/CenterWindow`) and Table 29's `FullScreen`; no row moves. Context:
ADRs 1866, 1886, 2224, 2225, 2290, 2308, 2309, 2316; trap 154.

## 1. The C window's title

`viewer_ffi::Window::title` built the name and the caption and stopped, so a C caller stating
`positions = 0` heard `NOT_CENTRED` on the status line and saw no title note, and a page whose
marks the union declined was named in the toolkits' titles and not in this one (ADR 2308 section
3). The title now appends `WindowState::title_notes()` exactly as `viewer-gtk` and `viewer-qt` do,
so the device lane's sentence, the declined pages and the centre not given are one string in every
window. The C window draws on the processor and owns no device lane, so its first note never
fires; the other two do, and `retitle` already owed a fresh title at each of their changes.

## 2. What the C driver prints

The toolkits' drive steps 105, 106 and 89 read their windows' titles and sentences; the C window
is not in that drive, so `c/drive_a_window.c` states the same three: the placed fixture placed a
second time with `positions = 0` (fitted, not moved, the sentence said and the title ending
"not centred: the platform places this window"); a deck stating `/PageMode /FullScreen` whose slide
is a link, under `--links=ask`, where the click's question waits — said, owed as no record — and
Escape puts it up to be answered "Do not"; and `--declined`, a run of its own, the union's folding
strokes drawn under `PDFVIEWER_PLANT_OPEN_ROW` and once without it, the title naming page 1 only
when planted. A presenting window asks for its timer for as long as it presents, so the driver
ticks it a bounded number of times past its first picture rather than until it rests.
`tests/a_c_program_drives_the_abi.rs` holds every line. With the notes planted out of the title,
the program fails at its uncentred step; the declined run's unplanted half is its own control.

## 3. Step 82's turn back

`82-retained-page` failed in GTK by "no retained picture was said after the turn back" — six runs
of six on this tree's binaries at loads 8 to 12, and twice in three on HEAD's in round 1741. GTK's
own key-binding trace and a stack taken while it waited settle which half is wrong: the Left
reaches the window's controller every time, and `viewer_host::NativeWindow::turn` then reads page
one on the toolkit thread and joins the drawer it spawned (ADR 1886; trap 154 keeps the read
there). The fixture's page is six hundred thousand fills, and the read held the toolkit thread for
4.9 s and 9.7 s in two probes, so nothing — the key's own release included — was handled until it
returned; then the retained picture was painted, said, and replaced by the frame about 0.7 s later.
The race is the drive's, in two places: a two-second ceiling for a sentence a read of seconds
precedes, and a photograph spoiled by the drawing thread's `page 1 rasterised`, which is said about
0.55 s before the frame reaches the window (`RenderReady`). The step now waits for page two's
frame before the turn back, waits for the sentence under the drive's thirty-second ceiling, counts
the photograph late only after `RenderReady`, and in `quorra-gtk` reads the window's own paint
record (`PDFVIEWER_ZOOM_PROBE`'s `raster-paint`, ADR 2225): the first paint of page one after the
turn must be pixels other than its frame's at the extent its frame is then painted at. Calibrated
on a recorded log, the reader names a retained paint, a frame and no paint apart. Four runs of
four in GTK at loads 11.5 to 17.6 and two of two in Qt.

## 4. What is not changed

The read stays on the toolkit thread: trap 154's measurement is about the turn's critical path,
and a window that showed the retained picture during the read would need the read elsewhere,
which is that trap's question and not a drive's. A page of this weight freezes the window for the
length of its read; that is the cost ADR 1886 accepted, now stated with its figure.
