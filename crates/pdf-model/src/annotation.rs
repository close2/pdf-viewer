//! Annotations, and where their appearance streams land on the page.
//!
//! An annotation is a dictionary in the page's `/Annots` array describing something drawn
//! *over* the page content: a form field, a highlight, a stamp, a signature. Almost all of
//! clause 12 is about how one behaves — what it does when clicked, what value it holds, how
//! it participates in a form. None of that is needed to *draw* one, and this module is
//! deliberately only about drawing.
//!
//! What draws is the appearance stream: `/AP /N` is a form `XObject`, and the interpreter
//! already runs those. So the work here is selection and placement — which annotations are
//! visible, which of an appearance dictionary's states applies, and the matrix ISO 32000-2
//! §12.5.5 defines from the stream's `/BBox` and `/Matrix` to the annotation's `/Rect`.
//!
//! An annotation with *no* appearance stream is handed to [`crate::appearance`], which
//! constructs one out of the entries its subtype's clause states. That module holds every
//! per-subtype rule; this one does not know a `Square` from a `Widget` except to ask.
//!
//! # A stored appearance is self-contained
//!
//! §12.5.2's closing sentence — quoted in full in [`crate::appearance`] — has a reader "ignore
//! the values of the C, IC, Border, BS, BE, BM, CA, ca, H, DA, Q, DS, LE, LL, LLE, and Sy
//! keys" when an appearance dictionary is present, and Table 166 says of `/CA` and `/ca` that
//! each "shall not be used if the annotation has an appearance stream ... in that case, the
//! appearance stream shall specify any transparency". So the opacity and blend mode below are
//! read for a *constructed* appearance and left at their defaults for a stored one.
//!
//! §12.5.5 states the opposite in one sentence — the appearance's group "shall be composited
//! ... using the values of the BM, ca and CA entries in the annotation dictionary" — and this
//! tree followed that reading until the twenty-first session. Two statements against one, and
//! the two explain themselves: the entries are what an appearance is *regenerated* from, and a
//! stream that carries its own `/ExtGState` would otherwise have the same opacity applied
//! twice. `highlight.pdf` is exactly that file: `/CA 0.8` on the annotation, `ca 0.8` inside
//! the stream.

use pdf_render::{Transform, geom::Point};
use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Stream};

/// What an appearance's content is: a stream the file stored, or one this crate wrote.
#[derive(Debug, Clone)]
pub(crate) enum Content {
    /// `/AP /N`, a form `XObject` (§12.5.5).
    Stored(Arc<Stream>),
    /// A content stream constructed from the annotation's appearance characteristics
    /// (§12.7.4.3, [`crate::appearance`]).
    Constructed(Vec<u8>),
}

/// An appearance stream, resolved and placed.
#[derive(Debug, Clone)]
pub(crate) struct Appearance {
    /// The content stream to run.
    pub content: Content,
    /// `AA` from §12.5.5: maps the appearance's own coordinates into the page's default
    /// user space, so that its bounding box covers the annotation's `/Rect`.
    pub transform: Transform,
    /// The appearance's `/BBox`, in the appearance's own coordinates, which §8.10.2 makes
    /// the clip for a form `XObject`'s content.
    pub bbox: [f32; 4],
    /// Table 166's `/ca`, the opacity "used for all nonstroking operations on all visible
    /// elements of the annotation", defaulting to `/CA` and then to 1. Only a constructed
    /// appearance has one.
    pub fill_alpha: f32,
    /// Table 166's `/CA`, the same for stroking operations. Only a constructed appearance has
    /// one.
    pub stroke_alpha: f32,
    /// The annotation's blend mode name, from `/BM`, if it names one and the appearance is
    /// constructed.
    pub blend: Option<String>,
    /// Whether this is a `Widget`, whose appearance `/NeedAppearances` may declare stale.
    pub is_widget: bool,
}

