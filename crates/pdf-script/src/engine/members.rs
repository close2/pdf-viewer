//! The members the census found refused and RFC 0008 section 4.2 admits, carried (ADRs 1626,
//! 1627): `global`, `this.dirty`, `this.info`, `this.getOCGs` and the `OCG` object, `util.printf`,
//! a button's `buttonGetCaption` and `buttonSetCaption`, a check box's `isBoxChecked` and
//! `checkThisBox` (ADR 1689), the three kinds of call that need a host — `app.alert` and
//! `app.response`, the four timer methods, and `app.beep` (ADR 1702). `event.commitKey`, `fieldFull` and `changeEx` are the event's own
//! and are installed with it (`bridge::begin`).
//!
//! Each member's meaning is Adobe's *JavaScript for Acrobat API Reference* — "Doc properties",
//! "Doc methods", "OCG", "util methods", "Field methods", "app methods", "global" — cited and never
//! quoted, a documented choice each under principle 5; where it is a standard's entry instead —
//! Table 349's information dictionary, §8.11's groups, Table 192's captions — the standard is
//! named.

use std::time::Instant;

use boa_engine::object::builtins::{JsArray, JsRegExp};
use boa_engine::object::{IntegrityLevel, ObjectInitializer};
use boa_engine::property::PropertyDescriptor;
use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsString, JsSymbol, JsValue, NativeFunction,
};
use pdf_model::aform::printf::{Argument, printf};
use pdf_model::view::{Face, FieldState, FieldType, Layer, Property, ScriptEdit, Sound};

use super::bridge::{
    accessor, data, field_name, function, integral, refusers, terminals, text_argument, widget_of,
    write_value,
};
use super::{State, guard, refuse};
use crate::surface::Holder;
use crate::{Answer, Buttons, Icon, Question, RefusalKind};

/// Installs the document's members: `global`, `dirty`, `info` and `getOCGs`.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn document(global: &JsObject, context: &mut Context) -> JsResult<()> {
    // `global`: an ordinary object of the document's own, for the realm's lifetime — in memory
    // and per document, as RFC 0008 section 4.2 admits it. What a document stores there is gone
    // when its realm is, and reaches no other document; the two members that would carry it
    // further are refused.
    let store = ObjectInitializer::new(context).build();
    refusers(&store, Holder::Store, context)?;
    data(global, "global", JsValue::from(store), false, context)?;

    let getter = function(context, "dirty", NativeFunction::from_fn_ptr(read_dirty));
    let setter = function(context, "dirty", NativeFunction::from_fn_ptr(write_dirty));
    accessor(global, "dirty", getter, setter, context)?;

    let getter = function(context, "info", NativeFunction::from_fn_ptr(read_info));
    let setter = function(
        context,
        "info",
        NativeFunction::from_copy_closure(|_this, _arguments, context| {
            Err(read_only("this.info=", context))
        }),
    );
    accessor(global, "info", getter, setter, context)?;

    let get_ocgs = function(context, "getOCGs", NativeFunction::from_fn_ptr(get_ocgs));
    data(global, "getOCGs", JsValue::from(get_ocgs), false, context)?;
    Ok(())
}

/// Installs `app.alert` and `app.response`, the four timer methods and `app.beep` (ADR 1702).
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn app(app: &JsObject, context: &mut Context) -> JsResult<()> {
    let alert = function(context, "alert", NativeFunction::from_fn_ptr(alert));
    data(app, "alert", JsValue::from(alert), false, context)?;
    let response = function(context, "response", NativeFunction::from_fn_ptr(response));
    data(app, "response", JsValue::from(response), false, context)?;
    let methods: [(&str, NativeFunction); 5] = [
        ("setInterval", NativeFunction::from_fn_ptr(set_interval)),
        ("setTimeOut", NativeFunction::from_fn_ptr(set_time_out)),
        ("clearInterval", NativeFunction::from_fn_ptr(clear_timer)),
        ("clearTimeOut", NativeFunction::from_fn_ptr(clear_timer)),
        ("beep", NativeFunction::from_fn_ptr(beep)),
    ];
    for (name, native) in methods {
        let method = function(context, name, native);
        data(app, name, JsValue::from(method), false, context)?;
    }
    Ok(())
}

/// `app.setInterval(cExpr, nMilliseconds)`: an interval object, its expression run every period.
fn set_interval(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    set_timer(arguments, true, context)
}

