//! Unit tests for the image module's internals that name items of two of its modules or more; a
//! test of one module sits in that module (ADR 1912 §2). Each names the clause or the ADR whose
//! claim it checks.

use super::convert::convert_three;
use crate::colour::ColourSpace;

/// A calibrated space whose conversion is neither the identity nor a device one.
///
/// `CalRGB` with the sRGB primaries and a gamma of 2.2, which puts every sample through
/// [`crate::colour::cie_to_srgb`] and so through the transcendental functions the memo
/// exists to avoid — the same shape as the `ICCBased` photograph this split was measured on,
/// without needing a profile in a test.
fn calibrated() -> ColourSpace {
    ColourSpace::CalRgb {
        white: [0.9505, 1.0, 1.089],
        black: [0.0; 3],
        gamma: [2.2; 3],
        matrix: [
            0.4124, 0.2126, 0.0193, 0.3576, 0.7152, 0.1192, 0.1805, 0.0722, 0.9505,
        ],
    }
}

/// A photograph-shaped raster: bands of repeated colour, with a run that crosses every cut.
fn raster(pixels: usize) -> Vec<u8> {
    (0..pixels)
        .flat_map(|index| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a sample value, deliberately wrapped to repeat"
            )]
            let value = (index % 37) as u8;
            [value, value.wrapping_mul(3), value.wrapping_add(200), 255]
        })
        .collect()
}

/// **The picture does not depend on how the raster was divided.**
///
/// The property the parallel split rests on, checked the way `render-cpu`'s
/// `strip_parallelism` checks its own: run the same conversion at several band sizes and
/// demand the bytes are identical. It holds here for a reason that does *not* hold there —
/// [`SampleMemo`] memoises a pure function of a sample tuple, so a band boundary changes
/// which conversions are *repeated* and never which answer is given, while a curve clipped
/// by a strip's edge is genuinely a different curve (ADR 0138).
///
/// Band sizes 1 and 3 are the interesting ones: they make the memo miss almost every time,
/// which is exactly the case a wrong key would show up in.
///
/// [`SampleMemo`]: super::convert::SampleMemo
#[test]
fn a_band_boundary_changes_no_pixel() {
    let space = calibrated();
    let pixels = 5_000;
    let decode = super::space::Decode::from_pairs(
        &[],
        &super::space::ColourSpace::Resolved(space.clone()),
        8,
    );
    let mut serial = raster(pixels);
    convert_three(
        &space,
        &decode,
        &mut serial,
        None,
        &crate::colour::Conversion::device(),
    );
    // A conversion that changed nothing would pass every comparison below.
    assert_ne!(serial, raster(pixels), "the space converts nothing");

    for band in [1, 3, 64, 999, pixels, pixels * 2] {
        let mut split = raster(pixels);
        convert_three(
            &space,
            &decode,
            &mut split,
            Some(band),
            &crate::colour::Conversion::device(),
        );
        assert_eq!(split, serial, "band of {band} pixels");
    }
}