/// What an entry in `/Annots` asks the page to draw.
#[derive(Debug, Clone)]
pub(crate) enum Decision {
    /// Run this appearance stream, and report `owed` if the construction fell short of what
    /// the clause asks for — a widget's background is drawable where its field's text is not,
    /// and losing either statement would be worse than making both.
    Draw {
        appearance: Box<Appearance>,
        owed: Option<String>,
    },
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
    let Some(rect) = rectangle(document, annotation, "Rect") else {
        return Decision::Unsupported(format!("{name}: no usable /Rect"));
    };
    // An annotation covering no area cannot show anything, whether its appearance is stored or
    // constructed — and Table 166 excuses a writer from supplying one for exactly that shape.
    if rect[2] - rect[0] <= 0.0 || rect[3] - rect[1] <= 0.0 {
        return Decision::Nothing;
    }

    let stored = match normal_appearance(document, annotation) {
        Normal::Stream(stream) => stream,
        Normal::Absent => return construct(document, annotation, &subtype, &name, rect),
        Normal::StateNotDefined => return Decision::Nothing,
    };

    // §8.10.2 makes `/BBox` required of a form `XObject`, and §12.5.5's algorithm starts by
    // transforming it. Without one there is nothing to map onto `/Rect`.
    let Some(bbox) = rectangle(document, &stored.dict, "BBox") else {
        return Decision::Unsupported(format!("{name}: appearance stream has no /BBox"));
    };
    let matrix = matrix(document, &stored.dict);

    Decision::Draw {
        appearance: Box::new(Appearance {
            transform: placement(bbox, matrix, rect),
            bbox,
            // §12.5.2 and Table 166: a stored stream states its own transparency, so the
            // annotation's `/ca`, `/CA` and `/BM` are not applied to it.
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend: None,
            is_widget: subtype == b"Widget",
            content: Content::Stored(stored),
        }),
        owed: None,
    }
}

/// Constructs an appearance for an annotation that has none, and places it.
///
/// The constructed stream is written in the page's own default user space, so its `/BBox` *is*
/// the annotation's `/Rect` and §12.5.5's algorithm reduces to the identity — which is worth
/// stating because it is the reason this path needs no second placement rule. The `/BBox` still
/// clips, which is §8.10.2 doing what §12.5.5 relies on: an appearance is "a self-contained
/// content stream that shall be rendered inside the annotation rectangle".
fn construct(
    document: &Document,
    annotation: &Dictionary,
    subtype: &[u8],
    name: &str,
    rect: [f32; 4],
) -> Decision {
    // §12.5.6.14: a popup is the window belonging to some *other* annotation, and §12.5.6.24's
    // projection is a measurement inside an activated 3D model — clause 13, which principle 5
    // excludes. Table 166 names both, with `Link`, as the subtypes a writer need not give an
    // appearance dictionary at all.
    if subtype == b"Projection" {
        return Decision::Nothing;
    }

    let constructed = crate::appearance::construct(document, annotation, subtype);
    let owed = constructed.report.map(|detail| format!("{name}: {detail}"));
    let Some(content) = constructed.content else {
        return match owed {
            Some(detail) => Decision::Unsupported(detail),
            None => Decision::Nothing,
        };
    };

    // Table 166: `/CA` is the opacity for stroking "all visible elements of the annotation in
    // its closed state, including its background and border", and `/ca` the same for
    // nonstroking — "If a ca entry is not present in this dictionary, then the value of this CA
    // entry shall also be used for nonstroking operations as well."
    let stroke_alpha = opacity(document, annotation, "CA");
    Decision::Draw {
        appearance: Box::new(Appearance {
            transform: Transform::IDENTITY,
            bbox: rect,
            fill_alpha: opacity(document, annotation, "ca")
                .or(stroke_alpha)
                .unwrap_or(1.0),
            stroke_alpha: stroke_alpha.unwrap_or(1.0),
            blend: document
                .get_key(annotation, "BM")
                .as_name()
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
            is_widget: subtype == b"Widget",
            content: Content::Constructed(content),
        }),
        owed,
    }
}

/// Reads one of Table 166's opacity entries, clamped to the range it states.
fn opacity(document: &Document, annotation: &Dictionary, key: &'static str) -> Option<f32> {
    document
        .get_key(annotation, key)
        .as_number()
        .map(|value| narrow(value).clamp(0.0, 1.0))
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
pub(crate) fn rectangle(
    document: &Document,
    dict: &Dictionary,
    key: &'static str,
) -> Option<[f32; 4]> {
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
