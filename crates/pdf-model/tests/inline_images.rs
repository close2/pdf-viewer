//! Inline images, against ISO 32000-2 §8.9.7's own rules.
//!
//! An inline image is the one image whose *extent* is a question. Everything else about it —
//! the samples, the colour space, the filters — is an ordinary image, and this file pins that
//! by rendering pages and reading pixels rather than by inspecting a dictionary: the mapping
//! from Table 91's abbreviated keys to real ones is only worth anything if a page comes out
//! right at the end of it.
//!
//! Three of the tests here are about the extent, and each is a case where a reader that gets
//! it wrong still draws *something*:
//!
//! - data holding a whitespace-delimited `EI` of its own, which a search stops at early;
//! - a `/Length` that says where the data ends, which is the only answer for filtered data;
//! - what runs after `EI`, which is what a reader that mislocates the end silently loses.
//!
//! The last is the reason the fixtures paint a marker rectangle after the image. A content
//! stream that resumes in the middle of image data does not fail — it executes whatever the
//! samples happen to spell, which is usually nothing, and the page comes out missing its
//! remaining content with no report at all.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are 40x40 pages where no index can overflow"
)]
#![expect(
    clippy::doc_markdown,
    reason = "these comments quote the standard, and a quotation with backticks added to \
              please a lint is no longer a quotation"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 40×40 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// A one-page PDF whose content stream is `content`, with `resources` as its resources.
///
/// The page is 40 units square and the content is written as raw bytes, because an inline
/// image's data is not text and several of these fixtures depend on exactly which bytes are
/// between `ID` and `EI`.
fn fixture(content: &[u8], resources: &str) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
             /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(b"\nendstream\nendobj\n");

    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    let mut rest = body.as_slice();
    while let Some(at) = position_after(rest, b"endobj\n") {
        offsets.push(out.len());
        out.extend_from_slice(&rest[..at]);
        rest = &rest[at..];
    }

    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

/// The offset just past the first occurrence of `needle`.
fn position_after(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| at + needle.len())
}

/// A 2×2 `DeviceRGB` image scaled over the whole page, written with Table 91's abbreviations.
///
/// Red and green on the top row, blue and white on the bottom, so no quadrant can be
/// confused with another under a flip in either axis — the mistake trap 2 in
/// `doc/HANDOVER.md` is about.
const QUADRANTS: &[u8] = b"\xff\x00\x00\x00\xff\x00\x00\x00\xff\xff\xff\xff";

/// Renders a fixture at one pixel per unit, and returns what could not be drawn with it.
fn render(bytes: Vec<u8>) -> (pdf_render::Raster, Vec<pdf_model::Unsupported>) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let unsupported = interpretation.unsupported.clone();
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");
    (raster, unsupported)
}

/// Renders a fixture that must draw completely.
fn render_complete(bytes: Vec<u8>) -> pdf_render::Raster {
    let (raster, unsupported) = render(bytes);
    assert!(
        unsupported.is_empty(),
        "the fixture should draw completely: {unsupported:?}"
    );
    raster
}

/// The RGBA at a point given in PDF coordinates, whose y runs the other way from a raster's.
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

/// Asserts a pixel is the given colour, allowing for the sampler's ramp between texels.
fn assert_colour(raster: &pdf_render::Raster, x: u32, y: u32, expected: [u8; 3], what: &str) {
    let [red, green, blue, alpha] = pixel(raster, x, y);
    assert!(alpha > 200, "{what}: nothing was drawn at {x},{y}");
    for (got, want) in [red, green, blue].into_iter().zip(expected) {
        assert!(
            got.abs_diff(want) < 24,
            "{what}: expected {expected:?} at {x},{y}, got {red},{green},{blue}"
        );
    }
}

/// The content stream after an image: a black square in the page's bottom-left corner.
///
/// Its whole job is to be *missing* when interpretation resumes inside the image data. The
/// square is 6×6 at the origin, clear of every quadrant test point — and it comes after the
/// `Q` that ends the image's own scaling, or the same six units would cover the page.
const MARKER: &[u8] = b" 0 0 0 rg 0 0 6 6 re f";

/// The marker square is there, which means interpretation resumed past `EI`.
fn assert_marker(raster: &pdf_render::Raster) {
    assert_colour(raster, 3, 3, [0, 0, 0], "the content after EI");
}

