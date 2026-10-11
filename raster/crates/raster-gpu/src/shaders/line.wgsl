// The line lane: a one-segment stroke's exact area in each pixel (raster ADR 0097).
//
// ISO 32000-2 §8.4.3.2 makes a stroke the set of points within half the line width of the
// path, and §8.4.3.3's Table 53 states the ends of an open segment. Under a transform that
// keeps the axes, the disc of half the width maps to an ellipse whose semi-axes `pen` lie
// along the device axes; dividing x and y by them makes the pen the unit disc, keeps a pixel
// an axis-aligned box, and makes the set an ordinary one in any direction: the band of
// half-width one around the segment — squared off at the endpoints, or carried one further
// past them for a projecting end — and, for a round end, the half of the unit disc beyond
// each endpoint. The host writes the endpoints, the pen, the cap and the rectangle the clip
// admits; this shader is a pure evaluator of the set's area in each pixel's unit cell, as
// rect.wgsl is of a rectangle's (ADR 0005's exact-area rule).
//
// The band's area in a box is the shoelace area of the band clipped to the box. A half disc's
// is the unit disc inside the box cut by the endpoint's square line, summed edge by edge of
// that convex polygon as a triangle where the edge is inside the disc and a circular sector
// where it is outside. The pieces overlap in no area — each half disc lies beyond its own
// end of the band — so the cell's coverage is their sum, times the pen's two semi-axes.
//
// Determinism (ADR 0082): `sqrt`, `atan2` and `normalize` are the operations whose precision
// WGSL leaves to the implementation, so a line's bytes are the same on one adapter frame
// after frame and close, not identical, across adapters — the relaxed contract's case.

struct Globals {
    // Target size in pixels, for the NDC mapping.
    target_size: vec2<f32>,
    // The device-space corner this attachment's texel (0, 0) is (ADR 0036); rect.wgsl
    // states the mapping both stages share.
    origin: vec2<f32>,
}

// Where the active soft mask sits and what it is elsewhere (ADR 0037).
struct MaskPlace {
    // Device corner of the mask's texel (0, 0) in .xy, its size in texels in .zw.
    rect: vec4f,
    // What the mask holds outside `rect`, in .x.
    outside: vec4f,
}

@group(0) @binding(0) var<uniform> globals: Globals;
// The lane bind group the rectangle and coverage lanes share; this lane reads its mask.
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

const HALF_PI: f32 = 1.5707963267948966;
// The margin a quad is grown by so that every pixel whose cell meets the ink has its centre
// inside it: a cell's centre is within half a pixel of the cell in each axis, so within
// √2/2 of any point of it.
const MARGIN: f32 = 0.7072;

struct Instance {
    // The segment's two device endpoints: from in .xy, to in .zw.
    @location(0) ends: vec4<f32>,
    // The pen's semi-axes along device x and y in .xy, and Table 53's end in .z:
    // 0 a butt, 1 a round, 2 a projecting end.
    @location(1) pen: vec3<f32>,
    // Premultiplied device RGB, premultiplied once at encode time.
    @location(2) color: vec4<f32>,
    // The rectangle the clip and the target admit, min in .xy and max in .zw.
    @location(3) admit: vec4<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) ends: vec4<f32>,
    @location(1) @interpolate(flat) pen: vec3<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) admit: vec4<f32>,
}

