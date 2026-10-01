# 1471 — A group resolved after it composites is drawn as its own frame and placed back as device pixels

Session 1318. Status: accepted. Code: `crates/render-raster/src/scene/own_space.rs`,
`crates/render-raster/src/scene.rs` (`refuse_untranslatable_group`, `Encoder::group`,
`Encoder::mask_id`). Tests: `crates/render-raster/tests/own_space_groups.rs`. Answers
`doc/QUORRA_FEEDBACK.md` section 43 on this side.

## 1. The question

Four pages of `doc/pdf.js` were refused by `render-raster` before a scene was built
(`REFUSED_BEFORE_THE_SCENE`): an isolated group compositing in a blending colour space of its own
(§11.6.6, §11.7.2) — four components in `bug1721218_reduced.pdf`, three CIE-based components in
`issue16742.pdf` and `issue5044.pdf` — and a luminosity soft mask whose `Y` is an sRGB profile's
own (§11.5.3) in `issue21346.pdf`. Each ends in a function of the *composited* pixel: §11.7.2's
"[t]he resulting colours shall then be interpreted in the group's colour space when the group is
subsequently composited with its backdrop", and §11.5.3's `Y` of the group composited onto its
backdrop. Section 43 asked raster for scene vocabulary (curves around an N-axis grid on a
`GroupSpec`, the same beside a luminosity mask, and a second body for the four-component pair).

## 2. The decision

No new scene vocabulary. Each such group is drawn by `render-raster` as a **frame of its own** on
the same device — a second `Encoder` over the same list, caches and transient list — read back,
resolved by the shared `pdf_render` functions (`resolve_blending`, `resolve_grey`, `resolve_cube`,
`SoftMask::values`, `SoftMask::paired_values`), uploaded as a device-pixel raster
(`raster_scene::MeshSpec` at the target's origin) and placed back by one fill of the target's
rectangle with `Paint::Mesh`:

- a group: inside a `GroupSpec` carrying the original's alpha, blend mode, clip, mask and
  compositing operator, `knockout: false` (the elements already composited with each other);
- a mask: as `MaskKind::Alpha` whose body's alpha is the mask value, with no transfer table (the
  shared reduction has applied it).

The scene that carries such a raster is marked as having read the view (ADR 0702), so the window
rebuilds it at the next placement rather than replaying it.

## 3. Why it is exact, not an approximation

§11.4.5 composites an isolated group's elements onto "a fully transparent initial backdrop", so
its result is a function of its own elements alone. Every group carrying a space of its own is
isolated (`pdf_render::GroupBlending`'s guarantee; the non-isolated combination stays refused, as
on the CPU backend), and a mask group is drawn on its own by §11.6.5.1. A separate frame therefore
holds the same pixels the group would hold inside the scene; this is not a read-back of a scene
under composition, which raster rightly has no door for. The resolution is the arithmetic the CPU
backend runs over its own buffers, so the two backends cannot answer the clause apart (trap 2).

## 4. Cost, measured

One extra frame per such group, two for a four-component one, at the target's size.
`issue16742`, `issue5044`, `issue21346`: 2-5 ms through raster. `bug1721218_reduced.pdf`: about
10 s, of which each of the two frames is about 3.8 s of raster's *geometry* encode — and the same
page drawn with the group as a plain group pays the same 3.7 s (3 518 clipped shadings, 14 192
outlines), so the cost is raster's on this content and not this construction's; it is reported
upstream (section 60 of `doc/QUORRA_FEEDBACK.md`). The CPU backend draws the page in 84 ms.

## 5. Consequences

`REFUSED_BEFORE_THE_SCENE` is empty; the four pages agree with the oracle at 1× and 4×. The
refusal tests in `headless_quorra.rs` and `mask_pair_refusal.rs` become closed-form fixtures in
`own_space_groups.rs`, whose expected values are worked from §11.7.2, §11.5.3 and the fixtures'
own numbers, not from `render-cpu`. Section 43's vocabulary remains a better answer if raster
grows it — it would cost no extra frame and survive a view change — and nothing here forecloses it.
