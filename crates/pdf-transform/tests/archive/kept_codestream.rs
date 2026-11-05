//! A bare JPEG 2000 codestream is given the JP2 file structure, its bytes kept, and every shipped
//! profile that answers a site takes it (R13 of `doc/adr/2490`, `doc/adr/2694`, `doc/adr/2695`).
//!
//! ISO 32000-2 §7.4.9 has the filter "expect to read a full JPX file structure", and ISO 19005-2
//! section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 restrict JPEG 2000 data to ITU-T T.801 M.9.2's
//! baseline, which a codestream with no boxes around it is outside of. The expected values come
//! from §7.4.9 too: a stated `/ColorSpace` "shall determine how the image samples are interpreted,
//! and the colour space specifications in the JPEG 2000 data shall be ignored", so the colour box
//! the file is given decides nothing, and the samples a reader decodes are the codestream's.

use pdf_model::colour::Conversion;
use pdf_model::image::{JpxSamples, ORDINARY_JPX_SAMPLES};
use pdf_render::Color;
use pdf_syntax::object::{Dictionary, Name, Stream};
use pdf_syntax::{Document, Limits, Object};

use super::{
    ArchivePlan, Conforming, Decision, Flavour, Level, Rewrite, Target, Verdict, conversion,
    convert_with_plan, decision, holds, plan_from, stream,
};

const ROW: &str = "graphics/jpeg2000-uses-the-baseline-feature-set";

/// Every shipped profile that answers the site, by name and text.
const ANSWERING: [(&str, &str); 5] = [
    (
        "as-if-printed",
        include_str!("../../../../doc/profiles/as-if-printed.toml"),
    ),
    (
        "as-shown",
        include_str!("../../../../doc/profiles/as-shown.toml"),
    ),
    (
        "only-metadata-loss",
        include_str!("../../../../doc/profiles/only-metadata-loss.toml"),
    ),
    (
        "no-data-loss",
        include_str!("../../../../doc/profiles/no-data-loss.toml"),
    ),
    (
        "state-every-loss",
        include_str!("../../../../doc/profiles/state-every-loss.toml"),
    ),
];

const REFUSE_ANY_LOSS: &str = include_str!("../../../../doc/profiles/refuse-any-loss.toml");

/// The two lowest levels of each part, which the census converts to.
const TARGETS: [Target; 2] = [Target::Two(Level::B), Target::Four(Flavour::Plain)];

/// A plan converting to `target` under what the profile `text` states.
fn planned(text: &str, target: Target) -> ArchivePlan {
    let config =
        pdf_transform::archive::Configuration::read(text, target).expect("the profile reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(text, target)
    }
}

/// A 4×1 eight-bit codestream of `pdf_jpx_write`'s, one plane per entry of `depths`, with no box
/// around it.
fn bare(depths: &[u8]) -> Vec<u8> {
    let planes: Vec<Vec<u32>> = depths
        .iter()
        .zip(1u32..)
        .map(|(depth, seed)| {
            let top = (1u32 << depth).saturating_sub(1);
            vec![0, top, seed.min(top), top / 2]
        })
        .collect();
    let planes: Vec<pdf_jpx_write::Plane<'_>> = planes
        .iter()
        .zip(depths)
        .map(|(samples, depth)| pdf_jpx_write::Plane {
            depth: *depth,
            samples,
        })
        .collect();
    pdf_jpx_write::codestream(4, 1, &planes).expect("the fixture's planes fit")
}

/// A page whose resources hold one 4×1 `JPXDecode` image of `data`, its dictionary stating
/// `extra`, after the further objects `objects` (numbered from 6, so the image is the next).
fn a_page_with(target: Target, data: &[u8], extra: &str, objects: Vec<String>) -> Vec<u8> {
    let base = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    let image = objects.len().saturating_add(6);
    Conforming {
        resources: format!("/XObject << /Im {image} 0 R >>"),
        binary_objects: vec![stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width 4 /Height 1 {extra} /Filter /JPXDecode \
                 /Length {}",
                data.len()
            ),
            data,
        )],
        objects,
        ..base
    }
    .build()
}

