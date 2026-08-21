//! Which of a JPEG 2000 codestream's channels carry colour, when the dictionary states a space.
//!
//! ISO 32000-2 §7.4.9 puts the answer in one sentence, of `/ColorSpace`:
//!
//! > If present, it shall determine how the image samples are interpreted, and the colour space
//! > specifications in the JPEG 2000 data shall be ignored. The number of ordinary colour
//! > channels in the JPEG 2000 data shall match the number of components in the colour space
//!
//! **Which channel is an *opacity* channel is read off those same specifications.** A JP2 file
//! carries a channel-definition box beside its colour specification box; a bare codestream — the
//! `\xff\x4f\xff\x51` the fixtures below open with — carries neither, so a codec has to
//! synthesise both. Synthesising three-channel sRGB for a four-channel codestream and calling
//! the fourth channel opacity is a defensible guess about a file that says nothing, and it is
//! exactly the guess the sentence above sets aside: the dictionary has spoken, and it says four.
//!
//! Table 87 says the same thing from the other side. With `/SMaskInData` 0 or absent, any
//! encoded soft-mask information is ignored, so reading the fourth channel as colour costs
//! nothing that the file asked for.
//!
//! **The witness is a crawled document rather than an invention**: a magazine page whose two
//! `/JPXDecode` photographs are bare four-component codestreams under `/ColorSpace /DeviceCMYK`,
//! both refused for "the colour space takes 4 components but the codestream has 3" and both
//! missing from the page, at −8.329 of 255 against three agreeing references (session 631,
//! ADR 0464). It is somebody else's crawled web page and is not in this repository, so the
//! fixture here is generated — the same rule `dct_components.rs` follows.
//!
//! The two tests after the first are the negative twins, and they are what keeps the rule from
//! becoming "make the counts agree": a file that *states* an opacity channel is believed, and a
//! file whose ordinary channels already match its declared space is left alone.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, and \
              the fixtures are 8x8 pages where no index can overflow"
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

/// An 8×8 four-component JPEG 2000 codestream, every pixel (200, 100, 50, 25).
///
/// A **bare codestream** — SOC, SIZ, COD, QCD, SOT, SOD, EOC — with no JP2 boxes at all, which
/// is what the witness document embeds and what leaves a codec nothing to read a colour space
/// or a channel definition out of. Its SIZ states four components of eight unsigned bits, its
/// COD states the reversible 5/3 wavelet, and it is therefore lossless: the four values above
/// come back exactly.
///
/// Generated rather than written, because a JPEG 2000 codestream cannot be written by hand
/// legibly and no crawled file may be committed here. The command, whose output this is with
/// its `COM` comment marker removed so that no encoder version is baked in:
///
/// ```sh
/// python3 -c "import numpy as np
/// np.concatenate([np.full(64, v, np.uint8) for v in (200, 100, 50, 25)]).tofile('cmyk.raw')"
/// opj_compress -i cmyk.raw -o cmyk.j2k -F 8,8,4,8,u -n 1 -r 1
/// ```
///
/// The `.raw` layout is one component after another rather than interleaved, which is what
/// `opj_compress` reads and `opj_decompress` writes — a round trip cannot tell the two apart, so
/// the first version of this fixture encoded four *identical* components and the test that
/// caught it was the one asserting a colour rather than the one asserting a count.
const FOUR_COMPONENT: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x32, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x07, 0x01, 0x01, 0x07, 0x01, 0x01,
    0x07, 0x01, 0x01, 0x07, 0x01, 0x01, 0xff, 0x52, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00,
    0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40, 0x40, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x52, 0x00, 0x01, 0xff, 0x93, 0xc3, 0xe7, 0x0a, 0x11, 0x50, 0x54, 0xa3, 0x6f,
    0xc7, 0xd4, 0x24, 0x11, 0x50, 0x54, 0xaf, 0xfc, 0x90, 0x88, 0x00, 0x00, 0x00, 0x18, 0x48, 0x45,
    0x98, 0x40, 0xc2, 0x42, 0x5f, 0xcf, 0xb4, 0x48, 0x14, 0x00, 0x5c, 0xaf, 0xfc, 0x90, 0x88, 0x00,
    0x00, 0x00, 0x18, 0x48, 0x45, 0x98, 0x40, 0xc2, 0x42, 0x5f, 0xcf, 0xb4, 0x3c, 0x11, 0x50, 0x54,
    0xaf, 0xfc, 0x90, 0x88, 0x00, 0x00, 0x00, 0x18, 0x48, 0x4b, 0xff, 0x7f, 0xff, 0xd9,
];

