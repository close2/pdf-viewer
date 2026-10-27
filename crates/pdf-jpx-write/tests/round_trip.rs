//! The writer's proof: what it writes, the tree's own JPEG 2000 decoder reads back sample for
//! sample, at every depth it takes (ADR 2470), and the boxes around the codestream say what the
//! writer meant them to (ADR 2480).
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

use pdf_jpx_write::{Colour, ColourBox, Error, MAX_DEPTH, Opacity, Plane, Space, file, jp2};
use pdf_model::jpeg2000::Headers;
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

/// `count` samples of a plane per depth, for a file of `depths.len()` planes.
fn planes_of(depths: &[u8], seed: u64) -> Vec<Vec<u32>> {
    let count = usize::try_from(GRID.0 * GRID.1).expect("small");
    depths
        .iter()
        .zip(seed..)
        .map(|(depth, seed)| samples(*depth, count, seed))
        .collect()
}

/// Every sample of `planes`, interleaved by pixel, equals the decode's.
fn assert_reads_back(raster: &Raster, planes: &[Vec<u32>], depths: &[u8]) {
    assert_eq!((raster.width, raster.height), GRID);
    assert_eq!(raster.channels(), planes.len());
    for (channel, depth) in depths.iter().enumerate() {
        assert_eq!(
            raster.depth_of(channel),
            *depth,
            "channel {channel}'s depth"
        );
    }
    for pixel in 0..planes[0].len() {
        for (channel, plane) in planes.iter().enumerate() {
            assert_eq!(
                raster.sample(pixel * planes.len() + channel),
                Some(plane[pixel]),
                "pixel {pixel}, channel {channel}"
            );
        }
    }
}

/// Four colour channels at depths past sixteen go into a JPX file whose boxes state what
/// ITU-T T.801 requires of one, and read back exactly.
///
/// M.8 has a JPX file's `BR` field `jpx\040` with a compatibility entry naming it, M.11.1 has
/// exactly one Reader Requirements box immediately after the file type box, and M.11.7.2
/// has the colour specification box state an enumerated space, here 12 (T.801 Table M.25,
/// CMYK), with an `APPROX` of 1 to 4. Opacity of either kind rides along as the fifth channel.
#[test]
fn four_deep_colour_channels_are_a_jpx_file_that_reads_back() {
    let depths = [20u8, 24, 17, 22, 19];
    let planes = planes_of(&depths, 11);
    let colours: Vec<Plane<'_>> = planes[..4]
        .iter()
        .zip(depths)
        .map(|(samples, depth)| Plane { depth, samples })
        .collect();
    for opacity in [None, Some(Opacity::Plain), Some(Opacity::Premultiplied)] {
        let alpha = Plane {
            depth: depths[4],
            samples: &planes[4],
        };
        let written = file(
            GRID,
            Space::Cmyk,
            &colours,
            opacity.map(|kind| (alpha, kind)),
            &[],
        )
        .expect("four planes fit CMYK");
        let headers = Headers::parse(&written).expect("a JPX file parses");
        let file_type = headers.file_type.as_ref().expect("an ftyp box");
        assert_eq!(&file_type.brand, b"jpx ");
        assert_eq!(&headers.top_level[..3], &[*b"jP  ", *b"ftyp", *b"rreq"]);
        assert_eq!(headers.colour.len(), 1);
        let colour = headers.colour[0];
        assert_eq!((colour.method, colour.enumerated), (1, Some(12)));
        assert!((1..=4).contains(&colour.approximation));
        let used = if opacity.is_some() { 5 } else { 4 };
        assert_eq!(headers.component_depths().len(), used);
        let raster = decode(&written);
        assert_eq!(raster.components, 4);
        assert_eq!(raster.has_opacity, opacity.is_some());
        assert_reads_back(&raster, &planes[..used], &depths[..used]);
    }
}

