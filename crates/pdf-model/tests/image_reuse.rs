//! What `image::RasterCache`'s key claims, one test per thing it claims.
//!
//! ISO 32000-2 §8.9.5's image `XObject` is decoded at every `Do`, and a page that draws one
//! thirty-six times decoded it thirty-six times until the five-hundred-and-thirty-ninth session
//! (ADRs 0373, 0374). A cache of the decoded raster is the fix, and **a cache's key is a claim**:
//! it says that two `Do`s the key cannot tell apart would have produced the same samples.
//!
//! `image::decode_parts` reads five things, and this file is one test per input the claim has to
//! hold for. Each is written so that it **fails if its component is dropped from the key** — the
//! second arm is compared against a fresh, uncached decode, so an entry answered in its place is
//! not merely a suspicious pointer but the wrong picture:
//!
//! | input | the clause that makes it an input | test |
//! |---|---|---|
//! | the stream | — | [`a_second_do_of_one_stream_reuses_its_raster`], [`a_stream_cannot_inherit_the_raster_of_one_whose_allocation_it_reuses`] |
//! | the resource dictionary | §8.6.5.1's named colour space, §8.6.5.6's `/Default*` | [`a_raster_is_not_shared_across_resource_dictionaries`] |
//! | the fill colour | §8.9.6.2's stencil paints the current colour | [`a_raster_is_not_shared_across_fill_colours`] |
//! | what it composites into | §11.4.7's halves, §11.6.5.1's mask group | [`a_raster_is_not_shared_across_compositing`] |
//! | the document | — | not in the key: the cache belongs to one interpretation |
//!
//! The sixth test is `doc/HANDOVER.md`'s trap 5 rather than a component: a raster answered from
//! the cache has to carry the reports a fresh decode carries, or the second `Do` silently stops
//! saying what the first said.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::colour::{Compositing, Half, PressId};
use pdf_model::image::{MaskCache, Parts, RasterCache, decode_parts};
use pdf_render::Color;
use pdf_syntax::{Dictionary, Document, Name, Object, Stream};

/// A document with no images in it at all.
///
/// The streams below are built in the test rather than read out of a file, which is what
/// `pdf_model::thumbnail::significant` does in the shipped tree and what
/// [`a_stream_cannot_inherit_the_raster_of_one_whose_allocation_it_reuses`] needs: a stream the
/// document's own object cache holds can never have its allocation reused, so a fixture read out
/// of a file could not put the pin under any pressure. What the document is still needed for is
/// resolution — `decode_parts` reads every entry through it.
fn document() -> Document {
    Document::open(assembled(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << >> >>\nendobj\n",
    ))
    .expect("the fixture opens")
}

