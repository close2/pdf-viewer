//! Independent embedded subsets retain their outlines, widths and cache identities.

#![expect(
    clippy::expect_used,
    reason = "a test fixture that cannot exercise the behavior must fail loudly"
)]

use std::collections::BTreeSet;
use std::io::Write as _;

use pdf_model::content::{FontCache, interpret_with_fonts};
use pdf_model::view::ViewState;
use pdf_render::{Command, PathCommand};
use pdf_syntax::Document;

const FACE: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

fn subset(glyphs: &[u16]) -> pdf_font::embed::Embedded {
    let glyphs: BTreeSet<u16> = glyphs.iter().copied().collect();
    pdf_font::embed::for_embedding(FACE, &glyphs).expect("the bundled face permits embedding")
}

fn cid(subset: &pdf_font::embed::Embedded, source_glyph: u16) -> u16 {
    *subset
        .glyphs
        .get(&source_glyph)
        .expect("the chosen source glyph is in the subset")
}

fn object(bytes: &mut Vec<u8>, offsets: &mut Vec<usize>, number: usize, body: &str) {
    offsets.push(bytes.len());
    write!(bytes, "{number} 0 obj\n{body}\nendobj\n").expect("writing to Vec");
}

fn stream(
    bytes: &mut Vec<u8>,
    offsets: &mut Vec<usize>,
    number: usize,
    dictionary: &str,
    data: &[u8],
) {
    offsets.push(bytes.len());
    write!(
        bytes,
        "{number} 0 obj\n<< {dictionary} /Length {} >>\nstream\n",
        data.len()
    )
    .expect("writing stream header");
    bytes.extend_from_slice(data);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
}

fn font_objects(
    bytes: &mut Vec<u8>,
    offsets: &mut Vec<usize>,
    type0: usize,
    subset: &pdf_font::embed::Embedded,
    base_name: &str,
    glyph_cid: u16,
    width: f32,
) {
    let cid_font = type0.saturating_add(1);
    let descriptor = type0.saturating_add(2);
    let program = type0.saturating_add(3);
    let metrics = pdf_font::shaping::face::metrics(FACE).expect("bundled metrics");
    let (ascent, descent) = (metrics.ascent, metrics.descent);
    let bbox = metrics
        .bounding_box
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    object(
        bytes,
        offsets,
        type0,
        &format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /{base_name} /Encoding /Identity-H /DescendantFonts [{cid_font} 0 R] >>"
        ),
    );
    object(
        bytes,
        offsets,
        cid_font,
        &format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{base_name} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {descriptor} 0 R /DW 1000 /W [{glyph_cid} [{width}]] /CIDToGIDMap /Identity >>"
        ),
    );
    object(
        bytes,
        offsets,
        descriptor,
        &format!(
            "<< /Type /FontDescriptor /FontName /{base_name} /Flags 4 /FontBBox [{bbox}] /ItalicAngle 0 /Ascent {ascent} /Descent {descent} /CapHeight 688 /StemV 80 /FontFile2 {program} 0 R >>"
        ),
    );
    stream(
        bytes,
        offsets,
        program,
        &format!("/Length1 {}", subset.program.len()),
        &subset.program,
    );
}

fn document(first: &pdf_font::embed::Embedded, second: &pdf_font::embed::Embedded) -> Document {
    let first_gid = cid(first, 42); // G
    let second_gid = cid(second, 38); // C
    let source_name = first
        .name
        .split_once('+')
        .expect("subset contains its source PostScript name")
        .1;
    let first_name = format!("ABCDEF+{source_name}");
    let second_name = format!("GHIJKL+{source_name}");
    let first_width = pdf_font::shaping::face::glyph(FACE, 'G')
        .expect("source cmap maps G")
        .1;
    let second_width = pdf_font::shaping::face::glyph(FACE, 'C')
        .expect("source cmap maps C")
        .1;
    let content = format!(
        "BT /F1 20 Tf 20 140 Td <{first_gid:04X}{first_gid:04X}> Tj ET\n\
         BT /F2 20 Tf 20 80 Td <{second_gid:04X}{second_gid:04X}> Tj ET\n\
         BT /F1 20 Tf 20 20 Td <{first_gid:04X}{first_gid:04X}> Tj ET"
    );
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    object(
        &mut bytes,
        &mut offsets,
        1,
        "<< /Type /Catalog /Pages 2 0 R >>",
    );
    object(
        &mut bytes,
        &mut offsets,
        2,
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    );
    object(
        &mut bytes,
        &mut offsets,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 180] /Resources << /Font << /F1 5 0 R /F2 9 0 R >> >> /Contents 4 0 R >>",
    );
    stream(&mut bytes, &mut offsets, 4, "", content.as_bytes());
    font_objects(
        &mut bytes,
        &mut offsets,
        5,
        first,
        &first_name,
        first_gid,
        first_width,
    );
    font_objects(
        &mut bytes,
        &mut offsets,
        9,
        second,
        &second_name,
        second_gid,
        second_width,
    );

    let xref = bytes.len();
    let size = offsets.len().saturating_add(1);
    writeln!(bytes, "xref\n0 {size}\n0000000000 65535 f ").expect("xref header");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("xref entry");
    }
    write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("trailer");
    Document::open(bytes).expect("the complete PDF opens")
}

