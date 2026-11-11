//! Consumer-level observation of the ADR-0359 damaged soft-mask group recovery policy.
//!
//! This deliberately compares a valid intact `RunLengthDecode` stream with a stream that has
//! the same encoded prefix but no EOD marker. The malformed case tests project recovery policy,
//! not a mandatory ISO recovery rule.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test fixture errors and invalid pixel coordinates should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::{Pages, Unsupported, interpret};
use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::{Damage, Document};
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;

/// One-page PDF with an intact or deliberately truncated `RunLengthDecode` mask group.
fn document(group_content: &str, with_eod: bool) -> Vec<u8> {
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
          /Resources << /ExtGState << /GS0 5 0 R >> >> >>"
            .to_vec(),
        plain_stream("q /GS0 gs 0 g 0 0 100 100 re f Q\n"),
        b"<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R \
          /BC [0.25] /TR << /FunctionType 2 /Domain [0 1] /Range [0 1] \
          /N 1 /C0 [1] /C1 [0] >> >> >>"
            .to_vec(),
        run_length_stream(group_content, with_eod),
    ];
    assemble(&objects)
}

fn plain_stream(content: &str) -> Vec<u8> {
    format!(
        "<< /Length {} >>\nstream\n{content}endstream",
        content.len()
    )
    .into_bytes()
}

/// `RunLengthEncode` literal data in chunks of at most 128 bytes, optionally ending with EOD.
fn run_length_stream(content: &str, with_eod: bool) -> Vec<u8> {
    let mut encoded = Vec::new();
    for chunk in content.as_bytes().chunks(128) {
        encoded.push(u8::try_from(chunk.len().saturating_sub(1)).expect("chunk length <= 128"));
        encoded.extend_from_slice(chunk);
    }
    if with_eod {
        encoded.push(128);
    }

    let mut stream = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 70 100] /Resources << >> \
         /Group << /Type /Group /S /Transparency /CS /DeviceGray >> \
         /Filter /RunLengthDecode /Length {} >>\nstream\n",
        encoded.len()
    )
    .into_bytes();
    stream.extend_from_slice(&encoded);
    stream.extend_from_slice(b"\nendstream");
    stream
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let mut tail = String::new();
    let _ = writeln!(tail, "xref\n0 {size}");
    tail.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(tail.as_bytes());
    out
}

fn interpret_page(bytes: Vec<u8>) -> (pdf_model::Interpretation, Document) {
    let document = Document::open(bytes).expect("complete xref and PDF container");
    let page = Pages::new(&document).get(0).expect("one page");
    (interpret(&document, &page), document)
}

fn rasterize(interpretation: &pdf_model::Interpretation) -> Raster {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, PIXEL_BUDGET).expect("valid 100x100 target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("supported CPU drawing");
    assert_eq!(
        (raster.width, raster.height),
        (100, 100),
        "pixel coordinates are 1:1 with PDF units"
    );
    raster
}

/// Raster row order runs opposite to PDF user-space y.
fn rgba_at(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    assert!(x < raster.width && y < raster.height);
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let index = usize::try_from(row.saturating_mul(raster.width).saturating_add(x))
        .expect("small raster")
        .saturating_mul(4);
    [
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
        raster.data[index + 3],
    ]
}

fn near_rgba(got: [u8; 4], expected: [u8; 4], label: &str) {
    for (channel, (actual, expected)) in got.into_iter().zip(expected).enumerate() {
        assert!(
            i16::from(actual).abs_diff(i16::from(expected)) <= 1,
            "{label}, RGBA channel {channel}: got {actual}, expected {expected} ±1"
        );
    }
}

#[test]
fn a_truncated_luminosity_mask_retains_decoded_prefix_and_uses_bc_transfer_outside() {
    // Prefix paints 0.75 grey in the left region; the complete control then paints white in
    // another region. The damaged stream ends after the prefix operator but before RunLength
    // EOD, so this consumer test can observe whether the prefix survives recovery.
    let prefix = "0.75 g 0 0 30 100 re f\n";
    let complete_group = format!("{prefix}1 g 35 0 30 100 re f\n");

    let (complete, complete_document) = interpret_page(document(&complete_group, true));
    assert!(
        !complete_document.was_recovered(),
        "intact control uses exact xref"
    );
    assert!(
        complete.is_complete(),
        "valid intact control: {:?}",
        complete.unsupported
    );
    let whole = rasterize(&complete);
    near_rgba(
        rgba_at(&whole, 10, 50),
        [191, 191, 191, 255],
        "full artwork's prefix-painted region",
    );
    near_rgba(
        rgba_at(&whole, 50, 50),
        [255, 255, 255, 255],
        "full artwork's white suffix-painted region",
    );
    near_rgba(
        rgba_at(&whole, 85, 50),
        [64, 64, 64, 255],
        "outside BBox uses TR(BC)=TR(0.25)=0.75",
    );

    // This intact positive control differs from the damaged stream by the EOD byte alone.
    // At x=50 and x=85 the group's backdrop is used inside and outside the BBox, respectively.
    let (prefix_only, prefix_document) = interpret_page(document(prefix, true));
    assert!(
        !prefix_document.was_recovered(),
        "prefix-only control uses exact xref"
    );
    assert!(
        prefix_only.is_complete(),
        "EOD makes the prefix-only stream complete"
    );
    let prefix_raster = rasterize(&prefix_only);
    near_rgba(
        rgba_at(&prefix_raster, 10, 50),
        [191, 191, 191, 255],
        "prefix-only control retains painted prefix",
    );
    near_rgba(
        rgba_at(&prefix_raster, 50, 50),
        [64, 64, 64, 255],
        "unpainted point inside BBox uses BC and TR",
    );
    near_rgba(
        rgba_at(&prefix_raster, 85, 50),
        [64, 64, 64, 255],
        "outside BBox uses BC and TR",
    );

    let (damaged, damaged_document) = interpret_page(document(prefix, false));
    assert!(
        !damaged_document.was_recovered(),
        "truncation is only in stream data"
    );
    let reports: Vec<_> = damaged
        .unsupported
        .iter()
        .filter_map(|item| match item {
            Unsupported::DamagedContentStream { stream } => Some(stream),
            _ => None,
        })
        .collect();
    assert_eq!(
        reports.len(),
        1,
        "one damaged mask group stream is reported"
    );
    assert_eq!(reports[0].damage, Damage::Truncated);
    assert_eq!(
        reports[0].kept,
        prefix.len(),
        "the entire literal prefix decoded"
    );
    let recovered = rasterize(&damaged);
    near_rgba(
        rgba_at(&recovered, 10, 50),
        [191, 191, 191, 255],
        "decoded prefix must remain visible under the recovered mask",
    );
    near_rgba(
        rgba_at(&recovered, 50, 50),
        [64, 64, 64, 255],
        "missing suffix inside BBox uses BC then TR",
    );
    near_rgba(
        rgba_at(&recovered, 85, 50),
        [64, 64, 64, 255],
        "outside BBox uses the specified BC then TR value",
    );
}