/// A body of numbered objects with the cross-reference section its offsets need.
fn assembled(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A dictionary from key–value pairs, so that a fixture reads as the dictionary it is.
fn dict(entries: Vec<(&str, Object)>) -> Dictionary {
    let mut built = Dictionary::new();
    for (key, value) in entries {
        built.insert(Name::new(key.as_bytes().to_vec()), value);
    }
    built
}

/// A name object, which is most of what an image dictionary's values are.
fn name(of: &str) -> Object {
    Object::Name(Name::new(of.as_bytes().to_vec()))
}

/// A 2×2 eight-bit image whose colour space is the name `/CS0`, with no filter.
///
/// Twelve bytes: three components per sample under `DeviceRGB`, and under `DeviceGray` the same
/// bytes are a 2×2 image read two to a row, which is what makes the resource dictionary visible
/// in the samples.
fn named_space_image() -> Arc<Stream> {
    Arc::new(Stream {
        dict: dict(vec![
            ("Type", name("XObject")),
            ("Subtype", name("Image")),
            ("Width", Object::Integer(2)),
            ("Height", Object::Integer(2)),
            ("BitsPerComponent", Object::Integer(8)),
            ("ColorSpace", name("CS0")),
        ]),
        data: Arc::from([10, 200, 30, 40, 90, 250, 120, 5, 60, 210, 15, 170].as_slice()),
        decryption_failed: false,
    })
}

/// A 2×2 stencil (§8.9.6.2), whose samples carry no colour of their own.
fn stencil() -> Arc<Stream> {
    Arc::new(Stream {
        dict: dict(vec![
            ("Type", name("XObject")),
            ("Subtype", name("Image")),
            ("Width", Object::Integer(2)),
            ("Height", Object::Integer(2)),
            ("ImageMask", Object::Boolean(true)),
        ]),
        // One row to a byte, `1 0` over `0 1`: §8.9.3 starts every row on a byte boundary.
        data: Arc::from([0b1000_0000, 0b0100_0000].as_slice()),
        decryption_failed: false,
    })
}

/// Resources naming `/CS0` as `space`.
fn resources_naming(space: &str) -> Dictionary {
    dict(vec![(
        "ColorSpace",
        Object::Dictionary(dict(vec![("CS0", name(space))])),
    )])
}

/// The samples of a decode, whatever route it took.
fn samples(parts: &Parts) -> Arc<[u8]> {
    match parts {
        Parts::Complete(image) => Arc::clone(&image.data),
        Parts::Masked { base, .. } => Arc::clone(&base.data),
    }
}

/// One decode with no cache at all, which is what every arm below is judged against.
fn decoded(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
    fill: Color,
    into: Compositing,
) -> Arc<[u8]> {
    let mut masks = MaskCache::default();
    let parts = decode_parts(document, stream, resources, fill, into, &mut masks)
        .expect("the fixture decodes");
    samples(&parts)
}

/// One decode through a cache.
fn cached(
    cache: &mut RasterCache,
    document: &Document,
    stream: &Arc<Stream>,
    resources: &Dictionary,
    fill: Color,
    into: Compositing,
) -> Arc<[u8]> {
    let mut masks = MaskCache::default();
    let parts = cache
        .parts(document, stream, resources, fill, into, &mut masks)
        .expect("the fixture decodes");
    samples(&parts)
}

/// **The point of the cache**: a second `Do` of one stream under one state decodes nothing.
///
/// The observable is the allocation rather than the bytes — two decodes of one image produce
/// equal samples whether or not anything was reused, and only a shared `Arc` says the second
/// `Do` did no work. It is also what makes the display list smaller, which is the memory half of
/// ADR 0374.
#[test]
fn a_second_do_of_one_stream_reuses_its_raster() {
    let document = document();
    let stream = named_space_image();
    let resources = resources_naming("DeviceRGB");
    let mut cache = RasterCache::default();

    let first = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        Color::BLACK,
        Compositing::Device,
    );
    let second = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        Color::BLACK,
        Compositing::Device,
    );

    assert!(
        Arc::ptr_eq(&first, &second),
        "a second Do of one stream decoded again"
    );
}

/// **The pin, which is what makes an address a name**: ADR 0317's argument, one crate over.
///
/// An entry is keyed by the identity of its stream's allocation, and it *holds* that allocation.
/// Were it to hold the address alone, a freed `Stream` would hand its address to the next one and
/// the next one would be answered with the first's samples. The loop is what gives the test teeth
/// — one iteration proves nothing about an allocator — and it fails on the second iteration if
/// the `Arc` in `Cached` is replaced by a bare address.
#[test]
fn a_stream_cannot_inherit_the_raster_of_one_whose_allocation_it_reuses() {
    let document = document();
    let resources = resources_naming("DeviceRGB");
    let mut cache = RasterCache::default();

    for round in 0..64_u8 {
        let stream = Arc::new(Stream {
            dict: named_space_image().dict.clone(),
            // A different picture every round, so an answer from a previous round is not
            // merely a suspicious pointer but the wrong samples.
            data: Arc::from([round, 200, 30, 40, 90, 250, 120, 5, 60, 210, 15, 170].as_slice()),
            decryption_failed: false,
        });
        let through_the_cache = cached(
            &mut cache,
            &document,
            &stream,
            &resources,
            Color::BLACK,
            Compositing::Device,
        );
        let fresh = decoded(
            &document,
            &stream,
            &resources,
            Color::BLACK,
            Compositing::Device,
        );
        assert_eq!(
            &*through_the_cache, &*fresh,
            "round {round} was answered with another stream's raster"
        );
        // Dropped here, which is the pressure: the next `Arc::new` may take this address.
    }
}

