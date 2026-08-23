//! Path construction, ISO 32000-2 §8.5.2 and §8.5.3, read off the display list.
//!
//! # Why this file exists
//!
//! Three of Table 58's and §8.5.3.3.1's sentences describe a command being *removed* from a
//! path rather than added to it, and none of them changes a pixel on its own. That is why
//! they were missing: every metric this project has looks at what was drawn.
//!
//! They stopped being invisible the moment §8.5.3.2's degenerate subpaths became marks. A
//! single-point subpath under round caps is now a dot, so an `m` the standard says leaves
//! "no vestige" in the path is a dot the document never asked for — 205 of them on
//! `bug1743245.pdf`'s first page, which writes that many consecutive `m` operators. A rule
//! that decides nothing can become a rule that decides everything when the clause beside it
//! is implemented.
//!
//! These assert against the display list rather than pixels, because what the clauses
//! describe is the path's own contents.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the one length computed \
              here is a fixed string's"
)]

use std::fmt::Write as _;

use pdf_render::{Command, FillRule, PathCommand, Point};
use pdf_syntax::Document;

/// Builds a one-page fixture whose content stream is `content`.
fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
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
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The commands of every path the content stream paints, in order.
fn paths(content: &str) -> Vec<Vec<PathCommand>> {
    let document = Document::open(fixture(content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { path, .. } | Command::Stroke { path, .. } => {
                Some(path.commands().to_vec())
            }
            _ => None,
        })
        .collect()
}

/// The single path the content stream paints.
fn one_path(content: &str) -> Vec<PathCommand> {
    let mut all = paths(content);
    assert_eq!(all.len(), 1, "the fixture paints exactly one path");
    all.remove(0)
}

