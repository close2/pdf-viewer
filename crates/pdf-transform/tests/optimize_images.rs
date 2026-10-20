//! `optimize --images` and the JPEG forms of `render` and `images`: the DCT encoder the owner
//! chose (`doc/questions/A01`), held to the owner's condition that nothing is re-encoded unless a
//! format is explicitly requested, and to the clauses the module documentation of
//! `pdf_transform::optimize_images` names (ADR 2138).
//!
//! The encoder runs in `pdf-sandbox`'s confined worker, so these tests need it built beside them:
//! `cargo build -p pdf-sandbox --bins` for the profile they run under (trap 10).
//!
//! **The fixture is built here, and that is a limit stated rather than hidden** (trap 8): it holds
//! one image of each kind the pass decides about, so that each decision is asked once; what real
//! producers' images do under the pass is the corpus walk's question, and ADR 2138 has its figures.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test code over a fixture of five images a few hundred bytes a side: a fixture that \
              cannot exercise the rule must fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::serialize::{ObjectStreams, Streams};
use pdf_syntax::{Document, Object, ObjectId};
use pdf_transform::images::ImagesPlan;
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::optimize_images::{ImageRecompression, ImageSavings};
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{Budget, MemorySinks, Origin, Plan, Policy, Report, Source, apply};

/// A deterministic picture with a photograph's statistics: smooth structure under fine noise, so
/// that `FlateDecode` keeps the noise and a JPEG at a moderate quality does not.
fn photograph(width: usize, height: usize, channels: usize) -> Vec<u8> {
    let mut state: u32 = 0x1234_5678;
    let mut out = Vec::with_capacity(width * height * channels);
    for y in 0..height {
        for x in 0..width {
            for channel in 0..channels {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let noise = i32::try_from(state >> 28).expect("four bits") - 8;
                let base = i32::try_from((x * 2 + y + channel * 40) % 200).expect("small") + 28;
                out.push(u8::try_from((base + noise).clamp(0, 255)).expect("clamped"));
            }
        }
    }
    out
}

/// One stream object's text and bytes.
fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// The image objects, by number, and what the pass is expected to do with each.
const RGB: u32 = 5;
const GREY: u32 = 6;
const INDEXED: u32 = 7;
const COLOUR_KEYED: u32 = 8;
const TINY: u32 = 9;

/// A one-page document drawing five images: a photograph in RGB and in grey behind
/// `FlateDecode`, which the pass replaces; an `Indexed` one and a colour-keyed one, which it
/// declines (§8.6.6.3, §8.9.6.4); and a two-by-two one whose JPEG cannot be smaller than its
/// twelve bytes, which it keeps.
fn fixture() -> Vec<u8> {
    let deflate = |data: &[u8]| pdf_syntax::serialize::flate_encode(data, 9).expect("deflates");
    let rgb = deflate(&photograph(96, 64, 3));
    let grey = deflate(&photograph(96, 64, 1));
    let indexed = deflate(&photograph(32, 32, 1));
    let keyed = deflate(&photograph(32, 32, 3));
    let content = b"q 96 0 0 64 0 0 cm /A Do Q q 96 0 0 64 100 0 cm /B Do Q \
                    q 32 0 0 32 0 70 cm /C Do Q q 32 0 0 32 40 70 cm /D Do Q \
                    q 2 0 0 2 80 70 cm /E Do Q";
    let palette: String = (0..=255u32).fold(String::new(), |mut hex, index| {
        let _ = write!(hex, "{:02X}{:02X}{:02X}", index, 255 - index, index / 2);
        hex
    });
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 110] /Contents 4 0 R /Resources \
           << /XObject << /A 5 0 R /B 6 0 R /C 7 0 R /D 8 0 R /E 9 0 R >> >> >>"
            .to_vec(),
        stream("", content),
        stream(
            "/Type /XObject /Subtype /Image /Width 96 /Height 64 /ColorSpace /DeviceRGB \
             /BitsPerComponent 8 /Filter /FlateDecode /Interpolate true",
            &rgb,
        ),
        stream(
            "/Type /XObject /Subtype /Image /Width 96 /Height 64 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /Filter /FlateDecode",
            &grey,
        ),
        stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width 32 /Height 32 /ColorSpace [/Indexed \
                 /DeviceRGB 255 <{palette}>] /BitsPerComponent 8 /Filter /FlateDecode"
            ),
            &indexed,
        ),
        stream(
            "/Type /XObject /Subtype /Image /Width 32 /Height 32 /ColorSpace /DeviceRGB \
             /BitsPerComponent 8 /Mask [0 40 0 40 0 40] /Filter /FlateDecode",
            &keyed,
        ),
        stream(
            "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB \
             /BitsPerComponent 8",
            &[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255],
        ),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut table = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(table.as_bytes());
    out
}

