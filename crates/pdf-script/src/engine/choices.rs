//! A field's options read from `/Opt`, and a choice made by index: `numItems`, `getItemAt`,
//! `exportValues` and `currentValueIndices` (ADR 1725).
//!
//! Each member's meaning is Adobe's *JavaScript for Acrobat API Reference* — "Field properties",
//! "Field methods" — cited and never quoted, a documented choice each under principle 5; what each
//! reads is the entry ISO 32000-2 states for it: Table 234's `/Opt` of a choice field, with
//! §12.7.5.4's `/V` and `/I` for what is selected, and Table 230's `/Opt` of a check box or a radio
//! button. The four members that would rewrite `/Opt` are refused by name (`crate::surface::
//! REFUSED`), since no reader's edit rewrites it.

use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::view::{FieldState, FieldType, ScriptEdit};

use super::bridge::{accessor, data, field_name, function, integral, terminals};
use super::members::read_only;
use super::{State, refuse};
use crate::RefusalKind;

/// Table 233 bit 22, `MultiSelect`.
const MULTI_SELECT: u32 = 1 << 21;

/// Installs a `Field`'s option members on the prototype every `Field` inherits.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn field(prototype: &JsObject, context: &mut Context) -> JsResult<()> {
    let getter = function(context, "numItems", NativeFunction::from_fn_ptr(num_items));
    let setter = function(
        context,
        "numItems",
        NativeFunction::from_copy_closure(|_this, _arguments, context| {
            Err(read_only("Field.numItems=", context))
        }),
    );
    accessor(prototype, "numItems", getter, setter, context)?;
    let getter = function(
        context,
        "exportValues",
        NativeFunction::from_fn_ptr(export_values),
    );
    let setter = function(
        context,
        "exportValues",
        NativeFunction::from_copy_closure(|_this, _arguments, context| {
            Err(refuse(
                "Field.exportValues=".to_owned(),
                RefusalKind::Unreachable(
                    "it rewrites Table 230's /Opt, which no reader's edit changes (ADR 1725)"
                        .to_owned(),
                ),
                context,
            ))
        }),
    );
    accessor(prototype, "exportValues", getter, setter, context)?;
    let getter = function(
        context,
        "currentValueIndices",
        NativeFunction::from_fn_ptr(read_indices),
    );
    let setter = function(
        context,
        "currentValueIndices",
        NativeFunction::from_fn_ptr(write_indices),
    );
    accessor(prototype, "currentValueIndices", getter, setter, context)?;
    let get_item_at = function(
        context,
        "getItemAt",
        NativeFunction::from_fn_ptr(get_item_at),
    );
    data(
        prototype,
        "getItemAt",
        JsValue::from(get_item_at),
        false,
        context,
    )?;
    Ok(())
}

/// The first terminal field a `Field` stands for, held to a choice field: the reference gives
/// `member` to a combo box and a list box alone, and any other field is refused by name.
fn choice_field(this: &JsValue, member: &str, context: &mut Context) -> JsResult<FieldState> {
    let state = first_state(this, context)?;
    match state {
        Some(state) if matches!(state.kind, FieldType::ComboBox | FieldType::ListBox) => Ok(state),
        Some(state) => Err(refuse(
            format!("Field.{member}"),
            RefusalKind::Unreachable(format!(
                "{member} is a combo box's or a list box's, Table 234's, and {} is a {} field",
                state.name,
                state.kind.adobe()
            )),
            context,
        )),
        None => Err(JsNativeError::typ()
            .with_message(format!("Field.{member}: the Field names no field"))
            .into()),
    }
}

/// The first terminal field a `Field` stands for, as the realm holds it.
fn first_state(this: &JsValue, context: &mut Context) -> JsResult<Option<FieldState>> {
    let name = field_name(this, context)?;
    let Some(first) = terminals(context, &name).into_iter().next() else {
        return Ok(None);
    };
    Ok(State::table(context, |table| table.fields.get(&first).cloned()).flatten())
}

/// `field.numItems`: how many entries Table 234's `/Opt` holds.
fn num_items(this: &JsValue, _arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let state = choice_field(this, "numItems", context)?;
    Ok(JsValue::from(
        u32::try_from(state.options.len()).unwrap_or(u32::MAX),
    ))
}

/// `field.getItemAt(nIdx, bExportValue)`: one `/Opt` entry — its export value where
/// `bExportValue` is true, as it is where absent, and the entry states one; its text otherwise.
///
/// `-1` is the last entry, as the reference has it. Any other index that names no entry is a
/// `RangeError`, since the reference states no answer for one.
fn get_item_at(this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let state = choice_field(this, "getItemAt", context)?;
    let number = arguments
        .first()
        .cloned()
        .unwrap_or_default()
        .to_number(context)?;
    let export = arguments
        .get(1)
        .filter(|value| !value.is_undefined())
        .is_none_or(JsValue::to_boolean);
    let index = if number.is_finite() && integral(number) == -1 {
        state.options.len().checked_sub(1)
    } else if number.is_finite() && number >= 0.0 {
        usize::try_from(integral(number))
            .ok()
            .filter(|index| *index < state.options.len())
    } else {
        None
    };
    let Some(option) = index.and_then(|index| state.options.get(index)) else {
        return Err(JsNativeError::range()
            .with_message(format!(
                "Field.getItemAt: {number} names none of {}'s {} items",
                state.name,
                state.options.len()
            ))
            .into());
    };
    let text = if export {
        option.export.as_deref().unwrap_or(&option.label)
    } else {
        &option.label
    };
    Ok(JsValue::from(JsString::from(text)))
}

