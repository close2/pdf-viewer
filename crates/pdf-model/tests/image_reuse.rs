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
//! | …named by its content, for §8.9.7's inline image | §8.9.7 puts it in the content stream rather than in an object | [`a_second_bi_of_one_inline_image_reuses_its_raster`], [`two_inline_images_do_not_share_a_raster`], [`every_cell_of_a_hatching_shares_one_decode`] |
//! | the resource dictionary | §8.6.5.1's named colour space, §8.6.5.6's `/Default*` | [`a_raster_is_not_shared_across_resource_dictionaries`] |
//! | the fill colour | §8.9.6.2's stencil paints the current colour | [`a_raster_is_not_shared_across_fill_colours`] |
//! | what it composites into | §11.4.7's halves, §11.6.5.1's mask group | [`a_raster_is_not_shared_across_compositing`] |
//! | …and whether the black point is compensated | §8.6.5.9 under §8.9.5.1 Table 87's `/Intent` | [`a_raster_is_not_shared_across_black_points`] |
//! | the document | — | not in the key: the cache belongs to one interpretation |
//!
//! One test is `doc/HANDOVER.md`'s trap 5 rather than a component: a raster answered from the
//! cache has to carry the reports a fresh decode carries, or the second `Do` silently stops
//! saying what the first said.
//!
//! **And the second row is a bound rather than a claim about correctness** (ADR 0399). A key
//! nothing can hit is not merely a cache that does no good: the probe is linear in the entries, so
//! one entry per draw makes a page quadratic in its own image count, and two corpus documents took
//! two minutes and twelve seconds of it before anybody named the shape.

#![expect(
    clippy::expect_used,
    clippy::cast_possible_truncation,
    reason = "test code: a malformed fixture should fail loudly, and the ICC fixture's constants \
              are written as the fixed-point values it encodes"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::colour::{Compositing, Conversion, Half};
use pdf_model::image::{MaskCache, NamedStream, Parts, RasterCache, StreamIdentity, decode_parts};
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
    let mut hex = String::new();
    for byte in dark_black_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    Document::open(assembled(&format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << >> >>\nendobj\n\
         4 0 obj\n<< /N 1 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\n\
         endstream\nendobj\n",
        hex.len().saturating_add(1)
    )))
    .expect("the fixture opens")
}

/// Object four: an ICC profile whose darkest colour is a tenth of its white point.
///
/// The only colour space family §8.6.5.9's black point moves is `ICCBased`, so it is the only
/// one that can show the fourth key component's second half. `tests/rendering_intent.rs` builds
/// the same shape and says what each field of the `lut16Type` encoding is.
fn dark_black_profile() -> Vec<u8> {
    let white: [u16; 3] = [31599, 32768, 27030];
    let dark: [u16; 3] = [white[0] / 10, white[1] / 10, white[2] / 10];

    let mut header = vec![0u8; 128];
    header[8] = 2;
    header[16..20].copy_from_slice(b"GRAY");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let mut tag = Vec::new();
    tag.extend_from_slice(b"mft2");
    tag.extend_from_slice(&[0; 4]);
    tag.extend_from_slice(&[1, 3, 2, 0]);
    for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
    }
    tag.extend_from_slice(&2u16.to_be_bytes());
    tag.extend_from_slice(&2u16.to_be_bytes());
    for value in [0u16, 0xFFFF] {
        tag.extend_from_slice(&value.to_be_bytes());
    }
    for value in white.iter().chain(dark.iter()) {
        tag.extend_from_slice(&value.to_be_bytes());
    }
    for _ in 0..3 {
        for value in [0u16, 0xFFFF] {
            tag.extend_from_slice(&value.to_be_bytes());
        }
    }

    let mut out = header;
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(b"A2B1");
    out.extend_from_slice(&144u32.to_be_bytes());
    out.extend_from_slice(&(tag.len() as u32).to_be_bytes());
    out.extend_from_slice(&tag);
    out
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
    into: &Conversion,
) -> Arc<[u8]> {
    let mut masks = MaskCache::default();
    let parts = decode_parts(document, stream, resources, fill, into, &mut masks)
        .expect("the fixture decodes");
    samples(&parts)
}

/// One decode through a cache, of a stream a resource dictionary handed out.
fn cached(
    cache: &mut RasterCache,
    document: &Document,
    stream: &Arc<Stream>,
    resources: &Dictionary,
    fill: Color,
    into: &Conversion,
) -> Arc<[u8]> {
    named(
        cache,
        document,
        NamedStream::allocation(stream),
        resources,
        fill,
        into,
    )
}

