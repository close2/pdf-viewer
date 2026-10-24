# 2300 — The presentation is turned by one pump, and parks while nobody can see it

Status: accepted for integration. Session 1733, 2026-10-23, the host slot of batch one hundred and
twelve, under the project owner's switch to code improvements and bug fixes of 2026-10-14 and
A287's Windows-native host. Clauses: ISO 32000-2 §12.4.4.1 (`/Dur`, Table 164's `/D`), §12.6.4.15,
§12.2 (the row stays `partial`, ADR 2301). Context: ADRs 0135, 0470, 1216, 1303, 1702, 1853, 2172,
2200. Code: `viewer-host/src/presentation_pump.rs`; its callers in `viewer-gtk/src/host/`,
`viewer-qt/src/host/` with `cpp/window.cpp`, `viewer-ffi/src/window.rs` and `window/chrome.rs`;
`quorra_window_shown`; `hosts/windows/Quorra.App/MainWindow.xaml.cs`. Tests:
`viewer-ffi/src/window/chrome_tests.rs::the_pump_advances_a_presentation_and_parks_it_while_the_window_is_hidden`;
drive steps 97 to 99.

## 1. One turn, three windows

`viewer_host::Clock` already decided how often a presentation is ticked and `script_timers::Ticker`
the same for a document's timers. The turn between them — drop §12.6.4.15's spent clock, choose
which clock carries the time, tell the core, react only where it said something, rasterise the
transition's frame — was written in `viewer-gtk` and in `viewer-qt`, alike but for the toolkit
call, and not at all in the C window: a WinUI window presenting a page that states `/Dur` stood
still on it, and its script timers never fired. ADR 2172's test applies, and the turn is now
`viewer_host::PresentationPump`, a trait every `NativeWindow` has through one blanket
implementation and none overrides, over a `Pump` held on `WindowState` (the script ticker, the
hidden flag, the owed synchronisation, and the rasteriser a transition's frames are drawn with,
made at the first frame rather than at launch). What a toolkit keeps is a one-shot armed for
`presentation_wait` after every command, a call to `turn_the_presentation` when it fires, and the
picture `Turned::Frame` hands it. GTK and Qt lose their `CpuRasterizer` and their ticker; the C
window turns the pump inside `quorra_window_tick` at the deadline its last entry point computed —
`quorra_window_interval` reads the window through `const`, so the wait is asked where the window
may change — and shows the frame as the page area's one picture.

## 2. A hidden window parks the presentation

§12.4.4.1: the `/Dur` entry is "the maximum length of time, in seconds, that the page shall be
displayed before the presentation automatically advances to the next page". A page in a window no
one can see is not displayed, so the time the window is hidden is not told to the core: the clock
stops ticking, and `Clock::resume` counts from the moment it comes back, the core keeping what the
page was displayed before. This is a reading of "displayed" and is written as one; the alternative,
a slide show advancing in a minimised window, is a presentation shown to nobody, and a window waking
ten times a second to count it spends a processor on nothing (principle 2). Two choices ride on it:
a transition in flight when the window is hidden ends on the page it was moving to, since Table 164
states a visual effect and no frame of it was seen; and a document's script timers keep running,
because `app.setInterval` is not display — while the presentation is parked its clock tells the core
nothing, so the script ticker carries the time and there is still one clock per window (ADR 1702).

Each window reads its platform's word for hidden: GDK's `ToplevelState::MINIMIZED`, which under X11
an unmap of a mapped window becomes where no window manager states `_NET_WM_STATE_HIDDEN`; Qt's
expose event on the window's `QWindow`; WinUI's `Window.VisibilityChanged`, through the new
`quorra_window_shown`. GTK and Qt take the notice on their idle queues and read the state when the
idle runs, because either toolkit may raise it inside a call the host makes while borrowed.

## 3. Another document in front parks the one left

ADR 2200 gave each tab's `Showing` its own `Presenting` and adopted it in the C window only. The
three windows now do the same through the pump: `WindowState::park_presentation` before the tab
changes, `bring_the_presentation_forward` after it — the clock, the face and the armed transition of
the document left are dropped unless the change is a document opened behind, which takes the front
only to give it back and keeps the front's clock (ADR 1303) — and `synchronize_if_owed` once the
queue has settled, which sends `Command::Present` with the arriving presentation's mode and calls
the new required `NativeWindow::apply_the_chrome`: GTK's `apply_chrome`, Qt's window flag, the C
window's chrome record. Closing a tab and §12.6.4.3's replacement in place do the same.

## 4. Step 83's flakiness was the camera's, twice

Thirty-two verdicts of `83-transition-frames` on HEAD's binaries — four runs in GTK, Qt and
`quorra` on the device and under `--cpu`, then eight in GTK and Qt with the camera's pictures and
their timeline dumped — at load averages from 20 to 53, read wrong six times, all in the two toolkit
windows. Two races, both between the window's clock and the drive's read, neither a defect of the
windows:

- **The camera started after page 1 had begun.** `p` starts §12.4.4.1's clock and the camera is a
  Python process started after it; under load the first picture came 0.6 s into page 1's one second,
  so page 1 was seen for 0.33 s and 0.385 s in Qt's two such runs, under the 0.4 s that makes a
  picture a settled page, and the first transition had no settled page before it. The first slide now states `/Dur 2`, and the camera
  watches for nine seconds, which its header already said.
- **A load-delayed tick kept a transition's last frame up as long as a page**, four times, twice in
  each toolkit window (0.40 s to 0.43 s in the three the camera dumped). That frame and the page it
  ends on differ by one unit in an antialiased edge row — the image draw's rounding, inside the one
  unit the judgement allows — and the camera called both settled, with nothing moving between them.
  Two adjacent settled pictures within that unit are now one page, the later kept.

## 5. What is not done

`quorra`'s presentation is `viewer-ui`'s own winit loop (ADRs 0441, 0473) and does not run the pump,
so a minimised `quorra` still counts the time; step 98 says so as "not offered". Under Wayland,
xdg-shell tells a client of no minimised state, so GTK parks there only once GDK states one; GTK
4.12's `SUSPENDED` would, and this tree binds 4.10. The WinUI caller is compiled against the real
SDK and its interop declaration checked; it is not run on Windows here.

## 6. Cost

One module of 329 lines in `viewer-host`, its tests included; the two toolkit windows lose their
own turn — GTK's presentation module goes from 221 lines to 161 with the visibility watcher added,
Qt's from 176 to 83, and each loses its rasteriser and its script ticker. One entry point added to
the C ABI, which changes no existing signature.
