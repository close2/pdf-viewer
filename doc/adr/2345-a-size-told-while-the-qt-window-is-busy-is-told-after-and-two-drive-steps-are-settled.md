# 2345 — A size the Qt window learns while it is busy is told afterwards, and two drive steps are settled

Status: implemented. Session 1755, 2026-10-26, the host slot of batch one hundred and fifteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clauses: ISO
32000-2 §12.2's Table 147 (`/FitWindow`, `/CenterWindow`); no row moves. Context: ADRs 1429,
2146, 2308, 2328.

## 1. Qt's `01-center-window` at 0,0

Round 1747 read `quorra-qt` at 0,0 twice at loads 30 to 40 and asked whether the drive read the
position before the compositor placed the window or the window had not placed itself. Under
`Xvfb` the drive runs no window manager, so nothing stands between the window's request and its
position. A probe launched the drive's fixture and read the position every 20 ms for up to 60 s.
HEAD's binary stayed at 0,0 for the whole wait in 7 of 25 launches, at loads 3 to 19. Every launch
that stayed shared one shape: its first frame was collected and painted in one pass, after the
outline's read.

With `NativeWindow::place_the_window`'s decision now on the launch topic, all nine such launches of
this tree said `Resize { width: 1000, height: 905 }` and then nothing. No `Resize` command reached
the core, so no frame followed to measure again, and the move was never asked. The cause was in
`MainWindow`'s `resizedTo` handler. `placeWindow` runs from `applyUpdates` inside a call into the
host, and in that shape Qt delivered the window's new size there, while `busy_` was set. The
handler dropped it, as it drops every call that would borrow the host twice. So it was the
window's fault, not the drive's.

**Decision.** A size that arrives while the window is busy is told once the loop turns, as the
size the page area has then (`PageArea::tellSize`); it is never dropped. With the change, 14 of 14
launches moved and were 1000×905. Two of them had the shape and were told 6 ms and 22 ms later.
In the six-pair alternation before it, the one such launch moved too.

**The drive waits for the move as well.** Even with the size told, the move comes one frame after
the fit's resize. In one launch that was 36 ms after the title the step waited for. The step now
waits up to 10 s for the window to leave 0,0, and a window still there is judged with its last
placing line beside it.

## 2. `88-archive-refused`

The step expected the drive's own fixture to be refused at `fonts/font-programs-embedded`. That
fixture names Helvetica with no descriptor. Since ADR 2146 the converter embeds the face this
reader draws that font from, writes a descriptor read off the face, and says so by name
("Helvetica (resource F1) is drawn from Liberation Sans Regular"). The output, held to PDF/A-2b
again, conforms. The window was right and the step's premise was older than ADR 2146. The step is
now `88-archive-substituted`. It asks for the written file, its header, its `pdfaid:part` and that
sentence.
