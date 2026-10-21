# 1666 — The repaint says what changed, and the WinUI controls are measured and combed

Session 1666, slot 4 of batch one hundred and three, a "host" round; 2026-10-21. RFC 0010 section
6's N3a, third step; ADRs 2168 and 2169. No Q file: no owner's word was needed. No ledger row moved.

**What CI's log said: nothing yet.** `gh run list --workflow ci.yml` still ends at run 39488089669
on `9da470de`, whose jobs have no `window`; `127b5dd3` and `66c6ef23` are not pushed. **The premise
"this machine cannot compile the C#" did not hold**: a .NET 10.0.402 SDK unpacked into this round's
scratch compiles both projects against the Windows App SDK 2.5.1 references (XAML stubbed, PRI
off). It found two `CS1061` in `PageArea.Show` — its parameter `pictures` hid the canvas field —
which the first `window` run would have stopped at; fixed. A planted `CS1061` was named.

**Built.** `QUORRA_OWED_REPAINT` carries `QUORRA_REPAINT_FRAMES`, `_PLACES`, `_MARKS` and a fresh
flag per picture (`quorra_owed_repaint`, `_repaint_fresh`), compared against what the last repaint
told — a SipHash-1-3 digest of each picture's bytes under the window's key — so a call that changed
nothing owes none (ADR 2168); the C# copies only fresh pictures and reads marks only on `MARKS`.
`quorra_window_traced_path` and `quorra_window_controls_measured` (ADR 2169): `w` is now the
window's, from `viewer_host::ControlFit`. In C#: Table 231's comb cells, rich text as its plain
value, the measure, §12.5.6.14's popups in `Notes.cs`, §12.9's path as a polyline.

**Found.** A measuring press and the `m` toggle dispatched nothing, so the C window owed no repaint
for the traced path; both now repaint. ADR 2137's five rows stand; `POPUPS` leaves its log-only list.

**Unfinished; what the next run judges.** The XAML compiler, PRI and MSIX tooling, the
self-contained runtime and `viewer_ffi.dll`'s MSVC build: CI's `window` job alone. A popup's
retyping, a rich note's runs, the chooser (N4), the panels and the cards are not built.

**Premise, hypothesis, shared files.** The log does not exist; no WinRT name was wrong, one C# name
was. The changed-set is a word and a list, not one bit. Shared with slot 6: `window.rs` (the
`repaint` body, the measuring press and toggle, `w`'s arm), `abi.rs`, `lib.rs`, `quorra.h` (window
section only), `drive_a_window.c`, the three count tests, `doc/todo/30`, `doc/state-of-play.md`.

**Gates.** `cargo nextest run -p viewer-ffi`: 0, 59 of 59, both C programs among them;
`header_and_interop_agree` green. `cargo fmt --check -p viewer-ffi`: 0. `cargo clippy -p viewer-ffi
--all-targets --no-deps`, `-D warnings`: 0. Planted "owe every drain": the window test fails. The
C# compile: 0 errors, 0 warnings (`TreatWarningsAsErrors`). Five project files XML-parsed, 0
refused, a planted `--` refused; the workflow parsed, eleven jobs. `tools/state.sh --round 1666
hosts`: 0, 349 entry points. `cargo test -p conformance`: 101, only the ledger test, on slot 3's
ADR 2167 not yet on disk. viewer-ffi's rustdoc: 46 unresolved links, none in this round's lines.