// The segment's direction in the unit-pen frame, of length one.
fn direction(ends: vec4<f32>, pen: vec2<f32>) -> vec2<f32> {
    return normalize((ends.zw - ends.xy) / pen);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, instance: Instance) -> VsOut {
    let corner = vec2<f32>(f32(vertex_index & 1u), f32(vertex_index >> 1u));
    // The ink lies in the band from one pen-radius before `from` to one past `to`, one
    // radius either side, in the unit-pen frame: a rectangle there, a parallelogram U × V
    // in the device. Each pair of its sides is moved MARGIN apart in the device, which in
    // the parallelogram's own coordinates is MARGIN times the other side's length over the
    // area — so the quad holds every cell centre within MARGIN of the ink.
    let pen = instance.pen.xy;
    let d = direction(instance.ends, pen);
    let n = vec2<f32>(-d.y, d.x);
    let origin = instance.ends.xy + (-d - n) * pen;
    let u = (instance.ends.zw - instance.ends.xy) + 2.0 * d * pen;
    let v = 2.0 * n * pen;
    let area = abs(u.x * v.y - u.y * v.x);
    let grow = vec2<f32>(MARGIN * length(v), MARGIN * length(u)) / area;
    let st = mix(-grow, vec2<f32>(1.0) + grow, corner);
    let pos = origin + st.x * u + st.y * v;
    let ndc = vec2<f32>(
        (pos.x - globals.origin.x) / globals.target_size.x * 2.0 - 1.0,
        1.0 - (pos.y - globals.origin.y) / globals.target_size.y * 2.0,
    );
    var out: VsOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.ends = instance.ends;
    out.pen = instance.pen;
    out.color = instance.color;
    out.admit = instance.admit;
    return out;
}

// A convex polygon of at most eight vertices: a quadrilateral cut by four lines gains at
// most four.
struct Polygon {
    v: array<vec2<f32>, 8>,
    count: u32,
}

// `p` less everything where dot(normal, x) < offset (Sutherland–Hodgman, one line).
fn cut(p: Polygon, normal: vec2<f32>, offset: f32) -> Polygon {
    var out: Polygon;
    out.count = 0u;
    for (var i = 0u; i < p.count; i++) {
        let here = p.v[i];
        let next = p.v[(i + 1u) % p.count];
        let sh = dot(normal, here) - offset;
        let sn = dot(normal, next) - offset;
        if sh >= 0.0 && out.count < 8u {
            out.v[out.count] = here;
            out.count += 1u;
        }
        if (sh >= 0.0) != (sn >= 0.0) && out.count < 8u {
            out.v[out.count] = here + (sh / (sh - sn)) * (next - here);
            out.count += 1u;
        }
    }
    return out;
}

// `p` held to the box [low, high].
fn in_box(p: Polygon, low: vec2<f32>, high: vec2<f32>) -> Polygon {
    let a = cut(p, vec2<f32>(1.0, 0.0), low.x);
    let b = cut(a, vec2<f32>(-1.0, 0.0), -high.x);
    let c = cut(b, vec2<f32>(0.0, 1.0), low.y);
    return cut(c, vec2<f32>(0.0, -1.0), -high.y);
}