/// `app.setTimeOut(cExpr, nMilliseconds)`: a timeout object, its expression run once.
fn set_time_out(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    set_timer(arguments, false, context)
}

/// The timer both methods set: recorded as [`ScriptEdit::Timer`] for the view state, which counts
/// it down on the host's ticks and runs the expression as a script of its own (ADR 1702).
///
/// Adobe's "app methods" page gives `cExpr` as the script and `nMilliseconds` as the period, and
/// answers an object that the matching `clear` method stops. Each of the rest is a documented
/// choice: the expression is the argument's `ToString`, the period its `ToNumber` truncated toward
/// zero and held to a `u32` (the view state raises it to its floor), and the object is an ordinary
/// one a script may add its own properties to — the reference's own example keeps a count on it —
/// carrying its number under a symbol no script can name.
fn set_timer(arguments: &[JsValue], repeat: bool, context: &mut Context) -> JsResult<JsValue> {
    let values = named(arguments, &["cExpr", "nMilliseconds"], context)?;
    let value = |index: usize| values.get(index).cloned().unwrap_or_default();
    let script = value(0).to_string(context)?.to_std_string_lossy();
    let asked = value(1).to_number(context)?;
    let period = if asked.is_finite() {
        u32::try_from(integral(asked).clamp(0, i64::from(u32::MAX))).unwrap_or(u32::MAX)
    } else {
        0
    };
    let held = State::table(context, |table| table.timer_key.clone()).flatten();
    let key = if let Some(key) = held {
        key
    } else {
        let key = JsSymbol::new(Some(JsString::from("timer"))).ok_or_else(|| {
            JsNativeError::range().with_message("the engine has no symbol left to name a timer by")
        })?;
        State::table(context, |table| table.timer_key = Some(key.clone()));
        key
    };
    let id = State::table(context, |table| {
        let id = table.next_timer;
        table.next_timer = id.wrapping_add(1);
        id
    })
    .ok_or_else(|| JsNativeError::error().with_message("the realm holds no document"))?;
    let object = ObjectInitializer::new(context).build();
    object.define_property_or_throw(
        key,
        PropertyDescriptor::builder()
            .value(JsValue::from(id))
            .writable(false)
            .enumerable(false)
            .configurable(false),
        context,
    )?;
    State::with(context, |record| {
        if record.edits.len() < super::MAX_EDITS {
            record.edits.push(ScriptEdit::Timer {
                id,
                script,
                period,
                repeat,
            });
        }
    });
    Ok(JsValue::from(object))
}

/// `app.clearInterval(oInterval)` and `app.clearTimeOut(oTime)`: the timer the object names stops,
/// recorded as [`ScriptEdit::ClearTimer`] (ADR 1702).
///
/// The reference's own example guards the call with `try`, because a stop pressed before the start
/// passes `undefined`; so a value that is no timer object throws a `TypeError`, and one of either
/// kind stops through either method — the reference names the two apart and states no difference
/// in what they do.
fn clear_timer(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let key = State::table(context, |table| table.timer_key.clone()).flatten();
    let held = arguments.first().and_then(JsValue::as_object);
    let id = match (key, held) {
        (Some(key), Some(object)) => object.get(key, context)?.as_number(),
        _ => None,
    };
    let Some(id) = id else {
        return Err(JsNativeError::typ()
            .with_message("the argument is not an interval or timeout object app.setInterval or app.setTimeOut answered")
            .into());
    };
    let id = u32::try_from(integral(id)).unwrap_or(u32::MAX);
    State::with(context, |record| {
        if record.edits.len() < super::MAX_EDITS {
            record.edits.push(ScriptEdit::ClearTimer { id });
        }
    });
    Ok(JsValue::undefined())
}

/// `app.beep(nType)`: one of the reference's five sounds, recorded as [`ScriptEdit::Beep`] for the
/// host to play or to say it cannot (ADR 1702). A number that names none is the default, as an
/// absent one is.
fn beep(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let values = named(arguments, &["nType"], context)?;
    let sound = numbered(
        &values.first().cloned().unwrap_or_default(),
        &Sound::ALL,
        context,
    )?;
    State::with(context, |record| {
        if record.edits.len() < super::MAX_EDITS {
            record.edits.push(ScriptEdit::Beep { sound });
        }
    });
    Ok(JsValue::undefined())
}

