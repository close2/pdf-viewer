// The image lane: a decoded RGBA image mapped into the unit square (ISO 32000-2
// §8.9.5), drawn as one quad per command.
//
// The fragment's device position maps through the device → texel transform carried
// in the params, giving the point's place in the bound texture's texel space — s to the
// right and t down from the image's top row (§8.9.4), one unit per texel — so the drawn
// quad only has to *cover* the footprint, never trace it. The transform is composed on
// the CPU in double precision and rounded once (`encode::rare::texel_transform`), so a
// placement of one device pixel per sample maps every pixel centre onto a whole number
// exactly and the nearest texel is the floor of it (§10.7.4).
//
// Coverage (ADR 0011, ADR 1492): a pixel's shape is the area of the image's parallelogram
// inside it, met with the clip rectangle — §10.7.4's departure (1), "a partly covered pixel
// is partly painted", which the caller's ledger reads for an image's edge as for a fill's.
// - An axis-preserving placement gets the analytic cell-overlap of the rectangle
//   lane (ADR 0005) against `image_rect`; the quad is the footprint expanded to pixel
//   bounds.
// - An oblique placement takes the same area of the parallelogram the unit square maps
//   to (`oblique_area`), so both placements draw an edge one way.
// - A residue clip arrives as a scratch tile met by `min`, like every lane (the caller's
//   ADR 1444); where both sets cut a pixel the encoder has already put their
//   intersection's area there (ADR 1480, ADR 1492).
//
// Filtering is the placement's **resolved** decision (§4.5, integration note 1):
// nearest goes through textureLoad (exact, adapter-invariant); linear goes through
// the hardware sampler (clamp-to-edge), whose interpolation precision is the
// driver's — the one place a §4.5 decision buys hardware variance, stated here and
// bounded by the tests' tolerance rather than hidden.

