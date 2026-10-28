//! A JPEG 2000 image with a component deeper than sixteen bits is written back as JPEG 2000
//! (`doc/adr/2484`).
//!
//! ISO 32000-2 Table 87 gives every filter but `JPXDecode` at most sixteen bits a sample, and
//! section 7.4.9 lets a JPEG 2000 component be deeper, so the `FlateDecode` copy the other
//! transcodes make has no field for the integers. The new data is a JP2 file of the tree's own
//! lossless writer, which ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 admit
//! inside ITU-T T.801 M.9.2's baseline. Every fixture is a baseline file whose one colour
//! specification is turned into enumerated code 3, which M.9.2.4's list does not hold, so the
//! data is off the baseline and nothing else about it is wrong.
use pdf_model::colour::Conversion;
use pdf_model::image::{JpxSamples, ORDINARY_JPX_SAMPLES};
use pdf_render::Color;
use pdf_syntax::object::Dictionary;
use pdf_syntax::{Document, Limits, Object};

use super::{
    Conforming, Decision, Level, Rewrite, Target, Verdict, conversion, convert_with_plan, decision,
    holds, keep_everything, plan_from, stream,
};

const ROW: &str = "graphics/jpeg2000-uses-the-baseline-feature-set";

/// `count` pseudo-random integers below 2^`depth`, the first 0 and the second the largest, so
/// that both ends of the range are in every plane.
fn deep_samples(depth: u8, count: usize, seed: u64) -> Vec<u32> {
    let mask = (1u64 << depth).saturating_sub(1);
    let mut state = seed | 1;
    (0..count)
        .map(|index| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let value = match index {
                0 => 0,
                1 => mask,
                _ => state & mask,
            };
            u32::try_from(value).expect("masked below 2^24")
        })
        .collect()
}

/// One JP2 box of type `kind` around `payload` (ISO/IEC 15444-1 I.4).
fn boxed(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = u32::try_from(payload.len().saturating_add(8))
        .expect("the fixture is small")
        .to_be_bytes()
        .to_vec();
    out.extend_from_slice(&kind);
    out.extend_from_slice(payload);
    out
}

/// An 8×8 file of `pdf_jpx_write` holding one plane of each depth in `depths` and, where
/// `opacity` gives a depth, an opacity channel after them; its colour specification is then
/// changed to enumerated code 3.
fn a_deep_jpx_off_the_baseline(depths: &[u8], opacity: Option<u8>) -> Vec<u8> {
    let planes: Vec<Vec<u32>> = depths
        .iter()
        .zip(3u64..)
        .map(|(depth, seed)| deep_samples(*depth, 64, seed))
        .collect();
    let colour = match depths.len() {
        1 => pdf_jpx_write::Colour::Greyscale,
        _ => pdf_jpx_write::Colour::Srgb,
    };
    let alpha = opacity.map(|depth| (depth, deep_samples(depth, 64, 41)));
    let mut file = pdf_jpx_write::jp2(
        (8, 8),
        colour,
        &planes
            .iter()
            .zip(depths)
            .map(|(samples, depth)| pdf_jpx_write::Plane {
                depth: *depth,
                samples,
            })
            .collect::<Vec<_>>(),
        alpha.as_ref().map(|(depth, samples)| {
            (
                pdf_jpx_write::Plane {
                    depth: *depth,
                    samples,
                },
                pdf_jpx_write::Opacity::Plain,
            )
        }),
    )
    .expect("the fixture's planes fit");
    // The colour specification box: its type, then METH, PREC, APPROX and the four-byte code.
    let at = file
        .windows(4)
        .position(|window| window == b"colr")
        .expect("the file has a colour specification");
    // The box's type is four bytes; METH, PREC and APPROX are three more, and the code's last
    // byte is the tenth after the type's start.
    *file
        .get_mut(at.saturating_add(10))
        .expect("the box holds its code") = 3;
    file
}