/// **§8.6.5.1**: a colour space named `/CS0` means what the resource dictionary in force says.
#[test]
fn a_raster_is_not_shared_across_resource_dictionaries() {
    let document = document();
    let stream = named_space_image();
    let mut cache = RasterCache::default();

    let rgb = cached(
        &mut cache,
        &document,
        &stream,
        &resources_naming("DeviceRGB"),
        Color::BLACK,
        Compositing::Device,
    );
    let grey = cached(
        &mut cache,
        &document,
        &stream,
        &resources_naming("DeviceGray"),
        Color::BLACK,
        Compositing::Device,
    );

    assert_ne!(&*rgb, &*grey, "the fixture does not distinguish the two");
    assert_eq!(
        &*grey,
        &*decoded(
            &document,
            &stream,
            &resources_naming("DeviceGray"),
            Color::BLACK,
            Compositing::Device,
        ),
        "the second resource dictionary was answered with the first's raster"
    );
}

/// **§8.9.6.2**: a stencil "designates places on the page that should either be marked with the
/// current colour or masked out", so the fill colour is part of what its samples are.
#[test]
fn a_raster_is_not_shared_across_fill_colours() {
    let document = document();
    let stream = stencil();
    let resources = Dictionary::new();
    let mut cache = RasterCache::default();
    let red = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let blue = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    let in_red = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        red,
        Compositing::Device,
    );
    let in_blue = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        blue,
        Compositing::Device,
    );

    assert_ne!(
        &*in_red, &*in_blue,
        "the fixture does not distinguish the two"
    );
    assert_eq!(
        &*in_blue,
        &*decoded(&document, &stream, &resources, blue, Compositing::Device),
        "the second fill colour was answered with the first's raster"
    );
}

/// **§11.4.7 and §11.6.5.1**: the same samples painted into a different quantity are different
/// numbers, and this one changes *within* an interpretation (ADRs 0220, 0262).
#[test]
fn a_raster_is_not_shared_across_compositing() {
    let document = document();
    let stream = named_space_image();
    let resources = resources_naming("DeviceRGB");
    let mut cache = RasterCache::default();
    let press = Compositing::Subtractive(Half::Black, PressId::ASSUMED);

    let on_the_device = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        Color::BLACK,
        Compositing::Device,
    );
    let on_the_press = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        Color::BLACK,
        press,
    );

    assert_ne!(
        &*on_the_device, &*on_the_press,
        "the fixture does not distinguish the two"
    );
    assert_eq!(
        &*on_the_press,
        &*decoded(&document, &stream, &resources, Color::BLACK, press),
        "the second quantity was answered with the first's raster"
    );
}

/// **Trap 5**: what the first `Do` reported, the ninth reports too.
///
/// Every report about an image is made from its *dictionary*, before anything is decoded — so a
/// raster answered from the cache cannot suppress one, and this is the test that says so rather
/// than the argument. The fixture is §7.3.8.2's short stream: a 4×4 eight-bit `DeviceGray` image
/// carrying four of its sixteen samples, which `image::short_of_its_grid` names, drawn twice.
#[test]
fn a_reused_raster_carries_the_reports_a_fresh_decode_carries() {
    let reports = |draws: &str| {
        let content = format!("40 0 0 40 0 0 cm {draws}");
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
             /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
             5 0 obj\n<< /Type /XObject /Subtype /Image /Width 4 /Height 4 \
             /BitsPerComponent 8 /ColorSpace /DeviceGray /Length 4 >>\nstream\n\x01\x02\x03\x04\
             \nendstream\nendobj\n",
            content.len()
        );
        let document = Document::open(assembled(&body)).expect("the fixture opens");
        let page = pdf_model::Pages::new(&document)
            .get(0)
            .expect("the fixture has a page");
        let interpretation = pdf_model::interpret(&document, &page);
        format!("{:?}", interpretation.unsupported)
    };

    let once = reports("/Im Do");
    assert!(
        once.contains("its samples stop at"),
        "the fixture reports nothing to compare: {once}"
    );
    assert_eq!(
        once,
        reports("/Im Do /Im Do"),
        "a second Do of one image reported something the first did not, or lost what it said"
    );
}