struct Params {
    // Device space → the bound texture's texel space, §8.3.3 layout.
    texel0: vec4f, // a, b, c, d
    texel1: vec4f, // e, f, constant alpha (§11.6.4.4's ca), filter (1 = linear)
    // The image's device rectangle (exact for the axis-preserving case).
    image_rect: vec4f,
    // Quad destination rectangle, device space.
    dest: vec4f,
    // Clip rectangle (ADR 0007's four floats).
    clip: vec4f,
    // Residue-clip source: origin.xy in scratch, use_scratch, axis-preserving flag.
    coverage: vec4f,
    target_size: vec2f,
    // The device-space corner this attachment's texel (0, 0) is (ADR 0036).
    //
    // A plan renders into a texture the size of *its own* content rather than the
    // target's, so device space and attachment space differ by this. Two places have to
    // agree about it and they are the whole of the mapping: the vertex stage subtracts it
    // before dividing by `target_size`, and the fragment stage adds it back to recover
    // the device pixel it is shading — clip rectangles, tile lookups and masks are all
    // stated in device space and would otherwise be read at the wrong place. Zero for the
    // frame's root, which renders into the target itself.
    origin: vec2f,
    // Where the active soft mask sits (ADR 0037): its device corner in .xy, its size
    // in texels in .zw.
    mask_rect: vec4f,
    // What that mask holds outside `mask_rect`, in .x: the reduce's output for a
    // transparent pixel, since the mask's group marks nothing out there. A size of
    // (0, 0) is an absent mask, and then this is 1 and admits everything.
    mask_outside: vec4f,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var image_tex: texture_2d<f32>;
@group(0) @binding(2) var image_sampler: sampler;
// The active soft mask, realised at its own plan's rectangle.
@group(0) @binding(3) var soft_mask_tex: texture_2d<f32>;
// The frame's scratch, holding a rasterised residue clip when one applies.
@group(0) @binding(4) var scratch_tex: texture_2d<f32>;

// The soft mask at a device pixel, given where the mask sits (ADR 0037). Identical in
// all six shaders that sample a mask; WGSL has no include, so the copies are kept
// textually the same, and tests/shader_copies.rs fails the build when they drift. The
// placement is an argument rather than a global because it reaches each lane in a
// different uniform; `soft_mask_tex` is the one name all six bind it under.
fn soft_mask_value(rect: vec4f, outside: f32, p: vec2f) -> f32 {
    let local = p - rect.xy;
    if any(local < vec2f(0.0)) || any(local >= rect.zw) {
        return outside;
    }
    return textureLoad(soft_mask_tex, vec2i(local), 0).r;
}

// This pass draws one image under one mask, so the placement is in its uniform.
fn soft_mask_at(p: vec2f) -> f32 {
    return soft_mask_value(params.mask_rect, params.mask_outside.x, p);
}

struct VsOut {
    @builtin(position) position: vec4f,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    let corner = vec2f(f32(vertex_index & 1u), f32(vertex_index >> 1u));
    let pos = mix(params.dest.xy, params.dest.zw, corner);
    var out: VsOut;
    out.position = vec4f(
        (pos.x - params.origin.x) / params.target_size.x * 2.0 - 1.0,
        1.0 - (pos.y - params.origin.y) / params.target_size.y * 2.0,
        0.0,
        1.0,
    );
    return out;
}

// The texel-space coordinate of a device point, through the carried transform.
fn to_texel(p: vec2f) -> vec2f {
    return vec2f(
        params.texel0.x * p.x + params.texel0.z * p.y + params.texel1.x,
        params.texel0.y * p.x + params.texel0.w * p.y + params.texel1.y,
    );
}

// One convex polygon of at most eight corners: the pixel's rectangle in texel space, as the
// four sides of the image's square cut it.
struct Polygon {
    corner: array<vec2f, 8>,
    count: u32,
}

// The part of `polygon` on the kept side of `axis = bound` (Sutherland–Hodgman): above it
// when `above`, below otherwise. A convex polygon gains at most one corner per side.
fn keep_side(polygon: Polygon, axis: u32, bound: f32, above: bool) -> Polygon {
    var out: Polygon;
    out.count = 0u;
    for (var i = 0u; i < polygon.count; i++) {
        let here = polygon.corner[i];
        let next = polygon.corner[(i + 1u) % polygon.count];
        let sh = select(bound - here[axis], here[axis] - bound, above);
        let sn = select(bound - next[axis], next[axis] - bound, above);
        if sh >= 0.0 && out.count < 8u {
            out.corner[out.count] = here;
            out.count += 1u;
        }
        if (sh >= 0.0) != (sn >= 0.0) && out.count < 8u {
            out.corner[out.count] = here + (next - here) * (sh / (sh - sn));
            out.count += 1u;
        }
    }
    return out;
}

// The area of an oblique image inside the device rectangle `[lo, hi]` — a pixel, already
// met with the clip rectangle — as a fraction of that pixel (ADR 1492). The rectangle is
// mapped into texel space, where the image is the box `[0, dims]`, by the carried transform;
// cut to the box there, and its area taken back to device space by the transform's
// determinant. The corners are stated from the pixel's centre `centre` (whose texel point is
// `st`), so the arithmetic is on offsets of a pixel's size rather than on page coordinates.
fn oblique_area(lo: vec2f, hi: vec2f, centre: vec2f, st: vec2f, dims: vec2f) -> f32 {
    if any(hi <= lo) {
        return 0.0;
    }
    let across = params.texel0.xy;
    let down = params.texel0.zw;
    var polygon: Polygon;
    polygon.count = 4u;
    polygon.corner[0] = across * (lo.x - centre.x) + down * (lo.y - centre.y);
    polygon.corner[1] = across * (hi.x - centre.x) + down * (lo.y - centre.y);
    polygon.corner[2] = across * (hi.x - centre.x) + down * (hi.y - centre.y);
    polygon.corner[3] = across * (lo.x - centre.x) + down * (hi.y - centre.y);
    polygon = keep_side(polygon, 0u, -st.x, true);
    polygon = keep_side(polygon, 0u, dims.x - st.x, false);
    polygon = keep_side(polygon, 1u, -st.y, true);
    polygon = keep_side(polygon, 1u, dims.y - st.y, false);
    var twice = 0.0;
    for (var i = 0u; i < polygon.count; i++) {
        let a = polygon.corner[i];
        let b = polygon.corner[(i + 1u) % polygon.count];
        twice += a.x * b.y - b.x * a.y;
    }
    let determinant = abs(across.x * down.y - down.x * across.y);
    return clamp(0.5 * abs(twice) / determinant, 0.0, 1.0);
}

// Geometric coverage met with the clip and the residue — the element's *shape* (§11.6.4.2: "For images
// … the shape shall be 1.0 inside the image rectangle and 0.0 outside it", met with
// §8.5.4's clip). The image's own alpha, the constant alpha and the soft mask are all
// opacity, not shape, and stay out of this product on purpose (ADR 0011, ADR 0066).
fn shape_at(p: vec2f, st: vec2f, dims: vec2f) -> f32 {
    // §10.7.4's intersection of the image's pixels with the clip rectangle's (the caller's
    // ADR 1435): both placements cut the pixel by the clip rectangle first and take the
    // image's area in what is left, so the clip's edge and the image's are each drawn once.
    var cov: f32;
    if params.coverage.w > 0.5 {
        let o_min = max(max(params.image_rect.xy, params.clip.xy), p);
        let o_max = min(min(params.image_rect.zw, params.clip.zw), p + vec2f(1.0, 1.0));
        let e = max(o_max - o_min, vec2f(0.0, 0.0));
        cov = e.x * e.y;
    } else {
        let lo = max(params.clip.xy, p);
        let hi = min(params.clip.zw, p + vec2f(1.0, 1.0));
        cov = oblique_area(lo, hi, p + vec2f(0.5, 0.5), st, dims);
    }
    if params.coverage.z > 0.5 {
        let texel = vec2i(params.coverage.xy + (p - params.dest.xy));
        // A residue byte carries no geometry: it meets the shape by `min`, the least value
        // never below §10.7.4's intersection (the caller's ADR 1444).
        cov = min(cov, textureLoad(scratch_tex, texel, 0).r);
    }
    return cov;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let p = floor(in.position.xy) + params.origin;
    let st = to_texel(p + vec2f(0.5, 0.5));
    let dims = vec2f(textureDimensions(image_tex));
    let shape = shape_at(p, st, dims);
    var sample: vec4f;
    if params.texel1.w > 0.5 {
        let tex_uv = clamp(st / dims, vec2f(0.0), vec2f(1.0));
        sample = textureSampleLevel(image_tex, image_sampler, tex_uv, 0.0);
    } else {
        // Nearest: the texel whose square holds the point, no sampler arithmetic.
        let texel = vec2i(clamp(floor(st), vec2f(0.0), dims - vec2f(1.0)));
        sample = textureLoad(image_tex, texel, 0);
    }
    // The texture is premultiplied at realisation (`device::textures::premultiplied`),
    // so the linear filter above never lends a transparent texel's colour to its
    // neighbours. The constant alpha (§11.6.4.4), the soft mask (§11.6.4.3) and the
    // image's own alpha are opacity, so all three scale the premultiplied source and
    // none enters `shape`.
    return sample * (shape * params.texel1.z * soft_mask_at(p));
}

// The knockout erase pass wants the shape alone (§11.4.6 with ADR 0010's algebra).
@fragment
fn fs_shape(in: VsOut) -> @location(0) vec4f {
    let p = floor(in.position.xy) + params.origin;
    let st = to_texel(p + vec2f(0.5, 0.5));
    return vec4f(0.0, 0.0, 0.0, shape_at(p, st, vec2f(textureDimensions(image_tex))));
}
