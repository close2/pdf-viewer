# 2511 — The hold's period is the display's refresh, read where each toolkit states it, and the Windows window reads the primary screen only

Status: implemented. Session 1838, batch one hundred and twenty-four. Context: ADR 2510 (the hold, whose
period this decides), `doc/questions/A657`, `doc/todo/37`'s rule 5 (`quorra` reads its cadence from
the display), ADR 0391 (the clock gets a picture every refresh). Code: `viewer_host::stand_in::Hold`
(`refresh_is`, `FLOOR`), `NativeWindow::refresh_millihertz`, `viewer-gtk`'s `Host::refresh_millihertz`,
`viewer-qt`'s `Host::display_refreshed`, `quorra_window_refresh` in `viewer-ffi`, and
`hosts/windows/Quorra.App/DisplayRefresh.cs`.

## 1. Decision

The period a window holds a picture for is **one refresh of the display it is on, as the toolkit states
it, and a sixtieth of a second where the toolkit states none or states zero**. `Hold::refresh_is` takes
the rate in millihertz, the unit the toolkits report in, and `FLOOR` is the sixtieth. The period is
one number in `viewer_host`; what differs by host is only where the rate is read.

| Window | Where the rate is read | When |
|---|---|---|
| `quorra-gtk` | `gdk::Monitor::refresh_rate` of the monitor under the window's surface | at every zoom command, because GTK states the monitor the window is on at that moment |
| `quorra-qt` | `QScreen::refreshRate` of the window's screen, told to Rust by `display_refreshed` | with every placement report, because a window changes display by moving |
| the Windows window | `GetDeviceCaps(VREFRESH)` of the primary screen's device context, told through `quorra_window_refresh` | at start and with every resize |

## 2. Why a sixtieth where no rate is stated

A window that knows nothing of its display needs some period, and the choice is a choice. A sixtieth of
a second is the slowest rate the owner's words admit, "at least 60Hz but 120Hz should be the target" (ADR 0383), so an unknown display
is held for no shorter than a known one would be: a shorter wait would stand in before a frame could
have been shown, and a longer one costs only latency. No clause of ISO 32000-2 bears on it, and the
rule 5 it follows reads the display.

Two reads cost something and one keeps a period. The Windows read is a device-context call at the first
resize, which is on the launch path, and again at each resize: a call that returns at once and touches no
adapter, priced by the CI `window` job's launch figures and not here. Qt's read is a property of the
screen object at every placement report, where GTK reads its monitor lazily at the zoom command, so
GTK's cost falls on a zoom and Qt's on a move or a resize. Where a GTK window has no surface or monitor
yet, `refresh_millihertz` answers `None` and the period already in the hold stays, which is the floor
until a rate has been read once and the last rate read after that.

## 3. The choice and its cost: the Windows window reads the primary screen only

`GetDeviceCaps` on the desktop's device context (`GetDC(0)`) answers the primary display. A window moved
to a second display of another refresh holds for the primary's period until it is next resized. Reading
the window's own monitor (`MonitorFromWindow` and `EnumDisplaySettings`) is the exact answer and needs
three more native calls whose behaviour on a mixed-DPI desktop is the thing the CI `window` job is the
only place to try; the code here is not run on Windows by the round that wrote it, and the simpler call
is the one whose failure is legible (a rate of 0 or 1 falls back to the floor). The cost is a hold too
short or too long by the difference between two refresh rates, on a window that is not on the primary
screen, and never a wrong picture: the hold ends in the scaled stand-in whatever its length. The exact
answer is owed if the owner reports it.

## 4. What is proved and what is not

The unit tests prove that a rate becomes its period, that zero and none become the floor, and that the
hold ends after exactly that period (`stand_in.rs`). The drive proves the two toolkit windows on one
display, where the period is whatever Xvfb's monitor states. `quorra_window_refresh` is proved to exist
and to take a handle and a rate by the ABI's header test. What the C# does on Windows is proved only by
CI's `window` job, which must build `Quorra.App` and publish a window that starts and reads its title.
