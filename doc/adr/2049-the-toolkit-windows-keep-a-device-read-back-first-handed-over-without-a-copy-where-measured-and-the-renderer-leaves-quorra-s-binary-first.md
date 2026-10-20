# 2049 — The toolkit windows keep a device: read back first, handed over without a copy where measured, and the renderer leaves `quorra`'s binary first

Session 1606, slot 8 of batch ninety-five, a "host" planning round. Status: **accepted as the
order**; nothing in it is built. Context: the owner's `doc/questions/A287` of 2026-10-19 ("If
possible gtk, qt, and dotnet should provide a way to still render on the GPU") and ADR 2048. RFC 0010
§4 applies the same list to the .NET host.

## 1. What the premises said, and what held

- **The GTK limit is ADR 0244's, not ADR 0604's.** ADR 0604 is about `--ignore-restrictions`.
  ADR 0244's Decision 2 is the one that says GTK 4 gives a widget no native surface and hands out
  no device. Both claims are still true of the public API.
- **ADR 0246's reason for Qt is half out of date.** It called `QRhi` "a private module". On this
  machine (Qt 6.12.0), `rhi/qrhi.h` ships with a header that says "part of the RHI API, with
  limited compatibility guarantees", and `QRhiWidget` is a public `QtWidgets` class. Because it is
  a `QWidget`, it composes inside a `QSplitter` like any other, which was the property ADR 0246
  wanted and did not find.
- **The hypothesis, "GTK's device path is a child surface rather than `GLArea`", did not hold as
  stated.** GTK 4.24.1 hands out no child surface. Its one subsurface belongs to
  `GtkGraphicsOffload` (4.14), which takes only a dmabuf texture. So the zero-copy path is a dmabuf
  texture, which GTK may move onto a subsurface, and `GLArea` is a separate, dearer route.

## 2. The one step every device path needs first

**S0 — the device renderer becomes a library.** The device lane exists in `quorra` alone:
`crates/viewer-ui/src/bin/quorra/renderer.rs`, `renderer/` and `surface.rs`, 6 324 lines (`wc -l`)
inside a *binary*. That covers the chooser (ADR 1831), the kept scene (ADR 1886), the presenter
thread and the oversize window. It is tied to winit at two seams only: an `EventLoopProxy` (the
wake) in five files, and `winit::window::Window` in `surface.rs` (the surface). Moved to a library
with a wake callback and a surface argument, it serves GTK, Qt, the C ABI and later macOS. Not
moved, each host writes the chooser again, and the third copy of a function is where hosts stop
agreeing (`doc/todo/30`). **Price**: a mechanical move proved by the multiset of its lines (habit
93), two seams, no pixel moved, about one round. No new `unsafe`.

## 3. The list, priced

Measured in this round's one `--clock` hold on the real adapter, headless, on cpu0–3 with four
rayon threads. The page is ISO 32000-2's printed page 7 and the list and the target are the same for
every lane. The figures are `crates/render-raster/examples/frame_race.rs`'s fastest of twelve,
three runs each, and `first_frame.rs`'s frames 4 to 10. The adapter line, from the same
constructor in the same hold, read "AMD Radeon 890M Graphics (RADV STRIX1) (IntegratedGpu,
Vulkan), cpu coverage lane".

