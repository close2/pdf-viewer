// The rectangle lane: exact analytic coverage for axis-aligned rectangles.
//
// RENDER_LIBRARY.md §6.4: a rectangle is not a path. No tiling, no binning, no edge
// list — one instanced quad per rectangle, and the fragment stage computes the exact
// area of overlap between the rectangle and each pixel's unit cell.
//
// Anti-aliasing by exact pixel-area coverage is a deliberate choice of ours, recorded
// in doc/adr/0005: ISO 32000-2 does not define anti-aliasing (§10.7.4 discusses only
// image interpolation), so the choice is documented as a choice rather than presented
// as normative.
//
// Determinism (§4.6, doc/adr/0005): every operation below is plain IEEE 754 f32
// arithmetic — floor, ceil, min, max, multiply, divide — with no derivatives, no
// subgroup operations, and no ordering dependence. Quad edges land on integers and
// pixel centres on half-integers, so no fragment is ever on a rasterisation tie.

struct Globals {
    // Target size in pixels. Only the vertex stage needs it, for the NDC mapping.
    target_size: vec2<f32>,
    // The device-space corner this attachment's texel (0, 0) is (ADR 0036).
    //
    // A plan renders into a texture the size of *its own* content rather than the
    // target's, so device space and attachment space differ by this. Two places have to
    // agree about it and they are the whole of the mapping: the vertex stage subtracts it
    // before dividing by `target_size`, and the fragment stage adds it back to recover
    // the device pixel it is shading — clip rectangles, tile lookups and masks are all
    // stated in device space and would otherwise be read at the wrong place. Zero for the
    // frame's root, which renders into the target itself.
    origin: vec2<f32>,
}

// Where the active soft mask sits and what it is elsewhere (ADR 0037). Per mask
// rather than per region, which is why it is in this group and not in `Globals`:
// the region is one pass's, and a pass draws batches under different masks.
struct MaskPlace {
    // Device corner of the mask's texel (0, 0) in .xy, its size in texels in .zw.
    rect: vec4f,
    // What the mask holds outside `rect`, in .x: the reduce's output for a
    // transparent pixel, since the mask's group marks nothing out there. A size of
    // (0, 0) is an absent mask, and then this is 1 and admits everything.
    outside: vec4f,
}

@group(0) @binding(0) var<uniform> globals: Globals;
// The active soft mask (§11.5's soft clip), realised at its own plan's rectangle.
// Bindings 0 and 1 of this group belong to the coverage lane's shader; the layout is
// shared so both lanes bind one group.
@group(1) @binding(2) var soft_mask_tex: texture_2d<f32>;
@group(1) @binding(3) var<uniform> mask_place: MaskPlace;

// The soft mask at a device pixel, given where the mask sits (ADR 0037). Identical in
// all seven shaders that sample a mask; WGSL has no include, so the copies are kept
// textually the same, and tests/shader_copies.rs fails the build when they drift. The
// placement is an argument rather than a global because it reaches each lane in a
// different uniform; `soft_mask_tex` is the one name all seven bind it under.
fn soft_mask_value(rect: vec4f, outside: f32, p: vec2f) -> f32 {
    let local = p - rect.xy;
    if any(local < vec2f(0.0)) || any(local >= rect.zw) {
        return outside;
    }
    return textureLoad(soft_mask_tex, vec2i(local), 0).r;
}

// This lane draws batches under different masks, so the placement is its own uniform.
fn soft_mask_at(p: vec2f) -> f32 {
    return soft_mask_value(mask_place.rect, mask_place.outside.x, p);
}

struct Instance {
    // Device-space rectangle: min.xy in .xy, max.xy in .zw. Already transformed and
    // ordered on the CPU (encode.rs), which is what keeps this shader a pure evaluator.
    @location(0) rect: vec4<f32>,
    // Premultiplied device RGB. Premultiplication happened once, at encode time —
    // straight alpha at the boundary, premultiplied internally (§3).
    @location(1) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) rect: vec4<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, instance: Instance) -> VsOut {
    // Four strip vertices; corner in {0,1}².
    let corner = vec2<f32>(f32(vertex_index & 1u), f32(vertex_index >> 1u));
    // The quad covers every pixel the rectangle touches: expanded outward to the
    // enclosing pixel grid so that partially-covered border pixels get fragments.
    let quad_min = floor(instance.rect.xy);
    let quad_max = ceil(instance.rect.zw);
    let pos = mix(quad_min, quad_max, corner);
    // Device pixel space (y down, origin top-left) to NDC (y up). The scene-to-device
    // transform, including the page's y flip, was applied on the CPU; this is only the
    // fixed pixels-to-NDC map.
    let ndc = vec2<f32>(
        (pos.x - globals.origin.x) / globals.target_size.x * 2.0 - 1.0,
        1.0 - (pos.y - globals.origin.y) / globals.target_size.y * 2.0,
    );
    var out: VsOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.rect = instance.rect;
    out.color = instance.color;
    return out;
}

