# Round 2125 — a media file is handed to this machine's player

A host round in the `audit` worktree: RFC 0015's first item, owed by A254. No exclusion is lifted and
nothing here plays audio or video.

## What it built

- `viewer_host::policy::MediaFiles`, `--media-files=refuse|ask|warn|open`, `ask` by default: in
  `Settings`, `Policy::MediaFiles`, the C ABI's `QUORRA_POLICY_MEDIA_FILES` = 7, the restrictions
  menu's `Act::HandingMedia` and a question of its own, `Subject::Media` (ADR 3201).
- `handing` decides; a clip that begins as a program, a file outside the document's directory and a
  URL not `http` or `https` are refused at every level. `hand_media` carries it out through an
  `Opener`; `Portal` calls `org.freedesktop.portal.OpenURI.OpenFile` by `gdbus` with the file as
  descriptor 0 (`@h 0`), or `OpenURI`, after writing an embedded clip as `clip.<ending from /CT>` in
  a fresh `0700` directory under `$XDG_RUNTIME_DIR` (ADR 3202).
- GTK and Qt through `native_window`, `quorra` (`Pending::Media`), and `quorra-confined`'s unconfined
  side, whose `r` menu holds both its acts; the renderer's system calls are unchanged. On Windows
  `Portal` declines by name.

## Ledger

§13.2.4.2 stays `implemented`, its note rewritten and two tests added. Counts unchanged: 788
`implemented`, 51 `out-of-scope`, 31 `inapplicable`, 11 `departed`, 2 `writer-side`.

**Gates.** Over the five viewer crates: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox`
build 0, nextest `--lib --bins --tests` under `xvfb-run -a` 699 of 700. The one failure was this
round's: a `viewer-ffi` test used 7 as an undefined policy, now 8, passing on re-run. No difference
against `expected-failures-current.json`. Rustdoc 0. `cargo test -p conformance --no-fail-fast` 0
once this paragraph was written.

## Left, in order

1. The WinUI window: `QUORRA_EVENT_MEDIA` handed to `Launcher.LaunchFileAsync`.
2. Sweep this program's old `quorra-media-*` directories at the next hand-off (ADR 3202).
3. Say a portal failure after the spawn in the window, not only on the terminal.

## Habit

A struct literal in a test is a census of every field: one field on `Settings` touched five test
files, one an owner audit test (`choice_commit_audit.rs`, one line, A868).
