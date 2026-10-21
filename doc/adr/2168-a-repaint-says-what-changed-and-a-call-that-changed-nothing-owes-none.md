# 2168 — A repaint says what changed, and a call that changed nothing owes none

Session 1666, RFC 0010 section 6's step N3a, third part. Status: **accepted**, in
`crates/viewer-ffi/src/window/repainted.rs` and `owed.rs`, `quorra.h`'s window section, and
`hosts/windows/Quorra.App/` (`MainWindow.xaml.cs`, `QuorraWindow.cs`, `PageArea.cs`). Context: ADRs
0244 finding 5, 2088, 2104, 2153 section 4; `viewer-qt`'s update flags (ADR 0246).

## 1. What was wrong

Every drain of `quorra_window` owed `QUORRA_OWED_REPAINT`, and the record said only "again". The
WinUI window therefore read every picture, copied its pixels across the boundary, converted them to
premultiplied BGRA in a managed loop and uploaded them, on every call — a pointer's move and every
step of a drag among them — and drew the marks again whether or not they had moved.

## 2. Decisions

1. **The record carries what changed**, as `quorra_owed_repaint`'s word of three bits:
   `QUORRA_REPAINT_FRAMES` (a picture's pixels, or how many pictures there are), `_PLACES` (where a
   picture is shown, or at what extent) and `_MARKS` (the selection, the find bar's occurrences,
   Annex O's highlight, §12.5.1's focus, §12.9's traced path); and `quorra_owed_repaint_fresh` says
   for each picture whether its pixels are ones the caller has not been handed. A caller copies
   those and moves the rest.
2. **Compared, not inferred.** The window remembers what the last repaint told: each picture by its
   page, its raster's size and layout, whether it is the retained page, and a digest of its bytes;
   each place by the bits of its origin and its extent; the marks as the caller reads them. A
   picture's pixels come from five sources (the viewer's frames, a page being drawn, the retained
   page, the oversized page's window, page one's look while it is read), so an event raised by each
   would be five places to forget one; the comparison is one, which is `shown`'s reason for the
   controls (ADR 0244 finding 5).
3. **A call that changed nothing owes no repaint.** The comparison runs wherever a repaint was owed
   before — the end of every drain and each of `NativeWindow`'s required repaints — and owes the
   record only where a bit is set. Two in one batch are merged where the first stood: the bits of
   either, and a picture new to either is new.
4. **The digest is SipHash-1-3 under a key drawn for the window** (`std`'s `RandomState`): no new
   dependency, and a document cannot arrange two frames to collide under a key it never sees.
   Measured at 6.3 ms for 32 MiB on this machine (`rustc -O`, one thread). Its cost, written down:
   two different frames collide with a probability near 2⁻⁶⁴, and the previous frame then stays
   until the next change.

## 3. Considered and refused

- **The raster's address as its identity**: the drawing thread's next raster is allocated while
  the old one lives, but two replacements between two comparisons can hand the first address back,
  and a page being drawn is written in place. Equal addresses would then hide new pixels.
- **A second copy of every pixel shown, compared byte for byte**: exact, and as much memory again
  as the pictures on the screen, for a probability of error smaller than the hardware's.
- **Repainting from `Event::Damage`**: "a bound on what changed", not a list (ADR 0244 finding 5).

## 4. What the gates show

`drive_a_window.c` reads the record: the find bar closed owes one repaint saying `MARKS` with no
fresh picture, a pointer moved where it was owes none, and the wheel's zoom says frames, places and
a fresh picture. A planted "owe every drain" fails the first of those (the pointer owed 1).