/// An inline image draws, with Table 91's and Table 92's abbreviations expanded.
///
/// `/W`, `/H`, `/BPC`, `/CS` and `/RGB` are all abbreviations, and a reader that does not
/// expand them has no width, no height and no colour space — so this is the whole of §8.9.7's
/// first half in one page.
#[test]
fn an_inline_image_draws_from_its_abbreviated_keys() {
    let mut content: Vec<u8> = b"q 40 0 0 40 0 0 cm BI /W 2 /H 2 /BPC 8 /CS /RGB ID ".to_vec();
    content.extend_from_slice(QUADRANTS);
    content.extend_from_slice(b" EI Q");
    content.extend_from_slice(MARKER);

    let raster = render_complete(fixture(&content, ""));
    assert_colour(&raster, 5, 35, [255, 0, 0], "top left");
    assert_colour(&raster, 35, 35, [0, 255, 0], "top right");
    assert_colour(&raster, 5, 15, [0, 0, 255], "bottom left");
    assert_colour(&raster, 35, 15, [255, 255, 255], "bottom right");
    assert_marker(&raster);
}

/// Sample data that spells ` EI ` does not end the image.
///
/// §8.9.3 fixes the layout of unfiltered samples, so a reader that computes the length from
/// `/W`, `/H`, `/BPC` and the colour space never has to look at the data at all. One that
/// searches for `EI` instead stops at the third sample here, draws a torn image, and resumes
/// interpretation inside the remaining samples — losing the marker square with no report.
#[test]
fn data_that_contains_ei_is_not_cut_short_by_it() {
    // Six `DeviceGray` samples, the middle four spelling ` EI ` — 0x20, 0x45, 0x49, 0x20.
    let samples: &[u8] = b"\xff EI \x00";
    let mut content: Vec<u8> = b"q 40 0 0 40 0 0 cm BI /W 6 /H 1 /BPC 8 /CS /G ID ".to_vec();
    content.extend_from_slice(samples);
    content.extend_from_slice(b" EI Q");
    content.extend_from_slice(MARKER);

    let raster = render_complete(fixture(&content, ""));
    assert_colour(&raster, 3, 20, [255, 255, 255], "the first sample");
    assert_colour(&raster, 36, 20, [0, 0, 0], "the sixth sample");
    assert_marker(&raster);
}

/// A document to hand the scanner, which needs one for its resource bounds.
///
/// The three tests below ask `crate::inline_image` where the data ended rather than reading
/// pixels, because that is the question, and a page cannot answer it: filtered data whose
/// end is misplaced usually still decodes to *something*, and the difference between the two
/// answers is bytes rather than colours.
fn scanning_document() -> Document {
    Document::open(fixture(b"", "")).expect("the fixture is a valid PDF")
}

/// Scans the inline image in `content`, whose `BI` is the first two bytes.
fn scan(content: &[u8]) -> pdf_model::inline_image::Scan {
    scan_window(content, true)
}

/// The same, saying whether `content` is all of the content stream that is left.
///
/// `false` is a *window* over a longer stream, which is how a page's `/Contents` actually
/// reaches the interpreter: an end the held bytes cannot locate is then a request for more of
/// them rather than a licence to guess.
fn scan_window(content: &[u8], complete: bool) -> pdf_model::inline_image::Scan {
    pdf_model::inline_image::scan(
        &scanning_document(),
        content,
        2,
        &pdf_syntax::Dictionary::new(),
        complete,
    )
}

/// `/L` says where filtered data ends, and is used in preference to searching for `EI`.
///
/// §8.9.7: the value of `/L` "is the length of the data between the ID and EI operators
/// excluding the white-space delimiting those operators". It is the only answer available for
/// compressed data, whose bytes can spell anything — including the `EI` a search would stop
/// at, which is what this data does.
#[test]
fn a_stated_length_locates_the_end_of_filtered_data() {
    let content = b"BI /W 8 /H 1 /BPC 8 /CS /G /F /Fl /L 7 ID \x01 EI \x02\x03 EI Q";
    let scanned = scan(content);

    let stream = scanned.image.expect("an image");
    assert_eq!(stream.data.as_ref(), b"\x01 EI \x02\x03");
    // Past the second `EI`, which is the real one: two bytes of ` Q` are left.
    assert_eq!(scanned.resume, content.len() - 2);
}

