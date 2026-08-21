//! What a `/Decode` array means on a `JPEG 2000` image, and the one condition that silences it.
//!
//! ISO 32000-2 §7.4.9 rearranges three image-dictionary entries around the codestream, and the
//! third of them is not like the other two. `/ColorSpace` is optional and where present
//! overrides the codestream's; `/BitsPerComponent` is ignored either way. `/Decode` is
//! conditional, and the condition is not the filter:
//!
//! > If ColorSpace is absent, then the Decode array shall be ignored unless ImageMask is true
//!
//! Table 87's own `/Decode` row states it the same way round — "If the image uses the
//! JPXDecode filter and if ColorSpace is absent, the Decode array shall be ignored unless
//! ImageMask is true" — so a dictionary that states its colour space states the map into it as
//! well, exactly as it would for any other filter. Reading the condition as "this filter
//! ignores `/Decode`" costs a page its polarity in silence.
//!
//! **The witness is a crawled document rather than an invention**: a 16-page product catalogue
//! whose cover photograph and header are `/JPXDecode` CMYK images under `/ColorSpace [/ICCBased …]`
//! with `/Decode [1 0 1 0 1 0 1 0]`, drawn as their own complements — a green background as
//! dark purple, a black header as beige — silently, at +77.113 of 255 against three references
//! agreeing within 0.75 (session 636, ADR 0468). It is somebody else's crawled web page and is
//! not in this repository, so the fixture here is generated, the same rule `jpx_channels.rs`
//! and `dct_components.rs` follow.
//!
//! The third test is the negative twin and it is the clause's own condition: with no
//! `/ColorSpace` in the dictionary the same array changes nothing.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    reason = "test code: a malformed fixture, an out-of-range pixel or an unavailable decoder \
              should fail loudly, and the fixtures are 8x8 pages where no index can overflow"
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

/// Pixel budget, far above the 8×8 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// The one sample value the fixture codestream carries, in every pixel of every channel.
const SAMPLE: u8 = 200;

/// An 8×8 one-component JPEG 2000 codestream, every pixel [`SAMPLE`].
///
/// A **bare codestream** — SOC, SIZ, COD, QCD, SOT, SOD, EOC — with no JP2 boxes, so the
/// dictionary is the only thing that can say what the sample means. Its SIZ states one
/// component of eight unsigned bits and its COD the reversible 5/3 wavelet, so it is lossless
/// and the value above comes back exactly; `opj_decompress` on these bytes confirms it.
///
/// Generated rather than written, because a JPEG 2000 codestream cannot be written by hand
/// legibly. The command, whose output this is with its `COM` comment marker removed so that no
/// encoder version is baked in:
///
/// ```sh
/// python3 -c "import numpy as np; np.full(64, 200, np.uint8).tofile('gray.raw')"
/// opj_compress -i gray.raw -o gray.j2k -F 8,8,1,8,u -n 1 -r 1
/// ```
const ONE_COMPONENT: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x01, 0xff, 0x52, 0x00,
    0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40,
    0x40, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x23, 0x00, 0x01, 0xff, 0x93, 0xcf,
    0xb4, 0x48, 0x14, 0x00, 0x5c, 0xa3, 0x65, 0x5d, 0xb0, 0x00, 0x03, 0x09, 0x08, 0xd5, 0x0a, 0x18,
    0x48, 0x4b, 0xff, 0x7f, 0xff, 0xd9,
];

/// A one-page 8×8 PDF drawing [`ONE_COMPONENT`] over the whole page.
///
/// `entries` goes into the image dictionary beside `/Filter /JPXDecode`, which is where each
/// test states its `/ColorSpace` and its `/Decode`.
fn image_fixture(entries: &str) -> Vec<u8> {
    let content = b"q 8 0 0 8 0 0 cm /Im0 Do Q".as_slice();
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 8 8] \
          /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
    );
    body.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(b"\nendstream\nendobj\n");
    body.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 8 /Height 8 \
             /Filter /JPXDecode {entries} /Length {} >>\nstream\n",
            ONE_COMPONENT.len()
        )
        .as_bytes(),
    );
    body.extend_from_slice(ONE_COMPONENT);
    body.extend_from_slice(b"\nendstream\nendobj\n");
    assemble(&body)
}

/// Wraps a body of `endobj`-terminated objects in a header, a cross-reference table and a
/// trailer.
fn assemble(body: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    let mut rest = body;
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

/// The grey level in the middle of an 8×8 raster, which the fixtures make uniform.
fn centre(raster: &pdf_render::Raster) -> u8 {
    let at = ((4 * raster.width) as usize + 4) * 4;
    let [red, green, blue] = [raster.data[at], raster.data[at + 1], raster.data[at + 2]];
    assert_eq!(
        (red, red),
        (green, blue),
        "a one-component image in a grey space draws a neutral colour"
    );
    red
}

/// Skips a test rather than failing it where the confined image decoder cannot start.
fn sandbox_or_panic() {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
        panic!("the sandboxed image decoder is not available: {error}");
    }
}

/// With no `/Decode`, Table 88's default applies and the sample passes through.
///
/// The baseline the two tests below are read against: without it, "the array was applied"
/// and "the image decoded differently" cannot be told apart.
#[test]
fn a_declared_space_without_a_decode_array_draws_the_sample_itself() {
    sandbox_or_panic();

    let (raster, unsupported) = render(image_fixture("/ColorSpace /DeviceGray"));
    assert!(unsupported.is_empty(), "{unsupported:?}");
    assert_eq!(
        centre(&raster),
        SAMPLE,
        "Table 88's default pair for DeviceGray is [0 1], which is the identity on a channel"
    );
}

/// With a `/ColorSpace` stated, `/Decode [1 0]` inverts the samples.
///
/// §8.9.5.2 maps sample 0 to D min and the largest sample to D max, so `[1 0]` sends 200 of
/// 255 to 55 of 255. The expected value is the clause's arithmetic and not another renderer's
/// output: `1 − 200 ÷ 255` is `0.21568…`, which is 55 on an eight-bit channel.
#[test]
fn a_decode_array_beside_a_declared_space_is_applied() {
    sandbox_or_panic();

    let (raster, unsupported) = render(image_fixture("/ColorSpace /DeviceGray /Decode [1 0]"));
    assert!(unsupported.is_empty(), "{unsupported:?}");
    assert_eq!(
        centre(&raster),
        u8::MAX - SAMPLE,
        "/Decode [1 0] maps sample 0 to 1.0 and sample 255 to 0.0"
    );
}

/// With no `/ColorSpace` in the dictionary, the same array is ignored.
///
/// This is §7.4.9's condition itself, and it is what keeps the fix from becoming "apply
/// `/Decode` on this filter too": the codestream carries no JP2 boxes, so the space is the
/// one the clause's fallback names for a single ordinary channel — DeviceGray — and the array
/// beside it changes nothing at all.
#[test]
fn a_decode_array_with_no_declared_space_is_ignored() {
    sandbox_or_panic();

    let (raster, unsupported) = render(image_fixture("/Decode [1 0]"));
    assert!(unsupported.is_empty(), "{unsupported:?}");
    assert_eq!(
        centre(&raster),
        SAMPLE,
        "ColorSpace absent, so the Decode array shall be ignored unless ImageMask is true"
    );
}
