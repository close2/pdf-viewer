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

/// The content stream around one filtered inline image, with `encoded` as its data and `entries`
/// as the dictionary entries that name its filter.
///
/// Eight `DeviceGray` samples in a row, so every fixture below differs only in its filter and
/// its bytes.
fn image_under(entries: &str, encoded: &[u8]) -> Vec<u8> {
    let mut content: Vec<u8> = format!("BI /W 8 /H 1 /BPC 8 /CS /G {entries} ID ").into_bytes();
    content.extend_from_slice(encoded);
    content.extend_from_slice(b" EI Q");
    content
}

/// Asserts that `encoded` is what the scan takes as the image's data, and that interpretation
/// resumes past the real `EI` — the two bytes of ` Q` at the end of [`image_under`]'s stream.
fn assert_data_is(entries: &str, encoded: &[u8], what: &str) {
    let content = image_under(entries, encoded);
    let scanned = scan(&content);
    let stream = scanned.image.expect("an image");
    assert_eq!(stream.data.as_ref(), encoded, "{what}");
    assert_eq!(scanned.resume, content.len() - 2, "{what}: resume");
}

/// A `RunLengthDecode` image ends at its own EOD byte, not at an `EI` its runs happen to carry.
///
/// §7.4.5 needs no decoder to say where its data ends, and that is the whole of this route: "the
/// encoded data shall be a sequence of runs, where each run shall consist of a length byte
/// followed by 1 to 128 bytes of data", and "[a] length value of 128 shall denote EOD". Every
/// byte is therefore either a header or is counted by one, so a walk from header to header
/// reaches the 128 without reconstructing a sample.
///
/// The first run here holds ` EI ` literally, which is a white-space-delimited `EI` token
/// wherever a reader looks for one. A search stops there, keeps nothing of the image, and hands
/// four samples' worth of run data back to the lexer as operators.
#[test]
fn run_length_data_ends_at_its_own_eod_byte() {
    // Header 3 — four literal bytes — spelling ` EI `; then four more; then the EOD.
    let encoded: &[u8] = b"\x03 EI \x03\xff\xff\xff\xff\x80";
    assert!(
        encoded.windows(4).any(|window| window == b" EI "),
        "the fixture is worth nothing unless the encoded bytes really carry an EI"
    );
    assert_data_is("/F /RL", encoded, "the run-length data is whole");
}

/// The twin: the same runs with no `EI` in them end in the same place.
#[test]
fn run_length_data_without_an_ei_inside_it_ends_where_it_always_did() {
    let encoded: &[u8] = b"\x03\x01\x02\x03\x04\x03\xff\xff\xff\xff\x80";
    assert!(
        !encoded.windows(2).any(|window| window == b"EI"),
        "the twin is worth nothing unless the encoded bytes carry no EI at all"
    );
    assert_data_is("/F /RL", encoded, "the twin's run-length data is whole");
}

/// Through a window, a run-length walk that runs out of input asks for more bytes.
///
/// The same shape as the `FlateDecode` window test below, and it is the case the whole
/// derivation exists for: an inline image large enough to matter is larger than a window, and
/// answering with the search there would put the guess back exactly where it costs most.
#[test]
fn a_window_that_cuts_the_run_length_eod_asks_for_more_bytes() {
    let encoded: &[u8] = b"\x03 EI \x03\xff\xff\xff\xff\x80";
    let content = image_under("/F /RL", encoded);
    // Cut before the EOD byte itself, and well past the ` EI ` inside the first run.
    let cut = content.len() - 6;

    let scanned = scan_window(&content[..cut], false);
    assert_eq!(
        scanned.image.expect_err("the window cuts the data"),
        pdf_model::inline_image::InlineImageError::Truncated
    );
}

/// A JPEG codestream carrying SOI, one scan and EOI, with `entropy` as its entropy-coded data.
///
/// §7.4.8 states `DCTDecode`'s framing by reference — the data is "encoded in the JPEG baseline
/// format in accordance with ISO/IEC 10918 (all parts)" — so what a fixture has to be right
/// about is that standard's marker structure rather than any coefficient. The scan header is a
/// well-formed SOS for one component: `Ls` 8, one component, selector 1, tables 0, `Ss` 0, `Se`
/// 63, `Ah`/`Al` 0.
fn jpeg_codestream(entropy: &[u8]) -> Vec<u8> {
    assert!(
        !entropy.contains(&0xff),
        "entropy-coded data with an FF in it would need 10918-1's stuffed zero after it"
    );
    let mut out: Vec<u8> = vec![0xff, 0xd8];
    out.extend_from_slice(&[0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3f, 0x00]);
    out.extend_from_slice(entropy);
    out.extend_from_slice(&[0xff, 0xd9]);
    out
}

/// A `DCTDecode` image ends at ISO/IEC 10918-1's EOI, not at an `EI` its entropy data spells.
///
/// The bytes an entropy coder emits are arbitrary, so ` EI ` can stand anywhere inside them —
/// and here it does. §7.3.8.2's self-limiting data is what settles it, with 10918-1 supplying
/// the marker the clause promises rather than §7.4.8 spelling one out.
#[test]
fn a_jpeg_ends_at_its_own_end_of_image_marker() {
    let encoded = jpeg_codestream(b"\x11\x22 EI \x33\x44");
    assert!(
        encoded.windows(4).any(|window| window == b" EI "),
        "the fixture is worth nothing unless the encoded bytes really carry an EI"
    );
    assert_data_is("/F /DCT", &encoded, "the codestream is whole");
}

