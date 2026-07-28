//! Annotations, and where their appearance streams land on the page.
//!
//! An annotation is a dictionary in the page's `/Annots` array describing something drawn
//! *over* the page content: a form field, a highlight, a stamp, a signature. Almost all of
//! clause 12 is about how one behaves — what it does when clicked, what value it holds, how
//! it participates in a form. None of that is needed to *draw* one, and this module is
//! deliberately only about drawing.
//!
//! What draws is the appearance stream: `/AP /N` is a form `XObject`, and the interpreter
//! already runs those. So the work here is entirely selection and placement — which
//! annotations are visible, which of an appearance dictionary's states applies, and the
//! matrix ISO 32000-2 §12.5.5 defines from the stream's `/BBox` and `/Matrix` to the
//! annotation's `/Rect`.
//!
//! # What is not here
//!
//! An annotation with no appearance stream is *reported*, not synthesised. Constructing one
//! from `/IC`, `/C`, `/BS`, `/Border` and the subtype's own rules is a separate and much
//! larger job — it is a different drawing routine per annotation type — and guessing at it
//! would put plausible-looking marks on the page that the document never described.

use pdf_render::{Transform, geom::Point};
use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Stream};

/// An appearance stream, resolved and placed.
#[derive(Debug, Clone)]
pub(crate) struct Appearance {
    /// The form `XObject` to run.
    pub stream: Arc<Stream>,
    /// `AA` from §12.5.5: maps the appearance's own coordinates into the page's default
    /// user space, so that its bounding box covers the annotation's `/Rect`.
    pub transform: Transform,
    /// The appearance's `/BBox`, in the appearance's own coordinates, which §8.10.2 makes
    /// the clip for a form `XObject`'s content.
    pub bbox: [f32; 4],
    /// The annotation's constant opacity, from `/CA`.
    pub alpha: f32,
    /// The annotation's blend mode name, from `/BM`, if it names one.
    pub blend: Option<String>,
    /// Whether this is a `Widget`, whose appearance `/NeedAppearances` may declare stale.
    pub is_widget: bool,
}

/// What an entry in `/Annots` asks the page to draw.
#[derive(Debug, Clone)]
pub(crate) enum Decision {
    /// Run this appearance stream.
    Draw(Box<Appearance>),
    /// Draw nothing, and say nothing — the document asked for nothing to be drawn.
    Nothing,
    /// Draw nothing, because this crate cannot. The string says what, for the report.
    Unsupported(String),
}

/// The annotation subtypes ISO 32000-2 Table 171 defines.
///
/// Used only for the `Invisible` flag, whose meaning in Table 167 is conditional on the
/// subtype being outside this list: "applies only to annotations which do not belong to one
/// of the standard annotation types and for which no annotation handler is available".
const STANDARD_SUBTYPES: [&[u8]; 28] = [
    b"3D",
    b"Caret",
    b"Circle",
    b"FileAttachment",
    b"FreeText",
    b"Highlight",
    b"Ink",
    b"Line",
    b"Link",
    b"Movie",
    b"Polygon",
    b"PolyLine",
    b"Popup",
    b"PrinterMark",
    b"Projection",
    b"Redact",
    b"RichMedia",
    b"Screen",
    b"Sound",
    b"Square",
    b"Squiggly",
    b"Stamp",
    b"StrikeOut",
    b"Text",
    b"TrapNet",
    b"Underline",
    b"Watermark",
    b"Widget",
];

/// `/F` bit 1: render nothing for an unknown subtype with no handler.
const FLAG_INVISIBLE: i64 = 1;
/// `/F` bit 2: render nothing, whatever the subtype.
const FLAG_HIDDEN: i64 = 1 << 1;
/// `/F` bit 6: render nothing *on screen*. A viewer is a screen.
const FLAG_NO_VIEW: i64 = 1 << 5;

