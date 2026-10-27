//! Fuzzes the lossless JPEG 2000 writer, `pdf-jpx-write` (ADR 2470), by its round trip.
//!
//! The first bytes choose the grid (each side 1 to 64), the colour space (JP2's two, enumerated
//! CMYK, or a vendor colour specification box over one to four channels, which makes the file a
//! JPX file), the opacity channel, which channels state a sign bit, and each channel's depth
//! (1 to 24); the rest are samples, three bytes each masked to the
//! channel's depth, repeated where the input runs short. Two properties are under test, beyond
//! never panicking:
//!
//! - **The writer takes every valid input.** A plane at its own depth on its own grid is never
//!   refused.
//! - **The round trip is exact, signs included.** The tree's own decoder, `pdf-sandbox`, asked as a writer asks
//!   it, reads back every channel at its own depth and every sample as written, and the codestream
//!   states each sign bit as it was given — which is the
//!   whole claim a redaction rests on when it writes a cleared deep image back.

#![no_main]
#![expect(
    clippy::panic,
    reason = "a fuzz target states its properties by failing: `panic!` is how a violated one \
              reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_jpx_write::{ColourBox, MAX_DEPTH, Opacity, Plane, Space, file};
use pdf_sandbox::{Decoded, Request};

fuzz_target!(|data: &[u8]| {
    let Some((head, body)) = data.split_first_chunk::<6>() else {
        return;
    };
    let width = u32::from(head[0] % 64) + 1;
    let height = u32::from(head[1] % 64) + 1;
    // A vendor colour specification (T.801 Table M.22's method 4): 16 bytes of identifier.
    let vendor = ColourBox {
        method: 4,
        precedence: 0,
        approximation: 1,
        data: &[0x5a; 16],
    };
    let space = match head[2] % 4 {
        0 => Space::Greyscale,
        1 => Space::Srgb,
        2 => Space::Cmyk,
        _ => Space::Stated(vendor),
    };
    // The decoder reads a colour it cannot name by its component count, up to four in all, so a
    // vendor space is drawn over one to four channels and the opacity channel only fits under
    // three: past that it refuses the file the writer wrote correctly.
    let colours = space
        .channels()
        .unwrap_or_else(|| usize::from(head[5] % 4) + 1);
    let opacity = match head[3] % 3 {
        _ if space.channels().is_none() && colours == 4 => None,
        0 => None,
        1 => Some(Opacity::Plain),
        _ => Some(Opacity::Premultiplied),
    };
    let channels = colours.saturating_add(usize::from(opacity.is_some()));
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
    let signed: Vec<bool> = (0..channels)
        .map(|channel| head[4] >> (channel % 8) & 1 == 1)
        .collect();
    let file = match file(
        (width, height),
        space,
        &given[..colours],
        opacity.and_then(|kind| given.get(colours).map(|plane| (*plane, kind))),
        &signed,
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
    match pdf_model::jpeg2000::Headers::parse(&file) {
        Ok(headers) => {
            let states: Vec<bool> = headers
                .codestream
                .map_or_else(Vec::new, |stated| stated.depths)
                .iter()
                .map(|depth| depth.signed)
                .collect();
            assert!(states == signed, "the sign bits changed");
        }
        Err(error) => panic!("the written file's headers do not parse: {error}"),
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
