# 1737 — Table 147's placement is one decision, and a presentation holds a document's dialogues

Session 1737, slot 2 of batch 113, a clause round; ADRs 2308, 2309; no question. §12.2 stays
`partial`, its note rewritten to what remains: the WinUI half run on Windows (ADR 2301's argument).

**Built.** `viewer_host::native_window::placing`, Qt's `place_window` body as the one decision for
a window that resizes and moves itself; Qt and the new `quorra_window_place` call it, and with
`Fitting::Handed` gone the C window stops owing the record at every frame. Qt under Wayland and GTK
say a centre they cannot give and title it (`NOT_CENTRED_TITLE`). WinUI sets the `AppWindow`'s
full-screen presenter from the new `quorra_window_full_screen` and resizes or moves on
`QUORRA_OWED_PLACE`. The pump holds a question at the *ask* level (`NativeWindow::ask`, all nine
call sites), a script's question and the password prompt while full screen, says each waits, and
puts them up in order once it has left. Drive steps 105 (a 300 by 200 page's area read back from
the geometry record) and 106 (a link's question held until Escape).

**Premise and hypothesis.** The premise did not hold as written: GTK and Qt already honoured both
entries (ADR 1429), and what was missing was the C window's answer, Qt's silence under Wayland and
the title line. The hypothesis held for Qt (`/FitWindow` on both platforms, `/CenterWindow` on X11
only; GTK centres on neither); the hold is about sixty lines of code in the pump; the row stays
`partial` on the Windows run alone.

**Gates.** `rustfmt --check` on every touched file and `cargo fmt --check` for `viewer-host`,
`viewer-ffi`, `viewer-gtk`, `viewer-qt`: exit 0; warnings-denied clippy `--all-targets` and rustdoc
for the four: exit 0; their `cargo nextest run`: 445 run, 443 passed, the two stated entry-point
counts then raised to the batch's 359 and re-run green (the held test fails with the hold planted
out). The C window driver: one resize to 800 by 440, one move to 555, 305, then nothing owed.
`tools/dotnet-check.sh`: exit 0, 0 warnings (a planted name in the window refused). The drive under
Xvfb on an export (HEAD plus this round's files and hunks; the export's copy of the drive), steps
`drive`, 105, 106 and 97 in the three windows: 124 works, 0 wrong, 4 not offered — 106 in `quorra`,
01's centre in GTK, 27 twice; 105's page areas 800x534, 609x406, 619x413 against controls 800x1000,
609x1019, 619x997. `t2-toolkit_launch` in one `--clock` hold, HEAD and this tree interleaved twice
at load 3.3 to 3.4: every figure of both windows inside its band in all four passes, exit 0 each
(a first hold refused at once on a stale sandbox worker, trap 10). `tools/state.sh docs`: exit 101,
`raster-gpu`'s `Self::offsets` link, a sibling's; these four crates' rustdoc exit 0. `cargo test -p
conformance`: exit 0 (an earlier run's one failure, `fuzz_workspace`'s `ctutils`, was not this tree's).

**Unfinished.** The WinUI half is compiled, not run. `quorra`'s and `quorra-confined`'s cards are
drawn inside the presentation and are not held (ADR 2309 §3). The C window's title has no
`title_notes`, so a C caller that cannot move sees no title note.
