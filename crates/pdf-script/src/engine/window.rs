//! The window's view of the document: `this.zoom`, `this.zoomType`, `this.layout`, `this.scroll`
//! and the `zoomtype` constants (ADR 1736).
//!
//! Each member's meaning is Adobe's *JavaScript for Acrobat API Reference* — "Doc properties",
//! "Doc methods" — cited and never quoted, a documented choice each under principle 5. **A read
//! is what the host told**: the view state hands every event the window's view as a host last
//! said it ([`pdf_model::view::WindowView`]). **A write is a request**: it changes what the realm
//! reads back for the rest of the run and records a [`ScriptEdit::View`] that a host carries out
//! as it carries a person's zoom, layout or scroll, the latest of each kind in a run standing. A
//! value that names nothing a host can draw changes nothing, and the run's notes say so — the way
//! `this.pageNum` past the last page turns none (ADR 1640).

use boa_engine::object::ObjectInitializer;
use boa_engine::{Context, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::view::{ScriptEdit, ViewChange, ZoomType};
use pdf_model::viewer_preferences::PageLayout;

use super::State;
use super::bridge::{accessor, data, function};
use super::members::named;

/// The reference's bounds on `zoom`, in per cent: "between 8.33% and 6400%".
const ZOOM_PERCENT: std::ops::RangeInclusive<f64> = 8.33..=6400.0;

/// Table 29's six `/PageLayout` names, which are the six values the reference lists for `layout`.
const LAYOUTS: [(PageLayout, &str); 6] = [
    (PageLayout::SinglePage, "SinglePage"),
    (PageLayout::OneColumn, "OneColumn"),
    (PageLayout::TwoColumnLeft, "TwoColumnLeft"),
    (PageLayout::TwoColumnRight, "TwoColumnRight"),
    (PageLayout::TwoPageLeft, "TwoPageLeft"),
    (PageLayout::TwoPageRight, "TwoPageRight"),
];

/// Installs `zoom`, `zoomType`, `layout` and `scroll` on the document, and the `zoomtype`
/// constants beside it.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn install(global: &JsObject, context: &mut Context) -> JsResult<()> {
    let members: [(&str, NativeFunction, NativeFunction); 3] = [
        (
            "zoom",
            NativeFunction::from_fn_ptr(read_zoom),
            NativeFunction::from_fn_ptr(write_zoom),
        ),
        (
            "zoomType",
            NativeFunction::from_fn_ptr(read_zoom_type),
            NativeFunction::from_fn_ptr(write_zoom_type),
        ),
        (
            "layout",
            NativeFunction::from_fn_ptr(read_layout),
            NativeFunction::from_fn_ptr(write_layout),
        ),
    ];
    for (name, read, write) in members {
        let getter = function(context, name, read);
        let setter = function(context, name, write);
        accessor(global, name, getter, setter, context)?;
    }
    let scroll = function(context, "scroll", NativeFunction::from_fn_ptr(scroll));
    data(global, "scroll", JsValue::from(scroll), false, context)?;
    let constants = ObjectInitializer::new(context).build();
    for zoom_type in ZoomType::ALL {
        data(
            &constants,
            zoom_type.constant(),
            JsValue::from(JsString::from(zoom_type.adobe())),
            false,
            context,
        )?;
    }
    data(global, "zoomtype", JsValue::from(constants), false, context)?;
    Ok(())
}

/// `this.zoom`: the magnification the host told, in per cent; `undefined` where no host has said
/// how large the page is drawn.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function's signature is the engine's, and every native answers a result"
)]
fn read_zoom(_this: &JsValue, _arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let zoom = State::table(context, |table| table.view.zoom).flatten();
    Ok(zoom.map_or_else(JsValue::undefined, JsValue::from))
}

/// `this.zoom = n`: a fixed magnification of `n` per cent, inside the reference's bounds.
///
/// A fixed magnification is what the reference's `NoVary` names, so the realm reads `zoomType`
/// as `NoVary` afterwards; a value that is not a number, or falls outside 8.33 to 6400, changes
/// nothing and is noted.
fn write_zoom(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let asked = arguments
        .first()
        .cloned()
        .unwrap_or_default()
        .to_number(context)?;
    if !ZOOM_PERCENT.contains(&asked) {
        State::with(context, |record| {
            record.note(&format!(
                "the script set this.zoom to {asked}, which is outside the 8.33 to 6400 per cent \
                 the reference allows, so the view does not change (ADR 1736)"
            ));
        });
        return Ok(JsValue::undefined());
    }
    State::table(context, |table| {
        table.view.zoom = Some(asked);
        table.view.zoom_type = ZoomType::NoVary;
    });
    ask(context, ViewChange::Zoom(asked));
    Ok(JsValue::undefined())
}