/// Installs `util.printf`.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn util(util: &JsObject, context: &mut Context) -> JsResult<()> {
    let printf = function(context, "printf", NativeFunction::from_fn_ptr(print_format));
    data(util, "printf", JsValue::from(printf), false, context)
}

/// Installs `AFExactMatch`, the one function of Adobe's form library whose argument is a pattern
/// rather than a literal, so that it is the engine's and never Tier 0's (ADR 1652).
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn library(global: &JsObject, context: &mut Context) -> JsResult<()> {
    let exact = function(
        context,
        "AFExactMatch",
        NativeFunction::from_fn_ptr(exact_match),
    );
    data(global, "AFExactMatch", JsValue::from(exact), false, context)
}

/// The most patterns `AFExactMatch` reads of an array, the bound `this.resetForm` reads a list to.
const MAX_PATTERNS: u64 = u16::MAX as u64;

/// `AFExactMatch(rePatterns, cString)`: the one-based position of the first pattern that matches
/// the whole string, or 0 where none does.
///
/// Adobe publishes nothing of this function: not the *JavaScript for Acrobat API Reference*, nor
/// the *Interapplication Communication* guide's argument menus, nor any other page of
/// `adobe/dc-acrobat-sdk-docs` at `ab3b42a7` names it (searched whole). What the tree takes is its
/// name and the convention its neighbours keep, each a documented choice (ADR 1652): *exact* is a
/// match of the whole string — the text `String.prototype.match` finds first is the string itself;
/// a list answers a position counted from one, so that 0 is the answer that is false; and one
/// pattern is a list of one, so it answers 1 or 0, which a script's `if` reads as the same truth.
fn exact_match(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let patterns = arguments.first().cloned().unwrap_or_default();
    let text = arguments
        .get(1)
        .cloned()
        .unwrap_or_default()
        .to_string(context)?;
    let listed: Vec<JsValue> = match patterns.as_object() {
        Some(list) if JsRegExp::from_object(list.clone()).is_err() && list.is_array() => {
            let length = list
                .get(JsString::from("length"), context)?
                .to_length(context)?;
            let mut listed = Vec::new();
            for index in 0..length.min(MAX_PATTERNS) {
                listed.push(list.get(index, context)?);
            }
            listed
        }
        _ => vec![patterns],
    };
    let matcher = context
        .intrinsics()
        .constructors()
        .string()
        .prototype()
        .get(JsString::from("match"), context)?;
    let Some(matcher) = matcher.as_callable() else {
        return Err(JsNativeError::typ()
            .with_message("String.prototype.match is not a function")
            .into());
    };
    let subject = JsValue::from(text.clone());
    for (position, pattern) in (1_u32..).zip(listed) {
        let found = matcher.call(&subject, &[pattern], context)?;
        let Some(found) = found.as_object() else {
            continue;
        };
        if found.get(0, context)?.as_string().as_ref() == Some(&text) {
            return Ok(JsValue::from(position));
        }
    }
    Ok(JsValue::from(0))
}

/// Installs a field's `buttonGetCaption` and `buttonSetCaption`.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn field(prototype: &JsObject, context: &mut Context) -> JsResult<()> {
    let get = function(
        context,
        "buttonGetCaption",
        NativeFunction::from_fn_ptr(get_caption),
    );
    data(
        prototype,
        "buttonGetCaption",
        JsValue::from(get),
        false,
        context,
    )?;
    let set = function(
        context,
        "buttonSetCaption",
        NativeFunction::from_fn_ptr(set_caption),
    );
    data(
        prototype,
        "buttonSetCaption",
        JsValue::from(set),
        false,
        context,
    )?;
    let checked = function(
        context,
        "isBoxChecked",
        NativeFunction::from_fn_ptr(is_box_checked),
    );
    data(
        prototype,
        "isBoxChecked",
        JsValue::from(checked),
        false,
        context,
    )?;
    let check = function(
        context,
        "checkThisBox",
        NativeFunction::from_fn_ptr(check_this_box),
    );
    data(
        prototype,
        "checkThisBox",
        JsValue::from(check),
        false,
        context,
    )
}

/// A write to a member the reference makes read-only here, refused by name.
fn read_only(member: &str, context: &mut Context) -> boa_engine::JsError {
    refuse(
        member.to_owned(),
        RefusalKind::Unreachable(
            "a document's script reads it and does not write it, as the reference has it in a \
             reader (ADR 1626)"
                .to_owned(),
        ),
        context,
    )
}

