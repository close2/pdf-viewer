# 1733 — The presentation is turned by one pump, and parks while nobody can see it

Session 1733, slot 6 of batch 112, a host round; ADRs 2300, 2301; no question. §12.2 stays `partial`,
its note rewritten to the WinUI residue (ADR 2301); §12.4.4.1 stays `implemented`, naming the pump.

**Built.** `viewer_host::presentation_pump`: `PresentationPump`, a trait every `NativeWindow` has by
one blanket implementation, over a `Pump` on `WindowState` (script ticker, hidden flag, owed
synchronisation, a rasteriser made at the first transition frame). GTK and Qt arm a one-shot for
`presentation_wait` and call `turn_the_presentation`, losing their own turn, rasteriser and ticker;
the C window turns it in `quorra_window_tick`, so WinUI gets `/Dur`, transitions and script timers.
A hidden window parks the clock and `Clock::resume` counts from its return: GDK's `MINIMIZED`, Qt's
expose, WinUI's `VisibilityChanged` through the new `quorra_window_shown`. GTK and Qt adopt ADR
2200's per-tab parking through `bring_the_presentation_forward`, `synchronize_if_owed` and the new
required `apply_the_chrome`. Drive steps 97 to 99 at the end: a presentation advances; a window
unmapped four seconds still names page 1 and advances once mapped (GTK, Qt); a full-screen deck turns
its slides while a second file opens beside it.

**Premise and hypothesis.** The premise held. The hypothesis held but for step 83: one module (329
lines with tests), three hosts without their own turn; the flakiness was two races, both the
camera's (ADR 2300 §4) — of 32 verdicts on HEAD's binaries at load 20 to 53, six wrong: twice a
camera started after page 1 began saw it under 0.4 s, four times a load-delayed last transition
frame was counted a fourth settled page. The first slide lasts two seconds, near-identical adjacent
settled pictures are one page, and the step waits for the window's line that it presents.

**Gates.** On an export of HEAD plus this round's `-U0` hunks: `rustfmt --check` on every touched
file, `cargo fmt --check`, warnings-denied clippy `--all-targets` and rustdoc for `viewer-host`,
`viewer-ffi`, `viewer-gtk`, `viewer-qt`: exit 0; their `cargo nextest run`: 436 passed, 3 skipped,
exit 0 (the new FFI test fails with `Clock::resume` planted out). `tools/dotnet-check.sh`: exit 0.
Full drive of the four windows: exit 0, 410 works, 6 wrong, 11 not offered — `88-archive-refused`
thrice and `66-signature-policy` in `quorra-confined`, wrong on HEAD's binaries too;
`82-retained-page` in Qt, 3 of 3 works rerun; `99-presentation-behind` in GTK, the step reading a
title with no page yet, fixed, then 6 of 6 works. Step 98 on HEAD's binaries: wrong in GTK and Qt
(control). Step 83 revised: 20 of 20 works at load 5.5 to 7.7. `t2-toolkit_launch` under `--clock`,
HEAD and this tree interleaved twice in one hold: pass one above the load ceiling, not judged; pass
two judged at load 12.0 to 10.6, every figure of both windows inside its band in both, exit 0.
`cargo tree`: the arms' `render-raster` reaches no crate touched here. `tools/state.sh docs`: exit
0. `cargo test -p conformance`: 462 passed, 2 failed, siblings' (record 1728, ADR 2296's file).

**Unfinished.** `quorra`'s winit presentation does not park (ADR 2300 §5); WinUI is compiled, not
run; no key changes tab while presenting, so a switch is witnessed by the FFI tests alone.
