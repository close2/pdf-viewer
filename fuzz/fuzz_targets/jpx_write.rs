//! Fuzzes the lossless JPEG 2000 writer, `pdf-jpx-write` (ADR 2470), by its round trip.
//!
//! The first bytes choose the grid (each side 1 to 64), the colour space, the opacity channel
//! and each channel's depth (1 to 24); the rest are samples, three bytes each masked to the
//! channel's depth, repeated where the input runs short. Two properties are under test, beyond
//! never panicking:
//!
//! - **The writer takes every valid input.** A plane at its own depth on its own grid is never
//!   refused.
//! - **The round trip is exact.** The tree's own decoder, `pdf-sandbox`, asked as a writer asks
//!   it, reads back every channel at its own depth and every sample as written — which is the
//!   whole claim a redaction rests on when it writes a cleared deep image back.

#![no_main]
#![expect(
    clippy::panic,
    reason = "a fuzz target states its properties by failing: `panic!` is how a violated one \
              reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_jpx_write::{Colour, MAX_DEPTH, Opacity, Plane, jp2};
use pdf_sandbox::{Decoded, Request};

fuzz_target!(|data: &[u8]| {
    let Some((head, body)) = data.split_first_chunk::<4>() else {
        return;
    };
    let width = u32::from(head[0] % 64) + 1;
    let height = u32::from(head[1] % 64) + 1;
    let colour = if head[2] & 1 == 0 {
        Colour::Greyscale
    } else {
        Colour::Srgb
    };
    let opacity = match head[3] % 3 {
        0 => None,
        1 => Some(Opacity::Plain),
        _ => Some(Opacity::Premultiplied),
    };
    let channels = colour
        .channels()
        .saturating_add(usize::from(opacity.is_some()));
    let Some((depths, samples)) = body.split_at_checked(channels) else {
        return;
    };
    let depths: Vec<u8> = depths
        .iter()
        .map(|byte| (byte % MAX_DEPTH).saturating_add(1))
        .collect();
    let count = usize::try_from(width.saturating_mul(height)).unwrap_or(0);
    let mut fed = samples.iter().copied().cycle();
    let planes: Vec<Vec<u32>> = depths
        .iter()
        .map(|depth| {
            let mask = (1u32 << depth).saturating_sub(1);
            (0..count)
                .map(|_| {
                    if samples.is_empty() {
                        return 0;
                    }
                    let bytes = [
                        0,
                        fed.next().unwrap_or(0),
                        fed.next().unwrap_or(0),
                        fed.next().unwrap_or(0),
                    ];
                    u32::from_be_bytes(bytes) & mask
                })
                .collect()
        })
        .collect();
    let given: Vec<Plane<'_>> = planes
        .iter()
        .zip(&depths)
        .map(|(samples, depth)| Plane {
            depth: *depth,
            samples,
        })
        .collect();
    let colours = colour.channels();
    let file = match jp2(
        (width, height),
        colour,
        &given[..colours],
        opacity.and_then(|kind| given.get(colours).map(|plane| (*plane, kind))),
    ) {
        Ok(file) => file,
        Err(error) => panic!("a valid input was refused: {error}"),
    };
    let request = Request::JpxWhole {
        data: &file,
        indices: false,
        samples: 1 << 20,
    };
    let raster = match pdf_sandbox::decode_here(&request) {
        Ok(Decoded::Raster(raster)) => raster,
        Ok(_) => panic!("JPXDecode answers a continuous-tone raster"),
        Err(error) => panic!("the written file does not decode: {error}"),
    };
    assert!(
        (raster.width, raster.height) == (width, height) && raster.channels() == channels,
        "the grid or the channel count changed"
    );
    for (channel, depth) in depths.iter().enumerate() {
        assert!(
            raster.depth_of(channel) == *depth,
            "channel {channel} came back at another depth"
        );
    }
    for pixel in 0..count {
        for (channel, plane) in planes.iter().enumerate() {
            assert!(
                raster.sample(pixel.saturating_mul(channels).saturating_add(channel))
                    == Some(plane[pixel]),
                "pixel {pixel} channel {channel} did not read back exactly"
            );
        }
    }
});