/// **A pixel's converted bytes are its own samples' conversion, whatever came before it.**
///
/// [`convert_three`] and [`convert_four`] answer a pixel equal to the one before it from that
/// one, and every other from the memo or the conversion (ADR 1469); this asks the conversion
/// itself for every pixel of a raster of runs, single pixels and alphas that differ inside a
/// run, and demands the same bytes — the alpha byte kept by the first, written opaque by the
/// second, which is what each did before. The second half's runs of 23 pixels cover whole blocks
/// of [`convert_three`]'s run test and end inside others, and the raster's last three pixels are
/// a tail no block reaches (ADR 2364).
#[test]
fn a_converted_pixel_is_its_own_samples_conversion() {
    let into = crate::colour::Conversion::device();
    let pixels: Vec<[u8; 4]> = (0..8_003usize)
        .map(|index| {
            // Runs of 1 to 9 equal samples, then of 23, under alphas that change inside them.
            let run = if index < 4_000 {
                index / (1 + index % 9)
            } else {
                4_000 + (index - 4_000) / 23
            };
            let value = u8::try_from(run % 251).expect("under 256");
            let alpha = u8::try_from(index % 256).expect("under 256");
            [value, value.wrapping_mul(7), value.wrapping_add(90), alpha]
        })
        .collect();
    let three = calibrated();
    let decode = super::space::Decode::from_pairs(
        &[],
        &super::space::ColourSpace::Resolved(three.clone()),
        8,
    );
    let mut converted: Vec<u8> = pixels.iter().flatten().copied().collect();
    convert_three(&three, &decode, &mut converted, None, &into);
    let four = ColourSpace::Cmyk;
    let decode_four = super::space::Decode::from_pairs(
        &[],
        &super::space::ColourSpace::Resolved(four.clone()),
        8,
    );
    let mut converted_four: Vec<u8> = pixels.iter().flatten().copied().collect();
    super::convert::convert_four(&four, &decode_four, &mut converted_four, None, &into);
    for (index, pixel) in pixels.iter().enumerate() {
        let value = |decode: &super::space::Decode, component: usize| {
            decode.value(component, usize::from(pixel[component]))
        };
        let colour = into.paint(
            &three,
            &[value(&decode, 0), value(&decode, 1), value(&decode, 2)],
        );
        let wanted = [
            super::convert::channel(colour.r),
            super::convert::channel(colour.g),
            super::convert::channel(colour.b),
            pixel[3],
        ];
        assert_eq!(converted[index * 4..index * 4 + 4], wanted, "pixel {index}");
        let colour = into.paint(
            &four,
            &[
                value(&decode_four, 0),
                value(&decode_four, 1),
                value(&decode_four, 2),
                value(&decode_four, 3),
            ],
        );
        let wanted = [
            super::convert::channel(colour.r),
            super::convert::channel(colour.g),
            super::convert::channel(colour.b),
            255,
        ];
        assert_eq!(
            converted_four[index * 4..index * 4 + 4],
            wanted,
            "pixel {index}"
        );
    }
}

/// **A soft mask multiplied in place is the combination [`apply_soft_mask`] would have built.**
///
/// On a pair on one grid, at a size the pool divides and one it does not, every byte of
/// [`opacity_multiplied_in_place`]'s raster is [`combine_on_the_finer_grid`]'s
/// under the same closure; and a raster something else still holds, or a mask on another
/// grid, is left untouched for that function to combine (ADR 1469).
///
/// [`opacity_multiplied_in_place`]: super::combine::opacity_multiplied_in_place
/// [`combine_on_the_finer_grid`]: super::combine::combine_on_the_finer_grid
#[test]
fn a_mask_multiplied_in_place_is_the_combination() {
    let raster = |width: u32, height: u32, seed: usize| pdf_render::Image {
        width,
        height,
        data: (0..(width * height * 4) as usize)
            .map(|index| u8::try_from((index * 53 + seed) % 256).expect("under 256"))
            .collect::<Vec<u8>>()
            .into(),
        interpolate: false,
        sample_alpha: pdf_render::SampleAlpha::Opacity,
    };
    let plane = |raster: pdf_render::Image| {
        super::soft_mask::MaskPlane::of_decoded(super::Flattened {
            image: raster,
            shortfall: None,
        })
    };
    for (width, height) in [(7, 5), (640, 512)] {
        let (image, mask) = (raster(width, height, 1), raster(width, height, 7));
        let wanted = super::combine::combine_on_the_finer_grid(
            &image,
            super::combine::MaskSamples::of_raster(&mask),
            |colour, sample| (colour, sample[0]),
        );
        let mask = plane(mask);
        // The mask held as a plane combines to the bytes it did as the raster it came from.
        let combined =
            super::combine::combine_on_the_finer_grid(&image, mask.samples(), |colour, sample| {
                (colour, sample[0])
            });
        assert_eq!(combined.data, wanted.data, "{width}x{height} as a plane");
        let mut multiplied = image.clone();
        // The clone shares the samples, so nothing may be written through it.
        assert!(!super::combine::opacity_multiplied_in_place(
            &mut multiplied,
            mask.samples()
        ));
        assert_eq!(multiplied.data, image.data);
        drop(image);
        assert!(super::combine::opacity_multiplied_in_place(
            &mut multiplied,
            mask.samples()
        ));
        assert_eq!(multiplied.data, wanted.data, "{width}x{height}");
    }
    let mut image = raster(6, 4, 1);
    assert!(!super::combine::opacity_multiplied_in_place(
        &mut image,
        plane(raster(3, 4, 7)).samples()
    ));
}