/// Decides what, if anything, an annotation contributes to the page.
pub(crate) fn decide(document: &Document, annotation: &Dictionary) -> Decision {
    let subtype = document
        .get_key(annotation, "Subtype")
        .as_name()
        .map(|name| name.as_bytes().to_vec())
        .unwrap_or_default();

    // §12.5.6.14: a popup is the window belonging to some *other* annotation, opened in
    // response to that one. It is never part of the page's own rendering, so its absence
    // is not a gap.
    if subtype == b"Popup" {
        return Decision::Nothing;
    }

    let flags = document
        .get_key(annotation, "F")
        .as_integer()
        .unwrap_or_default();
    if flags & FLAG_HIDDEN != 0 || flags & FLAG_NO_VIEW != 0 {
        return Decision::Nothing;
    }
    if flags & FLAG_INVISIBLE != 0 && !STANDARD_SUBTYPES.contains(&subtype.as_slice()) {
        return Decision::Nothing;
    }

    let name = String::from_utf8_lossy(&subtype).into_owned();
    let stream = match normal_appearance(document, annotation) {
        Normal::Stream(stream) => stream,
        Normal::Absent => return missing_appearance(document, annotation, &name),
        Normal::StateNotDefined => return Decision::Nothing,
    };

    let Some(rect) = rectangle(document, annotation, "Rect") else {
        return Decision::Unsupported(format!("{name}: no usable /Rect"));
    };
    // §8.10.2 makes `/BBox` required of a form `XObject`, and §12.5.5's algorithm starts by
    // transforming it. Without one there is nothing to map onto `/Rect`.
    let Some(bbox) = rectangle(document, &stream.dict, "BBox") else {
        return Decision::Unsupported(format!("{name}: appearance stream has no /BBox"));
    };
    let matrix = matrix(document, &stream.dict);

    Decision::Draw(Box::new(Appearance {
        transform: placement(bbox, matrix, rect),
        bbox,
        alpha: document
            .get_key(annotation, "CA")
            .as_number()
            .map_or(1.0, |value| narrow(value).clamp(0.0, 1.0)),
        blend: document
            .get_key(annotation, "BM")
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
        is_widget: subtype == b"Widget",
        stream,
    }))
}

/// Decides what to say about an annotation carrying no appearance stream.
///
/// Reporting every one of them would be noise, because for some the answer really is
/// "nothing is drawn" and that is not a gap. Only two cases are certain enough to be worth
/// separating out, and both are read from the document rather than assumed:
///
/// - A `Link`'s appearance without `/AP` is its border and nothing else (§12.5.6.5), so a
///   border width of zero means there was never anything to draw. §12.5.4 puts that width
///   in `/BS /W`, falling back to the third element of `/Border`, defaulting to 1.
/// - An annotation whose `/Rect` is empty covers no area, so nothing synthesised into it
///   could be visible either.
fn missing_appearance(document: &Document, annotation: &Dictionary, name: &str) -> Decision {
    if let Some(rect) = rectangle(document, annotation, "Rect")
        && (rect[2] - rect[0] <= 0.0 || rect[3] - rect[1] <= 0.0)
    {
        return Decision::Nothing;
    }
    if name == "Link" && border_width(document, annotation) == 0.0 {
        return Decision::Nothing;
    }
    Decision::Unsupported(format!("{name}: no appearance stream"))
}

/// Reads the border width §12.5.4 defines, in points.
fn border_width(document: &Document, annotation: &Dictionary) -> f32 {
    if let Some(style) = document.get_key(annotation, "BS").as_dict()
        && let Some(width) = document.get_key(style, "W").as_number()
    {
        return narrow(width);
    }
    // "If neither the Border nor the BS entry is present, the border shall be drawn as a
    // solid line with a width of 1 point."
    let border = document.get_key(annotation, "Border");
    let Some(border) = border.as_array() else {
        return 1.0;
    };
    border
        .get(2)
        .map(|item| document.resolve(item))
        .and_then(|item| item.as_number())
        .map_or(1.0, narrow)
}

/// What `/AP /N` resolved to.
///
/// The two failures are different in kind, and conflating them is what a check box costs:
/// an annotation with no `/AP` at all is a gap in this crate, while one whose `/AS` selects
/// a state the appearance dictionary does not define is a document saying "draw nothing" —
/// which is exactly how an unchecked box with only an `On` appearance is written.
enum Normal {
    /// A stream to run.
    Stream(Arc<Stream>),
    /// No `/AP`, no `/N`, or a `/N` that is neither a stream nor a state dictionary.
    Absent,
    /// `/N` is a state dictionary and `/AS` selected nothing from it.
    StateNotDefined,
}

