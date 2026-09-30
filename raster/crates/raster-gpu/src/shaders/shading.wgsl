// The shading lane: axial and radial ramp sweeps (ISO 32000-2 §8.7.4.5.2/.3) and
// the caller's pre-rasterised meshes, painted through a coverage source.
//
// The sweep parameter t is computed in the *shading's own space*: the fragment's
// device position maps back through the inverse of the command × viewport transform
// carried in the params, so a sheared or rotated placement sweeps correctly. The
// ramp was sampled to straight-RGBA texels on the CPU at upload (deterministic);
// t indexes it with textureLoad — no sampler, no filtering, adapter-invariant — through
// `ramp_texel`, which places a hard step at its own offset (ADR 1389).
//
// Coverage comes either from the frame's scratch (a rasterised fill/stroke shape,
// ADR 0008) or analytically from the coverage rectangle for the rectangle case, and
// is modulated by the clip rectangle and soft mask like every lane.

struct Params {
    // Inverse device transform, §8.3.3 layout: a, b, c, d, e, f.
    inv0: vec4f, // a, b, c, d
    inv1: vec4f, // e, f, kind (0 axial, 1 radial, 2 mesh), extend flags packed
    // Axial: start.xy, end.xy. Radial: start.xy, end.xy. Mesh: left, top, 0, 0.
    geo0: vec4f,
    // Radial: start_radius, end_radius. Others unused.
    geo1: vec4f,
    // Quad destination rectangle, device space.
    dest: vec4f,
    // Coverage source: origin.xy in scratch, use_scratch, unused.
    coverage: vec4f,
    // Analytic coverage rectangle (the shape itself when no scratch tile).
    coverage_rect: vec4f,
    // Clip rectangle.
    clip: vec4f,
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
// The ramp (RAMP_RESOLUTION x RAMP_ROWS: colours, bounds, layout) for axial/radial, or
// the mesh raster.
@group(0) @binding(1) var paint_tex: texture_2d<f32>;
@group(0) @binding(2) var scratch_tex: texture_2d<f32>;
// The active soft mask, realised at its own plan's rectangle.
@group(0) @binding(3) var soft_mask_tex: texture_2d<f32>;

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

// This pass draws one shading under one mask, so the placement is in its uniform.
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

// The sweep parameter, or -1 when the point is outside an unextended range —
// §8.7.4.5.2/.3: where extension is off, nothing is painted there at all, which is
// not the same as painting the end colour.
fn sweep_t(p_shading: vec2f) -> f32 {
    let kind = params.inv1.z;
    // Extend flags packed as bits: 1 = beyond start, 2 = beyond end.
    let bits = i32(params.inv1.w);
    let extend_start = (bits & 1) != 0;
    let extend_end = (bits & 2) != 0;
    if kind < 0.5 {
        // §8.7.4.5.2: the projection onto the axis.
        let d = params.geo0.zw - params.geo0.xy;
        let len_sq = dot(d, d);
        if len_sq <= 0.0 {
            return -1.0;
        }
        var t = dot(p_shading - params.geo0.xy, d) / len_sq;
        if t < 0.0 {
            if !extend_start { return -1.0; }
            t = 0.0;
        }
        if t > 1.0 {
            if !extend_end { return -1.0; }
            t = 1.0;
        }
        return t;
    }
    // §8.7.4.5.3: the largest s with |p − c(s)| = r(s), r(s) >= 0, where centre and
    // radius interpolate linearly between the two circles.
    let cd = params.geo0.zw - params.geo0.xy;
    let rd = params.geo1.y - params.geo1.x;
    let pd = p_shading - params.geo0.xy;
    let a = dot(cd, cd) - rd * rd;
    let b = dot(pd, cd) + params.geo1.x * rd;
    let c = dot(pd, pd) - params.geo1.x * params.geo1.x;
    var s: f32;
    if abs(a) < 1e-6 {
        if abs(b) < 1e-12 {
            return -1.0;
        }
        s = c / (2.0 * b);
    } else {
        let disc = b * b - a * c;
        if disc < 0.0 {
            return -1.0;
        }
        let root = sqrt(disc);
        let s1 = (b + root) / a;
        let s2 = (b - root) / a;
        // The larger s whose radius is non-negative (§8.7.4.5.3's preference).
        s = max(s1, s2);
        if params.geo1.x + s * rd < 0.0 {
            s = min(s1, s2);
            if params.geo1.x + s * rd < 0.0 {
                return -1.0;
            }
        }
    }
    if s < 0.0 {
        if !extend_start { return -1.0; }
        s = 0.0;
    }
    if s > 1.0 {
        if !extend_end { return -1.0; }
        s = 1.0;
    }
    return s;
}

// Coverage met with the clip at the fragment's cell — the element's *shape* (§11.6.4.2 met with
// §8.5.4's clip), shared by both entry points. The soft mask is not in it (ADR 0066):
// Table 57's alpha source flag reads §11.6.4.3's mask as opacity by default, so
// `fs_main` multiplies it in and `fs_shape` does not.
fn shape_at(p: vec2f) -> f32 {
    // Coverage: a scratch tile's byte, or the analytic rectangle's cell overlap.
    // §10.7.4's intersection of the mark's pixels with the clip's (the caller's ADR 1435):
    // two rectangles meet in a rectangle, so that branch intersects them before taking the
    // cell overlap and is exact; a scratch byte carries no geometry, so it meets the clip
    // by `min`, the area of the intersection's upper bound and never below it.
    if params.coverage.z > 0.5 {
        let texel = vec2i(params.coverage.xy + (p - params.dest.xy));
        let overlap_min = max(params.clip.xy, p);
        let overlap_max = min(params.clip.zw, p + vec2f(1.0, 1.0));
        let extent = max(overlap_max - overlap_min, vec2f(0.0, 0.0));
        return min(textureLoad(scratch_tex, texel, 0).r, extent.x * extent.y);
    }
    let o_min = max(max(params.coverage_rect.xy, params.clip.xy), p);
    let o_max = min(min(params.coverage_rect.zw, params.clip.zw), p + vec2f(1.0, 1.0));
    let e = max(o_max - o_min, vec2f(0.0, 0.0));
    return e.x * e.y;
}

// The straight-alpha paint at the fragment, or a negative alpha sentinel where the
// shading paints nothing at all (unextended sweep, or outside the mesh raster).
fn paint_at(p: vec2f) -> vec4f {
    if params.inv1.z > 1.5 {
        // Mesh: the pre-rasterised samples sit at absolute device pixels.
        let dims = vec2f(textureDimensions(paint_tex));
        let texel = p - params.geo0.xy;
        if texel.x < 0.0 || texel.y < 0.0 || texel.x >= dims.x || texel.y >= dims.y {
            return vec4f(0.0, 0.0, 0.0, -1.0);
        }
        return textureLoad(paint_tex, vec2i(texel), 0);
    }
    // Map back to the shading's own space, then sweep.
    let centre = p + vec2f(0.5, 0.5);
    let inv = params.inv0;
    let q = vec2f(
        inv.x * centre.x + inv.z * centre.y + params.inv1.x,
        inv.y * centre.x + inv.w * centre.y + params.inv1.y,
    );
    let t = sweep_t(q);
    if t < 0.0 {
        return vec4f(0.0, 0.0, 0.0, -1.0);
    }
    return textureLoad(paint_tex, vec2i(ramp_texel(t), 0), 0);
}

// One word of the ramp's rows 1 and 2: four RGBA8 bytes, little-endian. An `rgba8unorm`
// channel reads as `byte / 255`, which `round(· × 255)` returns to the byte exactly.
fn ramp_word(x: i32, row: i32) -> u32 {
    let b = vec4u(round(textureLoad(paint_tex, vec2i(x, row), 0) * 255.0));
    return b.x | (b.y << 8u) | (b.z << 16u) | (b.w << 24u);
}

// The colour texel for `t` (ADR 1389; `device/ramp.rs`'s `texel_for` states it in Rust).
//
// §8.7.4.5.3 makes a pixel's colour the function's at the `t` of its centre, so a hard
// step — a §7.10.4 stitching bound, two stops at one offset — is found by comparing `t`
// with the bound itself, carried exactly in row 1, and never by rounding onto a grid.
// Segment k lies between row 1's texels k and k + 1; it is the first whose upper offset
// is above `t` (closed on the left), or the last, and a `t` at or below the first offset
// is the first segment's. Only within the segment does the index round.
fn ramp_texel(t: f32) -> i32 {
    let count = i32(ramp_word(0, 2));
    var k = 0;
    if t > bitcast<f32>(ramp_word(0, 1)) {
        var lo = 0;
        var hi = count - 1;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if t < bitcast<f32>(ramp_word(mid + 1, 1)) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        k = lo;
    }
    let lower = bitcast<f32>(ramp_word(k, 1));
    let upper = bitcast<f32>(ramp_word(k + 1, 1));
    let info = ramp_word(k + 1, 2);
    let base = i32(info & 0xffffu);
    let texels = i32(info >> 16u);
    if texels <= 1 || upper <= lower {
        return base;
    }
    let u = clamp((t - lower) / (upper - lower), 0.0, 1.0);
    return base + i32(round(u * f32(texels - 1)));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let p = floor(in.position.xy) + params.origin;
    let straight = paint_at(p);
    if straight.a < 0.0 {
        return vec4f(0.0);
    }
    return vec4f(straight.rgb * straight.a, straight.a) * (shape_at(p) * soft_mask_at(p));
}

// The knockout erase pass wants the shape alone (§11.4.6, ADR 0010). Where an
// unextended shading paints nothing, no mark is made and nothing knocks out —
// shape 0, not shape-with-zero-opacity (ADR 0011 records the reading of
// §11.3.7.2). A mesh raster's own alpha is antialiased triangle coverage and so
// counts as shape.
@fragment
fn fs_shape(in: VsOut) -> @location(0) vec4f {
    let p = floor(in.position.xy) + params.origin;
    let straight = paint_at(p);
    if straight.a < 0.0 {
        return vec4f(0.0);
    }
    var shape = shape_at(p);
    if params.inv1.z > 1.5 {
        shape = shape * straight.a;
    }
    return vec4f(0.0, 0.0, 0.0, shape);
}