/// A part 2 page whose resources hold the image `data` under `/ColorSpace` `space`, with the
/// `extra` image entries.
fn a_page_with_the_image(data: &[u8], space: &str, extra: &str) -> Vec<u8> {
    Conforming {
        resources: "/XObject << /Im 6 0 R >>".to_owned(),
        binary_objects: vec![stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace {space} {extra} \
                 /Filter /JPXDecode /Length {}",
                data.len()
            ),
            data,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// The page's one image, `/Im`, from `bytes`.
fn the_image(held: &Document) -> std::sync::Arc<pdf_syntax::object::Stream> {
    let pages = pdf_model::Pages::new(held);
    let page = pages.get(0).expect("the one page");
    let Object::Stream(image) = held.get_key(
        held.get_key(&page.dict, "Resources")
            .as_dict()
            .map(|resources| held.get_key(resources, "XObject"))
            .expect("the page's resources")
            .as_dict()
            .expect("the page's XObjects"),
        "Im",
    ) else {
        panic!("the image is a stream");
    };
    image
}

/// The samples a reader decodes from the page's image, undisturbed by any colour step.
fn samples_of(held: &Document) -> JpxSamples {
    let image = the_image(held);
    pdf_model::image::jpx_samples(
        held,
        &image.dict,
        &Dictionary::new(),
        (&image.data, ORDINARY_JPX_SAMPLES),
    )
    .expect("the image decodes")
}

/// The eight-bit colours a reader draws from the page's image.
fn drawn(held: &Document) -> Vec<u8> {
    let image = the_image(held);
    let pages = pdf_model::Pages::new(held);
    let page = pages.get(0).expect("the one page");
    pdf_model::image::decode(
        held,
        &image,
        &page.resources,
        Color::BLACK,
        &Conversion::device(),
    )
    .expect("the image decodes")
    .image
    .data
    .to_vec()
}

/// The conversion of `source` under a profile whose `preserve` answers the baseline site.
fn converted(source: &[u8]) -> (pdf_transform::Report, Vec<u8>) {
    let target = Target::Two(Level::B);
    let plan = plan_from(&keep_everything(), target);
    let (report, output) = convert_with_plan(source, &plan);
    let answered = decision(&report, ROW);
    let Decision::Configured { rewrite, .. } = answered else {
        panic!("the profile's preserve answers the site: {answered:?}");
    };
    assert_eq!(rewrite, Rewrite::Jpeg2000TranscodedToFlate);
    let output = output.unwrap_or_else(|| panic!("{:?}", conversion(&report).decided));
    (report, output)
}

/// Three colour components of 20 bits: the integers go back into a `JPXDecode` stream at the
/// same depth with no `/Decode` restated, the colour space is the dictionary's own, the output
/// conforms, and a reader draws what it drew.
#[test]
fn components_deeper_than_sixteen_bits_are_written_back_as_jpeg_2000() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let data = a_deep_jpx_off_the_baseline(&[20, 20, 20], None);
    let source = a_page_with_the_image(&data, "/DeviceRGB", "");
    let target = Target::Two(Level::B);
    let failed: Vec<&str> = holds(&source, target)
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(failed.contains(&ROW), "{failed:?}");

    let (report, output) = converted(&source);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let rows = &conversion(&report).transcoded_images;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].depths, vec![20, 20, 20]);
    assert_eq!((rows[0].filter, rows[0].bits), ("JPXDecode", 20));
    assert!(
        rows[0].soft_mask.is_none(),
        "the opacity travels in the file"
    );

    let before = Document::open_with_limits(source, Limits::DEFAULT).expect("the source opens");
    let after = Document::open_with_limits(output, Limits::DEFAULT).expect("the output opens");
    let copy = the_image(&after);
    assert_eq!(
        after.get_key(&copy.dict, "Filter"),
        Object::Name(pdf_syntax::object::Name::new(&b"JPXDecode"[..]))
    );
    assert!(
        after.get_key(&copy.dict, "BitsPerComponent").is_null(),
        "Table 87 states none for JPXDecode"
    );
    assert!(after.get_key(&copy.dict, "Decode").is_null());
    assert_eq!(
        after.get_key(&copy.dict, "ColorSpace"),
        before.get_key(&the_image(&before).dict, "ColorSpace"),
        "the colour space is the dictionary's own"
    );
    let (was, is) = (samples_of(&before), samples_of(&after));
    assert_eq!(is.colour, was.colour, "every integer keeps its value");
    assert_eq!(is.depths, vec![20, 20, 20]);
    assert_eq!(drawn(&after), drawn(&before), "and is drawn as it was");
}

