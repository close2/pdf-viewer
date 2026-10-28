//! An embedded file this reader can draw, drawn on pages appended to the archive.
//!
//! ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9 admit an embedded file only where it
//! conforms to a part of ISO 19005, and a `preserve` row at either site lays out what the
//! removal takes (`doc/adr/1014`'s permission, the loss `attachment-as-pages`). A text file is
//! set as the characters it holds (`super::preserve`); this file is the other two shapes R15
//! names, and both end as [`super::derived::ImportedPages`], the shape a derived file's pages
//! already take, so that one path composes, reports and links every drawn page:
//!
//! - **a PDF** is converted by this converter to the outer target exactly as a derived file is
//!   ([`super::derived::convert`], the same budget, one level) and its pages are imported by
//!   [`super::derived::import_pages`]: each page a form `XObject` in a wrapper that also draws
//!   its annotations' normal appearances;
//! - **an image** in a format this reader decodes — PNG (W3C's Portable Network Graphics) and a
//!   JPEG whose frame header names a process §7.4.8's `DCTDecode` reads — becomes one image
//!   `XObject` on one page of its own size.
//!
//! **The device colour either brings is licensed by the archive's own output intent**, the
//! construction `doc/adr/2539` gives a derived file's pages: the families a drawn file uses are
//! named beside it, and `super::prepare` asks for the intent as a page painting in them would
//! (`doc/adr/2536`'s drawn-attachments section).

use std::io::Cursor;

use pdf_archive::survey::DeviceFamily;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use super::COMPRESSION_LEVEL;
use super::decision::Because;
use super::derived::ImportedPages;
use super::prepare::Spare;
use super::preserve::Marks;

/// What an embedded file's first bytes say it is, among the shapes this file draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    /// §7.5.2's header, `%PDF-` at the file's first byte.
    Pdf,
    /// PNG's eight-byte signature.
    Png,
    /// A JPEG interchange file's start-of-image marker, `FF D8`, and the next marker's `FF`.
    Jpeg,
}

/// The shape `bytes` open with, or `None` for a file this reader does not draw.
pub(super) fn shape_of(bytes: &[u8]) -> Option<Shape> {
    if bytes.starts_with(b"%PDF-") {
        Some(Shape::Pdf)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(Shape::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(Shape::Jpeg)
    } else {
        None
    }
}

/// One image ready to be written as an image `XObject`.
#[derive(Debug, Clone)]
pub(super) struct Image {
    /// Its width and height in samples.
    pub(super) width: u32,
    /// See `width`.
    pub(super) height: u32,
    /// The device family its samples are in.
    pub(super) family: DeviceFamily,
    /// Bits per component: 8, or 16 for a sixteen-bit PNG.
    bits: u8,
    /// The stream's data, already encoded, and the filter that decodes it.
    data: Vec<u8>,
    /// `DCTDecode` for a JPEG, whose bytes are the file's own; `FlateDecode` otherwise.
    filter: &'static [u8],
    /// A PNG's alpha channel, Flate-encoded, written as §11.6.5.2's soft-mask image.
    alpha: Option<Vec<u8>>,
}

/// Why an image is not drawn because it has more samples than the budget's image ceiling.
const IMAGE_PAST_THE_BUDGET: &str = "the image has more pixels than this conversion's budget lets \
     one image decode (Budget::max_pixels)";

/// Why a PNG is not drawn because it does not decode.
const PNG_DOES_NOT_DECODE: &str = "the PNG file does not decode";

/// Why a JPEG is not drawn because no frame header precedes its first scan.
const JPEG_NO_FRAME: &str = "the JPEG file states no frame header before its first scan, so its \
     size and components are unknown";

/// Why a JPEG is not drawn because of the coding process its frame header names.
const JPEG_PROCESS: &str = "the JPEG file's frame header names a coding process other than the \
     baseline, extended sequential or progressive Huffman processes this reader's DCTDecode \
     reads, at eight bits per sample";

/// Why a JPEG is not drawn because of its component count.
const JPEG_COMPONENTS: &str = "the JPEG file has a component count other than one or three; a \
     four-component file's ink polarity is stated only by the conventions of Adobe's APP14 \
     segment, which this converter does not take as the file's word";

/// Why an image is not drawn because its page would be outside the sizes a page may take.
const IMAGE_PAGE_SIZE: &str = "the image's sides are too far apart for one page to show it whole \
     between the 3 and 14 400 units ISO 19005-2 section 6.1.13 holds a page boundary to";

/// Why an image is not drawn because no object number is free.
const IMAGE_NO_NUMBER: &str = "no free object number was found for a drawn image's objects";

/// Reads an image file of `shape`, or the sentence saying why it is not drawn.
///
/// `max_pixels` is the budget's per-image ceiling, checked against the header before a sample
/// is decoded.
pub(super) fn image(bytes: &[u8], shape: Shape, max_pixels: u64) -> Result<Image, &'static str> {
    match shape {
        Shape::Png => png_image(bytes, max_pixels),
        Shape::Jpeg => jpeg_image(bytes, max_pixels),
        Shape::Pdf => Err(PNG_DOES_NOT_DECODE),
    }
}

