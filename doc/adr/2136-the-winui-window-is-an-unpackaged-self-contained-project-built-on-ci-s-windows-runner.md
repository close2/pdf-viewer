# 2136 — The WinUI window is an unpackaged, self-contained project built on CI's Windows runner

Session 1650, RFC 0010 section 6's step N3a, first part. Status: **accepted**; the project and its job
are written and the workflow parsed, and **nothing here has been compiled**: this machine has no
Windows and no .NET SDK, so the first `window` run is the project's first compilation. Context: the
owner's `doc/questions/A501` (WinUI 3), ADRs 0247, 2024, 2054, 2073, 2088, 2104, 2120.

## 1. Decisions

1. **`hosts/windows/Quorra.App/`, beside `Quorra.Interop`, referencing it.** The declarations stay
   `internal` (`Native.cs`: a pointer-level surface is not what a window programs against), and
   `Quorra.Interop.csproj` names the window as the one assembly that sees them
   (`InternalsVisibleTo`). Inside the window, `QuorraWindow.cs` is the only file that calls them: each
   method is one entry point, its status checked and its strings and pixels copied out before it
   returns, so nothing the library owns outlives the call that lent it (ADR 0247 section 3).
2. **Unpackaged and self-contained** (`WindowsPackageType=None`, `WindowsAppSDKSelfContained`): the
   build output runs as it stands, with no MSIX to install and no Windows App SDK runtime to fetch.
   Packaging, with the workers beside the executable, is N4's.
3. **Windows App SDK 2.5.1 and SDK build tools 10.0.26100.9169**, exact, each published more than two
   weeks before this session (NuGet: 2026-09-16 and 2026-08-26). 2.x is the line in force beside
   1.8's servicing; the 2.0.1 release notes, the ones read, list additions and name no removal of
   what this window uses, and the first run is what says whether that held through 2.5.
4. **`net10.0-windows10.0.19041.0`, x64 only**: `Quorra.Interop`'s release, and the one platform
   `windows-latest` builds `viewer_ffi.dll` for.
5. **The processor lane**: each picture `quorra_window_frame` hands over is copied into a
   `WriteableBitmap` in an `Image` on a `Canvas`, at the origin and the shown extent the window states,
   converted from straight RGBA to the premultiplied BGRA the bitmap holds, rounded to nearest. The
   window is told the pixels are on the screen from the next `CompositionTarget.Rendered`, the frame
   that shows them, not from the line that set them.
6. **The loop is quorra.h's**: a resize, a key, the pointer, the wheel and the timer, each one call
   and then every owed batch until one is empty (bounded at 64, said past it), then the timer armed
   at `quorra_window_interval`. Command-line words go through `quorra_settings_take`, so this window
   takes the other windows' words with their refusals; further documents open beside once the first
   resize has joined the first.
7. **Keys by virtual key.** A letter's virtual key is already the letter whatever Shift says, which
   is the folding the header asks for. The three punctuation keys VirtualKey does not name are taken
   at the US layout's places (`VK_OEM_PLUS`, `VK_OEM_MINUS`, `VK_OEM_2`), a documented choice of this
   step; the number pad's Add, Subtract and Divide reach the same keys on any layout.
8. **A `window` job on `windows-latest`** builds `viewer_ffi.dll` with `cargo build --release -p
   viewer-ffi`, which no other job builds for Windows, then the window, then checks that the library
   was copied beside it. Its ceiling, 45 minutes, is priced as ADR 2073's first was: a healthy run is
   put near twenty minutes against `platforms`' Windows row (4–13 min, warm cache, two debug
   executables of a larger graph).
9. **`interop`'s ceiling is restated at two minutes**: its healthy runs on main took 24 s and 26 s,
   twice the longer is 52 s, and the unit is the minute; the second minute is kept because the
   sample is two runs and the setup step alone ranged from 1 s to 12 s.

## 2. What only CI can judge, and what the first run will tell

Whether the XAML compiles; whether `dotnet build` drives the Windows App SDK's targets unpackaged
with no `Package.appxmanifest`; whether the generated code is clean under `TreatWarningsAsErrors`
and nullable; whether the WinRT names this file assumes (`InputKeyboardSource`,
`InputSystemCursor`, `WindowsRuntimeBufferExtensions.CopyTo`, `CompositionTarget.Rendered`) are
where it assumes them; whether `viewer-ffi` builds for `x86_64-pc-windows-msvc` at all (`ring`'s
build script needs MSVC's `lib.exe`, so `cargo check --target` here stops before it). A window
**drawn** is seen on a Windows machine only; no job runs it.

## 3. Consequences

The window shows the page, its title and its status line, and takes the keys, the pointer and the
wheel. The marks over the page, the controls, the panels and the cards are N3a's later rounds;
ADR 2137 says what this window does with a record it cannot answer meanwhile. Ctrl and the wheel
magnify in the other windows and reach no entry point of `quorra_window`: said, and owed to the ABI.
