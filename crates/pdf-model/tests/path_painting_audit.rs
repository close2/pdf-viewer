//! ISO 32000-2 §8.5.3, Table 59 path-painting operator discriminators.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixture construction is bounded; malformed fixtures should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Color, Command, FillRule, Paint, PathCommand, Point};
use pdf_syntax::Document;

fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /Half << /CA 0.5 /ca 0.5 >> >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn commands(content: &str) -> Vec<Command> {
    let document = Document::open(fixture(content)).expect("valid fixture");
    assert!(!document.was_recovered(), "fixture uses its declared xref");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let result = pdf_model::interpret(&document, &page);
    assert!(result.is_complete(), "path operators are fully interpreted");
    result.display_list.commands().to_vec()
}

fn fill_rule(command: &Command) -> Option<FillRule> {
    match command {
        Command::Fill { fill_rule, .. } => Some(*fill_rule),
        _ => None,
    }
}

#[test]
fn close_and_stroke_matches_explicit_close_then_stroke() {
    let path = "10 10 m 30 10 l 30 30 l";
    let got = commands(&format!("{path} s"));
    let explicit = commands(&format!("{path} h S"));
    assert_eq!(got, explicit, "s must have the same effect as h S");
    assert_eq!(got.len(), 1, "s emits a stroke");
    let stroke = match got.first() {
        Some(Command::Stroke { path, .. }) => Some(path.commands()),
        _ => None,
    };
    assert!(
        stroke.is_some_and(|path| !path.is_empty()),
        "s strokes the closed path"
    );
    assert_eq!(
        stroke,
        Some(
            &[
                PathCommand::MoveTo(Point::new(10.0, 10.0)),
                PathCommand::LineTo(Point::new(30.0, 10.0)),
                PathCommand::LineTo(Point::new(30.0, 30.0)),
                PathCommand::Close,
            ][..]
        ),
        "the equivalent stroke has the expected path"
    );
}

#[test]
fn combined_fill_stroke_operators_emit_both_parts_with_the_selected_rule() {
    let path = "10 10 m 30 10 l 30 30 l";
    for (operator, expected) in [("B", FillRule::NonZero), ("B*", FillRule::EvenOdd)] {
        let got = commands(&format!("{path} {operator}"));
        assert_eq!(got.len(), 2, "{operator} emits fill and stroke");
        assert_eq!(fill_rule(&got[0]), Some(expected), "{operator} fill rule");
        assert!(
            matches!(got[1], Command::Stroke { .. }),
            "{operator} strokes"
        );
        let fill = match &got[0] {
            Command::Fill { path, .. } => Some(path),
            _ => None,
        };
        let stroke = match &got[1] {
            Command::Stroke { path, .. } => Some(path),
            _ => None,
        };
        assert!(fill.is_some() && stroke.is_some(), "B-family command order");
        assert_eq!(fill, stroke, "both parts use identical path geometry");
        assert_eq!(
            fill.map(|path| path.commands()),
            Some(
                &[
                    PathCommand::MoveTo(Point::new(10.0, 10.0)),
                    PathCommand::LineTo(Point::new(30.0, 10.0)),
                    PathCommand::LineTo(Point::new(30.0, 30.0)),
                ][..]
            ),
            "B preserves the open stroke geometry"
        );
    }
}

fn end_of_path(command: &Command) -> Option<&PathCommand> {
    match command {
        Command::Fill { path, .. } | Command::Stroke { path, .. } => path.commands().last(),
        _ => None,
    }
}

#[test]
fn close_fill_stroke_operators_close_before_both_parts() {
    let path = "10 10 m 30 10 l 30 30 l";
    for (operator, expected) in [("b", FillRule::NonZero), ("b*", FillRule::EvenOdd)] {
        let got = commands(&format!("{path} {operator}"));
        assert_eq!(got.len(), 2, "{operator} emits fill and stroke");
        assert_eq!(fill_rule(&got[0]), Some(expected), "{operator} fill rule");
        assert!(
            matches!(got[1], Command::Stroke { .. }),
            "{operator} strokes"
        );
        let fill_end = end_of_path(&got[0]);
        let stroke_end = end_of_path(&got[1]);
        assert!(
            fill_end.is_some() && stroke_end.is_some(),
            "both commands carry paths"
        );
        assert_eq!(
            fill_end, stroke_end,
            "both portions use the same closed path"
        );
        assert!(matches!(fill_end, Some(PathCommand::Close)));
        let explicit = if operator == "b" { "h B" } else { "h B*" };
        assert_eq!(got, commands(&format!("{path} {explicit}")));
    }
}

#[test]
fn combined_fill_stroke_uses_the_fill_and_stroke_colours_separately() {
    let got = commands("1 g 0 G 10 10 m 30 10 l 30 30 l B");
    assert_eq!(got.len(), 2, "B emits fill and stroke");
    let fill = match got.first() {
        Some(Command::Fill { paint, .. }) => Some(paint),
        _ => None,
    };
    let stroke = match got.get(1) {
        Some(Command::Stroke { paint, .. }) => Some(paint),
        _ => None,
    };
    assert!(
        fill.is_some() && stroke.is_some(),
        "B emits fill followed by stroke"
    );
    assert_eq!(
        fill,
        Some(&Paint::Solid(Color::WHITE)),
        "g sets fill to white"
    );
    assert_eq!(
        stroke,
        Some(&Paint::Solid(Color::BLACK)),
        "G sets stroke to black"
    );
}