/// A `/L` that does not predict an `EI` is not believed.
///
/// The clause requires the entry to be right; a file where it is not would otherwise take
/// the rest of the page's content stream as image data. Checking it against the terminator
/// it predicts costs one comparison and turns a wrong length into a fallback rather than
/// into a blank page.
#[test]
fn a_length_that_does_not_predict_the_terminator_falls_back_to_the_search() {
    let content = b"BI /W 2 /H 1 /BPC 8 /CS /G /F /Fl /L 4000 ID \x01\x02 EI Q";
    let scanned = scan(content);

    let stream = scanned.image.expect("an image");
    assert_eq!(stream.data.as_ref(), b"\x01\x02");
    assert_eq!(scanned.resume, content.len() - 2);
}

/// A base-85 stream ends at its own EOD marker, not at an `EI` its characters happen to spell.
///
/// §8.9.7 makes the bytes after `ID` "a stream object's data", and §7.4.3 gives that data an
/// end: the two-character sequence (7Eh)(3Eh), over an alphabet of `!` through `u` and `z` that
/// cannot contain either byte. So the end is *stated* rather than searched for — which is what
/// the clause's own EXAMPLE writes, ending its `/F [/A85 /LZW]` image `…2HCqC~> EI`.
///
/// The data here spells a white-space-delimited `EI` of its own, which every character of is in
/// the base-85 alphabet or is white space §7.4.3 ignores. A reader that searches stops there,
/// draws a fifth of the image, and hands the rest of the encoded bytes back to the lexer as
/// operators. **The witness is a crawled document rather than an invention**: a 2951×178
/// photograph under `/F [/A85 /Fl]` whose first `EI` token stands 69 598 bytes into 1.29 MB of
/// base-85, which drew as a blank sheet where three references agree on 43.6 of 255 (session
/// 631, ADR 0464).
#[test]
fn a_base85_end_of_data_marker_ends_the_data() {
    let content = b"BI /W 8 /H 1 /BPC 8 /CS /G /F /A85 ID 87cU EI RD]j~> EI Q";
    let scanned = scan(content);

    let stream = scanned.image.expect("an image");
    assert_eq!(stream.data.as_ref(), b"87cU EI RD]j~>");
    // Past the second `EI`, which is the real one: two bytes of ` Q` are left.
    assert_eq!(scanned.resume, content.len() - 2);
}

/// A base-85 image whose marker is past the window asks for more bytes rather than searching.
///
/// The negative twin of the test above, and ADR 0454's lesson applied to this third answer: a
/// derived end the held bytes cannot *locate* is unanswerable, not wrong. Letting the search
/// run instead would stop at the `EI` the data spells — which is exactly the defect the marker
/// exists to avoid, reintroduced by the window.
#[test]
fn a_base85_marker_outside_the_window_is_truncated_rather_than_searched() {
    let content = b"BI /W 8 /H 1 /BPC 8 /CS /G /F /A85 ID 87cU EI RD]j";
    let scanned = scan_window(content, false);

    assert!(
        matches!(
            scanned.image,
            Err(pdf_model::inline_image::InlineImageError::Truncated)
        ),
        "the ~> marker is not in these bytes, so more of them are what is needed"
    );
    assert_eq!(scanned.resume, content.len());
}

/// An ASCII hex stream ends at its GREATER-THAN SIGN.
///
/// §7.4.2: "A GREATER-THAN SIGN (3Eh) indicates EOD (End Of Data)." The same rule as the
/// base-85 test above and from the same sentence of §8.9.7, and it is here because a clause
/// that states two routes is only implemented when both are (`doc/traps/parsers-and-streams.md`
/// trap 5).
///
/// The hex here is malformed — `I` is not a hexadecimal digit, so §7.4.2's "[a]ny other
/// characters shall cause an error" — and that is deliberate rather than sloppy: a *correctly*
/// encoded hex stream cannot spell `EI` at all, so this is the only shape in which the two
/// answers differ. Where the data ends is a question §7.4.2 answers before the filter runs, and
/// answering it with the stray `EI` would lose the rest of the page's content stream as well as
/// the image.
#[test]
fn an_ascii_hex_end_of_data_marker_ends_the_data() {
    let content = b"BI /W 8 /H 1 /BPC 8 /CS /G /F /AHx ID 4142 EI 4344> EI Q";
    let scanned = scan(content);

    let stream = scanned.image.expect("an image");
    assert_eq!(stream.data.as_ref(), b"4142 EI 4344>");
    assert_eq!(scanned.resume, content.len() - 2);
}

