//! §7.4.9's JPX image-mask channel constraint, exercised through a complete PDF page.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    reason = "a complete JP2/PDF fixture and exact pixel controls must fail loudly"
)]

use std::io::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 20;

/// `OpenJPEG` 2.5.4's complete lossless 4×2 JP2 file. Its `ihdr` says one component at one bit,
/// its enumerated colour box says greyscale, and the codestream decodes to rows `0111` / `0001`.
/// Generated from eight raw 0/1 bytes with `opj_compress -F 4,2,1,1,u -n 1 -r 1`; the container
/// boxes and their lengths are part of the data, as §7.4.9 requires.
const ONE_BIT_GREY_JP2: &[u8] = &[
    0x00, 0x00, 0x00, 0x0c, 0x6a, 0x50, 0x20, 0x20, 0x0d, 0x0a, 0x87, 0x0a, 0x00, 0x00, 0x00, 0x14,
    0x66, 0x74, 0x79, 0x70, 0x6a, 0x70, 0x32, 0x20, 0x00, 0x00, 0x00, 0x00, 0x6a, 0x70, 0x32, 0x20,
    0x00, 0x00, 0x00, 0x2d, 0x6a, 0x70, 0x32, 0x68, 0x00, 0x00, 0x00, 0x16, 0x69, 0x68, 0x64, 0x72,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x0f, 0x63, 0x6f, 0x6c, 0x72, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00,
    0x85, 0x6a, 0x70, 0x32, 0x63, 0xff, 0x4f, 0xff, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x04, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x04, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
    0x01, 0x01, 0xff, 0x52, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01,
    0xff, 0x5c, 0x00, 0x04, 0x40, 0x08, 0xff, 0x64, 0x00, 0x25, 0x00, 0x01, 0x43, 0x72, 0x65, 0x61,
    0x74, 0x65, 0x64, 0x20, 0x62, 0x79, 0x20, 0x4f, 0x70, 0x65, 0x6e, 0x4a, 0x50, 0x45, 0x47, 0x20,
    0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x20, 0x32, 0x2e, 0x35, 0x2e, 0x34, 0xff, 0x90, 0x00,
    0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x13, 0x00, 0x01, 0xff, 0x93, 0xd1, 0x80, 0x08, 0xed, 0x7f,
    0xff, 0xd9,
];

fn pdf(image_mask: bool, width: u32, height: u32, decode: &str) -> Vec<u8> {
    let content = b"0 1 0 rg 0 0 40 40 re f 1 0 0 rg q 40 0 0 40 0 0 cm /Im Do Q";
    let image_entries = if image_mask {
        format!("/ImageMask true {decode}")
    } else {
        String::new()
    };
    let mut body = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
    );
    body.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(b"\nendstream\nendobj\n");
    body.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {width} /Height {height} \
             /Filter /JPXDecode {image_entries} /Length {} >>\nstream\n",
            ONE_BIT_GREY_JP2.len()
        )
        .as_bytes(),
    );
    body.extend_from_slice(ONE_BIT_GREY_JP2);
    body.extend_from_slice(b"\nendstream\nendobj\n");
    assemble(&body)
}

fn assemble(body: &[u8]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    let mut remaining = body;
    while let Some(end) = remaining.windows(7).position(|bytes| bytes == b"endobj\n") {
        let after = end + 7;
        offsets.push(out.len());
        out.extend_from_slice(&remaining[..after]);
        remaining = &remaining[after..];
    }
    assert!(remaining.is_empty(), "all fixture objects are terminated");

    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out
}

fn interpret(image_mask: bool, width: u32, height: u32, decode: &str) -> pdf_model::Interpretation {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
        panic!("sandboxed JPEG 2000 decoder unavailable: {error}");
    }
    let document =
        Document::open(pdf(image_mask, width, height, decode)).expect("complete PDF 1.7 fixture");
    assert!(!document.was_recovered(), "fixture xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    pdf_model::interpret(&document, &page)
}

