# 3408 — The sound path on Windows and macOS

Session 2176. Status: accepted. Context: ADRs 3317, 3329, 2663; `tools/platform-check.sh`.

## Decision

1. **What builds.** `quorra-sound` with its `device` feature (`cpal` over WASAPI) and
   `pdf-media-worker` check clean, `-D warnings --all-targets`, for `x86_64-pc-windows-msvc` and
   `x86_64-pc-windows-gnu` on this machine. `viewer-host` does not cross-check here: `ring`'s build
   script, under `rustls` and `ureq` (ADR 1327), needs a C compiler for the target, which this
   machine lacks; CI's native Windows job is where it is built. macOS is not checked here: the
   `aarch64-apple-darwin` target is not installed, and `tools/platform-check.sh` does not list
   `quorra-sound` or `viewer-host`. No platform dependency was added.
2. **What a window does.** The windows that play here are GTK, Qt and `quorra`
   (`Command::PlaysHere(true)`); each finds `quorra-sound` beside its executable with the
   platform's suffix (`program_beside_executable`), and a helper missing or failing is said in the
   title and on the terminal while the window goes on (ADR 3317 decision 5) — the sound is not
   handed elsewhere. The C ABI's window (`viewer-ffi`, WinUI and AppKit hosts) sends no
   `PlaysHere`, so every clip is judged for the hand-off and given to the program hosting it, which
   is the platform's own player, said as before (ADR 3217).
3. **Left**: a hand-off fallback in `quorra` where no helper is installed, and a macOS check, each a
   later round's.
