Status: complete
Given: 2026-10-20, in conversation — transcribed by the round
Owes: none — the choice rides A287's owed Windows-native build, and RFC 0010's first steps (N0, N1, N2) proceed whatever the answer was

> I have executed the sudo command and agree with your recommendation.  Please push and answer both.

Reading: the recommendation adopted is the question's own, quoted and attributed: *"WinUI 3. It
is the only one of the three where a device-drawn page and the controls over it compose without
a workaround."*

What the answer settles: the Windows-native host A287 owes is a **WinUI 3** application — a
`SwapChainPanel` the device targets directly, XAML controls composing over it, which is the
property ADR 2049 asks of Qt's `QRhiWidget` and the one a macOS `NSView` backed by a
`CAMetalLayer` has, so the choice keeps A287's macOS clause in mind. The costs are named with
it: the XAML compiler is Windows-only, so builds run on CI's `windows-latest`, and the Windows
App SDK ships with it at runtime. **WPF** (a device surface in an `HwndHost` nothing draws over,
and a `D3DImage` route needing a D3D9Ex surface wgpu does not make) and **WinForms** (a child
`HWND` with sibling controls, with the flicker of the older toolkit) are declined.