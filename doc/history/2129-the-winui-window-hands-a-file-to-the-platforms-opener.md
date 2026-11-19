# Round 2129 — the WinUI window hands a file to the platform's opener

A host round in the `audit` worktree, the clause 13 lane's second build: the three items round 2125
left. Nothing here plays audio or video.

## What it built

- **WinUI** (ADR 3217). A `quorra_window` hands a clip, a movie's or a sound's file, and §12.6.4.6's
  launched document to its host as `QUORRA_OWED_OPEN_FILE` (18, the path) or `QUORRA_OWED_OPEN_URI`
  (19, a media URL). `QUORRA_OWED_KIND_COUNT` is now 20. The WinUI window gives each record to
  `Windows.System.Launcher` (`MainWindow.Launch`) and hands a refusal back through the new entry
  point `quorra_window_open_failed`, which brings the entry-point count to 384. `NativeWindow`
  gained `hand_media_over` and `hand_launched_over`, whose defaults keep the GTK and Qt behaviour;
  the C ABI's window overrides both. The level, the question card and the written clip are the
  code every window shares. The C# is compiled only by CI's Windows job, because this machine has
  no .NET.
- **Sweep** (ADR 3218). At each hand-off that writes a clip, `quorra-media-*` directories older
  than a day and owned by this user are removed. A day is chosen because it outlasts a sitting.
- **A portal failure after the call** (ADR 3219). `PortalCalls`, held in `WindowState`, brings the
  `gdbus` thread's failure back to the GTK, Qt and C ABI windows, which say it.
  `quorra` and `quorra-confined` say everything on the terminal and still print it there.

## Ledger

§13.2.4.2 stays `implemented`, with its note extended and three tests added. Counts are unchanged.

**Gates.** Five viewer crates: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox` build 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 706 of 707, the failure this round's (the C driver's
owed-kind count; it now drives `quorra_window_open_failed`), passing re-run. No manifest difference.

## Left, in order

1. A link (§12.6.4.8) on the WinUI window still goes to `xdg-open`, which Windows lacks. The same
   record would carry it.
2. Watching `Launcher` start a player on Windows: `tools/drive-windows.sh`.

## Habit

A `cfg(windows)` branch is untestable on the machine that builds the library. A record that the
host carries out can be tested here.
