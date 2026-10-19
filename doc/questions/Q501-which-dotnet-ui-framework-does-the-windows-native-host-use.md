# Q501 — Which .NET UI framework does the Windows-native host use: WinUI 3, WPF or WinForms?

Source: RFC 0010 §3.3 (session 1606), written for the owner's `A287` of 2026-10-19 ("we will add a
windows native (dotnet) version").
Status: **open** — answered when `A501-…` exists beside this file.

## Why it needs the owner

A287 says ".NET" and "windows native". It does not say which of .NET's three Windows UI frameworks.
They differ in what a reader sees, in which Windows versions run the program, in what it ships with,
and in where it can be built. That is a product choice, not something a reading of the tree settles.

## What decides it

The page is drawn by the device, and the form controls are platform controls placed over the page
(`Command::Delegate`, `doc/todo/30`). The frameworks differ on exactly that:

- **WinUI 3**: a `SwapChainPanel` is a surface wgpu 30 targets directly, and XAML controls compose
  over it. It cannot be built on this Linux machine: the XAML compiler is Windows-only, so builds
  run on CI's `windows-latest`. It needs the Windows App SDK at runtime.
- **WPF**: a device surface sits in an `HwndHost`, and no WPF element can be drawn over one. The
  other route, `D3DImage`, needs a D3D9Ex shared surface that wgpu does not make. It compiles on
  Linux with `EnableWindowsTargeting` and runs on Windows only.
- **WinForms**: a child `HWND` for the page, and controls as sibling `HWND`s over it. It works, with
  the flicker and the look of the older toolkit.

## What the tree does meanwhile

Nothing is built. RFC 0010's first steps do not depend on the answer: N0 (the C# declarations and
the Rust test that holds them to `quorra.h`), N1 (three missing commands and the key table) and N2
(the `quorra_window` layer). A round may take those first.

## Recommendation

**WinUI 3.** It is the only one of the three where a device-drawn page and the controls over it
compose without a workaround. That is the property ADR 2049 asks of Qt's `QRhiWidget` and the one a
macOS `NSView` backed by a `CAMetalLayer` has. Its cost, no local build, is what CI's Windows runner
is there for.
