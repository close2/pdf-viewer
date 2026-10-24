# 2308 — Table 147's placement is one decision the self-placing windows ask, and WinUI presents full screen natively

Status: implemented, for integration. Session 1737, 2026-10-24, the clause slot of batch one
hundred and thirteen. Clause: ISO 32000-2 §12.2, Table 147's `/FitWindow` and `/CenterWindow`, and
Table 29's `FullScreen` as the C window's caller shows it. Context: ADRs 1429, 1831, 2172, 2225,
2300, 2301.

## 1. What the text asks

Table 147: `/FitWindow` is "[a] flag specifying whether to resize the document's window to fit the
size of the first displayed page", `/CenterWindow` one "specifying whether to position the
document's window in the centre of the screen". Table 29's `FullScreen` is "[f]ull-screen mode,
with no menu bar, window controls, or any other window visible". `quorra`, `quorra-gtk` and
`quorra-qt` already answered both entries (ADR 1429); the C window handed `QUORRA_OWED_PLACE` to its
caller with nothing to answer it by, and owed it again at every frame, because it never learned
that the placement was made.

## 2. The decision

- **One decision for every window that resizes and moves itself by request.**
  `viewer_host::native_window::placing` is `viewer-qt`'s former `place_window` body: a window
  states its inner size, its frame, its screen's work area and whether the platform lets it move;
  the answer is one resize (asked again at the next frame), one move once it fits, or placed.
  `viewer-qt` and the new `quorra_window_place` both call it, so the C caller and Qt cannot
  disagree. GTK keeps its own fit (a default size, no frame to measure) and no move.
- **Every window measures and fits itself.** `Fitting::Handed` existed only for the C window; with
  a call to answer the record, the C window fits as the toolkits do, and the enumeration is gone:
  a size taken while the fit is owed asks it again in every window.
- **A centre the platform will not give is said and titled.** Qt on Wayland now passes
  `positions = false` and is told nothing to move; the window says why and its title carries
  "not centred: the platform places this window" while the document is in front, as GTK's now
  does beside its existing sentence (ADR 1831's shape: the terminal and the title).
- **WinUI carries it out.** `QUORRA_OWED_PLACE` reads the `AppWindow`'s client size, frame and the
  display's work area as logical pixels and resizes or moves by the answer; `QUORRA_OWED_CHROME`
  and the present acts read the new `quorra_window_full_screen` and set the `AppWindow`'s
  full-screen presenter, which takes the title bar and frame Table 29 names.

## 3. What this does not claim

The WinUI code is compiled by `tools/dotnet-check.sh` against the real SDK references, not run; the
C driver proves the library's half (a 200 by 100 page fitted at 800 wide becomes a window 800 by
440, centred at 555, 305 on 1920 by 1080, then nothing owed). The C window's title still carries no
`WindowState::title_notes`, so a C caller stating `positions = 0` hears the sentence but sees no
title note; that title is the caller's record and is left as it is.
