// The game-like lane's three stages, for RFC 0013's probe. The view is the only thing a
// zoom step changes, and it arrives here as one uniform: page space to device pixels.

struct View {
    // ISO 32000-2 §8.3.2's [a b c d e f], page space to device pixels.
    abcd: vec4<f32>,
    // e, f, and the target's width and height in pixels.
    ef_size: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;

struct Stencilled {
    @builtin(position) position: vec4<f32>,
    // Sampled per sample, so a curve's inside is decided where the stencil is counted —
    // at each of the MSAA samples, not once per pixel.
    @location(0) @interpolate(perspective, sample) uv: vec2<f32>,
}

struct Covered {
    @builtin(position) position: vec4<f32>,
    @location(0) colour: vec4<f32>,
}

fn clip_of(page: vec2<f32>) -> vec4<f32> {
    let device = vec2<f32>(
        view.abcd.x * page.x + view.abcd.z * page.y + view.ef_size.x,
        view.abcd.y * page.x + view.abcd.w * page.y + view.ef_size.y,
    );
    let size = view.ef_size.zw;
    return vec4<f32>(device.x / size.x * 2.0 - 1.0, 1.0 - device.y / size.y * 2.0, 0.0, 1.0);
}

@vertex
fn stencil_vertex(@location(0) page: vec2<f32>, @location(1) uv: vec2<f32>) -> Stencilled {
    var out: Stencilled;
    out.position = clip_of(page);
    out.uv = uv;
    return out;
}

// A chord's fan triangle: every sample it covers counts, so nothing is tested.
@fragment
fn fan_fragment(in: Stencilled) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}

// A quadratic's control triangle: Loop and Blinn's curve is u² − v = 0, and the side that
// holds the chord is u² − v < 0.
@fragment
fn curve_fragment(in: Stencilled) -> @location(0) vec4<f32> {
    if in.uv.x * in.uv.x - in.uv.y >= 0.0 {
        discard;
    }
    return vec4<f32>(0.0);
}

@vertex
fn cover_vertex(@location(0) page: vec2<f32>, @location(2) colour: vec4<f32>) -> Covered {
    var out: Covered;
    out.position = clip_of(page);
    out.colour = colour;
    return out;
}

@fragment
fn cover_fragment(in: Covered) -> @location(0) vec4<f32> {
    return in.colour;
}

// An image: a textured quad over its unit square, multiplied by the command's constant alpha
// (carried in the colour's fourth byte), and no stencil.
@group(1) @binding(0) var samples: texture_2d<f32>;
@group(1) @binding(1) var sampling: sampler;

struct Textured {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
}

@vertex
fn image_vertex(
    @location(0) page: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
) -> Textured {
    var out: Textured;
    out.position = clip_of(page);
    out.uv = uv;
    out.alpha = colour.a;
    return out;
}

@fragment
fn image_fragment(in: Textured) -> @location(0) vec4<f32> {
    let sample = textureSample(samples, sampling, in.uv);
    return vec4<f32>(sample.rgb, sample.a * in.alpha);
}