/// Colour components of 20, 18 and 24 bits: ISO 19005-2 section 6.2.8.3 requires one depth for
/// every colour channel of the JPEG 2000 data, so the integers are written at the widest, 24, and
/// the `/Decode` pair of each is restated so that section 8.9.5.2's map gives every integer the
/// value it had at its own depth: `D max` becomes `(2^24 - 1) / (2^n - 1)`.
#[test]
fn components_of_different_depths_are_written_at_the_widest_with_their_decode_widened() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let data = a_deep_jpx_off_the_baseline(&[20, 18, 24], None);
    let source = a_page_with_the_image(&data, "/DeviceRGB", "");
    let target = Target::Two(Level::B);
    let (report, output) = converted(&source);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let rows = &conversion(&report).transcoded_images;
    assert_eq!(rows[0].depths, vec![20, 18, 24]);
    assert_eq!((rows[0].filter, rows[0].bits), ("JPXDecode", 24));

    let before = Document::open_with_limits(source, Limits::DEFAULT).expect("the source opens");
    let after = Document::open_with_limits(output, Limits::DEFAULT).expect("the output opens");
    let (was, is) = (samples_of(&before), samples_of(&after));
    assert_eq!(was.depths, vec![20, 18, 24]);
    assert_eq!(
        is.depths,
        vec![24, 24, 24],
        "one depth for the colour channels"
    );
    assert_eq!(is.colour, was.colour, "every integer keeps its value");
    let copy = the_image(&after);
    let decode: Vec<f64> = after
        .get_key(&copy.dict, "Decode")
        .as_array()
        .expect("the pairs are restated")
        .iter()
        .filter_map(|item| after.resolve(item).as_number())
        .collect();
    let widened = |own: i32| f64::from(2f32.powi(24) - 1.0) / f64::from(2f32.powi(own) - 1.0);
    let expected = [0.0, widened(20), 0.0, widened(18), 0.0, 1.0];
    assert_eq!(decode.len(), expected.len(), "{decode:?}");
    for (found, wanted) in decode.iter().zip(expected) {
        assert!(
            (found - wanted).abs() < 1e-3,
            "{decode:?} against {expected:?}"
        );
    }
}

/// An opacity channel of its own depth stays in the file: `/SMaskInData` is carried as it stood,
/// no soft-mask image is made, and the premultiplication, which Table 87's code states, is the
/// producer's.
#[test]
fn a_deep_opacity_channel_stays_in_the_file() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    for code in [1, 2] {
        let data = a_deep_jpx_off_the_baseline(&[20, 20, 20], Some(20));
        let source = a_page_with_the_image(&data, "/DeviceRGB", &format!("/SMaskInData {code}"));
        let (report, output) = converted(&source);
        let rows = &conversion(&report).transcoded_images;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert!(rows[0].soft_mask.is_none());

        let before = Document::open_with_limits(source, Limits::DEFAULT).expect("it opens");
        let after = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
        let copy = the_image(&after);
        assert_eq!(
            after.get_key(&copy.dict, "SMaskInData").as_integer(),
            Some(code),
            "Table 87's entry is the producer's"
        );
        assert!(after.get_key(&copy.dict, "SMask").is_null());
        let (was, is) = (samples_of(&before), samples_of(&after));
        assert!(
            was.opacity.is_some(),
            "the fixture carries an opacity channel"
        );
        assert_eq!(is.opacity, was.opacity, "the opacity channel's integers");
        assert_eq!(is.opacity_depth, 20, "at its depth");
        assert_eq!(is.colour, was.colour);
        assert_eq!(is.premultiplied, was.premultiplied);
        assert_eq!(is.premultiplied, code == 2);
    }
}

/// One grey component of 17 bits, the shallowest that needs the three-byte field, is written
/// back too, and its `/Decode` stays the dictionary's own.
#[test]
fn a_grey_component_of_seventeen_bits_is_written_back_with_its_own_decode() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let data = a_deep_jpx_off_the_baseline(&[17], None);
    let source = a_page_with_the_image(&data, "/DeviceGray", "/Decode [1 0]");
    let target = Target::Two(Level::B);
    let (_, output) = {
        let plan = plan_from(&keep_everything(), target);
        let (report, output) = convert_with_plan(&source, &plan);
        (report, output)
    };
    // A DeviceGray image beside the part 2 fixture's RGB intent may stop the file on a row of its
    // own, which the profile answers; what is asked here is the image.
    let output = output.expect("the conversion writes the file");
    let before = Document::open_with_limits(source, Limits::DEFAULT).expect("it opens");
    let after = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let copy = the_image(&after);
    assert_eq!(
        after.get_key(&copy.dict, "Decode"),
        before.get_key(&the_image(&before).dict, "Decode"),
        "one depth, so the pair is not restated"
    );
    assert_eq!(samples_of(&after).colour, samples_of(&before).colour);
    assert_eq!(drawn(&after), drawn(&before));
}

