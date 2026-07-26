# ADR 0004 — Test the GPU path headlessly; never skip it

Status: accepted, 2026-07-26.

## Context

The GPU backend is the part of the renderer least amenable to automated testing: it
normally needs a window, a display server and a driver. CI runners have none of the
first two and no GPU at all. The project owner also asked that verification be
automatic wherever possible, with manual checking reserved for confirming the real
thing works.

## Decision

Three parts.

**`render-gpu` never creates a window or a surface.** It renders into an offscreen
texture and reads the pixels back. `wgpu::InstanceDescriptor::new_without_display_handle`
declines to acquire a display handle at all, and `request_adapter` is called with
`compatible_surface: None`, so adapter selection never touches windowing. Presentation
belongs to `viewer-ui`, which reuses the same `GpuContext`.

**CI uses a software Vulkan implementation.** `mesa-vulkan-drivers` supplies `lavapipe`,
which runs Vello's compute pipeline on the CPU. `GpuContext::new_headless_software()`
selects it explicitly, so a CI-only rendering difference can be reproduced on a
developer machine instead of being debugged through pushes.

**The GPU tests fail rather than skip when no adapter is present.** A skipped GPU suite
is worse than a failing one: it reports success while verifying nothing, and a missing
driver in CI is exactly the kind of silent regression that survives for months. The
panic message names the package to install.

## Consequences

The entire GPU path — scene translation, clip re-nesting, compute rasterisation, texture
readback, row-padding removal — is covered by `cargo test` with no display. What remains
for manual verification is only whether a window appears and presents correctly.

Verified on the development machine: the offscreen path works under RADV as an
unprivileged user with no X authority, because `/dev/dri/renderD128` is world-accessible
and the render node is all that headless rendering needs.

## Measured findings

**Hardware and software adapters agreed byte-for-byte.** RADV on a Radeon 890M and
`llvmpipe` produced identical output for the basic scene. Vello's compute pipeline has no
driver-dependent fixed-function rasterisation, so this is expected rather than lucky.

This **revises the assumption in `doc/PLAN.md`** that goldens must be per-backend or
tolerance-based on account of driver variance. For the vector path they can be shared.
The claim is pinned by `hardware_and_software_adapters_agree_exactly`, which is written
so that if it ever fails the conclusion is not "the code broke" but "this assumption no
longer holds and goldens must become per-adapter". It has been checked on one vendor and
one scene; text and image rendering may yet differ.

**CPU and GPU agree to within antialiasing noise.** `tiny-skia` against Vello on the
basic scene:

```text
mean error 0.0136/255   worst tile 0.44   differing channels 0.08%   max 28
```

The cross-backend gates were set from these measurements — an order of magnitude above
the observed values, and far below the ~150 worst-tile error a single missing shape
produces. Thresholds derived from measurement rather than guesswork, per `CLAUDE.md`
principle 2.

## Note on the display-list design

The two backends want opposite clip representations: `tiny-skia` takes a flat coverage
mask per clip, Vello takes a pushed and popped layer stack. `render-gpu` therefore
re-nests the flat display list, tracking the open chain and diffing it per command.

That both backends must translate is evidence the flat form is the right neutral
representation. Had the display list been shaped like either library's model, the other
backend would have paid for it — and, more importantly, the two would no longer consume
identical input, which is the entire basis for using one to validate the other.
