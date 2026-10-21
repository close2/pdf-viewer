// A finished readback frame put onto the caller's opaque backdrop, on the device, in the
// buffer the frame was copied into (`Target::ReadbackOnto`, the caller's doc/adr/2150).
//
// # The byte-agreement contract
//
// The caller composited its medium on the host, over the straight-alpha raster this crate
// handed back: premultiply again, clear the pixels outside its crop, source-over its medium
// in eight bits, demultiply. This pass is that arithmetic in integers, word for word, so a
// page drawn either way is the same bytes:
//
// - **The round trip is a clamp.** ADR 0005's demultiply followed by the caller's
//   `(s·a + 127) / 255` premultiply gives back `min(c, a)` for every one of the 65 536
//   (alpha, channel) pairs — a valid premultiplied channel unchanged, and one a hair above its
//   alpha (unorm blending's ≤ 1-ulp excess) held to it. The caller's ADR checks all of them.
// - **Over an opaque colour is exact in eight bits**: `c + (b·(255 − a) + 127) / 255` for each
//   channel, which makes the alpha `a + (255 − a) = 255` and cannot pass 255 because
//   `c ≤ a`. So every pixel leaves opaque and the caller's demultiply is the identity.
//
// Integers throughout, on the packed words the copy wrote (little-endian RGBA, the buffer
// layout WebGPU fixes), so nothing here depends on a driver's float rounding.

struct Params {
    // `[left, top, right, bottom)` of the pixels that keep what was drawn.
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    // The backdrop, packed as the pixels are: red in the low byte, alpha 255 in the high.
    colour: u32,
    width: u32,
    height: u32,
    // The copy's row pitch in words: `COPY_BYTES_PER_ROW_ALIGNMENT` pads each row.
    words_per_row: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read_write> pixels: array<u32>;

@compute @workgroup_size(16, 16)
fn onto(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.width || id.y >= params.height) {
        return;
    }
    let index = id.y * params.words_per_row + id.x;
    var word = pixels[index];
    let kept = id.x >= params.left && id.x < params.right
        && id.y >= params.top && id.y < params.bottom;
    if (!kept) {
        word = 0u;
    }
    let alpha = word >> 24u;
    // Opaque already: held to its alpha, every channel is itself, and nothing shows through.
    if (alpha == 255u) {
        return;
    }
    let clear = 255u - alpha;
    var out = 0xff000000u;
    for (var shift = 0u; shift < 24u; shift += 8u) {
        let channel = min((word >> shift) & 0xffu, alpha);
        let below = (params.colour >> shift) & 0xffu;
        out |= (channel + (below * clear + 127u) / 255u) << shift;
    }
    pixels[index] = out;
}