/// An eight-bit grey or RGB image unpacked by the table route is the image the per-sample
/// route builds, pixel for pixel: a `/Decode` that is not the identity and the default one,
/// whose tables the route leaves unread (ADR 2348), a width whose rows are not a multiple of
/// eight bytes, and a stream that stops two and a half samples into its last row, whose
/// uncarried samples §7.3.8.2 leaves unpainted (ADR 1433).
#[test]
fn the_table_route_unpacks_the_per_sample_routes_pixels() {
    for (space, stated) in [
        (super::space::ColourSpace::Gray, vec![1.0_f32, 0.0]),
        (
            super::space::ColourSpace::Rgb,
            vec![0.2, 0.9, 1.0, 0.0, 0.0, 0.5],
        ),
        (super::space::ColourSpace::Gray, vec![]),
        (super::space::ColourSpace::Rgb, vec![]),
    ] {
        let decode = super::space::Decode::from_pairs(&stated, &space, 8);
        assert_eq!(decode.is_identity(), stated.is_empty(), "{space:?}");
        let into = super::Conversion::device();
        let samples = super::unpack::Samples {
            bits: 8,
            space: &space,
            decode: &decode,
            colour_key: None,
            fill: pdf_render::Color::BLACK,
            into: &into,
            matte: None,
        };
        let (width, height) = (13_usize, 7_usize);
        let components = space.components();
        let row_bytes = width * components;
        let short = row_bytes * (height - 1) + components * 2 + 1;
        let data: Vec<u8> = (0..short)
            .map(|index| u8::try_from((index * 37 + 11) % 256).expect("under 256"))
            .collect();
        assert!(super::unpack::eight_bit_device_tables(&samples).is_some());
        let tabled = super::unpack::unpack(
            &data,
            u32::try_from(width).expect("small"),
            u32::try_from(height).expect("small"),
            &samples,
        )
        .expect("unpacked");
        let mut expected = Vec::new();
        for y in 0..height {
            let start = (y * row_bytes).min(data.len());
            let row = &data[start..(start + row_bytes).min(data.len())];
            let carried = row.len() / components;
            for x in 0..width {
                if x < carried {
                    expected.extend_from_slice(&super::unpack::sample_rgba(
                        &samples,
                        None,
                        &mut None,
                        row,
                        x,
                        y * width + x,
                    ));
                } else {
                    expected.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }
        assert_eq!(tabled, expected, "{space:?}");
    }
}

/// The per-sample route's pixels for a one-component image, built from
/// [`sample_rgba`] row by row with §7.3.8.2's unpainted tail — what the byte route
/// must reproduce.
///
/// [`sample_rgba`]: super::unpack::sample_rgba
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a test helper over a grid of at most nineteen by five samples"
)]
fn per_sample(
    samples: &super::unpack::Samples,
    palette: Option<&[pdf_render::Color]>,
    data: &[u8],
    (width, height): (usize, usize),
) -> Vec<u8> {
    let bits = usize::try_from(samples.bits).expect("a depth");
    let row_bytes = (width * bits).div_ceil(8);
    let mut expected = Vec::new();
    for y in 0..height {
        let start = (y * row_bytes).min(data.len());
        let row = &data[start..(start + row_bytes).min(data.len())];
        for x in 0..width {
            if x < row.len() * 8 / bits {
                expected.extend_from_slice(&super::unpack::sample_rgba(
                    samples,
                    palette,
                    &mut None,
                    row,
                    x,
                    y * width + x,
                ));
            } else {
                expected.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    expected
}

/// Whether the byte route takes `samples` and answers every width from one sample to past
/// two bytes with the per-sample route's pixels, over rows that end inside a byte (§8.9.3's
/// padding, which no pixel may be read from) and a stream that stops partway into its last
/// row.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a test helper over a grid of at most nineteen by five samples"
)]
fn the_byte_route_matches(
    samples: &super::unpack::Samples,
    palette: Option<&[pdf_render::Color]>,
    what: &str,
) {
    assert!(
        super::unpack::packed_pixels(samples, palette).is_some(),
        "{what} takes the byte route"
    );
    let bits = usize::try_from(samples.bits).expect("a depth");
    for width in 1..=19_usize {
        let height = 5_usize;
        let row_bytes = (width * bits).div_ceil(8);
        let short = row_bytes * (height - 1) + row_bytes.div_ceil(2);
        let data: Vec<u8> = (0..short)
            .map(|index| u8::try_from((index * 89 + 23) % 256).expect("under 256"))
            .collect();
        let tabled = super::unpack::unpack(
            &data,
            u32::try_from(width).expect("small"),
            u32::try_from(height).expect("small"),
            samples,
        )
        .expect("unpacked");
        assert_eq!(
            tabled,
            per_sample(samples, palette, &data, (width, height)),
            "{what}, {width} wide"
        );
    }
}

/// ISO 32000-2 §8.9.6.2's stencil through the byte route: under the default `[0 1]` "a sample
/// value of 0 shall mark the page with the current colour, and a 1 shall leave the previous
/// contents unchanged", and `[1 0]` reverses the two — each pixel the per-sample route's, in a
/// current colour that is not opaque (ADR 1557).
#[test]
fn a_stencil_unpacks_a_byte_at_a_time_to_the_per_sample_routes_pixels() {
    let space = super::space::ColourSpace::Mask;
    let into = super::Conversion::device();
    for stated in [vec![], vec![0.0_f32, 1.0], vec![1.0, 0.0]] {
        let decode = super::space::Decode::from_pairs(&stated, &space, 1);
        let samples = super::unpack::Samples {
            bits: 1,
            space: &space,
            decode: &decode,
            colour_key: None,
            fill: pdf_render::Color {
                r: 0.8,
                g: 0.1,
                b: 0.3,
                a: 0.6,
            },
            into: &into,
            matte: None,
        };
        the_byte_route_matches(&samples, None, &format!("a stencil under {stated:?}"));
    }
}

/// ISO 32000-2 §8.9.5.2's `/Decode` on a `DeviceGray` image at one, two and four bits
/// through the byte route: the default, the inverting `[1 0]` and a general pair whose two
/// ends are neither black nor white each give the per-sample route's pixels. A §8.9.6.4
/// colour key keeps the image on the per-sample route, because its test is on the raw sample
/// (ADR 1557).
#[test]
fn a_grey_image_unpacks_a_byte_at_a_time_under_its_decode_array() {
    let space = super::space::ColourSpace::Gray;
    let into = super::Conversion::device();
    for bits in [1, 2, 4] {
        for stated in [vec![], vec![1.0_f32, 0.0], vec![0.2, 0.7]] {
            let decode = super::space::Decode::from_pairs(&stated, &space, bits);
            let samples = super::unpack::Samples {
                bits,
                space: &space,
                decode: &decode,
                colour_key: None,
                fill: pdf_render::Color::BLACK,
                into: &into,
                matte: None,
            };
            the_byte_route_matches(&samples, None, &format!("{bits}-bit grey, {stated:?}"));
            let keyed = super::unpack::Samples {
                colour_key: Some(&[(1, 1)]),
                ..samples
            };
            assert!(super::unpack::packed_pixels(&keyed, None).is_none());
        }
    }
}

/// ISO 32000-2 §8.6.6.3's `Indexed` space through the byte route at each of Table 87's
/// depths to eight, over a table shorter than the depth can index — so that the samples past
/// `hival` meet [`palette`]'s answer for them on both routes — and under an inverting
/// `/Decode` (ADR 1557).
///
/// [`palette`]: super::unpack::palette
#[test]
fn an_indexed_image_unpacks_a_byte_at_a_time_at_every_depth_to_eight() {
    let into = super::Conversion::device();
    for bits in [1, 2, 4, 8] {
        let high = (1_usize << bits) / 2 + 1;
        let lookup: Vec<f32> = (0..=high)
            .flat_map(|entry| {
                let at = f32::from(u8::try_from(entry % 256).expect("under 256")) / 255.0;
                [at, 1.0 - at, 0.5]
            })
            .collect();
        let resolved = ColourSpace::Indexed {
            base: Box::new(ColourSpace::Rgb),
            lookup,
            high,
        };
        let space = super::space::ColourSpace::Resolved(resolved.clone());
        let top = f32::from(u16::try_from((1_u32 << bits) - 1).expect("at most 255"));
        for stated in [vec![], vec![top, 0.0]] {
            let decode = super::space::Decode::from_pairs(&stated, &space, bits);
            let palette = super::unpack::palette(&resolved, bits, &decode, &into);
            let samples = super::unpack::Samples {
                bits,
                space: &space,
                decode: &decode,
                colour_key: None,
                fill: pdf_render::Color::BLACK,
                into: &into,
                matte: None,
            };
            the_byte_route_matches(
                &samples,
                Some(&palette),
                &format!("{bits}-bit indexed, {stated:?}"),
            );
        }
    }
}

/// The tabulated inversion of §11.6.5.2's pre-blending writes the per-sample route's pixels:
/// grey and RGB under a `/Decode` that is not the identity, a matte inside and one at the
/// range's edge, every mask value from 0 to 255 met, a row the mask leaves wholly transparent,
/// which the inversion in place writes without its samples (ADR 2348), a mask shorter than the
/// image (whose missing values are 0), and a stream that stops short of its last row (ADR 1457).
#[test]
fn the_matte_table_route_unpacks_the_per_sample_routes_pixels() {
    for (space, stated, matte) in [
        (
            super::space::ColourSpace::Gray,
            vec![1.0_f32, 0.0],
            vec![0.25_f32],
        ),
        (
            super::space::ColourSpace::Rgb,
            vec![0.2, 0.9, 1.0, 0.0, 0.0, 0.5],
            vec![0.0, 1.0, 0.6],
        ),
    ] {
        let decode = super::space::Decode::from_pairs(&stated, &space, 8);
        let into = super::Conversion::device();
        let (width, height) = (37_usize, 9_usize);
        let mut alpha: Vec<u8> = (0..width * height - 5)
            .map(|index| u8::try_from((index * 53 + 7) % 256).expect("under 256"))
            .collect();
        alpha[width..2 * width].fill(0);
        let prematte = super::unpack::Prematte {
            matte: &matte,
            alpha: &alpha,
            opacity: false,
        };
        let samples = super::unpack::Samples {
            bits: 8,
            space: &space,
            decode: &decode,
            colour_key: None,
            fill: pdf_render::Color::BLACK,
            into: &into,
            matte: Some(&prematte),
        };
        let components = space.components();
        let row_bytes = width * components;
        let short = row_bytes * (height - 1) + components * 2 + 1;
        let data: Vec<u8> = (0..short)
            .map(|index| u8::try_from((index * 37 + 11) % 256).expect("under 256"))
            .collect();
        assert!(super::unpack::matted_eight_bit_device_tables(&samples, width, height).is_none());
        let inversions = super::unpack::matted_eight_bit_device_tables(&samples, 256, 256)
            .expect("an image at the floor is tabulated");
        let tabled = super::unpack::unpack_matted_eight_bit_device(
            &data,
            (width, height),
            (row_bytes, components),
            &inversions,
            &alpha,
        );
        let mut expected = Vec::new();
        for y in 0..height {
            let start = (y * row_bytes).min(data.len());
            let row = &data[start..(start + row_bytes).min(data.len())];
            let carried = row.len() / components;
            for x in 0..width {
                if x < carried {
                    expected.extend_from_slice(&super::unpack::sample_rgba(
                        &samples,
                        None,
                        &mut None,
                        row,
                        x,
                        y * width + x,
                    ));
                } else {
                    expected.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }
        assert_eq!(tabled, expected, "{space:?}");
        // The same samples four bytes a pixel, as a `DCTDecode` frame holds them: the rows
        // whose samples and mask values are whole take the fixed-size walk, the last row
        // (short of both) the general one, and every pixel is the per-sample route's.
        let mut framed = Vec::new();
        for pixel in data.chunks_exact(components) {
            framed.extend_from_slice(pixel);
            framed.resize(framed.len() + 4 - components, 0);
        }
        framed.extend_from_slice(&data[data.len() / components * components..]);
        let from_frame = super::unpack::unpack_matted_eight_bit_device(
            &framed,
            (width, height),
            (width * 4, 4),
            &inversions,
            &alpha,
        );
        assert_eq!(from_frame, expected, "{space:?}, four bytes a pixel");
        // And a frame that covers its grid under a mask that does, inverted where it lies: the
        // whole rows of the same fixture, every pixel the per-sample route's.
        let whole = width * (height - 1);
        let mut in_place = framed[..whole * 4].to_vec();
        assert!(super::dispatch::invert_matte_in_place(
            &mut in_place,
            (width, height - 1),
            &inversions,
            (&alpha, false),
        ));
        assert_eq!(in_place, expected[..whole * 4], "{space:?}, in place");
        assert!(!super::dispatch::invert_matte_in_place(
            &mut framed.clone(),
            (width, height),
            &inversions,
            (&alpha, false),
        ));
    }
}

/// §11.6.5.2's inversion with the soft mask's opacity written in the same pass is, byte for
/// byte, the inversion followed by the multiplication [`apply_soft_mask`] makes: every
/// mask value from 0 to 255 against every sample value, in grey and in RGB (ADR 1519). The
/// multiplication's `(255 × α + 127) ÷ 255` is `α` exactly, so no rounding is taken or saved.
///
/// [`apply_soft_mask`]: super::soft_mask::apply_soft_mask
#[test]
fn a_matte_inverted_with_its_opacity_is_the_inversion_then_the_multiplication() {
    for (space, matte) in [
        (super::space::ColourSpace::Gray, vec![0.25_f32]),
        (super::space::ColourSpace::Rgb, vec![0.0, 1.0, 0.6]),
    ] {
        let decode = super::space::Decode::from_pairs(&[], &space, 8);
        let into = super::Conversion::device();
        let (width, height) = (256_usize, 256_usize);
        let alpha: Vec<u8> = (0..width * height)
            .map(|index| u8::try_from(index / width).expect("a row under 256"))
            .collect();
        let prematte = super::unpack::Prematte {
            matte: &matte,
            alpha: &alpha,
            opacity: true,
        };
        let samples = super::unpack::Samples {
            bits: 8,
            space: &space,
            decode: &decode,
            colour_key: None,
            fill: pdf_render::Color::BLACK,
            into: &into,
            matte: Some(&prematte),
        };
        let tables = super::unpack::matted_eight_bit_device_tables(&samples, width, height)
            .expect("an image at the floor is tabulated");
        // Each row one mask value, each column one sample value in every component.
        let frame: Vec<u8> = (0..width * height)
            .flat_map(|index| {
                let sample = u8::try_from(index % width).expect("a column under 256");
                [sample, sample.wrapping_add(85), sample.wrapping_add(170), 0]
            })
            .collect();
        let mut fused = frame.clone();
        assert!(super::dispatch::invert_matte_in_place(
            &mut fused,
            (width, height),
            &tables,
            (&alpha, true),
        ));
        let mut inverted = frame;
        assert!(super::dispatch::invert_matte_in_place(
            &mut inverted,
            (width, height),
            &tables,
            (&alpha, false),
        ));
        let mut image = pdf_render::Image {
            width: u32::try_from(width).expect("256"),
            height: u32::try_from(height).expect("256"),
            data: inverted.into(),
            interpolate: false,
            sample_alpha: pdf_render::SampleAlpha::Shape,
        };
        let plane = super::combine::MaskSamples {
            width: image.width,
            height: image.height,
            data: &alpha,
            stride: 1,
        };
        assert!(super::combine::opacity_multiplied_in_place(
            &mut image, plane
        ));
        assert_eq!(&fused[..], &image.data[..], "{space:?}");
    }
}

/// The combined raster is, pixel for pixel, the nearest sample of each source on the finer
/// grid and their alphas' rounded product — asked of a pair on one grid, which takes the
/// paired walk, and of pairs whose grids differ in either axis or both, which take the
/// per-column one (ADR 1433).
#[test]
fn a_combined_raster_is_its_sources_nearest_samples() {
    let raster = |width: u32, height: u32, seed: usize| pdf_render::Image {
        width,
        height,
        data: (0..(width * height * 4) as usize)
            .map(|index| u8::try_from((index * 53 + seed) % 256).expect("under 256"))
            .collect::<Vec<u8>>()
            .into(),
        interpolate: false,
        sample_alpha: pdf_render::SampleAlpha::Opacity,
    };
    for ((iw, ih), (mw, mh)) in [((6, 4), (6, 4)), ((3, 4), (6, 5)), ((7, 2), (2, 9))] {
        let (image, mask) = (raster(iw, ih, 1), raster(mw, mh, 7));
        let combined = super::combine::combine_on_the_finer_grid(
            &image,
            super::combine::MaskSamples::of_raster(&mask),
            |colour, sample| (colour, sample[0]),
        );
        // The same mask held one byte a sample takes the same walk to the same bytes.
        let plane = super::soft_mask::MaskPlane::of_decoded(super::Flattened {
            image: mask.clone(),
            shortfall: None,
        });
        let from_plane =
            super::combine::combine_on_the_finer_grid(&image, plane.samples(), |colour, sample| {
                (colour, sample[0])
            });
        assert_eq!(from_plane.data, combined.data);
        let (width, height) = (iw.max(mw) as usize, ih.max(mh) as usize);
        assert_eq!(
            (combined.width as usize, combined.height as usize),
            (width, height)
        );
        for y in 0..height {
            for x in 0..width {
                let at = ((y * ih as usize / height) * iw as usize + x * iw as usize / width) * 4;
                let mask_at =
                    ((y * mh as usize / height) * mw as usize + x * mw as usize / width) * 4;
                let alpha =
                    (u16::from(image.data[at + 3]) * u16::from(mask.data[mask_at]) + 127) / 255;
                let out = (y * width + x) * 4;
                assert_eq!(
                    combined.data[out..out + 4],
                    [
                        image.data[at],
                        image.data[at + 1],
                        image.data[at + 2],
                        u8::try_from(alpha).expect("a byte")
                    ]
                );
            }
        }
    }
}