/// A PNG, decoded with palette and low bit depths expanded and its alpha split off.
fn png_image(bytes: &[u8], max_pixels: u64) -> Result<Image, &'static str> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|_| PNG_DOES_NOT_DECODE)?;
    let (width, height) = (reader.info().width, reader.info().height);
    if u64::from(width).saturating_mul(u64::from(height)) > max_pixels {
        return Err(IMAGE_PAST_THE_BUDGET);
    }
    let size = reader.output_buffer_size().ok_or(PNG_DOES_NOT_DECODE)?;
    let mut buffer = vec![0; size];
    let frame = reader
        .next_frame(&mut buffer)
        .map_err(|_| PNG_DOES_NOT_DECODE)?;
    buffer.truncate(frame.buffer_size());
    let (family, colour, with_alpha) = match frame.color_type {
        png::ColorType::Grayscale => (DeviceFamily::Gray, 1_usize, false),
        png::ColorType::GrayscaleAlpha => (DeviceFamily::Gray, 1, true),
        png::ColorType::Rgb => (DeviceFamily::Rgb, 3, false),
        png::ColorType::Rgba => (DeviceFamily::Rgb, 3, true),
        // `EXPAND` turns a palette into RGB samples, so this arm is a file the transformation
        // did not reach, and it is refused rather than guessed at.
        png::ColorType::Indexed => return Err(PNG_DOES_NOT_DECODE),
    };
    let bits: u8 = match frame.bit_depth {
        png::BitDepth::Sixteen => 16,
        _ => 8,
    };
    // A sample is one byte or two, big-endian in both formats (§8.9.2's order is PNG's).
    let sample: usize = if bits == 16 { 2 } else { 1 };
    let (samples, alpha) = if with_alpha {
        let stride = colour.saturating_add(1).saturating_mul(sample);
        let pixels = buffer.len().checked_div(stride).unwrap_or(0);
        let mut samples = Vec::with_capacity(pixels.saturating_mul(colour).saturating_mul(sample));
        let mut alpha = Vec::with_capacity(pixels.saturating_mul(sample));
        for pixel in buffer.chunks_exact(stride) {
            let (colours, opacity) = pixel.split_at(colour.saturating_mul(sample));
            samples.extend_from_slice(colours);
            alpha.extend_from_slice(opacity);
        }
        (samples, Some(alpha))
    } else {
        (buffer, None)
    };
    let data = flate_encode(&samples, COMPRESSION_LEVEL).ok_or(PNG_DOES_NOT_DECODE)?;
    let alpha = alpha
        .map(|alpha| flate_encode(&alpha, COMPRESSION_LEVEL).ok_or(PNG_DOES_NOT_DECODE))
        .transpose()?;
    Ok(Image {
        width,
        height,
        family,
        bits,
        data,
        filter: b"FlateDecode",
        alpha,
    })
}

