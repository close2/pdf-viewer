# The stack, and what is deliberately not in it

Moved out of `CLAUDE.md` so that the principles file holds only principles. The *rationale*
for most of these — why Rust, why not raw Vulkan, why CPU first, what the image codecs cost —
is `doc/PLAN.md` §1; `doc/crate-map.md` says which crate each choice lives in.

## Stack

| Area | Choice |
|---|---|
| Language | Rust |
| Rasterizer | GPU first, for page one and every page after it; `tiny-skia` as the correctness oracle and as the fallback for a frame the device refuses — behind one trait |
| Fonts | `skrifa` (+ Type1/Type3 handled in-tree) |
| Windowing | `winit` |
| Dialogs | `ashpd` (XDG desktop portal — native KDE dialogs, any toolkit) |
| Accessibility | `AccessKit` (AT-SPI on Linux) |
| Parallelism | `rayon` |
| Deflate | `flate2` with `zlib-rs` backend (pure Rust, ~C speed) |
| Spec model | Arlington PDF Model → generated validation layer |

**Not used:** `rustybuzz`. PDF content streams carry already-positioned glyphs; shaping
them again would move glyphs away from where the document specifies. It may return later,
scoped strictly to text *we* generate (annotations, form fields with non-embedded fonts).