// The element's SHAPE at a pixel: the area of (rect ∩ pixel cell), which is
// §11.6.4.2's object shape antialiased by ADR 0005's area rule, met with §8.5.4's
// clip — "The effective shape is the intersection of the object's intrinsic shape
// with the clipping path". Each extent is in [0, 1] by construction, so the product
// is the exact area — no approximation at corners, where the two partial extents
// multiply.
//
// **Neither the soft mask nor the paint's alpha is in it** (ADR 0066). Table 57's
// alpha source flag governs both together — "whether the current soft mask and alpha
// constant parameters shall be interpreted as shape values ( true ) or opacity values
// ( false )" — and its initial value is `false`. A scene carries no such flag, so both
// are §11.6.4.3's `qm` and §11.6.4.4's `qk`, and both stay out of `f`.
fn shape_at(in: VsOut, px: vec2<f32>) -> f32 {
    let overlap_min = max(in.rect.xy, px);
    let overlap_max = min(in.rect.zw, px + vec2<f32>(1.0, 1.0));
    let extent = max(overlap_max - overlap_min, vec2<f32>(0.0, 0.0));
    return extent.x * extent.y;
}

// The device pixel this fragment shades. `position.xy` is the pixel centre
// (k + 0.5 exactly); floor recovers the pixel's integer corner, and the cell is
// [px, px+1) × [py, py+1).
fn pixel_of(in: VsOut) -> vec2<f32> {
    return floor(in.position.xy) + globals.origin;
}

// The source alpha the mark is drawn with: `f × q` for a mark whose only opacity
// input besides the paint is §11.6.4.3's mask. §11.3.6 sees only the product, which
// is why this and `shape_at` differ only where §11.4.6 reads the two apart.
fn coverage_at(in: VsOut) -> f32 {
    let px = pixel_of(in);
    return shape_at(in, px) * soft_mask_at(px);
}

// Premultiplied source scaled by coverage; the fixed-function blend is
// (ONE, ONE_MINUS_SRC_ALPHA), the premultiplied over operator.
// Where the source is opaque the blend keeps none of the backdrop, so what this hands the target
// is the pixel, and it is handed over as the level nearest it (ADR 2324): RADV exports a
// fragment's colour as a half float rounded toward zero, up to an eighth of a level low, and a
// level survives that export. Where it is not opaque the blend decides the pixel from the
// backdrop and the unrounded source, one rounding as the oracle's compositing takes it; rounding
// the source there too would round twice and move every antialiased edge (ADR 2311 §3).
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let source = in.color * coverage_at(in);
    return select(source, nearest_level(source), source.a >= 1.0);
}

// The level nearest each component, `⌊255 c + ½⌋ / 255` of the exact product (ADR 2311), so a
// component a hair below a half level — `0.7` is 178.499 997 levels — is not carried onto it by the
// `f32` product's rounding. The product is taken in integers (ADR 2324): a component in `0..=1` is
// its 24-bit significand `m` times `2^-s`, and `255 m` fits a `u32`, so `⌊510 c⌋` is one shift and
// the level is `(⌊510 c⌋ + 1) / 2` in integers. An `fma` residual stated the same rule and RADV
// did not honour it (ADR 2324 §2). Five shaders hand the target a level through it, and the copies
// are kept textually the same.
fn nearest_level(c: vec4f) -> vec4f {
    let bits = bitcast<vec4u>(clamp(c, vec4f(0.0), vec4f(1.0)));
    let biased = (bits >> vec4u(23u)) & vec4u(255u);
    let normal = biased != vec4u(0u);
    let significand = (bits & vec4u(0x7fffffu)) | select(vec4u(0u), vec4u(0x800000u), normal);
    let halving = vec4u(149u) - max(biased, vec4u(1u));
    let doubled = select(vec4u(0u), (significand * 255u) >> halving, halving < vec4u(32u));
    return vec4f((doubled + 1u) >> vec4u(1u)) / 255.0;
}

// The element's shape alone, for §11.4.6's knockout erase (ADR 0066). The erase pass
// scales the backdrop by (1 − shape) through the (ZERO, ONE_MINUS_SRC_ALPHA) blend;
// the add pass then deposits shape · element (ADR 0010's two-pass knockout), and that
// deposit *does* carry the mask, because it is the element's own contribution.
@fragment
fn fs_shape(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(shape_at(in, pixel_of(in)));
}
