# 3217 — A window hosted through the C ABI hands a file to its host, which opens it

Session 2129. Status: accepted. Context: ADRs 2171, 3201, 3202, RFC 0010, `doc/questions/A501`.

## Decision

A `quorra_window` no longer calls the desktop portal or `xdg-open` itself. Its two hand-offs — a
media clip, a movie's or a sound's file (ADR 3201) and §12.6.4.6's launched document (ADR 2171) —
become records its host carries out: `QUORRA_OWED_OPEN_FILE` (18) with the file's path, and
`QUORRA_OWED_OPEN_URI` (19) with a media clip's URL. `QUORRA_OWED_KIND_COUNT` is 20. The host's
opener answers later, so `quorra_window_open_failed(window, handed, why)` hands a refusal back, and
the window says it (ADR 3219).

`viewer_host::NativeWindow` gained two methods with the GTK and Qt behaviour as their defaults,
`hand_media_over` and `hand_launched_over`. The C ABI's window overrides both. Everything decided
before the hand-off is still the one shared code path: the level, the question (the same card, a
`QUORRA_OWED_QUESTION`), the refusals at every level and the clip written into its own `0700`
directory. Only the last step crosses the boundary, through an `Opener` that collects instead of
calling.

## Why the host, and on every platform

The only program hosting a `quorra_window` is the WinUI window, which is the platform's own program
and has the opener that this library lacks: `Windows.System.Launcher.LaunchFileAsync` and
`LaunchUriAsync`. The portal does not exist on Windows. A behaviour that depended on `cfg(windows)`
could not be tested on the machine that builds this library, while the record can be tested here.
A C host on Linux receives the same record and can open the file with whatever its desktop
provides.

A path that is not UTF-8 is refused by name rather than converted, because after a lossy conversion
it would name a different file. An embedded clip's descriptor is closed before the record is
owed, because the host opens the file by its path, which is how `StorageFile.GetFileFromPathAsync`
takes it.

## What only CI checks

The C# (`MainWindow.Launch`, `QuorraWindow.OpenFailed`, the declaration and the constants) is
compiled only by CI's Windows job, because this machine has no .NET. `header_and_interop_agree`
checks here that the constants and the declaration match the header. Whether `Launcher` starts a
player was not observed.