fn cross2(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

// A polygon's area, the shoelace sum halved.
fn shoelace(p: Polygon) -> f32 {
    var sum = 0.0;
    for (var i = 0u; i < p.count; i++) {
        sum += cross2(p.v[i], p.v[(i + 1u) % p.count]);
    }
    return abs(sum) * 0.5;
}

// The signed angle from u to v.
fn angle(u: vec2<f32>, v: vec2<f32>) -> f32 {
    return atan2(cross2(u, v), dot(u, v));
}

// The signed area of the unit disc inside the triangle (0, p, q): the part of the edge inside
// the disc is a triangle with the origin, each part outside a sector of the disc.
fn disc_in_triangle(p: vec2<f32>, q: vec2<f32>) -> f32 {
    let e = q - p;
    let a = dot(e, e);
    if a == 0.0 {
        return 0.0;
    }
    let b = dot(p, e);
    let c = dot(p, p) - 1.0;
    let discriminant = b * b - a * c;
    if discriminant <= 0.0 {
        return 0.5 * angle(p, q);
    }
    let root = sqrt(discriminant);
    let t0 = clamp((-b - root) / a, 0.0, 1.0);
    let t1 = clamp((-b + root) / a, 0.0, 1.0);
    let m0 = p + t0 * e;
    let m1 = p + t1 * e;
    var area = 0.5 * cross2(m0, m1);
    // A sector's ends are outside the disc, never at its centre, so its angle is defined.
    if t0 > 0.0 {
        area += 0.5 * angle(p, m0);
    }
    if t1 < 1.0 {
        area += 0.5 * angle(m1, q);
    }
    return area;
}

// The box [low, high] as a polygon.
fn box_polygon(low: vec2<f32>, high: vec2<f32>) -> Polygon {
    var p: Polygon;
    p.v[0] = low;
    p.v[1] = vec2<f32>(high.x, low.y);
    p.v[2] = high;
    p.v[3] = vec2<f32>(low.x, high.y);
    p.count = 4u;
    return p;
}

// The area of the half of the unit disc about `centre` on the side `outward` points to,
// inside the box [low, high].
fn half_disc_in_box(centre: vec2<f32>, outward: vec2<f32>, low: vec2<f32>, high: vec2<f32>) -> f32 {
    if any(centre + vec2<f32>(1.0) <= low) || any(centre - vec2<f32>(1.0) >= high) {
        return 0.0;
    }
    // The whole half disc, which is most of a dot's end at the sizes this lane is for.
    if all(centre - vec2<f32>(1.0) >= low) && all(centre + vec2<f32>(1.0) <= high) {
        return HALF_PI;
    }
    let part = cut(box_polygon(low, high), outward, dot(outward, centre));
    var sum = 0.0;
    for (var i = 0u; i < part.count; i++) {
        sum += disc_in_triangle(part.v[i] - centre, part.v[(i + 1u) % part.count] - centre);
    }
    return abs(sum);
}

// The element's SHAPE at a pixel: the area of (line ∩ pixel cell ∩ the clip's rectangle),
// §11.6.4.2's object shape by ADR 0005's area rule met with §8.5.4's clip. As in
// rect.wgsl, neither the soft mask nor the paint's alpha is in it (ADR 0066).
fn shape_at(in: VsOut, px: vec2<f32>) -> f32 {
    let low = max(px, in.admit.xy);
    let high = min(px + vec2<f32>(1.0, 1.0), in.admit.zw);
    if any(high <= low) {
        return 0.0;
    }
    // The unit-pen frame, with the cell's low corner at its origin so that every coordinate
    // the clipping meets is the size of the cell rather than of the page.
    let pen = in.pen.xy;
    let a = (in.ends.xy - low) / pen;
    let b = (in.ends.zw - low) / pen;
    let top = (high - low) / pen;
    let d = normalize(b - a);
    let n = vec2<f32>(-d.y, d.x);
    let reach = select(vec2<f32>(0.0), d, in.pen.z > 1.5);
    var band: Polygon;
    band.v[0] = a - reach - n;
    band.v[1] = b + reach - n;
    band.v[2] = b + reach + n;
    band.v[3] = a - reach + n;
    band.count = 4u;
    var area = shoelace(in_box(band, vec2<f32>(0.0), top));
    if in.pen.z > 0.5 && in.pen.z < 1.5 {
        area += half_disc_in_box(b, d, vec2<f32>(0.0), top);
        area += half_disc_in_box(a, -d, vec2<f32>(0.0), top);
    }
    return clamp(area * pen.x * pen.y, 0.0, 1.0);
}

// The device pixel this fragment shades: `position.xy` is the pixel centre, and floor
// recovers its integer corner.
fn pixel_of(in: VsOut) -> vec2<f32> {
    return floor(in.position.xy) + globals.origin;
}

// The source alpha: shape times §11.6.4.3's mask, as rect.wgsl's `coverage_at`.
fn coverage_at(in: VsOut) -> f32 {
    let px = pixel_of(in);
    return shape_at(in, px) * soft_mask_at(px);
}

// Premultiplied source scaled by coverage, under the pipeline's blend state.
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color * coverage_at(in);
}

// The element's shape alone, for §11.4.6's knockout erase (ADR 0066), as rect.wgsl's.
@fragment
fn fs_shape(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(shape_at(in, pixel_of(in)));
}
