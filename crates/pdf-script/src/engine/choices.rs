//! A field's options read from `/Opt`, and a choice made by index: `numItems`, `getItemAt`,
//! `exportValues` and `currentValueIndices` (ADR 1725).
//!
//! Each member's meaning is Adobe's *JavaScript for Acrobat API Reference* — "Field properties",
//! "Field methods" — cited and never quoted, a documented choice each under principle 5; what each
//! reads is the entry ISO 32000-2 states for it: Table 234's `/Opt` of a choice field, with
//! §12.7.5.4's `/V` and `/I` for what is selected, and Table 230's `/Opt` of a check box or a radio
//! button. The four members that rewrite Table 234's `/Opt` — `setItems`, `insertItemAt`,
//! `deleteItemAt`, `clearItems` — set the whole list a script left as one property of the field,
//! which the view state's appearance, a host's control and a save each read as the entry it writes
//! (ADR 1737).

use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::form::Choice;
use pdf_model::view::{FieldState, FieldType, MAX_PAGES, Property, ScriptEdit};

use super::bridge::{accessor, data, field_name, function, integral, terminals};
use super::members::{named, read_only};
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
    let rewriters: [(&str, NativeFunction); 4] = [
        ("setItems", NativeFunction::from_fn_ptr(set_items)),
        ("insertItemAt", NativeFunction::from_fn_ptr(insert_item_at)),
        ("deleteItemAt", NativeFunction::from_fn_ptr(delete_item_at)),
        ("clearItems", NativeFunction::from_fn_ptr(clear_items)),
    ];
    for (name, native) in rewriters {
        let callable = function(context, name, native);
        data(prototype, name, JsValue::from(callable), false, context)?;
    }
    Ok(())
}

/// `field.setItems(oArray)`: the list of options replaced by `oArray`'s, in its order.
///
/// The reference makes an element that converts to a string an option whose text and export value
/// are that string, and an element that is an array of two an option whose text is the first and
/// export value the second — the reverse of Table 234's own pair, which puts the export value
/// first. An array element with fewer than two entries is a `TypeError`, and a list longer than the
/// [`MAX_PAGES`] options a realm holds a `RangeError`.
fn set_items(this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    choice_field(this, "setItems", context)?;
    let first = arguments.first().cloned().unwrap_or_default();
    let list = match first.as_object() {
        Some(object) if object.is_array() => object.clone(),
        _ => {
            let values = named(arguments, &["oArray"], context)?;
            let value = values.first().cloned().unwrap_or_default();
            match value.as_object() {
                Some(object) if object.is_array() => object.clone(),
                _ => {
                    return Err(JsNativeError::typ()
                        .with_message("Field.setItems: oArray is not an array")
                        .into());
                }
            }
        }
    };
    let length = list
        .get(JsString::from("length"), context)?
        .to_length(context)?;
    if usize::try_from(length).map_or(true, |length| length > MAX_PAGES) {
        return Err(JsNativeError::range()
            .with_message(format!(
                "Field.setItems: {length} items are more than the {MAX_PAGES} a field's options \
                 hold here (ADR 1737)"
            ))
            .into());
    }
    let mut options = Vec::new();
    for index in 0..length {
        let element = list.get(index, context)?;
        let option = match element.as_object() {
            Some(pair) if pair.is_array() => {
                let count = pair
                    .get(JsString::from("length"), context)?
                    .to_length(context)?;
                if count < 2 {
                    return Err(JsNativeError::typ()
                        .with_message(format!(
                            "Field.setItems: item {index} is an array of {count}, and an item \
                             given as an array is its text and its export value"
                        ))
                        .into());
                }
                let label = pair
                    .get(0, context)?
                    .to_string(context)?
                    .to_std_string_lossy();
                let export = pair
                    .get(1, context)?
                    .to_string(context)?
                    .to_std_string_lossy();
                option_of(label, Some(export))
            }
            _ => option_of(element.to_string(context)?.to_std_string_lossy(), None),
        };
        options.push(option);
    }
    rewrite(this, context, |_, _| Some(options.clone()))
}

/// `field.insertItemAt(cName, cExport, nIdx)`: one option inserted, its export value `cExport`
/// where given and `cName` otherwise, at `nIdx` — the top where it is absent or 0, the end where it
/// is -1, as the reference has it. Any other place that is not in the list is a `RangeError`.
fn insert_item_at(
    this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let state = choice_field(this, "insertItemAt", context)?;
    let values = named(arguments, &["cName", "cExport", "nIdx"], context)?;
    let label = values
        .first()
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_lossy();
    let export = match values.get(1) {
        Some(value) if !value.is_null_or_undefined() => {
            Some(value.to_string(context)?.to_std_string_lossy())
        }
        _ => None,
    };
    let place = values.get(2).cloned().unwrap_or_default();
    let number = if place.is_undefined() {
        0.0
    } else {
        place.to_number(context)?
    };
    if state.options.len() >= MAX_PAGES {
        return Err(JsNativeError::range()
            .with_message(format!(
                "Field.insertItemAt: {} already holds the {MAX_PAGES} options a field's options \
                 hold here (ADR 1737)",
                state.name
            ))
            .into());
    }
    let valid = number.is_finite()
        && (integral(number) == -1
            || (number >= 0.0
                && usize::try_from(integral(number)).is_ok_and(|at| at <= state.options.len())));
    if !valid {
        return Err(JsNativeError::range()
            .with_message(format!(
                "Field.insertItemAt: {number} is no place among {}'s {} items",
                state.name,
                state.options.len()
            ))
            .into());
    }
    let option = option_of(label, export);
    rewrite(this, context, move |held, _| {
        let mut options = held.to_vec();
        let at = if integral(number) == -1 {
            options.len()
        } else {
            usize::try_from(integral(number))
                .unwrap_or(usize::MAX)
                .min(options.len())
        };
        options.insert(at, option.clone());
        Some(options)
    })
}