#[test]
fn n_emits_no_marks_and_the_following_path_is_independent() {
    let got = commands("10 10 m 30 10 l n 40 40 m 60 40 l S");
    assert_eq!(got.len(), 1, "n emits no fill or stroke");
    assert!(
        matches!(got[0], Command::Stroke { .. }),
        "the later path is stroked"
    );
    let path = match &got[0] {
        Command::Stroke { path, .. } => Some(path.commands()),
        _ => None,
    };
    assert_eq!(
        path,
        Some(
            &[
                PathCommand::MoveTo(Point::new(40.0, 40.0)),
                PathCommand::LineTo(Point::new(60.0, 40.0)),
            ][..]
        ),
        "n ends the earlier path before the next path is built"
    );
}

fn rendered(content: &str) -> pdf_render::Raster {
    use pdf_render::Rasterizer as _;
    let mut list = pdf_render::DisplayList::new(pdf_render::Size::new(100.0, 100.0));
    for command in commands(content) {
        list.push(command);
    }
    let target = pdf_render::TargetSpec::for_page(&list, 1.0, 1 << 20).expect("small page target");
    render_cpu::CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported path scene")
}

fn pixel(raster: &pdf_render::Raster, x: usize, y: usize) -> &[u8] {
    let at = (y * raster.width as usize + x) * 4;
    raster.data.get(at..at + 4).expect("bounded sample pixel")
}

#[test]
fn fill_implicitly_closes_each_disconnected_open_subpath() {
    // §8.5.3.3.1: both open squares are filled together. Closing only the last
    // subpath or ignoring the first loses the left square; painting stroke only
    // leaves both sampled interiors white.
    let paths = "10 10 m 30 10 l 30 30 l 10 30 l 60 60 m 80 60 l 80 80 l 60 80 l";
    for operator in ["f", "F", "f*"] {
        let raster = rendered(&format!("{paths} {operator}"));
        assert_eq!(
            pixel(&raster, 20, 80),
            [0, 0, 0, 255],
            "{operator}: first interior"
        );
        assert_eq!(
            pixel(&raster, 70, 30),
            [0, 0, 0, 255],
            "{operator}: second interior"
        );
        assert_eq!(
            pixel(&raster, 45, 55),
            [255, 255, 255, 255],
            "{operator}: disconnected gap"
        );
    }
}

#[test]
fn crossing_subpaths_of_one_stroke_composite_only_once() {
    // §8.5.3.2: the entire stroked path is one object for transparency. At the
    // intersection, two separate S operations give 0.25 white, a single S 0.5.
    let separate = rendered("/Half gs 10 w 10 50 m 90 50 l S 50 10 m 50 90 l S");
    gray_near(pixel(&separate, 50, 50), 64, "two objects composite twice");
    let together = rendered("/Half gs 10 w 10 50 m 90 50 l 50 10 m 50 90 l S");
    gray_near(pixel(&together, 50, 50), 128, "one object's crossing");
    gray_near(pixel(&together, 20, 50), 128, "one straight leg");
}

#[test]
fn combined_fill_and_stroke_do_not_composite_their_overlap_twice() {
    // §11.6.2 and §11.7.4.4: fill and stroke are portions of one object.
    // The inner half of a 10-unit stroke overlaps the fill at page (22.5,49.5).
    let separate = rendered("/Half gs 10 w 20 20 60 60 re f 20 20 60 60 re S");
    gray_near(
        pixel(&separate, 22, 50),
        64,
        "separate objects are the control",
    );
    for operator in ["B", "B*", "b", "b*"] {
        let together = rendered(&format!("/Half gs 10 w 20 20 60 60 re {operator}"));
        gray_near(pixel(&together, 22, 50), 128, "{operator}: shared band");
        gray_near(pixel(&together, 50, 50), 128, "{operator}: fill only");
        gray_near(pixel(&together, 17, 50), 128, "{operator}: stroke only");
        assert_eq!(
            pixel(&together, 10, 50),
            [255, 255, 255, 255],
            "{operator}: outside"
        );
    }
}

/// §11.2's alpha equations yield 127.5 for one half-opacity black object on
/// white, 63.75 for two. One 8-bit level allows raster quantization; the two
/// results remain separated by more than sixty levels.
fn gray_near(sample: &[u8], expected: u8, detail: &str) {
    assert_eq!(sample.get(3), Some(&255), "opaque page backdrop");
    assert!(
        sample
            .get(..3)
            .expect("RGB sample")
            .iter()
            .all(|value| value.abs_diff(expected) <= 1),
        "{detail}: expected near {expected}, got {sample:?}"
    );
}