fn plan(images: Option<ImageRecompression>) -> OptimizePlan {
    OptimizePlan {
        source: 0,
        names: "out.pdf".parse().expect("a pattern"),
        prune: true,
        object_streams: ObjectStreams::Disable,
        streams: Streams::DEFAULT,
        linearize: false,
        images,
    }
}

fn run(bytes: &[u8], plan: &Plan) -> (Report, Vec<(String, Vec<u8>)>) {
    let sinks = MemorySinks::new();
    let report = apply(
        plan,
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the plan applies");
    (report, sinks.into_outputs())
}

fn optimized(bytes: &[u8], images: Option<ImageRecompression>) -> (ImageSavings, Vec<u8>) {
    let (report, mut outputs) = run(bytes, &Plan::Optimize(plan(images)));
    let Some(Origin::Optimized { savings, .. }) = report.outputs.first().map(|out| &out.origin)
    else {
        panic!("an optimised output");
    };
    (savings.images, outputs.remove(0).1)
}

/// The image the page names `/A`…`/E`, as the rewritten document states it.
fn image(document: &Document, name: &str) -> std::sync::Arc<pdf_syntax::object::Stream> {
    let pages = pdf_model::Pages::new(document);
    let page = pages.get(0).expect("one page");
    let id = page.id.expect("an indirect page");
    let resources = document
        .get_key_of(id, "Resources")
        .map(|value| document.resolve(&value))
        .expect("resources");
    let xobjects = resources
        .as_dict()
        .and_then(|dict| dict.get("XObject"))
        .map(|value| document.resolve(value))
        .expect("an XObject dictionary");
    let reference = xobjects
        .as_dict()
        .and_then(|dict| dict.get(name))
        .cloned()
        .expect("the image");
    document
        .resolve(&reference)
        .as_stream()
        .cloned()
        .expect("a stream")
}

fn filter(stream: &pdf_syntax::object::Stream) -> Option<Vec<u8>> {
    match stream.dict.get("Filter") {
        Some(Object::Name(name)) => Some(name.as_bytes().to_vec()),
        Some(Object::Array(items)) => items
            .last()
            .and_then(Object::as_name)
            .map(|name| name.as_bytes().to_vec()),
        _ => None,
    }
}

/// The owner's condition: with nothing asked for, no image is re-encoded, and none is a JPEG.
#[test]
fn without_images_asked_for_no_image_is_re_encoded() {
    let (savings, out) = optimized(&fixture(), None);
    assert_eq!(savings, ImageSavings::default());
    let read = Document::open(out).expect("the output opens");
    for name in ["A", "B", "C", "D", "E"] {
        assert_ne!(
            filter(&image(&read, name)).as_deref(),
            Some(&b"DCTDecode"[..]),
            "/{name}"
        );
    }
}

/// Asked for, the pass replaces the two photographs, declines §8.6.6.3's indices and §8.9.6.4's
/// exact colour key, and keeps the image whose JPEG would be larger than its producer's bytes.
#[test]
fn asked_for_the_photographs_are_replaced_and_every_other_image_kept() {
    let source = fixture();
    let (savings, out) = optimized(&source, Some(ImageRecompression { quality: 80 }));
    assert_eq!(savings.encoded, 3, "{savings:?}");
    assert_eq!(savings.replaced, 2, "{savings:?}");
    assert_eq!(savings.declined, 2, "{savings:?}");
    assert_eq!(savings.refused, 0, "{savings:?}");
    assert!(savings.saved > 0);

    let read = Document::open(out).expect("the output opens");
    let original = Document::open(source).expect("the fixture opens");
    for name in ["A", "B"] {
        let replaced = image(&read, name);
        assert_eq!(
            filter(&replaced).as_deref(),
            Some(&b"DCTDecode"[..]),
            "/{name}"
        );
        assert!(replaced.dict.get("DecodeParms").is_none());
        // Every other entry crosses as the producer wrote it.
        for key in [
            "Width",
            "Height",
            "ColorSpace",
            "BitsPerComponent",
            "Interpolate",
        ] {
            assert_eq!(
                replaced.dict.get(key),
                image(&original, name).dict.get(key),
                "/{name} /{key}"
            );
        }
        assert!(replaced.data.len() < image(&original, name).data.len());
    }
    for name in ["C", "D", "E"] {
        assert_eq!(
            filter(&image(&read, name)),
            filter(&image(&original, name)),
            "/{name} keeps its producer's filter"
        );
    }
}

/// The replaced images draw: the reader's own §7.4.8 decode reads each back at its grid, and the
/// page reports no image it could not draw.
#[test]
fn the_replaced_images_decode_at_their_own_grid() {
    let (_, out) = optimized(&fixture(), Some(ImageRecompression { quality: 80 }));
    let (report, outputs) = run(
        &out,
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: "1".parse().expect("a selection"),
            min_pixels: 0,
            list_only: false,
            native: true,
            no_mask: false,
            format: ImageFormat::Png,
            names: "img-%d".parse().expect("a pattern"),
        }),
    );
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    let jpegs: Vec<_> = outputs
        .iter()
        .filter(|(name, _)| {
            std::path::Path::new(name).extension() == Some(std::ffi::OsStr::new("jpg"))
        })
        .collect();
    assert_eq!(jpegs.len(), 2, "the two replaced images, written natively");
    for (name, bytes) in jpegs {
        assert_eq!(bytes.get(..2), Some(&[0xFF, 0xD8][..]), "{name}");
    }
}

