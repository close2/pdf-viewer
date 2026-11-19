# 3202 — The hand-off goes through the desktop portal, called by `gdbus`

Session 2125. Status: accepted. Context: ADR 3201, ADR 1155 (`xdg-open` for a link), RFC 0015
section 3, `CLAUDE.md` principles 2 ("no heavy runtime") and 3.

## Decision

The opener is `org.freedesktop.portal.OpenURI` on the session bus: `OpenFile(s, h, a{sv})` for a
written clip or a file beside the document, `OpenURI(s, s, a{sv})` for a URL. `viewer_host::Portal`
makes the call by starting `gdbus call --session …`. A file reaches it as `gdbus`'s standard input,
descriptor 0, named in the call as `@h 0`, and `gdbus` passes its own descriptor across the bus. A
URL is quoted in `GVariant`'s text format (`gvariant_string`).

- **The portal, not `xdg-open`.** `OpenFile` takes a descriptor, so the same call works inside a
  Flatpak, where the player sees no path of ours, and outside one. `xdg-open` takes a different route
  on each desktop.
- **`gdbus`, not a D-Bus library.** The tree's `zbus` is used only by `viewer-accessibility` and needs
  an executor. One call per activation does not justify putting a runtime into every window.
  `gdbus` comes with `GLib`, which every desktop running the portal has, and it accepts a `h`
  argument. That `gdbus` passes a descriptor was confirmed from the strings of GLib 2.84's binary
  (`g_unix_fd_list_append`, "Error adding handle"). No call to a real portal was made, because none
  runs on the build machine.
- **The call is a child process waited for on a thread of its own** (`media-portal`), the same shape
  as `start_handler`. A portal that is slow to activate blocks no window. A failure after the spawn
  is printed on the terminal. A failure to spawn, to write or to open is said in the window.

## The temporary file, and its cost

An embedded clip is written to `clip.<ending>` in a fresh `0700` directory
`quorra-media-<pid>-<nanos>-<n>`. The directory is made with `create_dir`, never `create_dir_all`,
so an existing directory is never written into. It is placed under `$XDG_RUNTIME_DIR`, which belongs
to the user, lives in memory and is emptied at logout; where that variable is unset, the system's
temporary directory is used. The ending comes from `MEDIA_ENDINGS` keyed by Table 285's `/CT`. It is
never taken from the document's name, because an opener chooses a handler partly by the ending.
A type the table does not list gets no ending.

**The file is not removed after the hand-off.** The player opens it after the portal has replied,
and this program is not told when, so removing it would race the player. The file lives as long as
`$XDG_RUNTIME_DIR` does. A cleanup on window close would remove clips a player is still showing. A
later round may sweep this program's own directories older than the session at the next hand-off.

## Tests

`policy/media/tests.rs`: the arguments of the portal call, and a double standing in for `gdbus` (a
script given as `Portal::program`). The double records its arguments and reads descriptor 0, so the
test proves that the clip's bytes cross through the descriptor named `@h 0`. A recording `Opener`
checks the written file's bytes, its `0700` directory, its type-chosen name, and the URL route.