/// Distinct subsets of one face retain their own outlines when loaded through the public reader.
#[test]
fn public_reader_keeps_same_source_subsets_independent() {
    let first = subset(&[0, 42, 220, 488, 570]);
    let second = subset(&[0, 38, 97, 120, 135]);

    // Positive controls first: these inputs are distinct, both programs open, and both carry
    // their own visible outline. The third run repeats F1 so the cache must record a hit.
    assert_ne!(
        first.glyphs, second.glyphs,
        "different source glyph closures"
    );
    assert_ne!(
        first.program, second.program,
        "different subset program bytes"
    );
    let document = document(&first, &second);
    assert!(!document.was_recovered(), "exact cross-reference table");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let fonts = FontCache::new();
    let interpretation = interpret_with_fonts(&document, &page, &ViewState::of(&document), &fonts);
    assert!(
        interpretation.unsupported.is_empty(),
        "the complete page is interpreted: {:?}",
        interpretation.unsupported
    );
    let fills: Vec<(Vec<PathCommand>, f32, f32)> = interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill {
                path, transform, ..
            } => Some((path.commands().to_vec(), transform.e, transform.f)),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 6, "each of three two-glyph runs has outlines");
    assert!(fills.iter().all(|(outline, _, _)| !outline.is_empty()));
    assert_eq!(
        fills.first().map(|fill| &fill.0),
        fills.get(1).map(|fill| &fill.0),
        "repeated glyphs have the same F1 outline"
    );
    assert_ne!(
        fills.first().map(|fill| &fill.0),
        fills.get(2).map(|fill| &fill.0),
        "F1 G and F2 C have distinct outlines"
    );
    assert_eq!(fills[0].0, fills[4].0, "reusing F1 preserves its outline");
    let advances = [
        fills[1].1 - fills[0].1,
        fills[3].1 - fills[2].1,
        fills[5].1 - fills[4].1,
    ];
    let width_g = pdf_font::shaping::face::glyph(FACE, 'G')
        .expect("source cmap maps G")
        .1;
    let width_c = pdf_font::shaping::face::glyph(FACE, 'C')
        .expect("source cmap maps C")
        .1;
    for (actual, width) in advances.into_iter().zip([width_g, width_c, width_g]) {
        let expected = width * 20.0 / 1000.0;
        assert!(
            (actual - expected).abs() < 0.01,
            "the reader uses the source glyph's stated /W advance at 20 pt: {advances:?}"
        );
    }
    let report = fonts.report();
    assert_eq!(
        report.fonts, 2,
        "two font dictionaries are retained: {report:?}"
    );
    let warm = interpret_with_fonts(&document, &page, &ViewState::of(&document), &fonts);
    assert!(warm.is_complete(), "{:?}", warm.unsupported);
    assert_eq!(
        interpretation.display_list.commands(),
        warm.display_list.commands(),
        "warm cache retains the same glyphs and placement"
    );
    let warm_report = fonts.report();
    assert_eq!(warm_report.fonts, 2);
    assert_eq!(
        warm_report.misses, report.misses,
        "warm reuse adds no misses"
    );
    assert!(
        warm_report.hits > report.hits,
        "warm interpretation exercises the retained cache"
    );
}
