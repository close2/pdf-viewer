// A decoded 4:2:0 picture converted to RGBA8 on the device (RFC 0015 section 6, round 4; ADR 3529).
//
// The formula and its numbers are `pdf_render::video`'s, computed here in `f32` from the same
// integers the processor's oracle holds: the luma sample, and each chroma sample interpolated in
// sixteenths with weights in quarters, which is exact. Each pixel's level is taken in integers and
// written as a packed word into a storage buffer, so no render target's own conversion of a float
// to a level stands between the formula and the byte (traps 251, 256).

struct Params {
    size: vec2<u32>,
    chroma: vec2<u32>,
    site: vec2<u32>,
    row_words: u32,
    _pad0: u32,
    luma: f32,
    luma_offset: f32,
    _pad1: vec2<f32>,
    cb: vec4<f32>,
    cr: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var luma_plane: texture_2d<u32>;
@group(0) @binding(2) var cb_plane: texture_2d<u32>;
@group(0) @binding(3) var cr_plane: texture_2d<u32>;
@group(0) @binding(4) var<storage, read_write> pixels: array<u32>;

// The two chroma indices either side of luma index `index` at siting `site` (half samples), and the
// far one's weight in quarters: the position is (2 index − site) / 4 chroma samples.
fn taps(index: u32, site: u32, count: u32) -> vec3<u32> {
    let quarters = i32(index) * 2 - i32(site);
    let near = quarters >> 2u;
    let last = i32(count) - 1;
    return vec3<u32>(
        u32(clamp(near, 0, last)),
        u32(clamp(near + 1, 0, last)),
        u32(quarters & 3),
    );
}

fn sample(plane: texture_2d<u32>, column: u32, row: u32) -> u32 {
    return textureLoad(plane, vec2<u32>(column, row), 0).r;
}

// The chroma sample at luma position `at`, in sixteenths of a level.
fn sixteenths(plane: texture_2d<u32>, at: vec2<u32>) -> u32 {
    let across = taps(at.x, params.site.x, params.chroma.x);
    let down = taps(at.y, params.site.y, params.chroma.y);
    let upper = (4u - across.z) * sample(plane, across.x, down.x)
        + across.z * sample(plane, across.y, down.x);
    let lower = (4u - across.z) * sample(plane, across.x, down.y)
        + across.z * sample(plane, across.y, down.y);
    return (4u - down.z) * upper + down.z * lower;
}

@compute @workgroup_size(16, 16)
fn convert(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.size.x || id.y >= params.size.y {
        return;
    }
    let luma = f32(textureLoad(luma_plane, id.xy, 0).r);
    let blue = f32(sixteenths(cb_plane, id.xy)) - 2048.0;
    let red = f32(sixteenths(cr_plane, id.xy)) - 2048.0;
    let rgb = params.luma * (luma - params.luma_offset) + params.cb.xyz * blue + params.cr.xyz * red;
    let level = vec3<u32>(clamp(floor(rgb + 0.5), vec3<f32>(0.0), vec3<f32>(255.0)));
    pixels[id.y * params.row_words + id.x] =
        level.r | (level.g << 8u) | (level.b << 16u) | (255u << 24u);
}
