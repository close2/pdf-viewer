//! The document's pages, its named destinations, `title`, `calculate` and `app.activeDocs`: the
//! `Doc` members RFC 0008 section 4.2 admits that the view state already answers (ADR 1724).
//!
//! Each member's meaning is Adobe's *JavaScript for Acrobat API Reference* — "Doc methods", "Doc
//! properties", "app properties" — cited and never quoted, a documented choice each under
//! principle 5; what each reads is the entry ISO 32000-2 states for it: §12.4.2's page labels,
//! Table 31's boundaries and `/Rotate`, §12.3.2.4's named destinations, Table 349's `/Title`, and
//! Table 224's `/CO`.

use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::view::{MAX_PAGES, PageState, ScriptEdit};

use super::bridge::{accessor, data, function, integral};
use super::members::{named, read_only};
use super::{State, refuse};
use crate::RefusalKind;

/// Installs the document's page members, `gotoNamedDest`, `title` and `calculate`.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn document(global: &JsObject, context: &mut Context) -> JsResult<()> {
    let methods: [(&str, NativeFunction); 4] = [
        ("getPageLabel", NativeFunction::from_fn_ptr(get_page_label)),
        ("getPageBox", NativeFunction::from_fn_ptr(get_page_box)),
        (
            "getPageRotation",
            NativeFunction::from_fn_ptr(get_page_rotation),
        ),
        (
            "gotoNamedDest",
            NativeFunction::from_fn_ptr(goto_named_dest),
        ),
    ];
    for (name, native) in methods {
        let callable = function(context, name, native);
        data(global, name, JsValue::from(callable), false, context)?;
    }
    let getter = function(context, "title", NativeFunction::from_fn_ptr(read_title));
    let setter = function(
        context,
        "title",
        NativeFunction::from_copy_closure(|_this, _arguments, context| {
            Err(read_only("this.title=", context))
        }),
    );
    accessor(global, "title", getter, setter, context)?;
    let getter = function(
        context,
        "calculate",
        NativeFunction::from_copy_closure(|_this, _arguments, context| {
            let off = State::table(context, |table| table.calculations_off).unwrap_or(false);
            Ok(JsValue::from(!off))
        }),
    );
    let setter = function(
        context,
        "calculate",
        NativeFunction::from_fn_ptr(write_calculate),
    );
    accessor(global, "calculate", getter, setter, context)?;
    Ok(())
}

/// Installs `app.activeDocs`.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn app(app: &JsObject, context: &mut Context) -> JsResult<()> {
    // RFC 0008 section 4.2 admits `activeDocs` for this document only, and section 4.3 excludes
    // every other: a realm holds one document, so the array holds it alone. Adobe's "app
    // properties" page lets a script outside a batch see only documents whose `disclosed` is true,
    // which is a rule about the others; this one is the realm's own.
    let getter = function(
        context,
        "activeDocs",
        NativeFunction::from_copy_closure(|_this, _arguments, context| {
            let this = JsValue::from(context.global_object());
            Ok(JsValue::from(JsArray::from_iter([this], context)))
        }),
    );
    let setter = function(
        context,
        "activeDocs",
        NativeFunction::from_copy_closure(|_this, _arguments, context| {
            Err(read_only("app.activeDocs=", context))
        }),
    );
    accessor(app, "activeDocs", getter, setter, context)?;
    Ok(())
}

/// `this.getPageLabel(nPage)`: §12.4.2's label of the page, or — where the document labels it
/// not — its number counted from one, the number a viewer shows for an unlabelled page; a
/// documented choice, since the reference says only that it answers the page's label.
fn get_page_label(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let values = named(arguments, &["nPage"], context)?;
    let (index, page) = page_argument(&values, 0, "this.getPageLabel", context)?;
    let label = page
        .label
        .unwrap_or_else(|| index.saturating_add(1).to_string());
    Ok(JsValue::from(JsString::from(label.as_str())))
}

