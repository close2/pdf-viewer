# 1658 — The marks and the controls reach the WinUI window, and Control and the wheel cross

Session 1658, slot 4 of batch one hundred and two, a "host" round; 2026-10-21. RFC 0010 section 6's
N3a, second step; ADRs 2152 and 2153. No Q file: no owner's word was needed. No ledger row moved.

**What CI's log said: nothing yet.** `gh run list --workflow ci.yml` ends at run 39488089669, on
`9da470de`; `127b5dd3`, which added the `window` job, is not pushed, so no `window` run exists and
nothing was fixed from a log. That run's `build (Windows)` and Windows snapshot are green with
`viewer-host` — `ring` under it — built by MSVC: 1650's missing `lib.exe` is this machine's.

**Built.** `quorra_window_zoom_wheel` beside `quorra_window_press`, with `QUORRA_WHEEL_LINES` and
`_PIXELS` (`window/wheel.rs`, `Window::wheel_zoom`): travel in the toolkit's unit, the fraction of a
step carried by the window's `viewer_host::ZoomWheel` (ADRs 1118, 2152), declared in C and C#,
counted, and driven by `drive_a_window.c`. In `Quorra.App` (871 lines added): Control and the wheel;
three layers in the page area; the selection, the find bar's occurrences, Annex O's highlight and
§12.5.1's ring drawn on every repaint in `UISettings`' highlight and accent at `viewer-qt`'s weights;
`Controls.cs`, a control per widget on `CONTROLS`, committed as the toolkit windows commit them and
moved when the pictures move (ADR 2153). ADR 2137's table stands: no declined record is answered.

**Found.** The window opens with `WidgetAppearances::Delegated`, so in step one a field's value was
on no layer: the page leaves the widget out for a control nobody built, and `CONTROLS` was only
logged (trap 5). Every `REPAINT` copies every picture's pixels again, a drag's included.

**Unfinished; what the next run judges.** No C# was compiled. The first `window` run judges both
steps: the XAML, the unpackaged Windows App SDK, `UISettings.UIElementColor`, `ComboBox.
TextSubmitted`, `ListBox`, viewer-ffi's MSVC build; `interop` judges the new declarations. Comb
cells, the file chooser, a control's measured fit, §12.9's path and the popups are not built.

**Premise, hypothesis, shared files.** The premise's log does not exist yet; the zoom was one entry
point and about twenty lines of C#, as hypothesised. `abi.rs`, `quorra.h`, `lib.rs`, the count tests
and `Quorra.Interop` are slot 6's too: my count is 341; slot 6's four bring `abi.rs` to 345.

**Gates.** `cargo nextest run -p viewer-ffi`: 0, 58 of 58, both C programs among them (an earlier
run hit "Text file busy", a neighbour building the same program). `cargo fmt --check -p viewer-ffi`:
0. `cargo clippy -p viewer-ffi --all-targets --no-deps`, `-D warnings`: 0 (without `--no-deps` it
stops at slot 1's `described.rs` mid-edit). `tools/state.sh --round 1658 hosts`: 0. The workflow:
parsed by PyYAML, eleven jobs. A scratch bracket-and-XML check over the C#, XAML and project files:
30 files, 0 problems, calibrated by a planted `}` and `--` comment; every `Native.` name used is
declared. `cargo test -p conformance --no-fail-fast`: 101, only `round_numbers`, on 1655's and
1657's scratch examples. No toolkit window's graph holds `viewer-ffi`, so no drive moved.