/// Filtered data with no terminator at all is reported, and consumes the rest of the stream.
///
/// The rest of the stream is the only safe place to resume: the bytes after `ID` are not a
/// program, so handing any of them back to the lexer risks drawing whatever they spell.
#[test]
fn filtered_data_with_no_terminator_is_reported() {
    let content = b"BI /W 2 /H 1 /BPC 8 /CS /G /F /Fl ID \x01\x02\x03";
    let scanned = scan(content);

    assert!(scanned.image.is_err(), "there is no EI to find");
    assert_eq!(scanned.resume, content.len());
}

/// A `/CS` name that is not a device space names a resource.
///
/// §8.9.7: "the value of the ColorSpace entry may also be the name of a colour space in the
/// ColorSpace subdictionary of the current resource dictionary". The space here is
/// `[/Indexed /DeviceRGB 1 <ff000000ff00>]`, whose two entries are red and green, so a reader
/// that ignored the resource and guessed a device space would draw grey.
#[test]
fn a_colour_space_name_is_looked_up_in_the_resources() {
    let mut content: Vec<u8> = b"q 40 0 0 40 0 0 cm BI /W 2 /H 1 /BPC 8 /CS /Cs1 ID ".to_vec();
    content.extend_from_slice(b"\x00\x01");
    content.extend_from_slice(b" EI Q");
    content.extend_from_slice(MARKER);

    let raster = render_complete(fixture(
        &content,
        "/ColorSpace << /Cs1 [/Indexed /DeviceRGB 1 <ff000000ff00>] >>",
    ));
    assert_colour(&raster, 5, 20, [255, 0, 0], "index 0 is red");
    assert_colour(&raster, 35, 20, [0, 255, 0], "index 1 is green");
    assert_marker(&raster);
}

/// An inline image that cannot be decoded is reported, and the rest of the page still draws.
///
/// The filter is `CCITTFaxDecode`, which this tree does not implement — so the image is
/// named as undrawn rather than silently skipped, which is trap 5, and the content after
/// `EI` still runs, which is what says the data was stepped over rather than executed.
#[test]
fn an_inline_image_we_cannot_decode_is_reported_and_the_page_continues() {
    let mut content: Vec<u8> =
        b"q 40 0 0 40 0 0 cm BI /W 2 /H 1 /BPC 8 /CS /G /F /CCF ID ".to_vec();
    content.extend_from_slice(b"\x01\x02\x03");
    content.extend_from_slice(b" EI Q");
    content.extend_from_slice(MARKER);

    let (raster, unsupported) = render(fixture(&content, ""));
    assert!(
        unsupported.iter().any(|item| matches!(
            item,
            pdf_model::Unsupported::Image { name } if name.starts_with("<inline>")
        )),
        "an undecodable inline image should be reported: {unsupported:?}"
    );
    assert_marker(&raster);
}

/// A `FlateDecode` stream carrying `payload` in one of RFC 1951's *stored* blocks.
///
/// A stored block holds its bytes literally, which is what makes the pair below say what it
/// means: the *encoded* data that reaches `inline_image` contains `payload` byte for byte, so
/// a test can put an `EI` inside a compressed stream and know that it is there rather than
/// hope a compressor put one there. RFC 1950's two-byte header and Adler-32 trailer wrap it,
/// which is what §7.4.4.1's `FlateDecode` names.
///
/// This is trap 8 in its usual shape: no producer writes this file, and the rule it pins is
/// one the corpus cannot state.
fn flate_stored(payload: &[u8]) -> Vec<u8> {
    // 0x78 0x01 is RFC 1950's header for a 32 KiB window with no preset dictionary, and the
    // pair is divisible by 31 as FCHECK requires.
    let mut out: Vec<u8> = vec![0x78, 0x01];
    let len = u16::try_from(payload.len()).expect("the fixtures are tens of bytes");
    // BFINAL = 1, BTYPE = 00, then LEN and its ones' complement, little-endian.
    out.push(0x01);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(payload);
    let (mut low, mut high) = (1u32, 0u32);
    for &byte in payload {
        low = (low + u32::from(byte)) % 65521;
        high = (high + low) % 65521;
    }
    out.extend_from_slice(&((high << 16) | low).to_be_bytes());
    out
}

