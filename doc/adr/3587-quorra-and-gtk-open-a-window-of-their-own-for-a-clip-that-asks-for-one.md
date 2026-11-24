# 3587 — `quorra` and GTK open a window of their own for a clip that asks for one

Session 2221. Status: accepted. Context: RFC 0015 section 6 rounds 10 and 11; ADRs 3545, 3553,
3585, 3586; ISO 32000-2 §13.2.6.1 Tables 294 and 295.

## Decision

1. **What each says it can do.** GTK: windows, no placement (GTK 4 places no toplevel), Table 304's
   0, 2, 4, 5, 6 — sent through `Hearing::HereWithPicturesInWindows` from the launch. `quorra`:
   windows; placement and 1 and 3 off Wayland only — decided in `resumed` from the event loop and
   sent when the opened document is taken in, before any activation.
2. **One picture for both.** `VideoPlayer::window_picture` draws, on the processor with the
   oracle's rasteriser, Table 294's `/B` over the window and the picture fitted by Table 291's `/F`
   at the window's own size. GTK holds it as a texture in a `GtkPicture` filling a `gtk4::Window`;
   `quorra` presents it through a `SoftwareSurface` of a second winit window. A second device
   surface for a window showing a few pictures a second is not built; the page's layer stays the
   device's. Nothing is drawn over the page while the clip is in its own window.
3. **Placement (quorra off Wayland).** `screens::place` puts the window, its frame included — the
   document window's own frame is the measure of a title bar — in the window `/RT` names (the
   document window, which in this program is the application window too; the virtual desktop; the
   chosen monitor) at `/P`'s row and column, then applies `/O`: 0 leaves it, 1 moves it and
   shrinks it onto the monitor holding most of it, 2 refuses the clip, said. GTK and Wayland say
   the window manager placed it.
4. **What the reader keeps.** `/T` and `/UC` say what true requires and nothing of false, and
   §13.2.6.1 warns of "windows containing arbitrary media content" shown without saying so: the
   window keeps its title bar and its close, and its title is the document's `/TT` ahead of the
   clip's label, which names the document and the clip. `/R` 2 is resizable; 0 and 1 are not —
   1's "only if aspect ratio is preserved" is honoured by never resizing, since neither toolkit
   keeps an aspect for a toplevel.
5. **Closing.** Its close and Escape stop the clip; the clip's end, a stop, or its document closing
   takes the window down at the next look.

## Costs

The `quorra` window itself is not driven by a test: the headless window has no event loop, so the
test stops at the judged window, the processor's picture and the window's raster. GTK's is driven
under a virtual display with one monitor and no window manager. No person has watched either. A
paused clip in a resized window shows its last picture stretched until the next. §13.2.6.3's
timespans still do not cut the pictures (`Shown::cut`); that is the next round's.