/// The pass leaves nothing for itself: a second run over its own output re-encodes no image it
/// replaced, and writes the same file (RFC 0002 section 9's idempotence, for the lossy pass too).
#[test]
fn a_second_pass_over_its_own_output_writes_the_same_file() {
    let asked = Some(ImageRecompression { quality: 80 });
    let (_, once) = optimized(&fixture(), asked);
    let (savings, twice) = optimized(&once, asked);
    assert_eq!(savings.replaced, 0, "{savings:?}");
    assert_eq!(once, twice);
}

/// `render --format jpeg` writes a baseline frame of the page's own grid, and PNG stays the
/// default: the word is the request.
#[test]
fn render_writes_a_baseline_jpeg_only_where_it_is_asked_for() {
    let render = |format| {
        Plan::Render(RenderPlan {
            source: 0,
            pages: "1".parse().expect("a selection"),
            size: Sizing::Dpi(72.0),
            format,
            page_box: None,
            annotations: true,
            names: "page-%d".parse().expect("a pattern"),
            strips: Some(render_cpu::MAX_STRIPS),
        })
    };
    let source = fixture();
    let (_, jpeg) = run(&source, &render(ImageFormat::Jpeg { quality: 90 }));
    let (_, png) = run(
        &source,
        &render(ImageFormat::parse("png").expect("a format")),
    );
    let jpeg = &jpeg[0].1;
    assert_eq!(jpeg.get(..2), Some(&[0xFF, 0xD8][..]));
    let at = jpeg
        .windows(2)
        .position(|pair| pair == [0xFF, 0xC0])
        .expect("SOF0: ISO/IEC 10918-1's baseline process, the one §7.4.8 names");
    let lines = u16::from_be_bytes([jpeg[at + 5], jpeg[at + 6]]);
    let across = u16::from_be_bytes([jpeg[at + 7], jpeg[at + 8]]);
    assert_eq!(
        (across, lines),
        (200, 110),
        "§8.3.2.3's 72 units to the inch at 72 dpi"
    );
    assert_eq!(
        png[0].1.get(1..4),
        Some(&b"PNG"[..]),
        "PNG where nothing else is said"
    );
    assert_eq!(
        ImageFormat::parse("jpeg"),
        Some(ImageFormat::Jpeg {
            quality: pdf_transform::render::JPEG_QUALITY
        })
    );
}

/// An object number the fixture states, for a reader of a failure.
#[test]
fn the_fixture_states_its_images_where_the_tests_say() {
    let document = Document::open(fixture()).expect("the fixture opens");
    for number in [RGB, GREY, INDEXED, COLOUR_KEYED, TINY] {
        assert!(document.get(ObjectId::new(number, 0)).as_stream().is_some());
    }
}