/// `this.getPageRotation(nPage)`: Table 31's `/Rotate`, inherited, as 0, 90, 180 or 270.
fn get_page_rotation(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let values = named(arguments, &["nPage"], context)?;
    let (_, page) = page_argument(&values, 0, "this.getPageRotation", context)?;
    Ok(JsValue::from(page.rotate))
}

/// `this.getPageBox(cBox, nPage)`: one of Table 31's boundaries of the page, in the reference's
/// "rotated user space" and its order — upper left, then lower right.
///
/// `cBox` is the reference's `Art`, `Bleed`, `Crop` (where it is absent) or `Trim`, or `Media`,
/// which its own example passes; each is §14.11.2.1's box after its defaults and its intersection
/// with the media box. `BBox` — the bounds of what the page draws — is no boundary Table 31 states,
/// and the realm is not told the page's marks, so it is refused by name.
///
/// Rotated user space is a documented choice, since the reference names it and defines it
/// nowhere: default user space turned clockwise by `/Rotate` — the direction Table 31 gives the
/// entry — and moved so that the turned media box's lower-left corner is where the media box's
/// own was. A page with no rotation answers its box as the file states it; a page turned a
/// quarter answers a box as wide as the page is shown.
fn get_page_box(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let values = named(arguments, &["cBox", "nPage"], context)?;
    let asked = values.first().cloned().unwrap_or_default();
    let name = if asked.is_undefined() {
        "Crop".to_owned()
    } else {
        asked.to_string(context)?.to_std_string_lossy()
    };
    let which = match name.as_str() {
        "Media" => 0,
        "Crop" => 1,
        "Bleed" => 2,
        "Trim" => 3,
        "Art" => 4,
        "BBox" => {
            return Err(refuse(
                "this.getPageBox(\"BBox\")".to_owned(),
                RefusalKind::Unreachable(
                    "the bounds of what a page draws is no boundary Table 31 states, and a \
                     document's realm is not told the page's marks (ADR 1724)"
                        .to_owned(),
                ),
                context,
            ));
        }
        _ => {
            return Err(JsNativeError::range()
                .with_message(format!(
                    "this.getPageBox: {name:?} is none of Art, Bleed, BBox, Crop, Media and Trim"
                ))
                .into());
        }
    };
    let (_, page) = page_argument(&values, 1, "this.getPageBox", context)?;
    let [x0, y0, x1, y1] = rotated(page.boxes[which], page.boxes[0], page.rotate);
    let corners = [x0, y1, x1, y0];
    Ok(JsValue::from(JsArray::from_iter(
        corners.into_iter().map(JsValue::from),
        context,
    )))
}

/// `rect` in rotated user space, normalised to `[x0, y0, x1, y1]` with the smaller of each pair
/// first: turned clockwise by `rotate` degrees and moved so that `media`'s turned lower-left
/// corner is where `media`'s own was ([`get_page_box`]).
fn rotated(rect: [f64; 4], media: [f64; 4], rotate: u16) -> [f64; 4] {
    let turn = |x: f64, y: f64| match rotate {
        90 => (y, -x),
        180 => (-x, -y),
        270 => (-y, x),
        _ => (x, y),
    };
    let normalised = |[x0, y0, x1, y1]: [f64; 4]| {
        let (ax, ay) = turn(x0, y0);
        let (bx, by) = turn(x1, y1);
        [ax.min(bx), ay.min(by), ax.max(bx), ay.max(by)]
    };
    let turned_media = normalised(media);
    let dx = media[0].min(media[2]) - turned_media[0];
    let dy = media[1].min(media[3]) - turned_media[1];
    let [x0, y0, x1, y1] = normalised(rect);
    [x0 + dx, y0 + dy, x1 + dx, y1 + dy]
}