/// What interpreting the content stream reported.
fn reports(content: &str) -> Vec<pdf_model::Unsupported> {
    let document = Document::open(fixture(content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page).unsupported
}

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// ISO 32000-2 §8.5.2.1, Table 58's `m`: where one `m` follows another, there is
///
/// > no vestige of the previous m operation remains in the path.
#[test]
fn a_move_overrides_the_move_before_it() {
    assert_eq!(
        one_path("10 10 m 20 20 m 30 30 m 40 40 l S"),
        [
            PathCommand::MoveTo(at(30.0, 30.0)),
            PathCommand::LineTo(at(40.0, 40.0)),
        ],
        "only the last of three consecutive moves survives"
    );
}

/// The override is *consecutive* moves only: one after a segment begins a new subpath.
#[test]
fn a_move_after_a_segment_begins_a_subpath() {
    assert_eq!(
        one_path("10 10 m 20 20 l 30 30 m 40 40 l S"),
        [
            PathCommand::MoveTo(at(10.0, 10.0)),
            PathCommand::LineTo(at(20.0, 20.0)),
            PathCommand::MoveTo(at(30.0, 30.0)),
            PathCommand::LineTo(at(40.0, 40.0)),
        ]
    );
}

/// ISO 32000-2 §8.5.3.3.1:
///
/// > if the last subpath in the path is a single-point open subpath (specified by a trailing
/// > m operator), it shall be disregarded and not considered to be part of the path
///
/// §8.5.3.2 says the same of stroking, so it holds for both operators.
#[test]
fn a_trailing_move_is_not_part_of_the_path() {
    for operator in ["S", "f"] {
        assert_eq!(
            one_path(&format!("10 10 m 20 20 l 30 30 m {operator}")),
            [
                PathCommand::MoveTo(at(10.0, 10.0)),
                PathCommand::LineTo(at(20.0, 20.0)),
            ],
            "{operator}: the trailing m is disregarded"
        );
    }
}

/// A path that is *only* a trailing move is no path at all, and paints nothing.
///
/// Before the rule was written down this reached `tiny-skia`, which refuses a path with no
/// segments, and the whole page failed to rasterise with `InvalidPath`.
#[test]
fn a_path_that_is_only_a_move_paints_nothing() {
    assert!(
        paths("10 10 m S").is_empty(),
        "a lone m states no path to paint"
    );
}

/// ISO 32000-2 §8.5.2.1, Table 58's `h`: "If the current subpath is already closed, h shall
/// do nothing."
#[test]
fn closing_a_closed_subpath_does_nothing() {
    assert_eq!(
        one_path("10 10 m 20 20 l h h h S"),
        [
            PathCommand::MoveTo(at(10.0, 10.0)),
            PathCommand::LineTo(at(20.0, 20.0)),
            PathCommand::Close,
        ]
    );
}

/// ISO 32000-2 §8.5.3.1, Table 59: `s` has "the same effect as the sequence h S", which
/// includes `h`'s own no-op rule.
#[test]
fn close_and_stroke_does_not_close_twice() {
    assert_eq!(
        one_path("10 10 m 20 20 l h s"),
        one_path("10 10 m 20 20 l h S")
    );
}

/// A single-point closed path survives, because §8.5.3.2 gives it a meaning.
///
/// The one shape `h` must *not* treat as already closed: `10 10 m h` is the clause's own
/// example of a degenerate subpath, which under round caps is a filled circle. Collapsing it
/// would delete a mark the standard asks for.
#[test]
fn a_single_point_closed_path_is_kept() {
    assert_eq!(
        one_path("10 10 m h S"),
        [PathCommand::MoveTo(at(10.0, 10.0)), PathCommand::Close,]
    );
}

/// ISO 32000-2 §8.5.2.1, Table 58's `re` is its own stated equivalent: a move, three lines and a close.
#[test]
fn a_rectangle_is_the_five_operations_table_58_names() {
    assert_eq!(
        one_path("10 20 30 40 re f"),
        [
            PathCommand::MoveTo(at(10.0, 20.0)),
            PathCommand::LineTo(at(40.0, 20.0)),
            PathCommand::LineTo(at(40.0, 60.0)),
            PathCommand::LineTo(at(10.0, 60.0)),
            PathCommand::Close,
        ]
    );
}

/// A `re` begins with an `m`, so it overrides a preceding one just as a written `m` does.
///
/// §8.5.2.1's Table 58 states `re` as the sequence `x y m … l h`, which makes the override rule above
/// apply to it word for word. Sixty paths on `issue12810.pdf`'s first page are exactly this
/// pair, and every one of them left a single-point subpath behind.
#[test]
fn a_rectangle_overrides_the_move_before_it() {
    assert_eq!(
        one_path("10 10 m 20 20 30 40 re f"),
        [
            PathCommand::MoveTo(at(20.0, 20.0)),
            PathCommand::LineTo(at(50.0, 20.0)),
            PathCommand::LineTo(at(50.0, 60.0)),
            PathCommand::LineTo(at(20.0, 60.0)),
            PathCommand::Close,
        ]
    );
}

/// `v` takes the current point as its first control point, `y` the endpoint as its second.
///
/// §8.5.2.2: "For the v operator, the first control point shall coincide with initial point
/// of the curve" and "For the y operator, the second control point shall coincide with final
/// point of the curve". Two operators that differ only in which control point is implied,
/// which is exactly the pair an implementation gets the wrong way round.
#[test]
fn the_implied_control_points_are_the_ones_the_clause_names() {
    assert_eq!(
        one_path("10 10 m 20 20 30 30 v S"),
        [
            PathCommand::MoveTo(at(10.0, 10.0)),
            PathCommand::CurveTo(at(10.0, 10.0), at(20.0, 20.0), at(30.0, 30.0)),
        ]
    );
    assert_eq!(
        one_path("10 10 m 20 20 30 30 y S"),
        [
            PathCommand::MoveTo(at(10.0, 10.0)),
            PathCommand::CurveTo(at(20.0, 20.0), at(30.0, 30.0), at(30.0, 30.0)),
        ]
    );
}

/// A `h` returns the current point to the subpath's start, so what follows starts there.
///
/// ISO 32000-2 §8.5.2.1, Table 58: "Appending another segment to the current path shall begin a new subpath, even
/// if the new segment begins at the endpoint reached by the h operation." The endpoint `h`
/// reached is the subpath's first point, which is where the next segment must start from —
/// and reading it as starting where it *ends* would make every such subpath look degenerate.
#[test]
fn a_segment_after_a_close_starts_where_the_close_returned_to() {
    assert_eq!(
        one_path("10 10 m 20 20 l h 30 30 l S"),
        [
            PathCommand::MoveTo(at(10.0, 10.0)),
            PathCommand::LineTo(at(20.0, 20.0)),
            PathCommand::Close,
            PathCommand::LineTo(at(30.0, 30.0)),
        ]
    );
}

/// ISO 32000-2 §8.5.2.1: a segment operator with no current point states no geometry.
///
/// > The trailing endpoint of the segment most recently added to the current path is referred to
/// > as the current point. If the current path is empty, the current point shall be undefined.
/// > Most operators that add a segment to the current path start at the current point; if the
/// > current point is undefined, an error shall be generated.
///
/// The clause gives such a segment no first endpoint and names no substitute, so it adds nothing
/// — and because the current point is defined as "[t]he trailing endpoint of the segment most
/// recently added", an operator that added none leaves it undefined and the next segment is
/// refused for the same reason. Until ADR 0563 all four operators were appended anyway, and
/// `tiny_skia::PathBuilder::inject_move_to_if_needed` began the subpath at the **origin of user
/// space**, so the page got an edge running from the corner that no operator asked for.
#[test]
fn a_segment_with_no_current_point_states_no_geometry() {
    for segment in [
        "30 30 l",
        "30 30 40 40 50 50 c",
        "30 30 40 40 v",
        "30 30 40 40 y",
    ] {
        assert_eq!(
            paths(&format!("10 10 m 20 20 l f {segment} S")),
            [vec![
                PathCommand::MoveTo(at(10.0, 10.0)),
                PathCommand::LineTo(at(20.0, 20.0)),
            ]],
            "{segment}: the second path states no geometry at all"
        );
    }
}

/// The refusal lasts until an `m` or an `re`, because nothing else defines a current point.
///
/// ISO 32000-2 §8.5.2.1: "the first one invoked shall be m or re to begin a new subpath". So a
/// run of segments after the error vanishes whole rather than being anchored to a point the file
/// never stated, and the subpath the file *does* state afterwards is drawn in full.
#[test]
fn a_refused_segment_defines_no_current_point_for_the_next_one() {
    assert_eq!(
        paths("10 10 m 20 20 l f 30 30 l 40 40 l 50 50 m 60 60 l S"),
        [
            vec![
                PathCommand::MoveTo(at(10.0, 10.0)),
                PathCommand::LineTo(at(20.0, 20.0)),
            ],
            vec![
                PathCommand::MoveTo(at(50.0, 50.0)),
                PathCommand::LineTo(at(60.0, 60.0)),
            ],
        ]
    );
}

/// ISO 32000-2 §8.5.2.1's "an error shall be generated", raised where a mark is lost.
///
/// §7.8.2 gives a content stream's other error the same shape — "when a PDF reader encounters an
/// operator in a content stream that it does not recognise, an error shall occur" — and this
/// program raises that one as an [`pdf_model::Unsupported`] too. Trap 5: a segment the file wrote
/// and the page does not carry may not pass in silence.
#[test]
fn a_segment_with_no_current_point_is_reported() {
    assert_eq!(
        reports("10 10 m 20 20 l f 30 30 l 40 40 l S"),
        [pdf_model::Unsupported::UndefinedCurrentPoint { segments: 2 }],
        "both refused segments are counted, and nothing else is reported"
    );
}

/// The same sentence costs `h` nothing, so `h` neither draws nor reports.
///
/// Table 58 states `h` as "appending a straight line segment from the current point to the
/// starting point of the subpath" — with the path empty there is no starting point either, so the
/// operator adds no segment on this invocation and falls outside the antecedent of §8.5.2.1's
/// sentence rather than inside its consequence. `content::path::close_subpath` already pushes
/// nothing onto an empty path, which is the whole of what the clause costs here; reporting it
/// would say a complete page is incomplete, and `Interpretation::is_complete` is what decides
/// whether the oracle judges a page at all (trap 11).
#[test]
fn a_close_with_no_current_point_neither_draws_nor_reports() {
    assert_eq!(
        one_path("h 10 20 30 40 re f"),
        [
            PathCommand::MoveTo(at(10.0, 20.0)),
            PathCommand::LineTo(at(40.0, 20.0)),
            PathCommand::LineTo(at(40.0, 60.0)),
            PathCommand::LineTo(at(10.0, 60.0)),
            PathCommand::Close,
        ],
        "the close contributes nothing and the rectangle after it is untouched"
    );
    assert_eq!(reports("h 10 20 30 40 re f"), [], "and nothing is reported");
}

/// A clipping path the standard has just disregarded clips *everything* out.
///
/// ISO 32000-2 §8.5.4 defines the region as "the same area that would be filled by the f
/// operator", and §8.5.3.3.1 has already removed the trailing `m` from the path, so there is
/// no such area at all. `issue9017_reduced.pdf` writes `568.938 673.022 m W n` around a
/// shading, and all three reference renderers leave that shading undrawn.
#[test]
fn a_clip_built_from_a_disregarded_path_admits_nothing() {
    let document = Document::open(fixture("10 10 m W n 0 0 100 100 re f"))
        .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    let clip = list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Fill { clip, .. } => Some(*clip),
            _ => None,
        })
        .expect("the fixture fills")
        .expect("the fill is clipped");
    assert!(
        list.clip(clip)
            .expect("the clip is in the list")
            .admits_nothing(),
        "an empty clipping path encloses nothing"
    );
}

