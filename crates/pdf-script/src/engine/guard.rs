//! The memory budget, checked before a built-in that an argument can make large runs.
//!
//! Boa has no memory ceiling (RFC 0008 section 5.1's table), so the budget is enforced where a
//! script chooses a size and a built-in honours it in one native call: an array's `length`, read by
//! every `Array.prototype` method that walks it — `new Array(5e6)` sets a length and allocates
//! nothing, and `fill` then allocates five million values in one call the step budget cannot see —
//! the lengths `concat`, `push` and `unshift` produce, the length `Array.from` copies, the string
//! `join` builds, and the strings `repeat`, `padStart` and `padEnd` are asked for. Each original is
//! replaced, before the script runs, by a function that checks the size and then calls it; the
//! original is held only by that function, so a script cannot reach around it.
//!
//! A size over the budget stops the script with an error Boa's own catch handling passes over —
//! an engine error, as its runtime limits are — after the record has noted which budget and what
//! was asked, so that `try { … } catch` cannot turn a refused allocation into a loop of them
//! (ADR 1590).

use boa_engine::error::{EngineError, PanicError};
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::property::{PropertyDescriptor, PropertyKey};
use boa_engine::{Context, JsError, JsObject, JsResult, JsString, JsValue, NativeFunction};

use super::State;
use crate::Exceeded;

/// What a guard measures before it calls the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Measure {
    /// `this.length`.
    Length,
    /// `this.length` plus the length of every array-like argument.
    Concatenated,
    /// `this.length` plus the number of arguments.
    Appended,
    /// `this.length`, and the string `join` would build with its separator.
    Joined,
    /// The first argument's `length`.
    Copied,
    /// The string `repeat` is asked for: `this.length` times the count.
    Repeated,
    /// The string `padStart` or `padEnd` is asked for: the first argument.
    Padded,
}

/// Replaces every built-in this module guards with its guarded form.
///
/// # Errors
///
/// The engine's, where a prototype refuses a property — which a context just constructed does not.
pub(super) fn install(context: &mut Context) -> JsResult<()> {
    let array = context.intrinsics().constructors().array().constructor();
    let array_prototype = context.intrinsics().constructors().array().prototype();
    let string_prototype = context.intrinsics().constructors().string().prototype();
    for key in array_prototype.own_property_keys(context)? {
        let PropertyKey::String(name) = &key else {
            continue;
        };
        let measure = match name.to_std_string_lossy().as_str() {
            "constructor" => continue,
            "concat" => Measure::Concatenated,
            "push" | "unshift" => Measure::Appended,
            "join" | "toString" | "toLocaleString" => Measure::Joined,
            _ => Measure::Length,
        };
        guard(&array_prototype, key.clone(), measure, context)?;
    }
    guard(
        &array,
        PropertyKey::from(JsString::from("from")),
        Measure::Copied,
        context,
    )?;
    for (name, measure) in [
        ("repeat", Measure::Repeated),
        ("padStart", Measure::Padded),
        ("padEnd", Measure::Padded),
    ] {
        guard(
            &string_prototype,
            PropertyKey::from(JsString::from(name)),
            measure,
            context,
        )?;
    }
    Ok(())
}

/// Replaces `holder[key]`, where it is a function, with one that measures first.
fn guard(
    holder: &JsObject,
    key: PropertyKey,
    measure: Measure,
    context: &mut Context,
) -> JsResult<()> {
    let original = holder.get(key.clone(), context)?;
    let Some(original) = original.as_callable() else {
        return Ok(());
    };
    let length = original
        .get(JsString::from("length"), context)?
        .to_u32(context)?;
    let length = usize::try_from(length).unwrap_or(usize::MAX);
    let name = match &key {
        PropertyKey::String(name) => name.clone(),
        PropertyKey::Symbol(_) | PropertyKey::Index(_) => JsString::from(""),
    };
    let guarded = FunctionObjectBuilder::new(
        context.realm(),
        NativeFunction::from_copy_closure_with_captures(
            move |this, arguments, original: &JsObject, context| {
                check(measure, this, arguments, context)?;
                original.call(this, arguments, context)
            },
            original,
        ),
    )
    .name(name)
    .length(length)
    .build();
    holder.define_property_or_throw(
        key,
        PropertyDescriptor::builder()
            .value(guarded)
            .writable(true)
            .enumerable(false)
            .configurable(true)
            .build(),
        context,
    )?;
    Ok(())
}

/// Checks one call against the budget, recording and stopping where it is over.
fn check(
    measure: Measure,
    this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<()> {
    let Some((elements, units)) = State::with(context, |record| {
        (record.budget.elements, record.budget.string_units)
    }) else {
        return Ok(());
    };
    let argument = |index: usize| arguments.get(index).cloned().unwrap_or_default();
    match measure {
        Measure::Length => within_elements(length_of(this, context)?, elements, context),
        Measure::Appended => within_elements(
            length_of(this, context)?
                .saturating_add(u64::try_from(arguments.len()).unwrap_or(u64::MAX)),
            elements,
            context,
        ),
        Measure::Concatenated => {
            let mut total = length_of(this, context)?;
            for value in arguments {
                total = total.saturating_add(if value.is_object() {
                    length_of(value, context)?
                } else {
                    1
                });
            }
            within_elements(total, elements, context)
        }
        Measure::Joined => {
            let length = length_of(this, context)?;
            within_elements(length, elements, context)?;
            let separator = match argument(0) {
                value if value.is_undefined() => 1,
                value => u64::try_from(value.to_string(context)?.len()).unwrap_or(u64::MAX),
            };
            within_units(length.saturating_mul(separator), units, context)
        }
        Measure::Copied => {
            let source = argument(0);
            if source.is_object() {
                within_elements(length_of(&source, context)?, elements, context)
            } else {
                Ok(())
            }
        }
        Measure::Repeated => {
            if this.is_null_or_undefined() {
                return Ok(());
            }
            let length = u64::try_from(this.to_string(context)?.len()).unwrap_or(u64::MAX);
            let count = argument(0).to_length(context)?;
            within_units(length.saturating_mul(count), units, context)
        }
        Measure::Padded => within_units(argument(0).to_length(context)?, units, context),
    }
}

/// ECMA-262's `LengthOfArrayLike` of `value`, or zero where it is not an object: the original then
/// says what is wrong with it.
fn length_of(value: &JsValue, context: &mut Context) -> JsResult<u64> {
    if value.is_null_or_undefined() {
        return Ok(0);
    }
    let object = value.to_object(context)?;
    object
        .get(JsString::from("length"), context)?
        .to_length(context)
}

/// Stops the script where `asked` elements are over `ceiling`.
fn within_elements(asked: u64, ceiling: u64, context: &mut Context) -> JsResult<()> {
    if asked > ceiling {
        return Err(stop(Exceeded::Elements { asked, ceiling }, context));
    }
    Ok(())
}

/// Stops the script where `asked` code units are over `ceiling`.
fn within_units(asked: u64, ceiling: u64, context: &mut Context) -> JsResult<()> {
    if asked > ceiling {
        return Err(stop(Exceeded::StringUnits { asked, ceiling }, context));
    }
    Ok(())
}

/// Records `exceeded` and answers the error that ends the run.
fn stop(exceeded: Exceeded, context: &Context) -> JsError {
    let sentence = exceeded.sentence();
    State::with(context, |record| {
        record.exceeded.get_or_insert(exceeded);
    });
    // Boa's engine errors are the ones its catch handling passes over, and of the two kinds a
    // host can construct, a panic error is the one that carries a sentence.
    EngineError::Panic(PanicError::new(sentence)).into()
}