/// The page a page method's argument at `position` names — zero where it is absent, as the
/// reference's default is — with its index; a `RangeError` for a number that names no page, and a
/// refusal naming the bound for a page past [`MAX_PAGES`].
fn page_argument(
    values: &[JsValue],
    position: usize,
    member: &str,
    context: &mut Context,
) -> JsResult<(usize, PageState)> {
    let value = values.get(position).cloned().unwrap_or_default();
    let number = if value.is_undefined() {
        0.0
    } else {
        value.to_number(context)?
    };
    let (pages, told) = State::table(context, |table| (table.pages, table.document.pages.len()))
        .unwrap_or_default();
    if !number.is_finite() || number < 0.0 || number.trunc() >= f64::from(pages) {
        return Err(JsNativeError::range()
            .with_message(format!(
                "{member}: nPage {number} names no page of this document's {pages}"
            ))
            .into());
    }
    let index = usize::try_from(integral(number)).unwrap_or(usize::MAX);
    if index >= told {
        return Err(refuse(
            member.to_owned(),
            RefusalKind::Unreachable(format!(
                "page {} is past the {MAX_PAGES} pages a document's realm is told of (ADR 1724)",
                index.saturating_add(1)
            )),
            context,
        ));
    }
    let page = State::table(context, |table| table.document.pages.get(index).cloned())
        .flatten()
        .ok_or_else(|| JsNativeError::error().with_message("the realm holds no document"))?;
    Ok((index, page))
}

/// `this.gotoNamedDest(cName)`: §12.3.2.4's named destination asked for, which the view state
/// looks up and turns to as it turns to `this.pageNum` ([`ScriptEdit::Destination`]).
fn goto_named_dest(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let values = named(arguments, &["cName"], context)?;
    let name = values
        .first()
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_lossy();
    State::note(context, ScriptEdit::Destination { name });
    Ok(JsValue::undefined())
}

/// `this.title`: Table 349's `/Title` as `this.info.Title` reads it, which the reference's "Doc
/// properties" page says supersedes it; `undefined` where the dictionary states none.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function's signature is the engine's, and every native answers a result"
)]
fn read_title(_this: &JsValue, _arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let title = State::table(context, |table| {
        table
            .document
            .info
            .iter()
            .find(|entry| entry.key == "Title")
            .map(|entry| entry.text.clone())
    })
    .flatten();
    Ok(title.map_or_else(JsValue::undefined, |title| {
        JsValue::from(JsString::from(title.as_str()))
    }))
}

/// `this.calculate = b`: whether Table 224's `/CO` is walked at all — read back for the realm's
/// lifetime and held by the view state, which walks no calculation while it is false, the
/// reference's `calculateNow` among them ([`ScriptEdit::Calculation`]).
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function's signature is the engine's, and every native answers a result"
)]
fn write_calculate(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let on = arguments.first().is_some_and(JsValue::to_boolean);
    State::table(context, |table| table.calculations_off = !on);
    State::note(context, ScriptEdit::Calculation { on });
    Ok(JsValue::undefined())
}

#[cfg(test)]
mod tests {
    use super::rotated;

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "every corner is a whole number, exact in a double, and turning one only moves \
                  and negates it"
    )]
    fn a_quarter_turned_letter_page_is_as_wide_as_it_is_shown() {
        let letter = [0.0, 0.0, 612.0, 792.0];
        assert_eq!(rotated(letter, letter, 0), letter);
        assert_eq!(rotated(letter, letter, 90), [0.0, 0.0, 792.0, 612.0]);
        assert_eq!(rotated(letter, letter, 180), letter);
        assert_eq!(rotated(letter, letter, 270), [0.0, 0.0, 792.0, 612.0]);
        // A crop box inset from the media box's lower-left corner, turned a quarter clockwise:
        // what was 36 from the bottom is 36 from the left.
        let crop = [10.0, 36.0, 600.0, 780.0];
        assert_eq!(rotated(crop, letter, 90), [36.0, 12.0, 780.0, 602.0]);
    }
}
