# 2317 — `quorra` parks its presentation, GTK reads `SUSPENDED`, a dead display's lock is free, and the device-select layer is obeyed

Status: implemented. Session 1741, 2026-10-24, the host slot of batch one hundred and thirteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clauses: ISO
32000-2 §12.4.4.1 (`/Dur`'s "displayed") and Table 29's `FullScreen`; no row moves. Context: ADRs
0508, 1303, 1702, 2200, 2250, 2298, 2300 (its section 5 is the list this closes), Q595.

## 1. `quorra`'s winit presentation parks

ADR 2300 read "displayed" as a window someone can see, and parked the pump the three other windows
turn; `quorra`'s presentation is `viewer-ui`'s own winit loop and counted on. winit 0.30 gives two
signals and no third: `WindowEvent::Occluded`, which X11 raises only for VisibilityNotify's fully
obscured state (nothing on an unmap) and Wayland and Windows never raise; and `Window::is_minimized`,
which reads X11's `_NET_WM_STATE_HIDDEN`, Windows' iconic state and macOS's miniaturised one, and
answers `None` under Wayland. The window is hidden on either. No event announces a window manager's
iconify, so the minimised state is read at each of the presentation's own wakes — one property read
per `Clock::RESTING`, only while a presentation runs. Each change calls `Clock::resume`, so the time
hidden is never told to the core; a transition in flight ends on its page and its held end frame
goes with it; the document's script timers carry the time while the clock is parked (ADR 1702).

## 2. A tab changed while presenting, and `quorra`'s presentation per tab

The premise that only the drive was missing did not hold in `quorra`: it kept one presentation per
window, so a document arriving in front of a running presentation stayed full screen and
presenting. It now keeps a `viewer_host::Presenting` per tab, as ADR 2200 and ADR 2300 §3 have the
other windows do: a tab brought forward, a tab closed and a document arriving in front each drop
the clock, the face and the armed transition of the one left, and the arriving document's own mode
decides; a document opened behind takes the front only to give it back and keeps the front's clock
(ADR 1303). No input changes the tab while presenting in any window — `viewer_host::keys` removes
Control+Tab then, and Table 29's `FullScreen` shows no strip of documents — so drive step
`100-presentation-switched` has the deck change it: a link on slide 1 opens a file in a new window
(`/NewWindow true` under `--remote-documents=open`), and Control+Tab, which the table allows once a
windowed document is in front, brings the deck back. It is judged by the windows' mode lines and
the slide timing, since under no window manager a full-screen window keeps its size. Step
`98-presentation-hidden` now runs in `quorra` too, hiding it as a window manager iconifies one:
`_NET_WM_STATE_HIDDEN`, then the unmap, and back in the other order. A fully obscuring window was
the other choice and would have tested occlusion rather than minimising.

## 3. GTK under Wayland parks on the compositor's `suspended`

xdg-shell states no minimised window, and GTK parked only on `MINIMIZED`. GTK 4.12 reads the
compositor's `suspended` state as `GDK_TOPLEVEL_STATE_SUSPENDED`; `gdk4` names it only behind its
`v4_12` feature, and the workspace binds `v4_10` because a floor is a runtime requirement on everyone
who installs the program (ADR 0508). Raising it would be the owner's question; reading the bit by its
ABI value is not, since a GTK before 4.12 never sets it — the move ADR 2250 made reading
`GdkSurface:scale` by name. Measured under a nested headless KWin 6.7.5 on a private bus, a GTK
4.24.1 window minimised by a KWin script went from state `0xaa40` to `0x1aa00` — `SUSPENDED` set,
`MINIMIZED` never — and back to `0xaa00` restored. The bit cannot be read off `Toplevel::state`:
`gdk4` converts the state by truncation, which drops every bit its feature set does not name, and a
first form of this change that tested `state().bits()` parked nothing. `watch_visibility` reads it
off the `state` property's own number, beside `MINIMIZED`. In the same nested session, a deck of
two-second slides minimised for four seconds after 0.67 s of its second slide: this tree parked at
the minimise, turned again at the restore and showed the third slide 1.40 s later; HEAD's binary
turned three slides while nobody could see them. No `Q604`.

## 4. A dead display's lock is free

Xvfb writes `/tmp/.X<n>-lock` with its pid and listens on `/tmp/.X11-unix/X<n>`, removes both on
`TERM` and leaves both when killed. The launch harness killed its server, every instrument read any
lock as a display in use, and the stale ones accumulated until they failed the gate. Two halves, in
one helper shared by path between `viewer-host`'s launch harness and `viewer-gtk`'s stand-in tests
(`toolkit_launch/x_lock.rs`) and restated in `tools/drive-windows.sh`: a lock that is this user's
and names a pid that is not running is free — lock and socket are removed and the number taken —
while another user's lock, a live server's and a lock not yet written are passed over, and trap
181's test (the lock names our own server's pid) still decides whether a number is ours, which the
drive now checks too; and a server is stopped with `TERM`, given two seconds, killed only after,
and whatever still names its pid removed. Planted, the old code skipped a stale lock and left two
more a run; the new reclaimed it and left none.

## 5. The device-select layer's choice is obeyed

Mesa's device-select layer reorders the Vulkan enumeration so the device it selects comes first:
`MESA_VK_DEVICE_SELECT=vendor:device` names one, `DRI_PRIME` names the GPU the display is not on.
`request_adapter` under `HighPerformance` sorts by device type and reads that order as nothing, and
ADR 2298's carried-list choice restated the ranking, so a `DRI_PRIME` choosing an integrated GPU
beside a discrete one was overruled. Where either variable is set and not empty and
`NODEVICE_SELECT` is not, the launch now takes the first enumerated adapter that can present — the
layer's choice; otherwise the ranking stands. `Options::adapter`'s pattern still comes first, and
the fallback to `request_adapter` is unchanged. On this machine `MESA_VK_DEVICE_SELECT=10005:0` puts
llvmpipe first in `wgpu`'s enumeration; a test re-runs its own binary under that variable and holds
the carried choice to it, and with the rule planted out it failed naming llvmpipe first. Q595's
launcher question is untouched: `NODEVICE_SELECT=1` still silences both variables.

## 6. Cost

A property read per hundred milliseconds while `quorra` presents; one bit test per GDK state notice;
no launch changes where neither variable is set. No pixel moves.