/// `field.deleteItemAt(nIdx)`: the option at `nIdx` deleted, or — where it is absent — the first
/// option selected now, as the reference's "the currently selected item" is one. With no index
/// and nothing selected nothing is deleted and the run says so; an index that names no option is a
/// `RangeError`.
fn delete_item_at(
    this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let state = choice_field(this, "deleteItemAt", context)?;
    let values = named(arguments, &["nIdx"], context)?;
    let place = values.first().cloned().unwrap_or_default();
    let asked = if place.is_undefined() {
        None
    } else {
        let number = place.to_number(context)?;
        let index = (number.is_finite() && number >= 0.0)
            .then(|| usize::try_from(integral(number)).ok())
            .flatten()
            .filter(|index| *index < state.options.len());
        let Some(index) = index else {
            return Err(JsNativeError::range()
                .with_message(format!(
                    "Field.deleteItemAt: {number} names none of {}'s {} items",
                    state.name,
                    state.options.len()
                ))
                .into());
        };
        Some(index)
    };
    if asked.is_none() && state.selected.is_empty() {
        State::with(context, |record| {
            record.note(&format!(
                "{}: the script called deleteItemAt with no index and nothing is selected, so no \
                 option is deleted (ADR 1737)",
                state.name
            ));
        });
        return Ok(JsValue::undefined());
    }
    rewrite(this, context, move |held, selected| {
        let index = asked.or_else(|| {
            selected
                .first()
                .and_then(|first| usize::try_from(*first).ok())
        })?;
        let mut options = held.to_vec();
        (index < options.len()).then(|| {
            options.remove(index);
            options
        })
    })
}

/// `field.clearItems()`: every option deleted.
fn clear_items(this: &JsValue, _arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    choice_field(this, "clearItems", context)?;
    rewrite(this, context, |_, _| Some(Vec::new()))
}

/// An option of `label`, whose export value is `export` where it differs: an export value equal
/// to the text is Table 234's plain text string.
fn option_of(label: String, export: Option<String>) -> Choice {
    Choice {
        export: export.filter(|export| *export != label),
        label,
    }
}

/// Rewrites the options of every terminal field the `Field` stands for, each to what `options`
/// makes of its own list and selection — `None` to leave it — as [`Property::Options`].
///
/// §12.7.5.4's `/V` names what is selected by its text, so an option a rewrite moved stays
/// selected where it now is; one a rewrite took away leaves the field with no selection, as the
/// reference's `deleteItemAt` says, and the field's choice is cleared as a person's empty choice
/// is ([`ScriptEdit::Choose`]).
fn rewrite(
    this: &JsValue,
    context: &mut Context,
    options: impl Fn(&[Choice], &[u32]) -> Option<Vec<Choice>>,
) -> JsResult<JsValue> {
    let name = field_name(this, context)?;
    for field in terminals(context, &name) {
        let Some(held) = State::table(context, |table| table.fields.get(&field).cloned()).flatten()
        else {
            continue;
        };
        if !matches!(held.kind, FieldType::ComboBox | FieldType::ListBox) {
            continue;
        }
        let Some(rewritten) = options(&held.options, &held.selected) else {
            continue;
        };
        let selected_labels: Vec<&str> = held
            .selected
            .iter()
            .filter_map(|index| usize::try_from(*index).ok())
            .filter_map(|index| held.options.get(index))
            .map(|option| option.label.as_str())
            .collect();
        let still: Vec<u32> = selected_labels
            .iter()
            .filter_map(|label| rewritten.iter().position(|option| option.label == *label))
            .filter_map(|index| u32::try_from(index).ok())
            .collect();
        let lost = still.len() < selected_labels.len();
        let edit = ScriptEdit::Property {
            field: field.clone(),
            widget: None,
            property: Property::Options(rewritten.clone()),
        };
        State::edit(context, &field, edit, move |state| {
            state.options = rewritten;
            state.selected = still;
        });
        if lost {
            let clear = ScriptEdit::Choose {
                field: field.clone(),
                indices: Vec::new(),
            };
            State::edit(context, &field, clear, |state| {
                state.selected.clear();
                state.value.clear();
            });
        }
    }
    Ok(JsValue::undefined())
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