/// Resolves `/AP /N` to a single stream, following `/AS` where it names a state.
///
/// §12.5.5: `/N` is "a single appearance stream or an appearance subdictionary", and where
/// it is a subdictionary the `/AS` entry chooses among the states. The clause also names the
/// behaviour when that choice fails — "PDF processors shall also attempt to provide
/// reasonable behaviour (such as displaying nothing) if an annotation's AS entry designates
/// an appearance state for which no appearance is defined in the appearance dictionary" —
/// so displaying nothing there is the specified answer rather than a shortfall.
fn normal_appearance(document: &Document, annotation: &Dictionary) -> Normal {
    let appearances = document.get_key(annotation, "AP");
    let Some(appearances) = appearances.as_dict() else {
        return Normal::Absent;
    };
    let normal = document.get_key(appearances, "N");

    if let Some(stream) = normal.as_stream() {
        return Normal::Stream(Arc::clone(stream));
    }
    let Some(states) = normal.as_dict() else {
        return Normal::Absent;
    };

    let selected = document.get_key(annotation, "AS");
    let resolved = selected
        .as_name()
        .and_then(|name| states.get(&String::from_utf8_lossy(name.as_bytes())))
        .map(|state| document.resolve(state));
    match resolved.as_ref().and_then(|state| state.as_stream()) {
        Some(stream) => Normal::Stream(Arc::clone(stream)),
        None => Normal::StateNotDefined,
    }
}

/// Computes `AA`, the matrix ISO 32000-2 §12.5.5 defines.
///
/// The three steps of the clause, in order:
///
/// 1. Transform `/BBox` by `/Matrix` and take the smallest upright rectangle around the
///    result — the *transformed appearance box*. Taking the bounding box of the four
///    mapped corners is what makes this work for a rotating `/Matrix`, where the
///    quadrilateral is not upright and its extent is larger than the box it came from.
/// 2. Compute `A`, mapping that box's lower-left and upper-right corners onto `/Rect`'s.
/// 3. `AA = Matrix × A`.
///
/// A degenerate transformed box — zero width or height, which a `/BBox` of zero extent or a
/// singular `/Matrix` produces — has no scale onto `/Rect`. The scale is taken as 1 on that
/// axis, so the appearance is positioned rather than divided by zero into nothing.
fn placement(bbox: [f32; 4], matrix: Transform, rect: [f32; 4]) -> Transform {
    let corners = [
        Point::new(bbox[0], bbox[1]),
        Point::new(bbox[2], bbox[1]),
        Point::new(bbox[2], bbox[3]),
        Point::new(bbox[0], bbox[3]),
    ]
    .map(|corner| matrix.apply(corner));

    let (mut low, mut high) = (corners[0], corners[0]);
    for corner in corners {
        low = Point::new(low.x.min(corner.x), low.y.min(corner.y));
        high = Point::new(high.x.max(corner.x), high.y.max(corner.y));
    }

    let span = |from: f32, to: f32, onto: f32| {
        let extent = to - from;
        if extent.abs() > 1e-9 {
            onto / extent
        } else {
            1.0
        }
    };
    let scale = Transform::scale(
        span(low.x, high.x, rect[2] - rect[0]),
        span(low.y, high.y, rect[3] - rect[1]),
    );

    let align = Transform::translate(-low.x, -low.y)
        .then(scale)
        .then(Transform::translate(rect[0], rect[1]));
    matrix.then(align)
}

/// Reads a `/Matrix` entry, defaulting to the identity.
fn matrix(document: &Document, dict: &Dictionary) -> Transform {
    let Some(values) = numbers(document, dict, "Matrix") else {
        return Transform::IDENTITY;
    };
    let at = |index: usize| values.get(index).copied().unwrap_or_default();
    if values.len() < 6 {
        return Transform::IDENTITY;
    }
    Transform::new(at(0), at(1), at(2), at(3), at(4), at(5))
}

/// Reads a rectangle entry, normalised so the first corner is the lower left.
///
/// §7.9.5 describes a rectangle as four numbers giving "a pair of diagonally opposite
/// corners", and says the lower-left-then-upper-right order is only *typical*. It states no
/// requirement to normalise, and an earlier version of this comment attributed one to it.
///
/// The requirement is §12.5.5's, inside the algorithm that places an appearance, and it is
/// stated as a property of the corners rather than as an operation on the array:
///
/// > A maps the lower-left corner (the corner with the smallest x and y coordinates) and the
/// > upper-right corner (the corner with the greatest x and y coordinates) of the transformed
/// > appearance box to the corresponding corners of the annotation's rectangle.
///
/// A `/Rect` written the other way round is common enough that not normalising means
/// annotations placed off the page.
fn rectangle(document: &Document, dict: &Dictionary, key: &'static str) -> Option<[f32; 4]> {
    let values = numbers(document, dict, key)?;
    let at = |index: usize| values.get(index).copied().unwrap_or_default();
    if values.len() < 4 || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    Some([
        at(0).min(at(2)),
        at(1).min(at(3)),
        at(0).max(at(2)),
        at(1).max(at(3)),
    ])
}

