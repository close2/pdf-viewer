# 3280 — A link in a window hosted through the C ABI goes to its host

Session 2144. Status: accepted. Rows: §12.6.4.8 (`implemented`, unchanged). Context: ADRs 1155
(the link levels), 3217 (the owed open records).

## Decision

ADR 3217 moved a `quorra_window`'s media and launched-document hand-offs to its host, and left
§12.6.4.8's link. A link still went to `viewer_host::open_uri`, which starts `xdg-open`. Windows has
no `xdg-open`, so in the WinUI window every link the level let through failed.

- `viewer_host::link_through` and `answered_through` take the opener. `link` and `answered` are
  those functions with `open_uri`, so the GTK and Qt windows and the two `viewer-ui` binaries are
  unchanged. The level, the question and every sentence stay in one place.
- `NativeWindow::hand_link_over` defaults to `open_uri`. The C ABI's window overrides it: it owes
  a `QUORRA_OWED_OPEN_URI` record (19), the same kind a media clip's URL uses, so the record
  count and the entry points do not move. A failure comes back through
  `quorra_window_open_failed`, as ADR 3219 has it.
- `viewer_host::handable_uri` is the scheme check (`LINK_SCHEMES`), taken out of `open_uri`. The
  override runs it before the host is given anything, so a host never receives a scheme no window
  would hand over.

## What only CI checks

The C# already launched an `OPEN_URI` record with `Launcher.LaunchUriAsync`; only its comment
changes. It is compiled only by CI's Windows job, because this machine has no .NET. Whether
Windows opens the browser was not observed (`tools/drive-windows.sh`).