/// The page's one image, `/Im`.
fn the_image(held: &Document) -> std::sync::Arc<Stream> {
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

/// The samples a reader decodes from the page's image.
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

/// `source` converted to `target` under the profile `text`, which must answer the site with the
/// transcode and write a conforming file.
fn kept(name: &str, text: &str, target: Target, source: &[u8]) -> (pdf_transform::Report, Vec<u8>) {
    let (report, output) = convert_with_plan(source, &planned(text, target));
    let answered = decision(&report, ROW);
    let Decision::Configured { rewrite, .. } = answered else {
        panic!("{name} answers the site at {target}: {answered:?}");
    };
    assert_eq!(rewrite, Rewrite::Jpeg2000TranscodedToFlate, "{name}");
    let output =
        output.unwrap_or_else(|| panic!("{name} {target}: {:?}", conversion(&report).decided));
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{name} {target}"
    );
    (report, output)
}

/// Three eight-bit channels under `DeviceRGB`, under every answering profile at both parts: the
/// codestream is the file's last box byte for byte, the dictionary is the producer's but for its
/// `/Length`, every sample decodes as it did and the page draws as it did.
#[test]
fn every_answering_profile_keeps_a_bare_codestream_in_a_jp2_file() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let codestream = bare(&[8, 8, 8]);
    for target in TARGETS {
        let source = a_page_with(
            target,
            &codestream,
            "/ColorSpace /DeviceRGB /BitsPerComponent 8",
            vec![],
        );
        let failed: Vec<&str> = holds(&source, target)
            .failures()
            .map(|failed| failed.id)
            .collect();
        assert!(failed.contains(&ROW), "{target}: {failed:?}");
        for (name, text) in ANSWERING {
            let (report, output) = kept(name, text, target, &source);
            let rows = &conversion(&report).transcoded_images;
            assert_eq!(rows.len(), 1, "{name} {target}: {rows:?}");
            assert!(rows[0].codestream_kept, "{name} {target}: {rows:?}");
            assert_eq!((rows[0].filter, rows[0].bits), ("JPXDecode", 8));
            assert_eq!(rows[0].before, codestream.len());

            let before = Document::open_with_limits(source.clone(), Limits::DEFAULT)
                .expect("the source opens");
            let after =
                Document::open_with_limits(output, Limits::DEFAULT).expect("the output opens");
            let (was, is) = (the_image(&before), the_image(&after));
            assert!(
                is.data.starts_with(b"\0\0\0\x0cjP  ") && is.data.ends_with(&codestream),
                "{name} {target}: a JP2 file ending in the producer's codestream"
            );
            for key in [
                "Filter",
                "ColorSpace",
                "BitsPerComponent",
                "Width",
                "Height",
            ] {
                assert_eq!(
                    after.get_key(&is.dict, key),
                    before.get_key(&was.dict, key),
                    "{name} {target}: /{key} is the producer's"
                );
            }
            let (was, is) = (samples_of(&before), samples_of(&after));
            assert_eq!(is.colour, was.colour, "{name} {target}: every sample");
            assert_eq!(is.depths, was.depths);
            assert_eq!(
                drawn(&after),
                drawn(&before),
                "{name} {target}: drawn as it was"
            );
        }
    }
}