/// `this.dirty`: whether the view state holds work no save has written, or what a script set it to
/// in this run.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function's signature is the engine's, and every native answers a result"
)]
fn read_dirty(_this: &JsValue, _arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let written = State::with(context, |record| record.dirty_written).flatten();
    let dirty = written
        .or_else(|| State::table(context, |table| table.dirty))
        .unwrap_or(false);
    Ok(JsValue::from(dirty))
}

/// `this.dirty = …`: what the script reads back until the run ends, and nothing else.
///
/// **A document does not decide whether its reader's work is saved** (ADR 1626). The reference
/// lets a script clear the mark so that a viewer does not ask to save; a reader's unsaved typing
/// that a document could declare saved is typing a close could lose, so the host's own mark is
/// never moved by a script. What a script wrote it reads back — the reference's example saves the
/// mark, changes a field and puts the mark back — and the run says so.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function's signature is the engine's, and every native answers a result"
)]
fn write_dirty(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let value = arguments.first().is_some_and(JsValue::to_boolean);
    State::with(context, |record| {
        if record.dirty_written.is_none() {
            record.note(&format!(
                "the script set this.dirty to {value}; it reads that back for the rest of the run, \
                 and whether the reader's work is saved stays the reader's to know (ADR 1626)"
            ));
        }
        record.dirty_written = Some(value);
    });
    Ok(JsValue::undefined())
}

/// Table 349's entries the reference calls standard, which it reads whatever their case.
const STANDARD_INFO: [&str; 9] = [
    "Title",
    "Author",
    "Subject",
    "Keywords",
    "Creator",
    "Producer",
    "CreationDate",
    "ModDate",
    "Trapped",
];

/// `this.info`: the document information dictionary as an object, read-only.
///
/// Every entry of Table 349's dictionary the view state read, under its key; the reference's nine
/// standard ones under their lower-case spelling as well, since it reads them in either case. The
/// two dates are `Date`s where §7.9.4 parses them, as the reference answers them. Each property
/// refuses a write by name, and the object takes no new one — the reference's reader does not
/// write the dictionary, and neither does a script here (ADR 1626).
fn read_info(_this: &JsValue, _arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let entries = State::table(context, |table| table.document.info.clone()).unwrap_or_default();
    let info = ObjectInitializer::new(context).build();
    for entry in entries {
        let value = match entry.moment {
            Some(moment) => {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a moment in milliseconds is exact in a double for every date a \
                              §7.9.4 string can write"
                )]
                let millis = moment as f64;
                let date = context.intrinsics().constructors().date().constructor();
                JsValue::from(date.construct(&[JsValue::from(millis)], None, context)?)
            }
            None => JsValue::from(JsString::from(entry.text.as_str())),
        };
        let mut names = vec![entry.key.clone()];
        if STANDARD_INFO.contains(&entry.key.as_str()) {
            names.push(entry.key.to_lowercase());
        }
        for name in names {
            if info.has_own_property(JsString::from(name.as_str()), context)? {
                continue;
            }
            let getter = function(
                context,
                &name,
                NativeFunction::from_copy_closure_with_captures(
                    |_this, _arguments, held: &JsValue, _context| Ok(held.clone()),
                    value.clone(),
                ),
            );
            let member = JsString::from(format!("this.info.{name}=").as_str());
            let setter = function(
                context,
                &name,
                NativeFunction::from_copy_closure_with_captures(
                    |_this, _arguments, member: &JsString, context| {
                        Err(read_only(&member.to_std_string_lossy(), context))
                    },
                    member,
                ),
            );
            accessor(&info, &name, getter, setter, context)?;
        }
    }
    info.set_integrity_level(IntegrityLevel::Sealed, context)?;
    Ok(JsValue::from(info))
}

/// `this.getOCGs(nPage)`: every group a person's layer switch can change, as `OCG` objects in
/// the alphabetical order of their names the reference gives the call without a page; `null` where
/// there is none.
///
/// With a page, the reference answers the groups that page's content uses — a walk of that page's
/// resources, which a realm is not handed — and the call is refused by name.
fn get_ocgs(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    if arguments.first().is_some_and(|page| !page.is_undefined()) {
        return Err(refuse(
            "this.getOCGs(nPage)".to_owned(),
            RefusalKind::Unreachable(
                "which groups one page's content uses is read from that page's resources, which \
                 a document's realm is not handed; getOCGs() with no page lists every group"
                    .to_owned(),
            ),
            context,
        ));
    }
    let mut layers =
        State::table(context, |table| table.document.layers.clone()).unwrap_or_default();
    if layers.is_empty() {
        return Ok(JsValue::null());
    }
    layers.sort_by(|left, right| left.name.cmp(&right.name));
    let mut objects = Vec::new();
    for layer in &layers {
        objects.push(JsValue::from(ocg(layer, context)?));
    }
    Ok(JsValue::from(JsArray::from_iter(objects, context)))
}