/// A `W` with no path in front of it is the other case, and leaves the clip alone.
///
/// §8.5.3.1 makes a painting operator with no current path an error; blanking everything
/// after it would be a worse answer to a malformed file than ignoring it, and the two cases
/// are distinguished by whether a path was stated at all rather than by what is left of one.
#[test]
fn a_clip_with_no_path_at_all_is_not_an_empty_clip() {
    let document =
        Document::open(fixture("W n 0 0 100 100 re f")).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    let clip = list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Fill { clip, .. } => Some(*clip),
            _ => None,
        })
        .expect("the fixture fills");
    assert_eq!(clip, None, "no path was stated, so no clip is built");
}

/// The fill rule each painting operator selects, ISO 32000-2 §8.5.3.1 Table 59.
///
/// Five operators fill and the starred three of them use the even-odd rule; `W` and `W*`
/// carry the same pair to the clip, which §8.5.4 defines as "the same area that would be
/// filled by the f operator". A rule read off the wrong operator is invisible on any path
/// that does not overlap itself, which is most of them.
#[test]
fn the_starred_operators_are_the_even_odd_ones() {
    let rule_of = |operator: &str| {
        let content = format!("10 10 m 90 10 l 90 90 l h {operator}");
        let document = Document::open(fixture(&content)).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::Fill { fill_rule, .. } => Some(*fill_rule),
                _ => None,
            })
            .expect("the fixture fills")
    };

    for operator in ["f", "F", "B", "b"] {
        assert_eq!(rule_of(operator), FillRule::NonZero, "{operator}");
    }
    for operator in ["f*", "B*", "b*"] {
        assert_eq!(rule_of(operator), FillRule::EvenOdd, "{operator}");
    }
}