/// One channel under `DeviceGray`, beside an `/SMask` of the producer's: the grey file's box
/// enumerates greyscale, and the soft mask is the same object, so the opacity is carried as it
/// stood.
#[test]
fn a_grey_codestream_keeps_its_soft_mask() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let codestream = bare(&[8]);
    let target = Target::Two(Level::B);
    let mask = "<< /Type /XObject /Subtype /Image /Width 4 /Height 1 /ColorSpace /DeviceGray \
                /BitsPerComponent 8 /Length 4 >>\nstream\n\u{0}\u{7f}\u{7f}\u{0}\nendstream"
        .to_owned();
    let source = a_page_with(
        target,
        &codestream,
        "/ColorSpace /DeviceGray /SMask 6 0 R",
        vec![mask],
    );
    let (name, text) = ANSWERING[0];
    let (report, output) = kept(name, text, target, &source);
    assert!(conversion(&report).transcoded_images[0].codestream_kept);
    let before = Document::open_with_limits(source, Limits::DEFAULT).expect("the source opens");
    let after = Document::open_with_limits(output, Limits::DEFAULT).expect("the output opens");
    let is = the_image(&after);
    assert!(is.data.ends_with(&codestream));
    let colr = is
        .data
        .windows(4)
        .position(|window| window == b"colr")
        .expect("a colour box");
    assert_eq!(
        is.data.get(colr.saturating_add(10)),
        Some(&17),
        "enumerated greyscale"
    );
    assert_eq!(
        after.get_key(&is.dict, "SMask").as_reference(),
        before
            .get_key(&the_image(&before).dict, "SMask")
            .as_reference(),
        "the producer's soft mask"
    );
    assert_eq!(samples_of(&after).colour, samples_of(&before).colour);
    assert_eq!(drawn(&after), drawn(&before));
}

/// Colour channels of 8, 4 and 8 bits: section 6.2.8.3 admits one depth for every colour channel
/// of JPEG 2000 data, so the codestream cannot be kept, and the image is transcoded to
/// `FlateDecode` with each `/Decode` pair widened — every sample still decoding as it did.
#[test]
fn colour_channels_of_different_depths_are_transcoded_rather_than_kept() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let target = Target::Two(Level::B);
    let source = a_page_with(target, &bare(&[8, 4, 8]), "/ColorSpace /DeviceRGB", vec![]);
    let (name, text) = ANSWERING[1];
    let (report, output) = kept(name, text, target, &source);
    let row = &conversion(&report).transcoded_images[0];
    assert!(!row.codestream_kept, "{row:?}");
    assert_eq!(row.filter, "FlateDecode");
    let before = Document::open_with_limits(source, Limits::DEFAULT).expect("the source opens");
    let after = Document::open_with_limits(output, Limits::DEFAULT).expect("the output opens");
    assert_eq!(
        after.get_key(&the_image(&after).dict, "Filter"),
        Object::Name(Name::new(&b"FlateDecode"[..]))
    );
    assert_eq!(drawn(&after), drawn(&before));
}

/// What no profile writes, each refused by name: a bare codestream whose dictionary states no
/// `/ColorSpace` — its colour is then the data's (§7.4.9), and a colour box this converter chose
/// would say what it is — under every answering profile; and the site under `refuse-any-loss`,
/// which answers nothing.
#[test]
fn a_codestream_with_no_stated_space_and_a_profile_that_answers_nothing_keep_the_refusal() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let target = Target::Two(Level::B);
    let unstated = a_page_with(target, &bare(&[8, 8, 8]), "", vec![]);
    for (name, text) in ANSWERING {
        let (report, output) = convert_with_plan(&unstated, &planned(text, target));
        assert!(output.is_none(), "{name}");
        let Decision::Refused(because) = decision(&report, ROW) else {
            panic!("{name}: the data's own colour keeps the refusal");
        };
        assert!(
            because
                .sentence()
                .contains("take the colour space from the JPEG 2000"),
            "{name}: {}",
            because.sentence()
        );
    }
    let stated = a_page_with(target, &bare(&[8, 8, 8]), "/ColorSpace /DeviceRGB", vec![]);
    let (report, output) = convert_with_plan(&stated, &planned(REFUSE_ANY_LOSS, target));
    assert!(output.is_none());
    let Decision::Refused(because) = decision(&report, ROW) else {
        panic!("refuse-any-loss answers nothing");
    };
    assert!(
        because.sentence().contains("JP2 file structure"),
        "{}",
        because.sentence()
    );
}