/// One `OCG` object: `name`, `state`, `initState`, `locked` and `constants`, and the methods the
/// surface lists refused.
fn ocg(layer: &Layer, context: &mut Context) -> JsResult<JsObject> {
    let object = ObjectInitializer::new(context).build();
    let id = (layer.number, layer.generation);
    for (name, read) in [
        ("name", Read::Name),
        ("state", Read::On),
        ("initState", Read::Initially),
        ("locked", Read::Locked),
    ] {
        let getter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, _arguments, context| {
                let layer = State::table(context, |table| {
                    table
                        .document
                        .layers
                        .iter()
                        .find(|held| (held.number, held.generation) == id)
                        .cloned()
                })
                .flatten();
                Ok(layer.map_or_else(JsValue::undefined, |layer| match read {
                    Read::Name => JsValue::from(JsString::from(layer.name.as_str())),
                    Read::On => JsValue::from(layer.on),
                    Read::Initially => JsValue::from(layer.initially_on),
                    Read::Locked => JsValue::from(layer.locked),
                }))
            }),
        );
        let setter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, arguments, context| {
                if read != Read::On {
                    return Err(read_only(&format!("OCG.{name}="), context));
                }
                let on = arguments.first().is_some_and(JsValue::to_boolean);
                switch(id, on, context);
                Ok(JsValue::undefined())
            }),
        );
        accessor(&object, name, getter, setter, context)?;
    }
    let states = ObjectInitializer::new(context)
        .property(
            JsString::from("on"),
            true,
            boa_engine::property::Attribute::READONLY,
        )
        .property(
            JsString::from("off"),
            false,
            boa_engine::property::Attribute::READONLY,
        )
        .build();
    let intents = ObjectInitializer::new(context)
        .property(
            JsString::from("design"),
            JsString::from("Design"),
            boa_engine::property::Attribute::READONLY,
        )
        .property(
            JsString::from("view"),
            JsString::from("View"),
            boa_engine::property::Attribute::READONLY,
        )
        .build();
    let constants = ObjectInitializer::new(context)
        .property(
            JsString::from("states"),
            states,
            boa_engine::property::Attribute::READONLY,
        )
        .property(
            JsString::from("intents"),
            intents,
            boa_engine::property::Attribute::READONLY,
        )
        .build();
    data(
        &object,
        "constants",
        JsValue::from(constants),
        false,
        context,
    )?;
    refusers(&object, Holder::Layer, context)?;
    Ok(object)
}

/// Which of an `OCG`'s properties an accessor reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    /// `name`.
    Name,
    /// `state`.
    On,
    /// `initState`.
    Initially,
    /// `locked`.
    Locked,
}

/// `OCG.state = on`: the group switched in the realm's record, the switch handed to the view state
/// as an edit, which makes it as a person's layer switch does (ADR 1626).
fn switch(id: (u32, u16), on: bool, context: &mut Context) {
    let switched = State::table(context, |table| {
        let before = table.document.layers.clone();
        let layer = table
            .document
            .layers
            .iter_mut()
            .find(|held| (held.number, held.generation) == id)?;
        layer.on = on;
        Some(before)
    })
    .flatten();
    let Some(before) = switched else {
        return;
    };
    State::with(context, |record| {
        record.layers_before.get_or_insert(before);
    });
    State::note(
        context,
        ScriptEdit::Layer {
            number: id.0,
            generation: id.1,
            on,
        },
    );
}