/// The twin: the same codestream with no `EI` in it ends in the same place.
#[test]
fn a_jpeg_without_an_ei_inside_it_ends_where_it_always_did() {
    let encoded = jpeg_codestream(b"\x11\x22\x33\x44\x55\x66");
    assert!(
        !encoded.windows(2).any(|window| window == b"EI"),
        "the twin is worth nothing unless the encoded bytes carry no EI at all"
    );
    assert_data_is("/F /DCT", &encoded, "the twin's codestream is whole");
}

/// A thumbnail inside an `APPn` segment carries its own EOI, and it does not end the image.
///
/// **This is why the answer is a walk over 10918-1's segments rather than a search for `FFD9`.**
/// An application segment may hold anything, and what a camera puts in one is an entire second
/// JPEG; its EOI stands hundreds of bytes before the outer image's. Stepping over each segment
/// by the length it states is what makes the inner marker invisible, and nothing weaker does.
///
/// The outer entropy data also spells ` EI `, so a reader has to get *both* right: a search for
/// `EI` stops in the middle and a search for `FFD9` stops at the thumbnail.
#[test]
fn a_jpeg_thumbnails_own_end_of_image_does_not_end_the_outer_one() {
    let thumbnail = jpeg_codestream(b"\x01\x02\x03\x04");
    let mut encoded: Vec<u8> = vec![0xff, 0xd8];
    // APP1, whose length counts its own two bytes and then the thumbnail.
    let length = u16::try_from(thumbnail.len() + 2).expect("a fixture of tens of bytes");
    encoded.extend_from_slice(&[0xff, 0xe1]);
    encoded.extend_from_slice(&length.to_be_bytes());
    encoded.extend_from_slice(&thumbnail);
    encoded.extend_from_slice(&[0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3f, 0x00]);
    encoded.extend_from_slice(b"\x11 EI \x22");
    encoded.extend_from_slice(&[0xff, 0xd9]);
    assert!(
        encoded.windows(2).filter(|w| *w == [0xff, 0xd9]).count() == 2,
        "the fixture is worth nothing unless there are two EOI markers in it"
    );

    assert_data_is("/F /DCT", &encoded, "the outer codestream is whole");
}

/// A Group 4 `CCITTFaxDecode` image ends at its end-of-block pattern.
///
/// §7.4.6 Table 11's `/EndOfBlock` defaults to true, which makes the pattern a requirement on
/// the data — "[t]he end-of-block pattern shall be the CCITT end-of-facsimile-block (EOFB) or
/// return-to-control (RTC) appropriate for the K parameter" — and `/K -1` selects Group 4, whose
/// EOFB is two of ITU-T T.4's end-of-line codes. The extent then follows from the clause's own
/// sentence about what a filter does there: "[w]hen a filter reaches EOD, it shall always skip
/// to the next byte boundary following the encoded data."
///
/// `00 10 01` is those twenty-four bits: eleven zeros and a one, twice over. The four bytes
/// before it spell ` EI `, which no valid sequence of T.4 codewords could be mistaken for and
/// which a forward search stops at all the same.
#[test]
fn a_group_4_fax_ends_at_its_end_of_block_pattern() {
    let encoded: &[u8] = b" EI \x00\x10\x01";
    assert!(
        encoded.windows(4).any(|window| window == b" EI "),
        "the fixture is worth nothing unless the encoded bytes really carry an EI"
    );
    assert_data_is(
        "/F /CCF /DP << /K -1 /Columns 8 >>",
        encoded,
        "the fax data is whole",
    );
}

/// The twin: the same Group 4 stream with no `EI` in it ends in the same place.
#[test]
fn a_group_4_fax_without_an_ei_inside_it_ends_where_it_always_did() {
    let encoded: &[u8] = b"\x11\x22\x33\x44\x00\x10\x01";
    assert!(
        !encoded.windows(2).any(|window| window == b"EI"),
        "the twin is worth nothing unless the encoded bytes carry no EI at all"
    );
    assert_data_is(
        "/F /CCF /DP << /K -1 /Columns 8 >>",
        encoded,
        "the twin's fax data is whole",
    );
}

/// Group 3 ends on six end-of-line codes rather than two, and `/K` is what says which.
///
/// Table 11 says only "appropriate for the K parameter"; T.4's RTC is six consecutive
/// end-of-line codes and T.6's EOFB is two, and reading `/K` is the whole of the difference. A
/// reader that took two everywhere would end this image a third of the way through its
/// end-of-block pattern — which the `EI` check would then refuse, so the cost is not a wrong
/// image but a fall back to the search that this route exists to replace.
#[test]
fn a_group_3_fax_ends_on_six_end_of_lines_rather_than_two() {
    // Six end-of-line codes: seventy-two bits, and the pattern repeats every three bytes.
    let encoded: &[u8] = b" EI \x00\x10\x01\x00\x10\x01\x00\x10\x01";
    assert_data_is(
        "/F /CCF /DP << /K 0 /Columns 8 >>",
        encoded,
        "the Group 3 fax data is whole",
    );
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