/// An opacity channel of another depth than the colour channels is written in a `bpcc` box
/// (ISO/IEC 15444-1 I.5.3.2). ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 hold
/// the colour channels to one depth and say nothing of an opacity channel's, so the copy is
/// written with each channel at its own depth and reads back to the same integers.
#[test]
fn a_deep_opacity_channel_of_another_depth_is_written_with_its_own_depth() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let data = a_deep_jpx_off_the_baseline(&[20, 20, 20], Some(18));
    let source = a_page_with_the_image(&data, "/DeviceRGB", "/SMaskInData 1");
    let (report, output) = converted(&source);
    assert_eq!(
        holds(&output, Target::Two(Level::B)).verdict(),
        Verdict::Conforms
    );
    let rows = &conversion(&report).transcoded_images;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(rows[0].soft_mask.is_none());
    let before = Document::open_with_limits(source, Limits::DEFAULT).expect("it opens");
    let after = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let (was, is) = (samples_of(&before), samples_of(&after));
    assert!(
        was.opacity.is_some(),
        "the fixture carries an opacity channel"
    );
    assert_eq!(is.opacity, was.opacity, "the opacity channel's integers");
    assert_eq!(is.opacity_depth, 18, "at its own depth");
    assert_eq!(is.depths, vec![20, 20, 20], "the colour channels at theirs");
    assert_eq!(is.colour, was.colour);
}

/// One grey channel and an opacity channel are two channels in the file. ISO 19005-2 section
/// 6.2.8.3 and ISO 19005-4 section 6.2.7.3 count the colour channels, of which there is one, so
/// the copy is written and reads back to the same integers.
#[test]
fn a_deep_grey_image_with_opacity_is_written_back() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let data = a_deep_jpx_off_the_baseline(&[22], Some(22));
    let source = a_page_with_the_image(&data, "/DeviceGray", "/SMaskInData 1");
    let (report, output) = converted(&source);
    assert_eq!(
        holds(&output, Target::Two(Level::B)).verdict(),
        Verdict::Conforms
    );
    let rows = &conversion(&report).transcoded_images;
    assert_eq!(rows.len(), 1, "{rows:?}");
    let before = Document::open_with_limits(source, Limits::DEFAULT).expect("it opens");
    let after = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let (was, is) = (samples_of(&before), samples_of(&after));
    assert_eq!(is.components, 1);
    assert!(
        was.opacity.is_some(),
        "the fixture carries an opacity channel"
    );
    assert_eq!(is.opacity, was.opacity, "the opacity channel's integers");
    assert_eq!(is.colour, was.colour);
    assert_eq!(drawn(&after), drawn(&before));
}

/// Four colour channels need a JPX file, which a JP2 file cannot name: the image keeps its
/// refusal, and says why.
#[test]
fn four_deep_channels_keep_the_refusal_and_name_it() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let planes: Vec<Vec<u32>> = (0..4u64)
        .map(|seed| deep_samples(20, 64, seed + 5))
        .collect();
    let codestream = pdf_jpx_write::codestream(
        8,
        8,
        &planes
            .iter()
            .map(|samples| pdf_jpx_write::Plane { depth: 20, samples })
            .collect::<Vec<_>>(),
    )
    .expect("the planes fit");
    let mut header = boxed(*b"ihdr", &[0, 0, 0, 8, 0, 0, 0, 8, 0, 4, 19, 7, 0, 0]);
    header.extend(boxed(*b"colr", &[1, 0, 0, 0, 0, 0, 3]));
    let mut data = boxed(*b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
    data.extend(boxed(*b"ftyp", b"jp2 \0\0\0\0jp2 "));
    data.extend(boxed(*b"jp2h", &header));
    data.extend(boxed(*b"jp2c", &codestream));
    let source = a_page_with_the_image(&data, "/DeviceCMYK", "");
    let plan = plan_from(&keep_everything(), Target::Two(Level::B));
    let (report, output) = convert_with_plan(&source, &plan);
    assert!(output.is_none());
    let Decision::Refused(because) = decision(&report, ROW) else {
        panic!(
            "four deep channels keep the refusal: {:?}",
            decision(&report, ROW)
        );
    };
    assert!(
        because.sentence().contains("other than one or three"),
        "{}",
        because.sentence()
    );
}