/// A one-page 8×8 PDF drawing [`FOUR_COMPONENT`] over the whole page.
///
/// `entries` goes into the image dictionary beside `/Filter /JPXDecode`, which is where each
/// test states its `/ColorSpace` and its `/SMaskInData`.
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
            FOUR_COMPONENT.len()
        )
        .as_bytes(),
    );
    body.extend_from_slice(FOUR_COMPONENT);
    body.extend_from_slice(b"\nendstream\nendobj\n");
    assemble(&body)
}

/// A one-page 8×8 PDF whose whole area is filled from `content`.
fn painted_fixture(content: &[u8]) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 8 8] \
          /Resources << >> /Contents 4 0 R >>\nendobj\n",
    );
    body.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    body.extend_from_slice(content);
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

/// The RGB in the middle of an 8×8 raster.
fn centre(raster: &pdf_render::Raster) -> [u8; 3] {
    let at = ((4 * raster.width) as usize + 4) * 4;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

/// A four-component codestream under a declared `/DeviceCMYK` draws all four as colour.
///
/// The expected colour is not predicted here and not taken from any other renderer: the same
/// four values are painted by the `k` operator on a second page, and §8.6.4.4 makes those the
/// same four subtractive components. What the test asserts is that they *arrived* — a reader
/// that lets the codec's synthesised sRGB decide would have three of them and refuse.
#[test]
fn four_channels_under_a_declared_cmyk_space_are_all_colour() {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
        panic!("the sandboxed image decoder is not available: {error}");
    }

    let (raster, unsupported) = render(image_fixture("/ColorSpace /DeviceCMYK"));
    assert!(
        unsupported.is_empty(),
        "the dictionary states four components and the codestream carries four: {unsupported:?}"
    );

    // 200, 100, 50 and 25 of 255, which is what the codestream holds.
    let painted = render(painted_fixture(
        b"0.7843137 0.3921569 0.1960784 0.0980392 k 0 0 8 8 re f",
    ))
    .0;
    let (drawn, expected) = (centre(&raster), centre(&painted));
    for (got, want) in drawn.into_iter().zip(expected) {
        assert!(
            got.abs_diff(want) <= 1,
            "the image drew {drawn:?} where the same CMYK fill draws {expected:?}"
        );
    }
}

/// A file that states an opacity channel is believed, and the mismatch stands.
///
/// §7.4.9: "If SMaskInData is non-zero, there shall be only one opacity channel in the JPEG
/// 2000 data and it shall apply to all colour channels." So this file has said that one of its
/// four channels is opacity, which leaves three for a space that takes four — and the refusal
/// above it is the clause's own arithmetic, not a codec's guess. Making the counts agree
/// whatever the file says is the thing this test exists to forbid.
#[test]
fn a_stated_opacity_channel_is_not_read_as_colour() {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
        panic!("the sandboxed image decoder is not available: {error}");
    }

    let (_, unsupported) = render(image_fixture("/ColorSpace /DeviceCMYK /SMaskInData 1"));
    assert!(
        unsupported
            .iter()
            .any(|report| format!("{report:?}").contains("the colour space takes 4 components")),
        "a stated opacity channel leaves three for a four-component space: {unsupported:?}"
    );
}

/// A declared space the ordinary channels already match is left alone.
///
/// `/DeviceRGB` over the same four channels needs no reinterpretation: three are colour, the
/// fourth is the opacity the codec found, and `/SMaskInData` absent makes Table 87 ignore it.
/// The page is then the first three components — 200, 100, 50 — and not the codestream's
/// fourth, which is what says the opacity channel was ignored rather than blended.
#[test]
fn a_space_the_channels_already_match_keeps_its_opacity_channel() {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
        panic!("the sandboxed image decoder is not available: {error}");
    }

    let (raster, unsupported) = render(image_fixture("/ColorSpace /DeviceRGB"));
    assert!(
        unsupported.is_empty(),
        "three ordinary channels match a three-component space: {unsupported:?}"
    );
    assert_eq!(
        centre(&raster),
        [200, 100, 50],
        "the first three channels are the colour, opaque"
    );
}