/// The same, with the name spelled out, for the two tests that are about it.
fn named(
    cache: &mut RasterCache,
    document: &Document,
    image: NamedStream<'_>,
    resources: &Dictionary,
    fill: Color,
    into: &Conversion,
) -> Arc<[u8]> {
    let mut masks = MaskCache::default();
    let parts = cache
        .parts(document, image, resources, fill, into, &mut masks)
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
        &Conversion::device(),
    );
    let second = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        Color::BLACK,
        &Conversion::device(),
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
            &Conversion::device(),
        );
        let fresh = decoded(
            &document,
            &stream,
            &resources,
            Color::BLACK,
            &Conversion::device(),
        );
        assert_eq!(
            &*through_the_cache, &*fresh,
            "round {round} was answered with another stream's raster"
        );
        // Dropped here, which is the pressure: the next `Arc::new` may take this address.
    }
}

/// **§8.9.7's image has no allocation that recurs**, so it is named by its content instead.
///
/// The two streams below are separately allocated and equal, which is exactly what
/// `content::run` produces for two `BI`s of one picture: it builds the stream out of the bytes
/// between `BI` and `EI` at every one of them. Under [`StreamIdentity::Allocation`] the second is
/// a miss — that is what this test fails as, and it is what the tree did before ADR 0399.
#[test]
fn a_second_bi_of_one_inline_image_reuses_its_raster() {
    let document = document();
    let resources = resources_naming("DeviceRGB");
    let mut cache = RasterCache::default();
    let first_stream = named_space_image();
    let second_stream = named_space_image();
    assert!(
        !Arc::ptr_eq(&first_stream, &second_stream),
        "the fixture shares one allocation, so it cannot show what it is for"
    );

    assert_eq!(
        StreamIdentity::of_inline(&first_stream),
        StreamIdentity::of_inline(&second_stream),
        "two equal inline images were given two identities"
    );
    let first = named(
        &mut cache,
        &document,
        NamedStream::inline(&first_stream),
        &resources,
        Color::BLACK,
        &Conversion::device(),
    );
    let second = named(
        &mut cache,
        &document,
        NamedStream::inline(&second_stream),
        &resources,
        Color::BLACK,
        &Conversion::device(),
    );

    assert!(
        Arc::ptr_eq(&first, &second),
        "a second BI of one inline image decoded again"
    );
}

/// **And two inline images that differ get two rasters**, which is the other half of the claim.
///
/// It fails if the content stops reaching the digest at all — a constant identity, or one taken
/// from the dictionary alone, which two hatchings of one page share. What it cannot exhibit is a
/// digest *collision*, which is why the exact comparison beside it is argued rather than tested:
/// there is no pair of eleven-byte inputs anybody can write down that `DefaultHasher` collides.
#[test]
fn two_inline_images_do_not_share_a_raster() {
    let document = document();
    let resources = resources_naming("DeviceRGB");
    let mut cache = RasterCache::default();
    let first_stream = named_space_image();
    let second_stream = Arc::new(Stream {
        dict: first_stream.dict.clone(),
        data: Arc::from([99, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11].as_slice()),
        decryption_failed: false,
    });

    let first = named(
        &mut cache,
        &document,
        NamedStream::inline(&first_stream),
        &resources,
        Color::BLACK,
        &Conversion::device(),
    );
    let second = named(
        &mut cache,
        &document,
        NamedStream::inline(&second_stream),
        &resources,
        Color::BLACK,
        &Conversion::device(),
    );

    assert_ne!(
        &*first, &*second,
        "the fixture does not distinguish the two"
    );
    assert_eq!(
        &*second,
        &*decoded(
            &document,
            &second_stream,
            &resources,
            Color::BLACK,
            &Conversion::device(),
        ),
        "the second inline image was answered with the first's raster"
    );
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
        &Conversion::device(),
    );
    let grey = cached(
        &mut cache,
        &document,
        &stream,
        &resources_naming("DeviceGray"),
        Color::BLACK,
        &Conversion::device(),
    );

    assert_ne!(&*rgb, &*grey, "the fixture does not distinguish the two");
    assert_eq!(
        &*grey,
        &*decoded(
            &document,
            &stream,
            &resources_naming("DeviceGray"),
            Color::BLACK,
            &Conversion::device(),
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
        &Conversion::device(),
    );
    let in_blue = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        blue,
        &Conversion::device(),
    );

    assert_ne!(
        &*in_red, &*in_blue,
        "the fixture does not distinguish the two"
    );
    assert_eq!(
        &*in_blue,
        &*decoded(&document, &stream, &resources, blue, &Conversion::device()),
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
    let press = Conversion::new(
        Compositing::Subtractive(Half::Black, pdf_model::colour::assumed_press()),
        true,
    );

    let on_the_device = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        Color::BLACK,
        &Conversion::device(),
    );
    let on_the_press = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        Color::BLACK,
        &press,
    );

    assert_ne!(
        &*on_the_device, &*on_the_press,
        "the fixture does not distinguish the two"
    );
    assert_eq!(
        &*on_the_press,
        &*decoded(&document, &stream, &resources, Color::BLACK, &press),
        "the second quantity was answered with the first's raster"
    );
}

