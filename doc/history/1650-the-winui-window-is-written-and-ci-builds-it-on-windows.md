# 1650 — The WinUI window is written, and CI builds it on Windows

Session 1650, slot 4 of batch one hundred and one, a "host" round; 2026-10-20. RFC 0010 section 6's
N3a, first step; ADRs 2136 and 2137. No Q file: no owner's word was needed. No ledger row moved.

**Built.** `hosts/windows/Quorra.App/`: an unpackaged, self-contained WinUI 3 project (Windows App
SDK 2.5.1, `net10.0-windows10.0.19041.0`, x64) referencing `Quorra.Interop`, whose csproj now names
it in `InternalsVisibleTo`. `QuorraWindow.cs` is the one file calling the declarations; `PageArea`
draws each of `quorra_window`'s pictures into an `Image` (straight RGBA to premultiplied BGRA) and
tells the window they are shown from the next `CompositionTarget.Rendered`; the title, the status
line, the keys (`Keys.cs`), the pointer, the wheel, the timer and the command-line words; the five
records that wait for a dialogue are declined in the header's own terms and said (ADR 2137). CI's
`window` job on `windows-latest` builds `viewer_ffi.dll`, the window, and checks the library landed
beside it, under a 45-minute ceiling priced as an estimate. `interop`'s ceiling: 15 to 2 minutes,
from its two healthy runs, 24 s and 26 s (`gh run list`, runs 39388089315, 39414723133).

**The premise and the hypothesis.** The premise held: `interop` has healthy runs now, two. The
hypothesis did not hold in its shape: the window hands over several pictures (frames and stand-ins),
so the area is a `Canvas` of `Image`s; 1 214 lines over ten files, the code-behind 426, the calls
in an owned layer of their own; the job 33 lines. The dozen P/Invoke details are CI's to find.

**Found.** `--release` in a csproj XML comment (XML forbids `--` there), caught by a scratch
bracket-and-XML check, calibrated by planting a stray `}` and a `--` comment, both named.
`quorra_window` has no entry point for Ctrl and the wheel, which magnify in the other windows: said,
and in `doc/todo/30`. `cargo check --target x86_64-pc-windows-msvc -p viewer-ffi` stops at `ring`'s
build script here (no `lib.exe`): no job has compiled `viewer-ffi` for Windows; `window` is the first.

**Unfinished, and what CI alone can judge.** Nothing here was compiled: no Windows, no `dotnet`. The
first `window` run says whether the XAML compiles, whether `dotnet build` drives the Windows App SDK
unpackaged with no package manifest, whether the generated code is clean under warnings-as-errors,
whether the WinRT names assumed are where assumed, and whether `viewer-ffi` builds with MSVC. No job
runs the window; a page drawn is seen on Windows only. Next: the marks, controls, panels and cards.

**Gates.** `cargo test -p viewer-ffi --test header_and_interop_agree`: 0, 4 of 4 (two earlier
attempts stopped at neighbours mid-edit in `pdf-transform`). The workflow: parsed by PyYAML, eleven
jobs, `window` among them. `cargo test -p conformance --no-fail-fast`: 101, 449 passed and 2 failed,
both a neighbour's (`batch`'s install list lacks `quorra-transform-worker`; `worker_features` finds
`bytemuck`'s features in the shipped worker), neither reading a file of this round. The scratch
check: 11 files, 0 problems. No Rust file was touched, so no crate's tier 1 is owed and no window's
graph moved. Duration: about 1 400 s.
