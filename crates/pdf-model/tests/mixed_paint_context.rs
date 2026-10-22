//! Independent state-boundary controls for §11.7.5.3 and stencil colour reuse.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "bounded generated fixture; malformed input and pixel bounds must fail the test"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;

const SOURCE: &str = "0.2 0.4 0.6 rg 0.6 0.2 0.4 RG";
const OTHER: &str = "0.7 0.1 0.3 rg 0.1 0.7 0.3 RG";
const MARKS: &str = "0 5 15 80 re f 5 w 25 5 m 25 85 l S \
    BT /F1 32 Tf 2 Tr 1 0 0 1 38 30 Tm (A) Tj ET \
    q 10 0 0 80 85 5 cm /Mask Do Q";
const MASK: &str = "q 80 0 0 80 5 5 cm /Mask Do Q";

/// One indirect stencil is deliberately shared by every use in the page.
fn fixture(content: &str) -> Vec<u8> {
    let content = format!("{content} /Half gs 0 0 0 0 k 450 0 20 100 re f");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 480 100] \
         /Group << /S /Transparency /CS /DeviceCMYK >> /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> /XObject << /Mask 6 0 R >> \
         /ExtGState << /Half << /ca 0.5 /CA 0.5 >> \
         /Default << /BG2 /Default /UCR2 /Default >> \
         /Changed << /BG << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >> \
         /UCR << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0.5] /N 1 >> >> >> >> >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len() + 1
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ImageMask true \
         /BitsPerComponent 1 /Filter /ASCIIHexDecode /Length 4 >>\nstream\n00>\nendstream"
            .to_owned(),
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(pdf, "{} 0 obj\n{object}\nendobj", index + 1).expect("write string");
    }
    let xref = pdf.len();
    writeln!(pdf, "xref\n0 7\n0000000000 65535 f ").expect("write string");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("write string");
    }
    write!(
        pdf,
        "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("write string");
    pdf.into_bytes()
}

fn render(content: &str) -> Vec<u8> {
    let document = Document::open(fixture(content)).expect("generated PDF opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpreted = pdf_model::interpret(&document, &page);
    let target =
        TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 20).expect("bounded target");
    render_cpu::CpuRasterizer::new()
        .rasterize(&interpreted.display_list, target)
        .expect("fixture rasterises")
        .data
}

fn cell(x: u32, marks: &str) -> String {
    format!("q 1 0 0 1 {x} 0 cm {marks} Q ")
}

/// Compare a warmed mixed-consumer interpretation with a separately interpreted fresh
/// page for each cell, selecting its sources only after its complete conversion state.
fn compare_cells(actual: &[u8], references: &[String; 4]) {
    for (cell, reference) in references.iter().enumerate() {
        let cold = render(reference);
        for y in 0..100 {
            let left = (y * 480 + cell * 110) * 4;
            let right = left + 100 * 4;
            assert_eq!(
                &actual[left..right],
                &cold[left..right],
                "cell {cell}, row {y}"
            );
        }
    }
}

#[test]
fn fill_stroke_text_and_stencil_restore_sources_context_and_alpha_together() {
    for (source, other) in [(SOURCE, OTHER), (OTHER, SOURCE)] {
        let actual = render(&format!(
            "{source} {} /Changed gs {} q /Default gs {other} /Half gs {} Q {}",
            cell(0, MARKS),
            cell(110, MARKS),
            cell(220, MARKS),
            cell(330, MARKS)
        ));
        compare_cells(
            &actual,
            &[
                format!("{source} {}", cell(0, MARKS)),
                format!("/Changed gs {source} {}", cell(110, MARKS)),
                format!("/Half gs {other} {}", cell(220, MARKS)),
                format!("/Changed gs {source} {}", cell(330, MARKS)),
            ],
        );
        // Each consumer must actually draw: agreement on missing text or a missing
        // image would otherwise make the cold-reference comparison vacuous.
        for (left, right) in [(0, 15), (20, 30), (38, 75), (85, 95)] {
            assert!(
                (10..90).any(|y| (left..right).any(|x| {
                    let at = (y * 480 + x) * 4;
                    actual[at..at + 4] != [255, 255, 255, 255]
                })),
                "missing consumer in columns {left}..{right}"
            );
        }
        // The first source under Changed has k=.4, BG=.6, UCR=.2:
        // CMYK(.6,.4,.2,.6), through the device press, is (52,62,79).
        if source == SOURCE {
            let at = (50 * 480 + 115) * 4;
            assert_eq!(&actual[at..at + 4], &[52, 62, 79, 255]);
        }
    }
}

#[test]
fn one_indirect_stencil_changes_source_and_returns_to_a_warmed_context() {
    let actual = render(&format!(
        "{SOURCE} {} /Changed gs {} q {OTHER} /Half gs {} Q {}",
        cell(0, MASK),
        cell(110, MASK),
        cell(220, MASK),
        cell(330, MASK)
    ));
    compare_cells(
        &actual,
        &[
            format!("{SOURCE} {}", cell(0, MASK)),
            format!("/Changed gs {SOURCE} {}", cell(110, MASK)),
            format!("/Changed gs /Half gs {OTHER} {}", cell(220, MASK)),
            format!("/Changed gs {SOURCE} {}", cell(330, MASK)),
        ],
    );
}