/// **§8.6.5.9**: the same samples under two black point settings are two rasters.
///
/// The fourth key component is the whole of `crate::colour::Conversion`, which is what is
/// composited into *and* whether black point compensation applies — and the second half changes
/// within an interpretation exactly as the first does: §8.6.5.8 gives an image three routes to a
/// rendering intent, one of which (Table 87's `/Intent`) is the image dictionary's own, so one
/// page can draw one stream under both answers. §8.6.5.9 is what makes the intent decide it:
///
/// > If the current render intent of an object is AbsColorimetric then the value of
/// > UseBlackPtComp shall be treated as OFF .
///
/// `ICCBased` is the only family the setting moves, which is why this fixture states one where
/// its neighbours name a device space.
#[test]
fn a_raster_is_not_shared_across_black_points() {
    let document = document();
    let stream = named_space_image();
    let resources = dict(vec![(
        "ColorSpace",
        Object::Dictionary(dict(vec![(
            "CS0",
            Object::Array(vec![
                name("ICCBased"),
                Object::Reference(pdf_syntax::ObjectId::new(4, 0)),
            ]),
        )])),
    )]);
    let mut cache = RasterCache::default();
    let without = Conversion::new(Compositing::Device, false);

    let compensated = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        Color::BLACK,
        &Conversion::device(),
    );
    let uncompensated = cached(
        &mut cache,
        &document,
        &stream,
        &resources,
        Color::BLACK,
        &without,
    );

    assert_ne!(
        &*compensated, &*uncompensated,
        "the fixture does not distinguish the two"
    );
    assert_eq!(
        &*uncompensated,
        &*decoded(&document, &stream, &resources, Color::BLACK, &without),
        "the second black point setting was answered with the first's raster"
    );
}

/// **The bound the witnesses asked for**: a hatching's cells cost one decode between them.
///
/// §8.7.3.1's tiling replays the cell's content stream once per lattice position, and where that
/// content is §8.9.7's inline image the interpreter builds a fresh stream at every one of them. So
/// a page's *draws* grow with the tiling and the entries `image::RasterCache` holds must not: a
/// probe is linear in them, and one entry per draw makes a page quadratic in its own cell count.
///
/// The fixture is the shape the two documents that found it state — an 8×8 stencil in a `/BBox
/// [0 0 1 1]` cell with `/XStep` and `/YStep` of one, poured over a 40 × 40 square, which is a
/// 42 × 42 lattice inside `MAX_TILES` — and the assertion is on the allocation because that says the
/// decode did not happen again. **Neither witness is committed**: both are `SafeDocs` members of a
/// Common Crawl archive, which `.gitignore` and `doc/third-party-data.md` keep out of this history
/// on licence grounds, so the shape is rebuilt here instead. ADR 0399.
#[test]
fn every_cell_of_a_hatching_shares_one_decode() {
    // Eight rows of one byte each: §8.9.3 starts every row of a one-bit image on a byte
    // boundary, so a diagonal is one set bit stepping across.
    let cell = "BI /IM true /W 8 /H 8 /BPC 1 /D [1 0] ID \x11\x22\x44\x08\x11\x22\x44\x08 EI";
    let content = "/CS0 cs 1 0 0 /P0 scn 0 0 40 40 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Contents 4 0 R \
         /Resources << /Pattern << /P0 5 0 R >> \
         /ColorSpace << /CS0 [/Pattern /DeviceRGB] >> >> >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 \
         /BBox [0 0 1 1] /XStep 1 /YStep 1 /Matrix [1 0 0 1 0 0] /Resources << >> \
         /Length {} >>\nstream\n{cell}\nendstream\nendobj\n",
        content.len(),
        cell.len(),
    );
    let document = Document::open(assembled(&body)).expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    let interpretation = pdf_model::interpret(&document, &page);

    let mut rasters: Vec<Arc<[u8]>> = Vec::new();
    for command in interpretation.display_list.commands() {
        if let pdf_render::Command::Image { image, .. } = command {
            match image {
                pdf_render::ImageSource::Decoded(decoded) => {
                    rasters.push(Arc::clone(&decoded.data));
                }
                other => panic!("the fixture's stencil arrived as {other:?} rather than decoded"),
            }
        }
    }

    // Forty-two rather than forty-one to a side: the lattice covers the fill's own bounds, so a
    // 40-unit span with a step of one touches the cells at 0 through 41 inclusive.
    assert_eq!(
        rasters.len(),
        42 * 42,
        "the fixture drew a different number of cells than the lattice states: {:?}",
        interpretation.unsupported
    );
    let first = rasters.first().expect("the assertion above counted them");
    assert!(
        rasters.iter().all(|raster| Arc::ptr_eq(raster, first)),
        "the cells hold {} separate rasters of one 8 × 8 stencil",
        rasters
            .iter()
            .map(|raster| Arc::as_ptr(raster).cast::<()>())
            .collect::<std::collections::HashSet<_>>()
            .len()
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
