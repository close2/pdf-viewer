//! Where a Type 3 glyph that declares no box marks the page, ISO 32000-2 §9.6.4 (ADR 1363).
//!
//! A redaction tests a Type 3 code against the box its glyph description marks within. `d1`
//! declares one, and Table 111 requires it to be correct; for a `d0` glyph Table 110's
//! `/FontBBox` is the box — unless all four of its numbers are zero, where the clause withdraws
//! it: "If all four elements of the rectangle are zero, a PDF processor shall make no assumptions
//! about glyph sizes based on the font bounding box." Then the file states the box nowhere, and
//! what is left is the glyph description itself — "glyphs shall be defined by streams of PDF
//! graphics operators". Its marks are what that stream paints, so they are measured by running
//! it — through the same interpreter the page was drawn by, so no second reading of the operators
//! exists to disagree with the first (trap 6).
//!
//! The glyph is run on its own page, under the resources §7.8.3 gives a glyph description, in
//! glyph space: the page's
//! base transform is undone on the bounds afterwards, so what comes back is the glyph-space box the
//! caller carries to the page by `/FontMatrix` and the text rendering matrix, exactly as it does a
//! declared one. §9.6.4 has the description inherit every graphics state parameter but the CTM
//! "from the graphics state at the point of invocation of the text-showing operator", and of those
//! the only ones a mark's extent depends on are the line width and the miter limit, which are
//! stated in front of the description; its own `w` or `M`, if any, then replaces them. The bound
//! is [`pdf_render::Command::device_bounds`]'s, which errs outward: a mitre at its limit, a curve's
//! control polygon. A description the interpreter could not draw in full is not measured at all —
//! a part left undrawn would be a part left unmeasured — and the page is refused by name.

use std::sync::Arc;

use pdf_model::Page;
use pdf_model::content::{base_transform, interpret};
use pdf_render::geom::{Point, Rect};
use pdf_render::{Command, Transform};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, Stream};

/// The glyph-space box a Type 3 glyph description's own marks lie within, `Ok(None)` where it
/// paints nothing, or the refusal by name.
///
/// `width` and `miter_limit` are the line width and miter limit in force at the text-showing
/// operator, which the description inherits.
pub(super) fn measured_marks(
    document: &Document,
    page: &Page,
    font: &pdf_model::type3::Type3Font,
    glyph: &Stream,
    (width, miter_limit): (f64, f64),
) -> Result<Option<[f64; 4]>, String> {
    let refused = |why: &str| {
        format!(
            "§9.6.4: a d0 glyph in a font whose /FontBBox is all zero states its marks nowhere but \
             in its description, and {why}; the page is refused"
        )
    };
    let data = document
        .decoded_stream_data(glyph)
        .ok_or_else(|| refused("the description does not decode"))?;
    let mut content = format!("{width} w {} M\n", miter_limit.max(1.0)).into_bytes();
    content.extend_from_slice(&data);

    let stated = document
        .get_key(&glyph.dict, "Resources")
        .as_dict()
        .cloned();
    let resources = font.resources(stated.as_ref(), &page.resources).clone();

    let mut probe = crate::render::page_to_draw(page, None, false);
    let mut length = Dictionary::new();
    length.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(content.len()).unwrap_or(i64::MAX)),
    );
    probe.dict.insert(
        Name::new(&b"Contents"[..]),
        Object::Stream(Arc::new(Stream {
            dict: length,
            data: Arc::from(content.as_slice()),
            decryption_failed: false,
        })),
    );
    probe.resources = resources;

    let interpretation = interpret(document, &probe);
    if let Some(first) = interpretation.unsupported.first() {
        return Err(refused(&format!(
            "running it left part of it undrawn ({first:?})"
        )));
    }
    let Some(display) = bounds(interpretation.display_list.commands())? else {
        return Ok(None);
    };
    let to_glyph = base_transform(&probe)
        .invert()
        .ok_or_else(|| refused("the page's base transform has no inverse"))?;
    let corners = [
        Point::new(display.min.x, display.min.y),
        Point::new(display.max.x, display.min.y),
        Point::new(display.max.x, display.max.y),
        Point::new(display.min.x, display.max.y),
    ]
    .map(|corner| to_glyph.apply(corner));
    let mut out = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for corner in corners {
        out[0] = out[0].min(f64::from(corner.x));
        out[1] = out[1].min(f64::from(corner.y));
        out[2] = out[2].max(f64::from(corner.x));
        out[3] = out[3].max(f64::from(corner.y));
    }
    Ok(Some(out))
}

/// The union of what a run of display-list commands can mark, a group's being its elements', or
/// the refusal where one states no bound.
fn bounds(commands: &[Command]) -> Result<Option<Rect>, String> {
    let mut union: Option<Rect> = None;
    for command in commands {
        let extent = if let Command::Group { commands, .. } = command {
            bounds(commands)?
        } else {
            Some(command.device_bounds(Transform::IDENTITY).ok_or_else(|| {
                "§9.6.4: a d0 glyph in a font whose /FontBBox is all zero paints a mark whose \
                 extent the display list does not bound; the page is refused"
                    .to_owned()
            })?)
        };
        if let Some(extent) = extent {
            union = Some(union.map_or(extent, |union| union.union(extent)));
        }
    }
    Ok(union)
}
