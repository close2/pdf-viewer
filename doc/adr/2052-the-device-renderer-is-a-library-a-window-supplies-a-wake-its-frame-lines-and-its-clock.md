# 2052 — The device renderer is a library, and a window supplies a wake, its frame lines and its clock

Session 1608, slot 2 of batch ninety-six, a "pixels" refactoring round. Status: **accepted** and
**built** as `crates/viewer-device/`. Context: ADR 2049 §2's S0, planned from the owner's
`doc/questions/A287`; ADR 2048's rule that no host-side type names a platform. Clauses: none; no
behaviour moves. ADR 2053 is the rule this move had to restate.

## 1. What the premise said, and what held

ADR 2049 counted the renderer's ties to `quorra`'s binary as two, both winit's: an
`EventLoopProxy` and `winit::window::Window`. **Neither half held.** `renderer/` names no
`winit::window::Window` at all — `QuorraWindowRenderer::new` already takes any
`wgpu::SurfaceTarget`, so the surface is the host's to make — and the proxy was one of five ties.
The other four are the binary's own modules: the stand-in vocabulary the render thread's `Job` is
written in (`stale`'s `Placed`, `Picture`, `Proxies`, `Retained`), `--coverage`'s
`CoverageChoice`, `surface.rs`'s lane decisions, and `viewer_ui::software`'s composition, which the
processor's half of a frame draws with. `viewer-ui` is the binary's own package, and a library
depending on it would be a cycle, so the composition had to come down too.

## 2. What moved, and where

| from | to | what |
|---|---|---|
| `bin/quorra/renderer.rs`, `renderer/` | `src/renderer.rs`, `renderer/` | the render thread, the presenter's half, the chooser |
| `bin/quorra/stale.rs`, `stale/` | `src/stale.rs`, `stale/` | `doc/todo/37`'s stand-in policy, both windows' |
| `surface.rs`'s lane half, `arguments.rs`'s `CoverageChoice` | `src/lane.rs` | which coverage lane, what a swapchain state means |
| `viewer-ui/src/software.rs` from its line 168 | `src/compose.rs` | Table 29's arrangement composed on the processor |

**A crate of its own, not a module of `viewer-host`.** `viewer-host`'s graph is 186 packages and
`viewer-device`'s 249 (`cargo tree -e normal`); the 63 between them are `wgpu`, `naga`, `ash` and
their kind. As a module of `viewer-host` they would enter `viewer-ffi`, `viewer-gtk`, `viewer-qt`
and `viewer-confined`, and the last is the confined worker's, which ADR 2053 forbids. A host takes a
device by naming this crate, which is also RFC 0010 §8's "one library" answered for the C ABI.

`viewer_ui::software` re-exports the composition and keeps `SoftwareError`, which takes each of
`ComposeError`'s three refusals as its variant of the same name and sentence. `quorra`'s root names
`viewer_device::{renderer, stale}`, so every `crate::renderer` and `crate::stale` in the binary reads
as it did.

## 3. What a window supplies

Three things, none a toolkit's: a `Wake` (a closure the render thread calls when a frame or a look
lands, ADR 1926; cloned rather than shared, because winit's proxy is not `Sync` on every platform),
`FrameLines` (where the stand-in policy's three `--trace frames` lines go) and `FrameClock` (what
`MustFollow` arms). The tagged `void *` of ADR 2048 and RFC 0010's N3b is the C ABI's shape and stays
in `abi.rs`; a Rust host hands `QuorraWindowRenderer` its `SurfaceTarget`, and no `unsafe` is added.

## 4. The proof that nothing moved

The multiset of lines (habit 93) of HEAD's 35 files against the 38 after it, under the move's
declared rewrites (paths, `pub(crate)` to `pub`, `SoftwareError` to `ComposeError` in the moved
half): every line on one side only is a `#[path]` removed, a seam's rewrite or its `rustfmt`
reflow, a new item or a doc. Two doc blocks went back to the functions they describe
(`coverage_for`'s, `Stale::plan`'s), which a multiset cannot see and moves no line. The record has
the counts and the gates.