/// A JPEG, read for its frame header only: its bytes become the stream's data unchanged.
///
/// The markers are walked from the start-of-image to the first frame header, each segment
/// skipped by its own length, so the walk is bounded by the file and visits each byte once.
fn jpeg_image(bytes: &[u8], max_pixels: u64) -> Result<Image, &'static str> {
    let mut at = 2usize;
    loop {
        // Fill bytes before a marker are `FF`s (ITU-T T.81 section B.1.1.2).
        while bytes.get(at) == Some(&0xFF) && bytes.get(at.saturating_add(1)) == Some(&0xFF) {
            at = at.saturating_add(1);
        }
        let (Some(0xFF), Some(&marker)) = (bytes.get(at), bytes.get(at.saturating_add(1))) else {
            return Err(JPEG_NO_FRAME);
        };
        let length = bytes
            .get(at.saturating_add(2)..at.saturating_add(4))
            .map(|two| usize::from(u16::from_be_bytes([two[0], two[1]])))
            .ok_or(JPEG_NO_FRAME)?;
        match marker {
            0xC0..=0xC2 => {
                let header = bytes
                    .get(at.saturating_add(4)..at.saturating_add(10))
                    .ok_or(JPEG_NO_FRAME)?;
                if header[0] != 8 {
                    return Err(JPEG_PROCESS);
                }
                let height = u32::from(u16::from_be_bytes([header[1], header[2]]));
                let width = u32::from(u16::from_be_bytes([header[3], header[4]]));
                // A height of zero is stated later by a DNL segment, which this walk does not
                // reach; the stream's `/Height` would then be a guess.
                if width == 0 || height == 0 {
                    return Err(JPEG_NO_FRAME);
                }
                if u64::from(width).saturating_mul(u64::from(height)) > max_pixels {
                    return Err(IMAGE_PAST_THE_BUDGET);
                }
                let family = match header[5] {
                    1 => DeviceFamily::Gray,
                    3 => DeviceFamily::Rgb,
                    _ => return Err(JPEG_COMPONENTS),
                };
                return Ok(Image {
                    width,
                    height,
                    family,
                    bits: 8,
                    data: bytes.to_vec(),
                    filter: b"DCTDecode",
                    alpha: None,
                });
            }
            0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => return Err(JPEG_PROCESS),
            0xDA | 0xD9 => return Err(JPEG_NO_FRAME),
            _ => {}
        }
        if length < 2 {
            return Err(JPEG_NO_FRAME);
        }
        at = at.saturating_add(2).saturating_add(length);
    }
}

/// The page one image is drawn on, written into the source's numbering.
///
/// **One sample is one unit**, so an image is drawn at the size it states in samples, which is
/// a choice: the standard does not say how large a page an attachment is drawn on, and a PNG's
/// `pHYs` or a JFIF density is the producer's statement for a printer rather than for this page.
/// It is scaled down, both sides alike, where a side would pass 14 400 units, and up where one
/// would fall under 3 — ISO 19005-2 section 6.1.13's page boundary limit, held at part 4 too so
/// that one drawn page is one page at every target — and refused where no one scale fits both.
///
/// # Errors
///
/// A [`Because`] naming the page size or the object numbers.
pub(super) fn image_page(
    source: &Document,
    image: &Image,
    spare: &mut Spare,
) -> Result<ImportedPages, Because> {
    let (width, height) = page_size(image.width, image.height)?;
    let mut written = Vec::new();
    let mut take = || {
        spare
            .take(source)
            .ok_or(Because::NotBuiltYet(IMAGE_NO_NUMBER))
    };
    let mask = match &image.alpha {
        Some(alpha) => {
            let at = take()?;
            written.push((
                at,
                image_stream(
                    image,
                    DeviceFamily::Gray,
                    alpha.clone(),
                    b"FlateDecode",
                    None,
                ),
            ));
            Some(at)
        }
        None => None,
    };
    let picture = take()?;
    written.push((
        picture,
        image_stream(image, image.family, image.data.clone(), image.filter, mask),
    ));
    let form = take()?;
    let content = format!("q {width} 0 0 {height} 0 0 cm /Drawn Do Q");
    let data = flate_encode(content.as_bytes(), COMPRESSION_LEVEL)
        .ok_or(Because::NotBuiltYet(IMAGE_NO_NUMBER))?;
    let mut xobjects = Dictionary::new();
    xobjects.insert(Name::new(&b"Drawn"[..]), Object::Reference(picture));
    let mut resources = Dictionary::new();
    resources.insert(Name::new(&b"XObject"[..]), Object::Dictionary(xobjects));
    let mut dict = Dictionary::new();
    dict.insert(name(b"Type"), Object::Name(name(b"XObject")));
    dict.insert(name(b"Subtype"), Object::Name(name(b"Form")));
    dict.insert(
        name(b"BBox"),
        Object::Array(
            [0.0, 0.0, width, height]
                .into_iter()
                .map(|value| Object::Real(f64::from(value)))
                .collect(),
        ),
    );
    dict.insert(name(b"Resources"), Object::Dictionary(resources));
    dict.insert(name(b"Filter"), Object::Name(name(b"FlateDecode")));
    dict.insert(name(b"Length"), Object::Integer(length(data.len())));
    written.push((
        form,
        Object::Stream(std::sync::Arc::new(Stream {
            dict,
            data: data.into(),
            decryption_failed: false,
        })),
    ));
    let page_box = [0.0, 0.0, width, height];
    Ok(ImportedPages {
        written,
        pages: vec![Marks {
            appearance: form,
            placement: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            media_box: page_box,
            crop_box: page_box,
            rotate: 0,
        }],
    })
}