/// `util.printf(cFormat, …)`: `pdf_model::aform::printf::printf`, the library's own number
/// writers (ADR 1626).
fn print_format(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let format = text_argument(arguments, 0, context)?;
    let mut values = Vec::new();
    let mut units = u64::try_from(format.encode_utf16().count()).unwrap_or(u64::MAX);
    for argument in arguments.iter().skip(1) {
        let value = match argument.as_number() {
            Some(number) => Argument::Number(number),
            None => Argument::Text(argument.to_string(context)?.to_std_string_lossy()),
        };
        if let Argument::Text(text) = &value {
            units = units.saturating_add(u64::try_from(text.len()).unwrap_or(u64::MAX));
        }
        values.push(value);
    }
    // Each conversion writes at most its width or its argument, so the output is held to the
    // string budget before it is built.
    let conversions = u64::try_from(format.matches('%').count()).unwrap_or(u64::MAX);
    let width = u64::try_from(pdf_model::aform::printf::MAX_WIDTH).unwrap_or(u64::MAX);
    guard::string_units(
        units.saturating_add(conversions.saturating_mul(width)),
        context,
    )?;
    match printf(&format, &values) {
        Ok(written) => Ok(JsValue::from(JsString::from(written.as_str()))),
        Err(refusal) => Err(refuse(
            "util.printf".to_owned(),
            RefusalKind::Library(refusal.sentence().to_owned()),
            context,
        )),
    }
}

/// The face `nFace` names, `Normal` where it is not passed; a number naming none is refused.
fn face_of(
    arguments: &[JsValue],
    index: usize,
    member: &str,
    context: &mut Context,
) -> JsResult<Face> {
    let Some(value) = arguments.get(index).filter(|value| !value.is_undefined()) else {
        return Ok(Face::Normal);
    };
    let number = value.to_number(context)?;
    Face::from_number(number).ok_or_else(|| {
        refuse(
            member.to_owned(),
            RefusalKind::Unreachable(
                "nFace is 0 for the normal caption, 1 for the down caption and 2 for the rollover \
                 caption"
                    .to_owned(),
            ),
            context,
        )
    })
}

/// The first terminal field a `Field` stands for, as the realm holds it.
fn first_state(this: &JsValue, context: &mut Context) -> JsResult<Option<FieldState>> {
    let name = field_name(this, context)?;
    let Some(first) = terminals(context, &name).into_iter().next() else {
        return Ok(None);
    };
    Ok(State::table(context, |table| table.fields.get(&first).cloned()).flatten())
}

/// `field.buttonGetCaption(nFace)`: Table 192's `/CA`, `/AC` or `/RC` of the widget the `Field`
/// stands for — its first, for a `Field` of every widget — empty where it states none.
fn get_caption(this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let face = face_of(arguments, 0, "Field.buttonGetCaption", context)?;
    let widget = widget_of(this, context)?;
    let caption = first_state(this, context)?
        .and_then(|state| {
            state
                .widget(widget)
                .and_then(|shown| shown.captions.get(face.index()).cloned())
        })
        .unwrap_or_default();
    Ok(JsValue::from(JsString::from(caption.as_str())))
}

/// `field.buttonSetCaption(cCaption, nFace)`: the caption as an edit of Table 192's entry, drawn
/// and saved through ADR 1617's route; refused for a field that is not a button, whose widget
/// Table 192 gives no caption.
fn set_caption(this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let caption = text_argument(arguments, 0, context)?;
    let face = face_of(arguments, 1, "Field.buttonSetCaption", context)?;
    let name = field_name(this, context)?;
    let widget = widget_of(this, context)?;
    for field in terminals(context, &name) {
        let kind = State::table(context, |table| {
            table.fields.get(&field).map(|state| state.kind)
        })
        .flatten();
        let button = matches!(
            kind,
            Some(FieldType::PushButton | FieldType::CheckBox | FieldType::RadioButton)
        );
        // Table 192: "the CA entry may be used with any type of button field", and the other two
        // are a push-button's only.
        let admitted = button && (face == Face::Normal || kind == Some(FieldType::PushButton));
        if !admitted {
            return Err(refuse(
                "Field.buttonSetCaption".to_owned(),
                RefusalKind::Unreachable(format!(
                    "{field} is not a field Table 192 gives the {} caption to: /CA is any \
                     button's, and /AC and /RC a push-button's",
                    face.key()
                )),
                context,
            ));
        }
        let property = Property::Caption(face, caption.clone());
        let edit = ScriptEdit::Property {
            field: field.clone(),
            widget,
            property: property.clone(),
        };
        State::edit(context, &field, edit, move |state| {
            state.apply(widget, &property);
        });
    }
    Ok(JsValue::undefined())
}

