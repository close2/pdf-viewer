# 2172 — The launch is started and joined once, and what a toolkit does at it is its parameter

Session 1668, batch one hundred and three's slot 6, a host round. Status: **accepted** and
**built**. Context: ADRs 1539 (the open on a thread of its own), 1580 (the reader's commands
before the document), 1811 (the bar past `WARN`), 1855 (page one drawn while it is read), 1429
(the fit asked again after a size), 2156 (the refusals reach a C window); habits 131 and 139.

## 1. What was copied

ADR 2156's round found the launch written three times: `viewer-gtk`'s `Opening::start`,
`viewer-qt`'s `Host::open` and `viewer-ffi`'s `Window::open` each built the opening commands,
started a `Loading` that sent the reader's commands, said the named directories' refusals, sent
the opening commands and anticipated page one, then built the `WindowState` from the same seven
values; and each window's first size ran the same steps — the viewport, the `Resize`, the extent
told to the drawing ahead, the wait bounded by `GLIMPSE_AFTER` or `WARN`, the join or the bar.

## 2. Decisions

1. **`viewer_host::native_window::Launch`**: `Launch::start(bytes, fragment, widget_appearances,
   settings, reader, refusals)` starts the thread; `Launch::state(path, trace)` is the window's
   state with it opening. The file stays the caller's to open, because what an unreadable one is
   called is the toolkit's error (`HostError::Unreadable`, `Status::Unreadable`).
2. **`NativeWindow::sized(width, height, scale, sizing) -> Joining`**, provided: the size told, the
   first joining the open. A window's validation of a size stays its own.
3. **The toolkit's part is typed** (habit 139). `Refusals::Terminal` (GTK, Qt: `note: ` on the
   terminal) or `Refusals::Sent(sender)` (the C window's `SAID` record). `Sizing { first, fitting }`:
   `FirstSize::Allocation` or `Resize`, the launch trace's word; `Fitting::Measured` (GTK and Qt
   fit themselves and say `Presenting::placed`, so a later size asks the fit again, ADR 1429) or
   `Fitting::Handed` (the C window hands `QUORRA_OWED_PLACE` once and never learns whether it was
   done, so asking again would owe the record on every resize). `Joining::StillOpening` is what
   GTK arms its `TICK` timer on; Qt's drawing pump and the C caller's tick already look.
4. `WindowState::sized` replaces GTK's and Qt's `opened` and the C window's `joined`.

## 3. Refused

- **`Launch::start` opening the file itself**: one line saved per window, and the error's type
  would have become this crate's.
- **The fit asked again in the C window**: the copy's difference is a difference of shape, not a
  missed step — nothing on that side calls `Presenting::placed`.
- **The trace's two words made one**: "first allocation" and "first resize" are each toolkit's
  name for the event, and no gate reads either.

## 4. Costs

One module (`native_window/launch.rs`), five types, one provided method and one field. The commands,
their order, the sentences and the trace lines are each window's as before; the drive's open steps
and `drive_a_window.c` judge them.