/// A colour specification the producer stated is written as it was, for a count of colour
/// channels no enumerated space has, and the file reads back exactly.
#[test]
fn a_stated_colour_box_is_carried_into_the_file_unchanged() {
    let depths = [18u8, 21, 24, 19, 20];
    let planes = planes_of(&depths, 5);
    let all: Vec<Plane<'_>> = planes
        .iter()
        .zip(depths)
        .map(|(samples, depth)| Plane { depth, samples })
        .collect();
    // M.11.7.3.3: a UUID and no parameters; five colour channels have no enumerated space.
    let vendor_uuid = *b"\x6b\xa7\xb8\x10\x9d\xad\x11\xd1\x80\xb4\x00\xc0\x4f\xd4\x30\xc8";
    let stated = ColourBox {
        method: 4,
        precedence: 0,
        approximation: 4,
        data: &vendor_uuid,
    };
    let written = file(GRID, Space::Stated(stated), &all, None, &[]).expect("five planes fit");
    let headers = Headers::parse(&written).expect("it parses");
    assert_eq!(headers.colour.len(), 1);
    let colour = headers.colour[0];
    assert_eq!((colour.method, colour.approximation), (4, 4));
    assert_eq!(colour.reserved, vendor_uuid);
    let raster = decode(&written);
    assert_eq!(raster.components, 5);
    assert_reads_back(&raster, &planes, &depths);
    // M.11.7.3.2: an ICC profile is the box's whole METHDAT.
    let profile = vec![7u8; 128];
    let icc = ColourBox {
        method: 3,
        precedence: 0,
        approximation: 1,
        data: &profile,
    };
    let written = file(GRID, Space::Stated(icc), &all[..4], None, &[]).expect("four planes fit");
    let headers = Headers::parse(&written).expect("it parses");
    assert_eq!(headers.colour[0].profile, Some(profile.as_slice()));
    assert_reads_back(&decode(&written), &planes[..4], &depths[..4]);
}

/// A signed component keeps its sign and its integers: only the sign bits of `Ssiz` and `BPC`
/// differ from the unsigned plane's file, and the decode is the same integers.
///
/// ISO/IEC 15444-1 Table A-11 marks a signed component with the high bit of `Ssiz`, Table I-6
/// the same for `BPC`; G.1.2 shifts only an unsigned component's DC level, so the coded bytes of
/// a signed plane carried as `s + 2^(depth − 1)` are an unsigned plane's.
#[test]
fn a_signed_component_differs_from_an_unsigned_one_in_its_sign_bits_alone() {
    let depths = [12u8, 20, 8];
    let planes = planes_of(&depths, 3);
    let all: Vec<Plane<'_>> = planes
        .iter()
        .zip(depths)
        .map(|(samples, depth)| Plane { depth, samples })
        .collect();
    let flags = [true, false, true];
    let unsigned = file(GRID, Space::Srgb, &all, None, &[]).expect("fits");
    let signed = file(GRID, Space::Srgb, &all, None, &flags).expect("fits");
    assert_eq!(unsigned.len(), signed.len());
    let headers = Headers::parse(&signed).expect("it parses");
    let found: Vec<(u8, bool)> = headers
        .component_depths()
        .iter()
        .map(|depth| (depth.bits, depth.signed))
        .collect();
    assert_eq!(found, [(12, true), (20, false), (8, true)]);
    let codestream = headers.codestream.as_ref().expect("a codestream");
    assert_eq!(
        codestream
            .depths
            .iter()
            .map(|depth| depth.signed)
            .collect::<Vec<_>>(),
        flags,
        "the SIZ marker states the same signs"
    );
    let differing = unsigned
        .iter()
        .zip(&signed)
        .filter(|(left, right)| left != right)
        .count();
    // Two components are signed: their `Ssiz` bytes and their `bpcc` bytes.
    assert_eq!(differing, 4, "four sign bits and no other byte");
    assert_reads_back(&decode(&signed), &planes, &depths);
}

/// Flags that do not number the planes, a colour box JPX cannot hold and a channel count a
/// space does not have are refused by name.
#[test]
fn a_file_the_standard_cannot_hold_is_refused() {
    let samples = [0u32; 4];
    let plane = Plane {
        depth: 8,
        samples: &samples,
    };
    let four = [plane; 4];
    assert_eq!(
        file((2, 2), Space::Cmyk, &four[..3], None, &[]),
        Err(Error::SpaceChannels {
            expected: 4,
            planes: 3
        })
    );
    assert_eq!(
        file((2, 2), Space::Cmyk, &four, None, &[true]),
        Err(Error::Signed {
            flags: 1,
            planes: 4
        })
    );
    for (method, approximation, data) in [
        (5u8, 1u8, &[0u8; 4][..]),
        (1, 0, &[0, 0, 0, 12]),
        (1, 5, &[0, 0, 0, 12]),
        (1, 1, &[0, 0, 12]),
        (4, 1, &[0; 15]),
        (3, 1, &[]),
    ] {
        let stated = ColourBox {
            method,
            precedence: 0,
            approximation,
            data,
        };
        assert_eq!(
            file((2, 2), Space::Stated(stated), &four, None, &[]),
            Err(Error::ColourBox),
            "method {method}, approximation {approximation}, {} bytes",
            data.len()
        );
    }
}