/// `this.zoomType`: the reference's name for how the host chooses the magnification.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function's signature is the engine's, and every native answers a result"
)]
fn read_zoom_type(
    _this: &JsValue,
    _arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let zoom_type = State::table(context, |table| table.view.zoom_type).unwrap_or_default();
    Ok(JsValue::from(JsString::from(zoom_type.adobe())))
}

/// `this.zoomType = t`: one of the reference's zoom types, as `zoomtype`'s constants spell them.
///
/// `Preferred` and `ReflowWidth` ask for the reader's own preferred magnification and for a
/// reflowed page, neither of which this program has to give; they and a name the reference does
/// not list change nothing and are noted.
fn write_zoom_type(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let asked = arguments
        .first()
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_lossy();
    let sentence = match ZoomType::from_adobe(&asked) {
        Some(zoom_type) if zoom_type.is_drawn() => {
            State::table(context, |table| table.view.zoom_type = zoom_type);
            ask(context, ViewChange::ZoomType(zoom_type));
            return Ok(JsValue::undefined());
        }
        Some(_) => format!(
            "the script set this.zoomType to {asked:?}, a magnification this program does not \
             have to give, so the view does not change (ADR 1736)"
        ),
        None => format!(
            "the script set this.zoomType to {asked:?}, which is none of the reference's zoom \
             types, so the view does not change (ADR 1736)"
        ),
    };
    State::with(context, |record| record.note(&sentence));
    Ok(JsValue::undefined())
}

/// `this.layout`: Table 29's name for how the pages are arranged in the window now.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function's signature is the engine's, and every native answers a result"
)]
fn read_layout(
    _this: &JsValue,
    _arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let layout = State::table(context, |table| table.view.layout).unwrap_or_default();
    let name = LAYOUTS
        .iter()
        .find(|(held, _)| *held == layout)
        .map_or("SinglePage", |(_, name)| name);
    Ok(JsValue::from(JsString::from(name)))
}

/// `this.layout = name`: one of Table 29's six arrangements; any other name changes nothing and
/// is noted.
fn write_layout(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let asked = arguments
        .first()
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_lossy();
    let Some((layout, _)) = LAYOUTS.into_iter().find(|(_, name)| *name == asked) else {
        State::with(context, |record| {
            record.note(&format!(
                "the script set this.layout to {asked:?}, which is none of Table 29's six page \
                 layouts, so the view does not change (ADR 1736)"
            ));
        });
        return Ok(JsValue::undefined());
    };
    State::table(context, |table| table.view.layout = layout);
    ask(context, ViewChange::Layout(layout));
    Ok(JsValue::undefined())
}

/// `this.scroll(nX, nY)`: this point of the current page — `this.pageNum` — brought to the middle
/// of the window.
///
/// The reference states the point "in rotated user space", which [`super::pages`] reads as
/// `getPageBox` does; it crosses in default user space, which is what a host places a
/// destination's coordinates from. A coordinate that is not a finite number, or a page past the
/// pages the realm is told of, scrolls nothing and is noted.
fn scroll(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let values = named(arguments, &["nX", "nY"], context)?;
    let mut point = [0.0; 2];
    for (coordinate, value) in point.iter_mut().zip(&values) {
        *coordinate = value.to_number(context)?;
    }
    let [x, y] = point;
    let page = State::table(context, |table| {
        let page = table.page;
        let state = usize::try_from(page)
            .ok()
            .and_then(|index| table.document.pages.get(index).cloned());
        (page, state)
    });
    let sentence = match page {
        Some((page, Some(state))) if x.is_finite() && y.is_finite() => {
            let (x, y) = super::pages::unrotated_point((x, y), state.boxes[0], state.rotate);
            ask(context, ViewChange::Scroll { page, x, y });
            return Ok(JsValue::undefined());
        }
        Some((_, Some(_))) => format!(
            "the script called this.scroll({x}, {y}), which names no point of the page, so the \
             view does not move (ADR 1736)"
        ),
        _ => "the script called this.scroll on a page the realm is not told of, so the view does \
              not move (ADR 1736)"
            .to_owned(),
    };
    State::with(context, |record| record.note(&sentence));
    Ok(JsValue::undefined())
}

/// Records `change` as the run's request of its kind, replacing an earlier one of the same kind:
/// the latest zoom, zoom type, layout or scroll of a run is the one carried.
fn ask(context: &Context, change: ViewChange) {
    State::with(context, |record| {
        let kind = std::mem::discriminant(&change);
        if let Some(earlier) = record.edits.iter_mut().find(|edit| {
            matches!(edit, ScriptEdit::View { change: held } if std::mem::discriminant(held) == kind)
        }) {
            *earlier = ScriptEdit::View { change };
        } else if record.edits.len() < super::MAX_EDITS {
            record.edits.push(ScriptEdit::View { change });
        }
    });
}
