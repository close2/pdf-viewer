//! The writer's proof: what it writes, the tree's own JPEG 2000 decoder reads back sample for
//! sample, at every depth it takes (ADR 2470).
//!
//! The decoder is `pdf-sandbox`'s, asked as a writer asks it ([`Request::JpxWhole`]), so the
//! integers compared are the ones a redaction would carry. The grids are odd-sized and pass a
//! 64-sample code-block edge, every depth's two extremes appear, and the rest is a fixed
//! pseudo-random sequence, so every coding pass and both of D.3.4's cleanup branches are met.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a test states its expectations by failing, over grids of a few thousand samples"
)]

use pdf_jpx_write::{Colour, Error, MAX_DEPTH, Opacity, Plane, jp2};
use pdf_sandbox::{Decoded, Raster, Request};

/// A grid wider than one code-block and taller than one stripe set, odd in both directions.
const GRID: (u32, u32) = (71, 67);

/// `count` samples below 2^`depth`: zero and the largest every seventh, a long run of zeros so
/// D.3.4's run-length branch is taken, and a xorshift sequence otherwise.
fn samples(depth: u8, count: usize, seed: u64) -> Vec<u32> {
    let mask = (1u64 << depth) - 1;
    let mut state = seed | 1;
    (0..count)
        .map(|index| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let value = match index % 7 {
                _ if (300..900).contains(&index) => 0,
                0 => 0,
                1 => mask,
                _ => state & mask,
            };
            u32::try_from(value).expect("masked below 2^24")
        })
        .collect()
}

/// The tree's decode of `file`, as a writer asks for it.
fn decode(file: &[u8]) -> Raster {
    let request = Request::JpxWhole {
        data: file,
        indices: false,
        samples: 1 << 24,
    };
    match pdf_sandbox::decode_here(&request) {
        Ok(Decoded::Raster(raster)) => raster,
        Ok(_) => panic!("a JPEG 2000 file decodes to a raster"),
        Err(error) => panic!("the written file does not decode: {error}"),
    }
}

/// Writes `planes` and reads every sample back.
fn round_trip(colour: Colour, planes: &[Vec<u32>], depths: &[u8], opacity: Option<Opacity>) {
    let colours = colour.channels();
    let all: Vec<Plane<'_>> = planes
        .iter()
        .zip(depths)
        .map(|(samples, depth)| Plane {
            depth: *depth,
            samples,
        })
        .collect();
    let file = jp2(
        GRID,
        colour,
        &all[..colours],
        opacity.map(|kind| (all[colours], kind)),
    )
    .expect("the planes fit");
    let raster = decode(&file);
    assert_eq!((raster.width, raster.height), GRID);
    assert_eq!(usize::from(raster.components), colours);
    assert_eq!(raster.has_opacity, opacity.is_some());
    let channels = raster.channels();
    assert_eq!(channels, planes.len());
    for (channel, depth) in depths.iter().enumerate() {
        assert_eq!(
            raster.depth_of(channel),
            *depth,
            "channel {channel}'s depth"
        );
    }
    let pixels = planes[0].len();
    for pixel in 0..pixels {
        for (channel, plane) in planes.iter().enumerate() {
            assert_eq!(
                raster.sample(pixel * channels + channel),
                Some(plane[pixel]),
                "depth {depths:?}, pixel {pixel}, channel {channel}"
            );
        }
    }
}

/// Every depth the writer takes, one component and three, reads back exactly.
#[test]
fn every_depth_reads_back_exactly() {
    let count = usize::try_from(GRID.0 * GRID.1).expect("small");
    for depth in 1..=MAX_DEPTH {
        let grey = vec![samples(depth, count, u64::from(depth))];
        round_trip(Colour::Greyscale, &grey, &[depth], None);
        let rgb: Vec<Vec<u32>> = (0..3)
            .map(|c| samples(depth, count, u64::from(depth) * 31 + c))
            .collect();
        round_trip(Colour::Srgb, &rgb, &[depth; 3], None);
    }
}

/// Components at different depths keep each its own, and an opacity channel of either kind is
/// read back as the opacity channel.
#[test]
fn mixed_depths_and_opacity_read_back_exactly() {
    let count = usize::try_from(GRID.0 * GRID.1).expect("small");
    let depths = [20u8, 12, 24, 17];
    let planes: Vec<Vec<u32>> = depths
        .iter()
        .zip(1u64..)
        .map(|(depth, seed)| samples(*depth, count, seed))
        .collect();
    round_trip(Colour::Srgb, &planes[..3], &depths[..3], None);
    round_trip(Colour::Srgb, &planes, &depths, Some(Opacity::Plain));
    round_trip(Colour::Srgb, &planes, &depths, Some(Opacity::Premultiplied));
    round_trip(
        Colour::Greyscale,
        &[planes[0].clone(), planes[3].clone()],
        &[depths[0], depths[3]],
        Some(Opacity::Plain),
    );
}

/// A grid of one sample, and one whose every sample is zero, are images too.
#[test]
fn the_smallest_and_the_emptiest_grids_read_back() {
    let one = jp2(
        (1, 1),
        Colour::Greyscale,
        &[Plane {
            depth: 19,
            samples: &[0x5_A5A5],
        }],
        None,
    )
    .expect("one sample");
    assert_eq!(decode(&one).sample(0), Some(0x5_A5A5));
    let zeros = vec![0u32; 70 * 9];
    let blank = jp2(
        (70, 9),
        Colour::Greyscale,
        &[Plane {
            depth: 18,
            samples: &zeros,
        }],
        None,
    )
    .expect("zeros");
    let raster = decode(&blank);
    assert!((0..zeros.len()).all(|index| raster.sample(index) == Some(0)));
}

/// What the writer cannot prove it writes exactly is refused by name.
#[test]
fn a_depth_past_the_proof_and_a_sample_past_its_depth_are_refused() {
    let samples = [0u32; 4];
    assert_eq!(
        jp2(
            (2, 2),
            Colour::Greyscale,
            &[Plane {
                depth: 25,
                samples: &samples
            }],
            None
        ),
        Err(Error::Depth {
            plane: 0,
            depth: 25
        })
    );
    assert_eq!(
        jp2(
            (2, 2),
            Colour::Greyscale,
            &[Plane {
                depth: 3,
                samples: &[0, 8, 0, 0]
            }],
            None
        ),
        Err(Error::Sample {
            plane: 0,
            index: 1,
            value: 8
        })
    );
    assert!(matches!(
        jp2(
            (2, 2),
            Colour::Srgb,
            &[Plane {
                depth: 8,
                samples: &samples
            }],
            None
        ),
        Err(Error::ColourChannels { .. })
    ));
}