/// The content stream around one filtered inline image, with `encoded` as its data.
fn filtered_image(encoded: &[u8]) -> Vec<u8> {
    let mut content: Vec<u8> = INLINE_FLATE.to_vec();
    content.extend_from_slice(encoded);
    content.extend_from_slice(b" EI Q");
    content
}

/// Twelve `DeviceGray` samples under `FlateDecode`, with no `/L`: answer 3's population.
const INLINE_FLATE: &[u8] = b"BI /W 12 /H 1 /BPC 8 /CS /G /F /Fl ID ";

/// Filtered data ends where its own filter says, not where a byte pair inside it reads as `EI`.
///
/// §8.9.7 makes the bytes between `ID` and `EI` "a stream object's data (see 7.3.8, "Stream
/// objects"), even though they do not follow the standard stream syntax", and §7.3.8.2 says of
/// a stream object's data that "most filters are defined so that the data shall be
/// self-limiting; that is, they use an encoding scheme in which an explicit end-of-data (EOD)
/// marker delimits the extent of the data". So a filtered extent is derivable, and the forward
/// search — which stops at the ` EI ` these compressed bytes carry — is not the answer.
///
/// Read with its twin below: that one is the same construction with no `EI` inside it, where
/// both answers agree, so the pair says the marker is being *used* rather than that a search
/// happened to be right.
#[test]
fn filtered_data_ends_at_the_filters_own_end_of_data() {
    // Twelve samples whose middle four spell ` EI ` — 0x20, 0x45, 0x49, 0x20 — which is a
    // white-space-delimited `EI` token wherever a reader looks for one.
    let encoded = flate_stored(b"\xff\xff\xff\xff EI \x00\x00\x00\x00");
    assert!(
        encoded.windows(4).any(|window| window == b" EI "),
        "the fixture is worth nothing unless the encoded bytes really carry an EI"
    );
    let content = filtered_image(&encoded);

    let scanned = scan(&content);
    let stream = scanned.image.expect("an image");
    assert_eq!(
        stream.data.as_ref(),
        encoded.as_slice(),
        "the data is the whole Flate stream, not the prefix before the EI inside it"
    );
    // Past the real `EI`, which is the one after the data: two bytes of ` Q` are left.
    assert_eq!(scanned.resume, content.len() - 2);
}

/// The twin: the same image with no `EI` in its compressed bytes ends in the same place.
#[test]
fn filtered_data_without_an_ei_inside_it_ends_where_it_always_did() {
    let encoded = flate_stored(b"\xff\xff\xff\xff\x11\x22\x33\x44\x00\x00\x00\x00");
    assert!(
        !encoded.windows(2).any(|window| window == b"EI"),
        "the twin is worth nothing unless the encoded bytes carry no EI at all"
    );
    let content = filtered_image(&encoded);

    let scanned = scan(&content);
    let stream = scanned.image.expect("an image");
    assert_eq!(stream.data.as_ref(), encoded.as_slice());
    assert_eq!(scanned.resume, content.len() - 2);
}

/// Through a window, a filter that runs out of input asks for more bytes rather than searching.
///
/// This is what makes the answer hold for an image larger than the reader's window, which is
/// the population it matters most for: `crate::content::reader` hands `scan` a window and grows
/// it while the answer is [`Truncated`](pdf_model::inline_image::InlineImageError::Truncated).
/// Answering with the forward search here would put the guess back for exactly the images the
/// derivation was written for — which is the shape ADR 0454 fixed for unfiltered data.
#[test]
fn a_window_that_cuts_the_filters_marker_asks_for_more_bytes() {
    let encoded = flate_stored(b"\xff\xff\xff\xff EI \x00\x00\x00\x00");
    let content = filtered_image(&encoded);
    // Two bytes short of the end of the encoded data, and well past the ` EI ` inside it.
    let cut = INLINE_FLATE.len() + encoded.len() - 2;

    let scanned = pdf_model::inline_image::scan(
        &scanning_document(),
        &content[..cut],
        2,
        &pdf_syntax::Dictionary::new(),
        false,
    );
    assert_eq!(
        scanned.image.expect_err("the window cuts the data"),
        pdf_model::inline_image::InlineImageError::Truncated
    );
}