/// The page's width and height in units for an image of `width` by `height` samples.
fn page_size(width: u32, height: u32) -> Result<(f32, f32), Because> {
    const LEAST: f64 = 3.0;
    const MOST: f64 = 14_400.0;
    let (width, height) = (f64::from(width), f64::from(height));
    let (short, long) = (width.min(height), width.max(height));
    let scale = if long > MOST {
        MOST / long
    } else if short < LEAST {
        LEAST / short
    } else {
        1.0
    };
    if short * scale < LEAST - f64::EPSILON || long * scale > MOST + f64::EPSILON {
        return Err(Because::NotBuiltYet(IMAGE_PAGE_SIZE));
    }
    // Both within Table C.1's range by the check above, so the narrowing loses no unit.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "both values lie between 3 and 14 400 by the check above, well inside f32"
    )]
    Ok(((width * scale) as f32, (height * scale) as f32))
}

/// An image `XObject` of `image`'s size in `family`, its data encoded under `filter`.
fn image_stream(
    image: &Image,
    family: DeviceFamily,
    data: Vec<u8>,
    filter: &'static [u8],
    mask: Option<ObjectId>,
) -> Object {
    let mut dict = Dictionary::new();
    dict.insert(name(b"Type"), Object::Name(name(b"XObject")));
    dict.insert(name(b"Subtype"), Object::Name(name(b"Image")));
    dict.insert(name(b"Width"), Object::Integer(i64::from(image.width)));
    dict.insert(name(b"Height"), Object::Integer(i64::from(image.height)));
    dict.insert(
        name(b"ColorSpace"),
        Object::Name(name(match family {
            DeviceFamily::Gray => b"DeviceGray",
            DeviceFamily::Rgb => b"DeviceRGB",
            DeviceFamily::Cmyk => b"DeviceCMYK",
        })),
    );
    dict.insert(
        name(b"BitsPerComponent"),
        Object::Integer(i64::from(image.bits)),
    );
    dict.insert(name(b"Filter"), Object::Name(name(filter)));
    dict.insert(name(b"Length"), Object::Integer(length(data.len())));
    if let Some(mask) = mask {
        dict.insert(name(b"SMask"), Object::Reference(mask));
    }
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}

/// A name from its bytes.
fn name(bytes: &[u8]) -> Name {
    Name::new(bytes)
}

/// A stream's `/Length`, which a `Vec` this process holds always fits.
fn length(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_shape_is_read_off_its_first_bytes() {
        assert_eq!(shape_of(b"%PDF-1.7\n"), Some(Shape::Pdf));
        assert_eq!(shape_of(b"\x89PNG\r\n\x1a\n...."), Some(Shape::Png));
        assert_eq!(shape_of(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(Shape::Jpeg));
        assert_eq!(shape_of(b"plain text"), None);
    }

    #[test]
    fn a_page_keeps_its_image_inside_the_sizes_a_page_may_take() {
        assert_eq!(page_size(200, 100).ok(), Some((200.0, 100.0)));
        assert_eq!(page_size(28_800, 100).ok(), Some((14_400.0, 50.0)));
        assert_eq!(page_size(1, 2).ok(), Some((3.0, 6.0)));
        assert!(
            page_size(100_000, 1).is_err(),
            "no one scale fits both sides"
        );
    }

    #[test]
    fn a_jpeg_frame_header_is_found_past_its_application_segments() {
        // SOI, an APP0 of four bytes' length, then SOF0: 8 bits, 2 high, 3 wide, 3 components.
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00,
            0x02, 0x00, 0x03, 0x03,
        ];
        let image = jpeg_image(&jpeg, 1_000).expect("the frame header is read");
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(image.family, DeviceFamily::Rgb);
        assert_eq!(jpeg_image(&jpeg, 5).err(), Some(IMAGE_PAST_THE_BUDGET));
        // An arithmetic-coded frame (SOF9) is refused by its process.
        let mut arithmetic = jpeg;
        arithmetic[9] = 0xC9;
        assert_eq!(jpeg_image(&arithmetic, 1_000).err(), Some(JPEG_PROCESS));
    }
}