/// `field.exportValues`: each widget's export value in the field table's order, a check box's or
/// a radio button's — Table 230's `/Opt` entry for the widget where the field states one, the name
/// its on state is selected by otherwise, and empty for a widget that names no single on state.
fn export_values(
    this: &JsValue,
    _arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let Some(state) = first_state(this, context)? else {
        return Ok(JsValue::undefined());
    };
    if !matches!(state.kind, FieldType::CheckBox | FieldType::RadioButton) {
        return Err(refuse(
            "Field.exportValues".to_owned(),
            RefusalKind::Unreachable(format!(
                "exportValues is a check box's or a radio button's, Table 230's, and {} is a {} \
                 field",
                state.name,
                state.kind.adobe()
            )),
            context,
        ));
    }
    let values: Vec<JsValue> = state
        .widgets
        .iter()
        .enumerate()
        .map(|(index, widget)| {
            let text = state
                .options
                .get(index)
                .map(|option| option.label.clone())
                .or_else(|| widget.on_state.clone())
                .unwrap_or_default();
            JsValue::from(JsString::from(text.as_str()))
        })
        .collect();
    Ok(JsValue::from(JsArray::from_iter(values, context)))
}

/// `field.currentValueIndices`: the selected entries' indices into `/Opt` as §12.7.5.4 reads `/V`
/// and `/I` — one as a number, several as an ascending array, and `-1` where the value names no
/// entry (an editable combo box's own text) or nothing is selected.
fn read_indices(
    this: &JsValue,
    _arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let state = choice_field(this, "currentValueIndices", context)?;
    Ok(match state.selected.as_slice() {
        [] => JsValue::from(-1),
        [one] => JsValue::from(*one),
        several => JsValue::from(JsArray::from_iter(
            several.iter().map(|index| JsValue::from(*index)),
            context,
        )),
    })
}

/// `field.currentValueIndices = n` or `= [n, …]`: the entries chosen by index, committed as a
/// person's choice is ([`ScriptEdit::Choose`]) on every terminal field the name stands for.
///
/// An index that names no entry is a `RangeError` and chooses nothing. Several indices on a field
/// whose `MultiSelect` flag is clear are cut to the first, as Table 233 bit 22's "at most one item
/// shall be selected" cuts a person's.
fn write_indices(
    this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let state = choice_field(this, "currentValueIndices", context)?;
    let value = arguments.first().cloned().unwrap_or_default();
    let mut asked: Vec<f64> = Vec::new();
    if let Some(list) = value.as_object().filter(|object| object.is_array()) {
        let length = list
            .get(JsString::from("length"), context)?
            .to_length(context)?;
        for index in 0..length.min(u64::from(u16::MAX)) {
            asked.push(list.get(index, context)?.to_number(context)?);
        }
    } else {
        asked.push(value.to_number(context)?);
    }
    let mut indices = Vec::new();
    for number in asked {
        let index = (number.is_finite() && number >= 0.0)
            .then(|| u32::try_from(integral(number)).ok())
            .flatten()
            .filter(|index| usize::try_from(*index).is_ok_and(|at| at < state.options.len()));
        let Some(index) = index else {
            return Err(JsNativeError::range()
                .with_message(format!(
                    "Field.currentValueIndices: {number} names none of {}'s {} items",
                    state.name,
                    state.options.len()
                ))
                .into());
        };
        indices.push(index);
    }
    indices.sort_unstable();
    indices.dedup();
    let name = field_name(this, context)?;
    for field in terminals(context, &name) {
        let Some(held) = State::table(context, |table| table.fields.get(&field).cloned()).flatten()
        else {
            continue;
        };
        let mut chosen = indices.clone();
        if held.flags & MULTI_SELECT == 0 {
            chosen.truncate(1);
        }
        let labels: Vec<String> = chosen
            .iter()
            .filter_map(|index| usize::try_from(*index).ok())
            .filter_map(|index| held.options.get(index))
            .map(|option| option.label.clone())
            .collect();
        let edit = ScriptEdit::Choose {
            field: field.clone(),
            indices: chosen.clone(),
        };
        State::edit(context, &field, edit, move |state| {
            state.selected = chosen;
            state.value = labels.first().cloned().unwrap_or_default();
        });
    }
    Ok(JsValue::undefined())
}