/// Reads an array of numbers, resolving each entry.
fn numbers(document: &Document, dict: &Dictionary, key: &'static str) -> Option<Vec<f32>> {
    Some(
        document
            .get_key(dict, key)
            .as_array()?
            .iter()
            .filter_map(|item| document.resolve(item).as_number().map(narrow))
            .collect(),
    )
}

fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a coordinate outside f32's range cannot place anything on a page"
    )]
    {
        value as f32
    }
}

#[cfg(test)]
mod tests {
    use pdf_render::{Transform, geom::Point};

    /// An appearance whose `/BBox` already matches its `/Rect` must not be moved.
    ///
    /// The identity case, and the one that catches a translation applied in the wrong
    /// direction — which a case starting at the origin cannot, because there both
    /// directions agree.
    #[test]
    fn an_appearance_already_the_size_of_its_rectangle_is_only_translated() {
        let placement = super::placement(
            [0.0, 0.0, 100.0, 50.0],
            Transform::IDENTITY,
            [200.0, 300.0, 300.0, 350.0],
        );
        let corner = placement.apply(Point::new(0.0, 0.0));
        assert!((corner.x - 200.0).abs() < 1e-3, "{corner:?}");
        assert!((corner.y - 300.0).abs() < 1e-3, "{corner:?}");
        let far = placement.apply(Point::new(100.0, 50.0));
        assert!((far.x - 300.0).abs() < 1e-3, "{far:?}");
        assert!((far.y - 350.0).abs() < 1e-3, "{far:?}");
    }

    /// A `/BBox` away from the origin is translated to the rectangle, not scaled by its
    /// own offset.
    #[test]
    fn a_bounding_box_away_from_the_origin_lands_on_the_rectangle() {
        let placement = super::placement(
            [10.0, 20.0, 30.0, 60.0],
            Transform::IDENTITY,
            [0.0, 0.0, 40.0, 80.0],
        );
        let low = placement.apply(Point::new(10.0, 20.0));
        assert!(low.x.abs() < 1e-3 && low.y.abs() < 1e-3, "{low:?}");
        let high = placement.apply(Point::new(30.0, 60.0));
        assert!((high.x - 40.0).abs() < 1e-3, "{high:?}");
        assert!((high.y - 80.0).abs() < 1e-3, "{high:?}");
    }

    /// A rotating `/Matrix` is measured by the box around the rotated quadrilateral.
    ///
    /// This is step 1 of the algorithm and the only reason it is a *quadrilateral* rather
    /// than a rectangle. A 45° rotation makes a 100×100 box span 141.42 on both axes, so
    /// scaling to a 100×100 `/Rect` must shrink it by that ratio — not leave it at 1, which
    /// is what using the untransformed `/BBox` extent would give, and which would push half
    /// the appearance outside the annotation.
    #[test]
    fn a_rotating_matrix_is_measured_after_it_rotates() {
        let root_half = std::f32::consts::FRAC_1_SQRT_2;
        let rotate = Transform::new(root_half, root_half, -root_half, root_half, 0.0, 0.0);
        let placement =
            super::placement([0.0, 0.0, 100.0, 100.0], rotate, [0.0, 0.0, 100.0, 100.0]);

        // Every corner of the `/BBox` must land inside the rectangle, and the extremes must
        // land on its edges.
        let mapped = [
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
        ]
        .map(|corner| placement.apply(corner));
        for point in mapped {
            assert!(
                (-1e-3..=100.001).contains(&point.x) && (-1e-3..=100.001).contains(&point.y),
                "{point:?} escaped the annotation rectangle"
            );
        }
        let widest = mapped.iter().map(|point| point.x).fold(f32::MIN, f32::max)
            - mapped.iter().map(|point| point.x).fold(f32::MAX, f32::min);
        assert!((widest - 100.0).abs() < 1e-2, "{widest}");
    }

    /// A `/BBox` with no area is positioned rather than divided by zero.
    #[test]
    fn a_degenerate_bounding_box_does_not_produce_infinities() {
        let placement = super::placement(
            [5.0, 5.0, 5.0, 5.0],
            Transform::IDENTITY,
            [0.0, 0.0, 10.0, 10.0],
        );
        let point = placement.apply(Point::new(5.0, 5.0));
        assert!(point.x.is_finite() && point.y.is_finite(), "{point:?}");
    }
}
