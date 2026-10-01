# 1318 — The four pages raster refused before the scene, and the page past the side limit

Batch forty-nine, robustness slot. ADRs 1471, 1472. `doc/QUORRA_FEEDBACK.md` section 60.

## What the four refusals were

All four were one construction at two scopes, and section 43 of `doc/QUORRA_FEEDBACK.md` had asked
raster for vocabulary to cover them. Three are an isolated group compositing in a blending colour
space of its own (§11.6.6, §11.7.2): four components in `bug1721218_reduced`, three CIE-based in
`issue16742` and `issue5044`. The fourth, `issue21346`, is a luminosity mask whose `Y` is an sRGB
profile's own (§11.5.3). Each ends in a function of the composited pixel.

## What was built

No scene vocabulary. Every such group is isolated, so §11.4.5 makes its result depend only on its
own elements. `render-raster`'s `scene/own_space.rs` draws it as a frame of its own on the same
device, resolves it with `pdf_render`'s shared arithmetic and places it back through
`Paint::Mesh`, inside a `GroupSpec` or as the body of an alpha mask (ADR 1471). raster-gpu is
unchanged, so 1316's crate was not touched. `REFUSED_BEFORE_THE_SCENE` is empty, and all four
pages agree at 1× and 4×. Seven closed-form fixtures (`tests/own_space_groups.rs`) replace the
five refusal tests.

`issue19517` (12 608 × 16 806 at 1×) is drawn in 4 096-pixel tiles stitched before the readback
passes (ADR 1472) and agrees. The reason: the window never asks for the frame, but
`Rasterizer::rasterize` promises the whole target, and the gate asks for exactly that.

## Found

- `bug1721218_reduced` costs about 3.7 s of raster `encode: geometry` per frame, whatever the
  coverage lane (3 518 clipped shadings), against 84 ms on the CPU backend. The cost is the same
  with the group drawn plain, so it is raster's; reported upstream.
- `issue1905` is refused at 1× by the in-progress raster-gpu diff (274 689 454 bytes against the
  268 435 456 budget, where 252 594 693 fit before). It does not reach this round's code. Relayed
  to 1316.
- The Type 3 cycle's fallback is said only on the terminal. `quorra` prints a `note:` line,
  `quorra-confined` an `eprintln!`, and GTK and Qt do not draw through raster at all. No window
  shows it. This was read from the code, not driven under Xvfb.
- The corpus walk at 1× now peaks at 10.7 GiB, because `issue19517` is compared whole.
