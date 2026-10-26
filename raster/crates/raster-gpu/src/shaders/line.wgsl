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
// admits **in the page's space** — after the line's own transform, before the view — and the
// vertex stage maps all three by the view `Globals` carries, resolving the width as ADR 0085's
// rule does for this view and collapsing the quad of a line the view admits nothing of, so that
// the instance stream is the page's and a zoom step draws from the stream the view before it
// left on the device (viewer ADR 2022). The fragment stage is a pure evaluator of the set's area
// in each pixel's unit cell, as rect.wgsl is of a rectangle's (ADR 0005's exact-area rule).
//
// The band's area in a box is the shoelace area of the band clipped to the box. A half disc's
// is the unit disc inside the box cut by the endpoint's square line, summed edge by edge of
// that convex polygon as a triangle where the edge is inside the disc and a circular sector
// where it is outside. The pieces overlap in no area — each half disc lies beyond its own
// end of the band — so the cell's coverage is their sum, times the pen's two semi-axes.
//
// Determinism (ADR 0082): `sqrt`, `atan2` and `normalize` are the operations whose precision
// WGSL leaves to the implementation, and WGSL section 15.7.5 lets a driver reassociate and fuse
// the view's multiply-adds, so a line's device endpoints may differ from the host's by an ulp:
// its bytes are the same on one adapter frame after frame and close, not identical, across
// adapters — the relaxed contract's case, which viewer ADR 2022 states for the view's mapping.

struct Globals {
    // Target size in pixels, for the NDC mapping.
    target_size: vec2<f32>,
    // The device-space corner this attachment's texel (0, 0) is (ADR 0036); rect.wgsl
    // states the mapping both stages share.
    origin: vec2<f32>,
    // The view, page to device: a b c d of the affine, then e f, then the viewport's size,
    // whose rectangle every line's clip meets (viewer ADR 2022). rect.wgsl and coverage.wgsl
    // declare the first two fields alone, which is the same buffer read shorter.
    view_linear: vec4<f32>,
    view_offset: vec2<f32>,
    view_size: vec2<f32>,
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

// Every field in the page's space (viewer ADR 2022).
struct Instance {
    // The segment's two endpoints after its own transform: from in .xy, to in .zw.
    @location(0) ends: vec4<f32>,
    // The pen's semi-axes along page x and y in .xy — or, under the fixed resolution, along
    // device x and y — and in .z Table 53's end (0 a butt, 1 a round, 2 a projecting end) plus
    // three times how the view resolves the width: 0 by its stretch, 1 by its stretch or one
    // device pixel where §10.7.5's adjustment finds it below half a pixel, 2 fixed.
    @location(1) pen: vec3<f32>,
    // Premultiplied device RGB, premultiplied once at encode time.
    @location(2) color: vec4<f32>,
    // The rectangle the clip's rectangular links admit, min in .xy and max in .zw.
    @location(3) admit: vec4<f32>,
}

// A line as the fragment stage measures it: the instance mapped to the device.
struct DeviceLine {
    // The two device endpoints: from in .xy, to in .zw.
    ends: vec4<f32>,
    // The pen's device semi-axes in .xy, and Table 53's end in .z.
    pen: vec3<f32>,
    // The rectangle the clip and the target admit on the device.
    admit: vec4<f32>,
}

// A page point on the device, by §8.3.3's equations.
fn on_view(p: vec2<f32>) -> vec2<f32> {
    let m = globals.view_linear;
    return vec2<f32>(m.x * p.x + m.z * p.y, m.y * p.x + m.w * p.y) + globals.view_offset;
}

// The instance under the view, by the steps `PagePen::on_device` states on the host (viewer
// ADR 2022): the endpoints mapped; the semi-axes each device axis receives under a view that
// keeps the axes — under a quarter turn the page's y stretch is the device's x — or one device
// pixel where the adjustment finds the device width below half a pixel; and the clip's
// rectangle mapped, its corners ordered, met with the viewport's.
fn device_line(instance: Instance) -> DeviceLine {
    let packed = u32(instance.pen.z + 0.5);
    let resolution = packed / 3u;
    let page_half = instance.pen.xy;
    let m = abs(globals.view_linear);
    var half = vec2<f32>(
        page_half.x * m.x + page_half.y * m.z,
        page_half.x * m.y + page_half.y * m.w,
    );
    if resolution == 2u {
        half = page_half;
    } else if resolution == 1u && 2.0 * max(half.x, half.y) < 0.5 {
        half = vec2<f32>(0.5);
    }
    let a = on_view(instance.admit.xy);
    let b = on_view(instance.admit.zw);
    var line: DeviceLine;
    line.ends = vec4<f32>(on_view(instance.ends.xy), on_view(instance.ends.zw));
    line.pen = vec3<f32>(half, f32(packed % 3u));
    line.admit = vec4<f32>(
        max(min(a, b), vec2<f32>(0.0)),
        min(max(a, b), globals.view_size),
    );
    return line;
}

// Whether no pixel can carry any of the line under its rectangle: the ink's box — the segment's
// grown by √2 of the pen each way, which holds every end Table 53 states — misses the rectangle.
// The fragment stage would find zero in every pixel; collapsing the quad is the cull the host
// made before the instance was the page's (viewer ADR 2022).
fn admits_nothing(line: DeviceLine) -> bool {
    let reach = line.pen.xy * 1.4143;
    let low = max(min(line.ends.xy, line.ends.zw) - reach, line.admit.xy);
    let high = min(max(line.ends.xy, line.ends.zw) + reach, line.admit.zw);
    return any(high <= low);
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) ends: vec4<f32>,
    @location(1) @interpolate(flat) pen: vec3<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) admit: vec4<f32>,
}

