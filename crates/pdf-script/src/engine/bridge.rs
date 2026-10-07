//! The host object model one field's `/K` and `/F` run against (ADR 1591).
//!
//! What is carried: `event` with `value`, `rc`, `willCommit`, `change`, `selStart`, `selEnd` and
//! `target`; the target as a field whose `value` reads the field's own value; `this.getField` of
//! that field; `console.println`; and the `AF*` library, each function a native that hands its
//! arguments to `pdf_model::aform` — the Rust Tier 0 runs, so that a format called from a script
//! and a format that is the whole script write the same characters. Everything else
//! [`crate::surface`] lists is a property whose every read and write throws a `NotAllowedError`.
//!
//! Adobe's reference makes the document `this` in a field script; here the document's members are
//! the global object's, since a script's top-level `this` is that object.

use boa_engine::object::builtins::JsArray;
use boa_engine::object::{FunctionObjectBuilder, ObjectInitializer};
use boa_engine::property::PropertyDescriptor;
use boa_engine::{Context, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::aform::{
    Call, Function, Keyed, Keystroke, Literal, Trigger, extract_nums, make_number, merge_change,
    parse_date,
};

use super::{State, guard, refuse};
use crate::request::byte_offset;
use crate::surface::{EXCLUDED, Holder, NOT_BRIDGED};
use crate::{Outcome, RefusalKind, Request};

/// Installs the guards and the host object model, and answers the event object.
///
/// # Errors
///
/// The engine's, where a property cannot be defined — which, on a context just constructed, it
/// can.
pub(super) fn install(context: &mut Context, request: &Request) -> JsResult<JsObject> {
    guard::install(context)?;
    let target = target(context, request)?;
    let event = event(context, request, &target)?;
    document(context, target, &event)?;
    Ok(event)
}

/// `event.target`: the field, with its `value` and the members it refuses.
fn target(context: &mut Context, request: &Request) -> JsResult<JsObject> {
    let field_value = JsValue::from(JsString::from(request.event.value.as_str()));
    let target = ObjectInitializer::new(context).build();
    let reads = function(
        context,
        "value",
        NativeFunction::from_copy_closure_with_captures(
            |_this, _arguments, value: &JsValue, _context| Ok(value.clone()),
            field_value,
        ),
    );
    let writes = function(
        context,
        "value",
        NativeFunction::from_copy_closure(|_this, _arguments, context| {
            Err(refuse(
                "Field.value".to_owned(),
                RefusalKind::Unreachable(
                    "a keystroke or format script changes its field through event.value, and \
                     this bridge writes no field from inside one (ADR 1591)"
                        .to_owned(),
                ),
                context,
            ))
        }),
    );
    target.define_property_or_throw(
        JsString::from("value"),
        PropertyDescriptor::builder()
            .get(reads)
            .set(writes)
            .enumerable(true)
            .configurable(false)
            .build(),
        context,
    )?;
    refusers(&target, Holder::Field, context)?;
    Ok(target)
}

/// `event`, as Table 199's `/K` or `/F` raises it.
fn event(context: &mut Context, request: &Request, target: &JsObject) -> JsResult<JsObject> {
    let event = ObjectInitializer::new(context).build();
    let event_value = |text: &str| JsValue::from(JsString::from(text));
    data(
        &event,
        "value",
        event_value(&request.event.value),
        true,
        context,
    )?;
    data(
        &event,
        "change",
        event_value(&request.event.change),
        true,
        context,
    )?;
    data(&event, "rc", JsValue::from(true), true, context)?;
    data(
        &event,
        "selStart",
        JsValue::from(request.event.selection_start),
        true,
        context,
    )?;
    data(
        &event,
        "selEnd",
        JsValue::from(request.event.selection_end),
        true,
        context,
    )?;
    data(
        &event,
        "willCommit",
        JsValue::from(request.event.will_commit),
        false,
        context,
    )?;
    data(
        &event,
        "target",
        JsValue::from(target.clone()),
        false,
        context,
    )?;
    refusers(&event, Holder::Event, context)?;
    Ok(event)
}

/// The global object as the document: `event`, `getField`, `app`, `util`, `console`, the library,
/// and every name the surface refuses.
fn document(context: &mut Context, target: JsObject, event: &JsObject) -> JsResult<()> {
    let global = context.global_object();
    data(
        &global,
        "event",
        JsValue::from(event.clone()),
        false,
        context,
    )?;
    let get_field = function(
        context,
        "getField",
        NativeFunction::from_copy_closure_with_captures(
            |_this, arguments, target: &JsValue, context| {
                let asked = arguments
                    .first()
                    .cloned()
                    .unwrap_or_default()
                    .to_string(context)?
                    .to_std_string_lossy();
                let own = State::with(context, |record| record.field.clone()).unwrap_or_default();
                if asked == own {
                    return Ok(target.clone());
                }
                Err(refuse(
                    format!("this.getField({asked:?})"),
                    RefusalKind::Unreachable(format!(
                        "the bridge reaches the event's own field, {own:?}, and no other (ADR \
                         1591)"
                    )),
                    context,
                ))
            },
            JsValue::from(target),
        ),
    );
    data(
        &global,
        "getField",
        JsValue::from(get_field),
        false,
        context,
    )?;
    refusers(&global, Holder::Doc, context)?;
    refusers(&global, Holder::Global, context)?;

    for (name, holder) in [("app", Holder::App), ("util", Holder::Util)] {
        let object = ObjectInitializer::new(context).build();
        refusers(&object, holder, context)?;
        data(&global, name, JsValue::from(object), false, context)?;
    }
    let console = ObjectInitializer::new(context).build();
    let println = function(
        context,
        "println",
        NativeFunction::from_copy_closure(|_this, arguments, context| {
            let line = arguments
                .first()
                .cloned()
                .unwrap_or_default()
                .to_string(context)?
                .to_std_string_lossy();
            State::with(context, |record| record.log(&line));
            Ok(JsValue::undefined())
        }),
    );
    data(&console, "println", JsValue::from(println), false, context)?;
    refusers(&console, Holder::Console, context)?;
    data(&global, "console", JsValue::from(console), false, context)?;

    for library in Function::ALL {
        let native = NativeFunction::from_copy_closure_with_captures(
            move |_this, arguments, event: &JsObject, context| {
                call_library(library, arguments, event, context)
            },
            event.clone(),
        );
        let callable = function(context, library.name(), native);
        data(
            &global,
            library.name(),
            JsValue::from(callable),
            false,
            context,
        )?;
    }
    Ok(())
}

/// Reads the event back into `outcome` after a run that finished.
///
/// The event's properties are data properties that cannot be redefined, so reading them runs no
/// script; a value the script left as an object is not converted, because converting it would.
pub(super) fn read_event(
    event: &JsObject,
    request: &Request,
    context: &mut Context,
    outcome: &mut Outcome,
) {
    // A read of a data property fails only where the object refuses it, which this one cannot; a
    // property that did would be counted as left as it was.
    let read = |name: &str, context: &mut Context| -> Option<JsValue> {
        event.get(JsString::from(name), context).ok()
    };
    outcome.rc = read("rc", context).is_none_or(|rc| rc.to_boolean());
    for (name, was, into) in [
        ("value", &request.event.value, &mut outcome.value),
        ("change", &request.event.change, &mut outcome.change),
    ] {
        let Some(left) = read(name, context) else {
            continue;
        };
        if left.is_object() {
            outcome.refusals.push(crate::Refusal {
                member: format!("event.{name}"),
                kind: RefusalKind::Unreachable(
                    "the script left an object in it, and a field holds text".to_owned(),
                ),
            });
            continue;
        }
        let Ok(text) = left.to_string(context) else {
            continue;
        };
        let text = text.to_std_string_lossy();
        if &text != was {
            *into = Some(text);
        }
    }
}

/// One `AF*` function called from a script: its arguments read as `pdf_model::aform`'s literals and
/// the call run against the event, or a helper answered directly.
fn call_library(
    library: Function,
    arguments: &[JsValue],
    event: &JsObject,
    context: &mut Context,
) -> JsResult<JsValue> {
    if let Some(answer) = helper(library, arguments, event, context)? {
        return Ok(answer);
    }
    let call = Call {
        function: library,
        arguments: literals(library, arguments, context)?,
    };
    let state = EventText::read(event, context)?;
    let refused = |sentence: &str, context: &mut Context| {
        refuse(
            library.name().to_owned(),
            RefusalKind::Library(sentence.to_owned()),
            context,
        )
    };
    match library.trigger() {
        Some(Trigger::Format) => match call.format(&state.value) {
            Ok(formatted) => set(event, "value", &formatted.text, context)?,
            Err(refusal) => return Err(refused(refusal.sentence(), context)),
        },
        Some(Trigger::Keystroke) => match call.keystroke(&state.keystroke()) {
            Ok(Keyed::Accepted { value: Some(value) }) => set(event, "value", &value, context)?,
            Ok(Keyed::Accepted { value: None }) => {}
            Ok(Keyed::Rejected { message }) => {
                reject(event, library, message.as_deref(), context)?;
            }
            Err(refusal) => return Err(refused(refusal.sentence(), context)),
        },
        Some(Trigger::Validate) => match call.validate(&state.value) {
            Ok(Some(message)) => reject(event, library, Some(&message), context)?,
            Ok(None) => {}
            Err(refusal) => return Err(refused(refusal.sentence(), context)),
        },
        Some(Trigger::Calculate) | None => {
            return Err(refuse(
                library.name().to_owned(),
                RefusalKind::Unreachable(
                    "it reads other fields' values, and this bridge reaches the event's own field \
                     only (ADR 1591)"
                        .to_owned(),
                ),
                context,
            ));
        }
    }
    Ok(JsValue::undefined())
}

/// One of the library's four helpers, answered as a value; `None` for every other function.
fn helper(
    library: Function,
    arguments: &[JsValue],
    event: &JsObject,
    context: &mut Context,
) -> JsResult<Option<JsValue>> {
    let text_argument = |index: usize, context: &mut Context| -> JsResult<String> {
        Ok(arguments
            .get(index)
            .cloned()
            .unwrap_or_default()
            .to_string(context)?
            .to_std_string_lossy())
    };
    let answer = match library {
        Function::MakeNumber => {
            let text = text_argument(0, context)?;
            make_number(&text).map_or_else(JsValue::null, JsValue::from)
        }
        Function::ExtractNums => {
            let text = text_argument(0, context)?;
            match extract_nums(&text) {
                Some(numbers) => JsValue::from(JsArray::from_iter(
                    numbers
                        .iter()
                        .map(|number| JsValue::from(JsString::from(number.as_str()))),
                    context,
                )),
                None => JsValue::null(),
            }
        }
        Function::MergeChange => {
            let source = arguments
                .first()
                .and_then(JsValue::as_object)
                .unwrap_or_else(|| event.clone());
            let keystroke = EventText::read(&source, context)?;
            JsValue::from(JsString::from(
                merge_change(&keystroke.keystroke()).as_str(),
            ))
        }
        Function::ParseDateEx => {
            let text = text_argument(0, context)?;
            let picture = text_argument(1, context)?;
            match parse_date(&text, &picture) {
                None => JsValue::null(),
                Some(moment) => {
                    let date = context.intrinsics().constructors().date().constructor();
                    let parts = [
                        f64::from(moment.year),
                        f64::from(moment.month).max(1.0) - 1.0,
                        f64::from(moment.day),
                        f64::from(moment.hour),
                        f64::from(moment.minute),
                        f64::from(moment.second),
                    ]
                    .map(JsValue::from);
                    JsValue::from(date.construct(&parts, None, context)?)
                }
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(answer))
}

/// A call's arguments as the library's literals, the trailing `undefined`s ECMAScript leaves for
/// arguments not passed dropped.
fn literals(
    library: Function,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<Vec<Literal>> {
    let mut literals = Vec::new();
    let last = arguments
        .iter()
        .rposition(|value| !value.is_undefined())
        .map_or(0, |index| index.saturating_add(1));
    for (index, value) in arguments.iter().take(last).enumerate() {
        match literal(value, context)? {
            Some(literal) => literals.push(literal),
            None => {
                return Err(refuse(
                    library.name().to_owned(),
                    RefusalKind::Library(format!(
                        "its argument {} is not a number, a string, a boolean or an array of \
                         strings, which are the values the library reads",
                        index.saturating_add(1)
                    )),
                    context,
                ));
            }
        }
    }
    Ok(literals)
}

/// Sets `event.rc` false, logging the library's message where it gave one.
fn reject(
    event: &JsObject,
    library: Function,
    message: Option<&str>,
    context: &mut Context,
) -> JsResult<()> {
    if let Some(message) = message {
        State::with(context, |record| {
            record.log(&format!("{}: {message}", library.name()));
        });
    }
    event.set(JsString::from("rc"), JsValue::from(false), true, context)?;
    Ok(())
}

/// One argument as the library's literal, `None` where it is no literal the library reads.
fn literal(value: &JsValue, context: &mut Context) -> JsResult<Option<Literal>> {
    if let Some(number) = value.as_number() {
        return Ok(Some(Literal::Number(number)));
    }
    if let Some(text) = value.as_string() {
        return Ok(Some(Literal::String(text.to_std_string_lossy())));
    }
    if let Some(flag) = value.as_boolean() {
        return Ok(Some(Literal::Boolean(flag)));
    }
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    if !object.is_array() {
        return Ok(None);
    }
    let length = object
        .get(JsString::from("length"), context)?
        .to_length(context)?;
    let mut items = Vec::new();
    for index in 0..length {
        let item = object.get(index, context)?;
        let Some(text) = item.as_string() else {
            return Ok(None);
        };
        items.push(text.to_std_string_lossy());
    }
    Ok(Some(Literal::Array(items)))
}

/// The event's text properties as the library reads them.
struct EventText {
    /// `event.value`.
    value: String,
    /// `event.change`.
    change: String,
    /// `event.selStart` and `event.selEnd`, as byte offsets into `value`.
    selection: (usize, usize),
    /// `event.willCommit`.
    will_commit: bool,
}

impl EventText {
    /// Reads them from an event object.
    fn read(event: &JsObject, context: &mut Context) -> JsResult<Self> {
        let text = |name: &str, context: &mut Context| -> JsResult<String> {
            Ok(event
                .get(JsString::from(name), context)?
                .to_string(context)?
                .to_std_string_lossy())
        };
        let value = text("value", context)?;
        let change = text("change", context)?;
        let start = event
            .get(JsString::from("selStart"), context)?
            .to_u32(context)?;
        let end = event
            .get(JsString::from("selEnd"), context)?
            .to_u32(context)?;
        let will_commit = event
            .get(JsString::from("willCommit"), context)?
            .to_boolean();
        let selection = (byte_offset(&value, start), byte_offset(&value, end));
        Ok(Self {
            value,
            change,
            selection,
            will_commit,
        })
    }

    /// The library's keystroke over these.
    fn keystroke(&self) -> Keystroke<'_> {
        Keystroke {
            value: &self.value,
            change: &self.change,
            selection: self.selection,
            will_commit: self.will_commit,
        }
    }
}

/// Defines `holder[name]` as a data property that cannot be redefined, writable where `writable`.
fn data(
    holder: &JsObject,
    name: &str,
    value: JsValue,
    writable: bool,
    context: &mut Context,
) -> JsResult<()> {
    holder.define_property_or_throw(
        JsString::from(name),
        PropertyDescriptor::builder()
            .value(value)
            .writable(writable)
            .enumerable(true)
            .configurable(false)
            .build(),
        context,
    )?;
    Ok(())
}

/// Writes a string into one of the event's properties.
fn set(event: &JsObject, name: &str, text: &str, context: &mut Context) -> JsResult<()> {
    event.set(
        JsString::from(name),
        JsValue::from(JsString::from(text)),
        true,
        context,
    )?;
    Ok(())
}

/// A function object named `name`.
fn function(context: &Context, name: &str, native: NativeFunction) -> JsObject {
    FunctionObjectBuilder::new(context.realm(), native)
        .name(JsString::from(name))
        .build()
        .into()
}

/// Defines every member [`crate::surface`] lists for `holder` on `object`, each an accessor that
/// throws on read and on write.
fn refusers(object: &JsObject, holder: Holder, context: &mut Context) -> JsResult<()> {
    let prefix = holder.prefix();
    for row in EXCLUDED.iter().filter(|row| row.holder == holder) {
        for member in row.members {
            refuser(object, prefix, member, Some(row.reason), context)?;
        }
    }
    for (_, members) in NOT_BRIDGED.iter().filter(|(listed, _)| *listed == holder) {
        for member in *members {
            refuser(object, prefix, member, None, context)?;
        }
    }
    Ok(())
}

/// One refused member: a getter and a setter that each throw its `NotAllowedError`.
fn refuser(
    object: &JsObject,
    prefix: &'static str,
    member: &'static str,
    reason: Option<&'static str>,
    context: &mut Context,
) -> JsResult<()> {
    let throws = move |_this: &JsValue, _arguments: &[JsValue], context: &mut Context| {
        let kind = reason.map_or(RefusalKind::NotBridged, |reason| {
            RefusalKind::Excluded(reason.to_owned())
        });
        Err(refuse(format!("{prefix}{member}"), kind, context))
    };
    let getter = function(context, member, NativeFunction::from_copy_closure(throws));
    let setter = function(context, member, NativeFunction::from_copy_closure(throws));
    object.define_property_or_throw(
        JsString::from(member),
        PropertyDescriptor::builder()
            .get(getter)
            .set(setter)
            .enumerable(false)
            .configurable(false)
            .build(),
        context,
    )?;
    Ok(())
}