fn render(image_mask: bool, width: u32, height: u32, decode: &str) -> pdf_render::Raster {
    let interpreted = interpret(image_mask, width, height, decode);
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    let target = TargetSpec::for_page(&interpreted.display_list, 1.0, GENEROUS)
        .expect("the 40x40 page fits the budget");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpreted.display_list, target)
        .expect("the JP2 image draws")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height - 1 - y;
    let at = usize::try_from((row * raster.width + x) * 4)
        .expect("the bounded 40x40 raster offset fits usize");
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn assert_rgb(raster: &pdf_render::Raster, x: u32, y: u32, expected: [u8; 3], label: &str) {
    let got = pixel(raster, x, y);
    assert!(got[3] > 240, "{label} should be opaque: {got:?}");
    for (channel, expected) in got[..3].iter().copied().zip(expected) {
        assert!(
            channel.abs_diff(expected) < 8,
            "{label} should be {expected:?}, got {got:?}"
        );
    }
}

#[test]
fn one_bit_jpx_image_mask_uses_codestream_samples_as_stencil() {
    // Positive control: the same JP2 as a normal image exposes black/white sample positions.
    let ordinary = render(false, 4, 2, "");
    assert_rgb(&ordinary, 5, 30, [0, 0, 0], "ordinary JP2 sample 0");
    assert_rgb(&ordinary, 25, 30, [255, 255, 255], "ordinary JP2 sample 1");
    assert_rgb(&ordinary, 5, 5, [0, 0, 0], "second-row JP2 sample 0");
    assert_rgb(&ordinary, 35, 5, [255, 255, 255], "second-row JP2 sample 1");

    // The valid one-channel/one-bit image now acts as a stencil: zero paints current red and
    // one leaves the green backdrop. The two asymmetric rows expose component/polarity flips.
    let mask = render(true, 4, 2, "");
    assert_rgb(&mask, 5, 30, [255, 0, 0], "top-left zero paints red");
    assert_rgb(&mask, 25, 30, [0, 255, 0], "top-row one preserves green");
    assert_rgb(&mask, 5, 5, [255, 0, 0], "bottom-left zero paints red");
    assert_rgb(
        &mask,
        35,
        5,
        [0, 255, 0],
        "bottom-right one preserves green",
    );

    // The clause and Table 87 exempt ImageMask from ignoring Decode when ColorSpace is absent.
    // Inverting the same one-bit samples must swap every painted and preserved cell.
    let reversed = render(true, 4, 2, "/Decode [1 0]");
    assert_rgb(
        &reversed,
        5,
        30,
        [0, 255, 0],
        "inverted top-left one preserves green",
    );
    assert_rgb(
        &reversed,
        25,
        30,
        [255, 0, 0],
        "inverted top-row one paints red",
    );
    assert_rgb(
        &reversed,
        5,
        5,
        [0, 255, 0],
        "inverted bottom-left one preserves green",
    );
    assert_rgb(
        &reversed,
        35,
        5,
        [255, 0, 0],
        "inverted bottom-right zero paints red",
    );
}

#[test]
fn jpx_dimension_mismatch_diagnostic_is_distinct_for_width_and_height() {
    // The valid 4×2 case is drawn in the preceding test. These malformed producer inputs vary
    // one dictionary dimension at a time; this records the implementation diagnostic, not an
    // ISO-mandated reader recovery policy. The complete JP2 header states 4×2.
    for (width, height) in [(5, 2), (4, 3)] {
        let mismatch = interpret(false, width, height, "");
        assert!(
            mismatch.unsupported.iter().any(|item| {
                let message = format!("{item:?}");
                message.contains("JPEG 2000 data states 4x2")
                    && message.contains(&format!("dictionary says {width}x{height}"))
            }),
            "the JPX header mismatch {width}x{height} should be reported: {:?}",
            mismatch.unsupported
        );
        assert!(
            !mismatch
                .display_list
                .commands()
                .iter()
                .any(|command| matches!(command, pdf_render::Command::Image { .. })),
            "current implementation omits the mismatched image ({width}x{height})"
        );
    }
}