// A run's direction, of length one. A run of no length is §8.5.3.2's dot, which only a round
// end reaches this lane with (the scene refuses it under the other two): its band has no area
// and its two half discs about the one point are the disc whichever way they face, so any unit
// vector is the answer, and `normalize` of zero is not one (viewer ADR 1857).
fn unit(run: vec2<f32>) -> vec2<f32> {
    if all(run == vec2<f32>(0.0)) {
        return vec2<f32>(1.0, 0.0);
    }
    return normalize(run);
}

// The segment's direction in the unit-pen frame, of length one.
fn direction(ends: vec4<f32>, pen: vec2<f32>) -> vec2<f32> {
    return unit((ends.zw - ends.xy) / pen);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, page: Instance) -> VsOut {
    let instance = device_line(page);
    var out: VsOut;
    if admits_nothing(instance) {
        // Four vertices at one point outside the clip volume: a strip of no area.
        out.position = vec4<f32>(2.0, 2.0, 0.0, 1.0);
        out.ends = instance.ends;
        out.pen = instance.pen;
        out.color = page.color;
        out.admit = instance.admit;
        return out;
    }
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
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.ends = instance.ends;
    out.pen = instance.pen;
    out.color = page.color;
    out.admit = instance.admit;
    return out;
}

// A convex polygon of at most eight vertices: a quadrilateral cut by four lines gains at
// most four.
//
// **Every loop over its vertices runs to the constant eight and leaves at `count`, and no vertex
// is written through an index computed at run time** — `image.wgsl`'s polygon and its reason
// (ADR 1928): DX12's FXC compiler refuses an array element written by a run-time index (X3500)
// and a loop it cannot unroll to make that index constant (X3511). A read by a run-time index is
// admitted, so the vertices are still read by `i`.
struct Polygon {
    v: array<vec2<f32>, 8>,
    count: u32,
}

// `p` with `vertex` after its last: each of the eight slots is written by a constant index and
// keeps what it held unless it is slot `count` (ADR 1928). A ninth vertex is dropped, as the
// caller's guard already ensures none arrives.
fn push_vertex(p: ptr<function, Polygon>, vertex: vec2<f32>) {
    let at = (*p).count;
    (*p).v[0] = select((*p).v[0], vertex, at == 0u);
    (*p).v[1] = select((*p).v[1], vertex, at == 1u);
    (*p).v[2] = select((*p).v[2], vertex, at == 2u);
    (*p).v[3] = select((*p).v[3], vertex, at == 3u);
    (*p).v[4] = select((*p).v[4], vertex, at == 4u);
    (*p).v[5] = select((*p).v[5], vertex, at == 5u);
    (*p).v[6] = select((*p).v[6], vertex, at == 6u);
    (*p).v[7] = select((*p).v[7], vertex, at == 7u);
    (*p).count = at + 1u;
}

// `p` less everything where dot(normal, x) < offset (Sutherland–Hodgman, one line).
fn cut(p: Polygon, normal: vec2<f32>, offset: f32) -> Polygon {
    var out: Polygon;
    out.count = 0u;
    for (var i = 0u; i < 8u; i++) {
        if i >= p.count {
            break;
        }
        let here = p.v[i];
        let next = p.v[(i + 1u) % p.count];
        let sh = dot(normal, here) - offset;
        let sn = dot(normal, next) - offset;
        if sh >= 0.0 && out.count < 8u {
            push_vertex(&out, here);
        }
        if (sh >= 0.0) != (sn >= 0.0) && out.count < 8u {
            push_vertex(&out, here + (sh / (sh - sn)) * (next - here));
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
    for (var i = 0u; i < 8u; i++) {
        if i >= p.count {
            break;
        }
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
    for (var i = 0u; i < 8u; i++) {
        if i >= part.count {
            break;
        }
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
    let d = unit(b - a);
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

// Premultiplied source scaled by coverage, under the pipeline's blend state; an opaque source is
// written as the level nearest it, as rect.wgsl's `fs_main` says (ADR 2324).
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

// The element's shape alone, for §11.4.6's knockout erase (ADR 0066), as rect.wgsl's.
@fragment
fn fs_shape(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(shape_at(in, pixel_of(in)));
}