/// `field.isBoxChecked(nWidget)`: whether widget `nWidget` of a check box or radio button is on —
/// whether the field's value is the name that widget's §12.7.5.2.3 on state is selected by.
///
/// The reference's "Field methods" page counts `nWidget` from zero; the count here is the field
/// table's, as `getField("name.N")`'s is (ADR 1664). Widgets that share an on state are checked
/// together, which is the page's own note on radio buttons sharing an export value. A widget the
/// field does not have, or one of a field that does not toggle, is not checked (ADR 1689).
fn is_box_checked(
    this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let index = widget_argument(arguments, "Field.isBoxChecked", context)?;
    let checked = first_state(this, context)?.is_some_and(|state| {
        state
            .widgets
            .get(index)
            .and_then(|widget| widget.on_state.as_deref())
            .is_some_and(|on| on == state.value)
    });
    Ok(JsValue::from(checked))
}

/// `field.checkThisBox(nWidget, bCheckIt)`: widget `nWidget` of a check box or radio button
/// turned on, or a check box's turned off, as the value its on state is selected by — the same
/// edit `field.value` makes, so the view state writes §12.7.5.2.3's `/V` and `/AS` as for a
/// person's click (ADR 1689).
///
/// `bCheckIt` is true where it is not passed. The reference's "Field methods" page lets only a
/// check box be unchecked this way, so `false` on a radio button changes nothing; and unchecking a
/// widget that is not the one on changes nothing either. A field that does not toggle, and a widget
/// with no on state to set, are refused by name.
fn check_this_box(
    this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let index = widget_argument(arguments, "Field.checkThisBox", context)?;
    let check = arguments
        .get(1)
        .filter(|value| !value.is_undefined())
        .is_none_or(JsValue::to_boolean);
    let Some(state) = first_state(this, context)? else {
        return Ok(JsValue::undefined());
    };
    let toggles = matches!(state.kind, FieldType::CheckBox | FieldType::RadioButton);
    let Some(on) = state
        .widgets
        .get(index)
        .and_then(|widget| widget.on_state.clone())
        .filter(|_| toggles)
    else {
        return Err(refuse(
            "Field.checkThisBox".to_owned(),
            RefusalKind::Unreachable(format!(
                "{} has no check box or radio button widget {index} with an on state to set",
                state.name
            )),
            context,
        ));
    };
    let value = if check {
        on
    } else if state.kind == FieldType::CheckBox && state.value == on {
        "Off".to_owned()
    } else {
        return Ok(JsValue::undefined());
    };
    write_value(
        &state.name,
        &JsValue::from(JsString::from(value.as_str())),
        context,
    )?;
    Ok(JsValue::undefined())
}

/// The zero-based widget a check box method's first argument names; a negative or absent one is
/// a `RangeError`, since the reference makes `nWidget` required.
fn widget_argument(arguments: &[JsValue], member: &str, context: &mut Context) -> JsResult<usize> {
    let number = arguments
        .first()
        .cloned()
        .unwrap_or_default()
        .to_number(context)?;
    if !number.is_finite() || number < 0.0 {
        return Err(JsNativeError::range()
            .with_message(format!("{member}: nWidget is a widget's index from zero"))
            .into());
    }
    Ok(usize::try_from(integral(number)).unwrap_or(usize::MAX))
}

/// The arguments of a call the reference lets a script pass positionally or as one object of
/// named properties: the object's properties where the first argument is an object, the
/// positions otherwise.
pub(super) fn named(
    arguments: &[JsValue],
    names: &[&str],
    context: &mut Context,
) -> JsResult<Vec<JsValue>> {
    let first = arguments.first().and_then(JsValue::as_object);
    match first {
        Some(object) if !object.is_callable() && arguments.len() == 1 => names
            .iter()
            .map(|name| object.get(JsString::from(*name), context))
            .collect(),
        _ => Ok(names
            .iter()
            .enumerate()
            .map(|(index, _)| arguments.get(index).cloned().unwrap_or_default())
            .collect()),
    }
}

/// A text argument that may be absent: `None` for `undefined` and `null`.
fn optional_text(value: &JsValue, context: &mut Context) -> JsResult<Option<String>> {
    if value.is_null_or_undefined() {
        return Ok(None);
    }
    Ok(Some(value.to_string(context)?.to_std_string_lossy()))
}