| target | processor (`render-cpu`, today's lane) | device drawn and read back | of which read back | of which passes after the device |
|---|---|---|---|---|
| 596×842 | 3.46–5.35 ms | 2.70–3.27 ms | 0.34–0.43 ms | 1.18–1.21 ms |
| 1132×1601 | 5.27–7.18 ms | 6.70–6.79 ms | 1.13–1.18 ms | 4.03–4.20 ms |

Bring-up is 19.5–21.0 ms and the device's first frame 10.1 / 16.0 ms. The copy into the toolkit,
steady, is 234 µs (GTK) and 231 µs (Qt), which is `doc/ui-boundary.md`'s held figure and was not
taken again. The "passes after the device" are `QuorraRasterizer::rasterize`'s own work on the
read-back raster, mostly the medium compose. A path that presents layers on the device does not pay
it.

| step | what it is | `unsafe` and rules it touches | new graph | saves per frame (above) | platforms | price |
|---|---|---|---|---|---|---|
| **G1 / Q1** (built, ADR 2070; the default lane, ADR 2071) | the device draws through S0's chooser, the frame is read back, and the toolkit is handed a `Raster` as today (`gdk::MemoryTexture`, `QImage`). No boundary message changes: it is `Rendered::Raster` | none | `raster-gpu` and `wgpu` enter `viewer-gtk` and `viewer-qt`; a second device in the process beside GSK's or Qt's own, whose first block is 128 MiB under wgpu's default hints (`crates/render-raster/examples/bring_up.rs`) | 0.6–2.1 ms at 596×842 (per run, device against processor); at 1132×1601, 0.4 ms in one run of three and 1.4 ms *lost* in the other two. It pays where the chooser predicts the device the faster lane | Linux today; any platform the toolkit runs on | one round for both windows, level, after S0 |
| **G2** | wgpu renders into an exportable Vulkan image. Its dmabuf goes to `gdk::DmabufTextureBuilder` (GTK 4.14) and the picture child shows it; `GtkGraphicsOffload` may put it on a subsurface | `viewer-gtk` loses `#![forbid(unsafe_code)]`, because `set_fd` and `build` are `unsafe fn` in gdk4 0.11.4. wgpu 30 has no public export, so `wgpu-hal`, `ash`, `VK_EXT_external_memory_dma_buf` and DRM format modifiers are needed. The feature floor rises from `v4_10` to `v4_14`, which is a runtime requirement (Cargo.toml's own note) | `ash` | read back 0.34–1.18 + passes 1.18–4.20 + copy 0.23 ms | Linux, Vulkan | two to three rounds, an ADR for the lifted rule, a round on the owner's session to see it |
| **G3** | `GtkGLArea` or `GdkGLTextureBuilder` with wgpu's GL backend on GTK's context | `wgpu-hal`'s external GL context is `unsafe`. The area draws on GTK's thread inside its frame clock, against ADR 0668's drawing off the event thread. The compute lane needs GL 4.3, and GL is wgpu's least-served backend | `wgpu-hal` GL | as G2 | Linux, GL | the dearest, and not taken |
| **Q2** | a `QRhiWidget` subclass in `crates/viewer-qt/cpp/` on `QRhiWidget::Api::Vulkan`: wgpu imports Qt's `VkDevice` and draws into the widget's `colorTexture()` | `create_device_from_hal` and `create_texture_from_hal` are `unsafe` and need `ash`. `raster-gpu` needs a constructor from a device it did not make. QRhi's guarantees are "limited". `viewer-qt` keeps its one bridge token and gains a second site | `ash` | as G2 | Linux, Vulkan; Qt's own default is D3D11 on Windows and Metal on macOS | two to three rounds and an ADR |
| **Q3** | `QWidget::createWindowContainer` over a Vulkan `QWindow`, tier 2 by raw handle | a native child window stacks above its sibling widgets, so the controls `Command::Delegate` places over the page would be under it | — | as G2 | — | **refused** by the overlay requirement |

## 4. The decision

1. **S0, then G1 and Q1 together.** The order follows ADR 0509's criterion: a reader gets the
   device lane in both toolkits, which costs no new message and no new `unsafe`, and the two
   windows stay level.
2. **Under G1 and Q1 the toolkit windows follow principle 2's lane rule, unchanged**, when the
   device draws. The device comes up off the event thread and nothing waits for warmth. Past
   `WARN`, the lane predicted faster draws, and the title and the terminal say so. No frame is drawn
   on the processor only to cover the bring-up. Whether the device is the toolkits' *default* lane
   is for G1's round to decide, on the toolkit launch figures with the bring-up in them. This
   record recommends the device, for principle 2's reason.
3. **G2 and Q2 wait for a profile.** Either is taken only after a page shows that the read-back,
   the passes after it and the copy hold the frame's time (habit 59), and only with an ADR for the
   rule it lifts. A third crate's `unsafe` changes a rule the owner stated (`doc/todo/30`). G3 and
   Q3 are not taken.
4. **Where a platform admits no device, the processor stays the lane** and says nothing new. That
   is A287's "if possible".
