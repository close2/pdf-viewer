# 1606 — The reference stays winit, a .NET host joins, and the toolkits keep a device

Session 1606, slot 8 of batch ninety-five, a "host" planning round; 2026-10-19. The owner's
`doc/questions/A287`. ADRs 2048 (supersedes ADR 0001) and 2049; RFC 0010; `doc/questions/Q501`.
Nothing under `crates/` was built.

**Written.** ADR 2048 answers ADR 0001's revisit condition, which had fired without a record. It
states where the tree depends on Qt and GTK, retires the "little conventional GUI" and `ashpd`
premises, and turns "do not depend on Qt" into a rule about the graphs of `viewer-ui` and
`viewer-ffi`, with the `cargo tree -i` command that checks it. ADR 2049 is the priced list for the
device paths. RFC 0010 proposes the .NET host on the C ABI: the eight missing commands priced, a
`quorra_window` layer over `NativeWindow`, and pixels first, then a WinUI 3 `SwapChainPanel`, with
macOS seams kept in mind. Q501 asks the owner which UI framework. Also edited: `doc/ui-boundary.md`
(a section), `doc/todo/30` (the key table moved from decided to owed), `doc/rfc/README.md` (the
index), and ADR 0001's status line.

**Premises that did not hold.** The GTK limit is ADR 0244's, not ADR 0604's: ADR 0604 is about
`--ignore-restrictions`. The device crate is `raster-gpu`, but the device *renderer* sits in
`quorra`'s binary (6 324 lines), so no other host can reach it yet. ADR 0246's "QRhi is private" is
out of date: Qt 6.12 ships `rhi/qrhi.h` with "limited compatibility guarantees", and `QRhiWidget`
is public. **The hypothesis held for .NET** (P/Invoke, wgpu's DX12 target, though as a
`SwapChainPanel` and not a bare swapchain). **It did not hold for GTK**: there is no child surface,
and the zero-copy path is a dmabuf texture. The brief dates A287 2026-10-20, but the file says
2026-10-19.

**Measured** (one `--clock` hold, Radeon 890M headless, cpu0–3, page 7 of ISO 32000-2): the
processor took 3.46–5.35 ms and the device read back 2.70–3.27 ms at 596×842. At 1132×1601 they
took 5.27–7.18 ms and 6.70–6.79 ms. The read-back itself was 0.34–1.18 ms and bring-up 19.5–21.0 ms.
`cargo tree --target x86_64-pc-windows-msvc`: every package `viewer-ffi` depends on is in `quorra`'s
Windows graph (145 of 226), which CI already builds there. A local `cargo check` for that target
stops at `cc` needing `lib.exe`.

**Unfinished.** RFC 0010's N0 is its first buildable step. ADR 2049's S0 comes before every device
path. No conformance test yet runs ADR 2048's `cargo tree -i` rule. Found, not fixed:
`crates/viewer-ui/src/lib.rs`'s module comment still says its dialogs go through `ashpd`.

**Gates.** `cargo test -p conformance`: 0. `--bin pointers`: 0, nothing naming these files. `--bin
quotations`: 0, the diverging count unchanged at 47. `tools/state.sh --round 1606 docs`: 0. No crate
was touched, so no tier-1 crate gate applies. Duration: about 1 800 s.