/// One of the reference's small numbered choices, the default where it is not passed or names
/// none.
fn numbered<T: Copy + Default>(value: &JsValue, all: &[T], context: &mut Context) -> JsResult<T> {
    if value.is_undefined() {
        return Ok(T::default());
    }
    let number = value.to_number(context)?;
    Ok(usize::try_from(integral(number))
        .ok()
        .and_then(|index| all.get(index).copied())
        .unwrap_or_default())
}

/// `app.alert(cMsg, nIcon, nType, cTitle, oDoc, oCheckbox)`: the message put to the person, and
/// the number of the button pressed returned (ADR 1627).
///
/// A check box the script asks for is not drawn; its `bAfterValue` is set to its `bInitialValue`,
/// the state nobody changed. `oDoc` names a document, and a script reaches only its own.
fn alert(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let values = named(
        arguments,
        &["cMsg", "nIcon", "nType", "cTitle", "oDoc", "oCheckbox"],
        context,
    )?;
    let value = |index: usize| values.get(index).cloned().unwrap_or_default();
    let message = value(0).to_string(context)?.to_std_string_lossy();
    let icon = numbered(&value(1), &Icon::ALL, context)?;
    let buttons = numbered(&value(2), &Buttons::ALL, context)?;
    let title = optional_text(&value(3), context)?;
    if let Some(checkbox) = value(5).as_object() {
        let initial = checkbox
            .get(JsString::from("bInitialValue"), context)?
            .to_boolean();
        checkbox.set(
            JsString::from("bAfterValue"),
            JsValue::from(initial),
            false,
            context,
        )?;
    }
    let question = Question::Alert {
        message,
        icon,
        buttons,
        title,
    };
    let button = match put(&question, context) {
        Answer::Pressed(button) if buttons.offers(button) => button,
        _ => buttons.dismissed(),
    };
    Ok(JsValue::from(button.returned()))
}

/// `app.response(cQuestion, cTitle, cDefault, bPassword, cLabel)`: the text typed, or `null` where
/// the person cancelled (ADR 1627).
fn response(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let values = named(
        arguments,
        &["cQuestion", "cTitle", "cDefault", "bPassword", "cLabel"],
        context,
    )?;
    let value = |index: usize| values.get(index).cloned().unwrap_or_default();
    let question = value(0).to_string(context)?.to_std_string_lossy();
    let title = optional_text(&value(1), context)?;
    let default = optional_text(&value(2), context)?.unwrap_or_default();
    let password = value(3).to_boolean();
    let label = optional_text(&value(4), context)?;
    // What a person types is held to the string budget like any other string a script is handed.
    let question = Question::Response {
        question,
        title,
        default,
        label,
        password,
    };
    match put(&question, context) {
        Answer::Typed(Some(text)) => {
            let units = u64::try_from(text.encode_utf16().count()).unwrap_or(u64::MAX);
            guard::string_units(units, context)?;
            Ok(JsValue::from(JsString::from(text.as_str())))
        }
        _ => Ok(JsValue::null()),
    }
}

/// Puts one question to the realm's asker, or — past the run's first — answers it as a closed
/// dialogue answers and counts it (RFC 0008 section 6.8); either way the run says what was asked
/// and what came back.
fn put(question: &Question, context: &mut Context) -> Answer {
    let first = State::with(context, |record| {
        if record.asked_once {
            record.unasked = record.unasked.saturating_add(1);
            false
        } else {
            record.asked_once = true;
            true
        }
    })
    .unwrap_or(false);
    if !first {
        return question.dismissed();
    }
    let Some(asker) = State::asker(context) else {
        return question.dismissed();
    };
    let started = Instant::now();
    let answer = asker.ask(question);
    let waited = started.elapsed();
    State::waited(context, waited);
    let password = matches!(question, Question::Response { password: true, .. });
    let said = match &answer {
        Answer::Pressed(button) => format!("was answered with button {}", button.returned()),
        Answer::Typed(Some(_)) if password => "was answered with a password".to_owned(),
        Answer::Typed(Some(text)) => format!("was answered {text:?}"),
        Answer::Typed(None) => "was cancelled".to_owned(),
        Answer::Unanswerable => "could not be put: nobody here can answer it, and it was \
                                 answered as a closed dialogue answers (ADR 1628)"
            .to_owned(),
    };
    State::with(context, |record| {
        record.note(&format!(
            "the script asked {} and {said}",
            question.summary()
        ));
    });
    match answer {
        Answer::Unanswerable => question.dismissed(),
        answer => answer,
    }
}
