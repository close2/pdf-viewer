//! The host object model a document's realm runs against (ADRs 1591, 1602, 1603).
//!
//! What is carried: the document as the global object — `getField` of any field, `getNthFieldName`,
//! `numFields`, `calculateNow`, `resetForm`, `pageNum` — whose write is a page turn the host makes
//! (ADR 1640) — and `numPages`; a `Field` for every field the
//! realm was told of, with the properties RFC 0008 section 4.2 admits for a field's value and its
//! appearance (`value`, `valueAsString`, `name`, `type`, `display`, `hidden`, `readonly`,
//! `required`, `textColor`, `fillColor`, `strokeColor`, `borderStyle`, `alignment`, `charLimit`,
//! the flags `multiline`, `password`, `comb`, `doNotScroll`, `style`, and `page`, `rect`, `doc`)
//! and its `getArray` and `setFocus`, and a `Field` of one widget of a field (ADR 1664); `event`
//! with what each site raises; `app`'s six properties naming the
//! viewer, `util.printd` and `util.printx` (ADR 1615); the reference's `display`, `border`, `font`
//! and `color` constants; `console` (ADR 1762); a field's `lineWidth`, `textSize` and `textFont`
//! and `event`'s keys and rich pair (ADR 1762); and the `AF*` library, each function a native that hands
//! its arguments to
//! `pdf_model::aform` — the Rust Tier 0 runs, so that a format called from a script and a format
//! that is the whole script write the same characters. Everything else [`crate::surface`] lists is
//! a property whose every read and write throws a `NotAllowedError`.
//!
//! **A write is an edit.** Setting a field's value or property changes the realm's table, so the
//! script reads back what it wrote, and records a [`ScriptEdit`] that the view state applies to its
//! edit log beside a person's typing (RFC 0008 section 6.4). A keystroke, format or validate script
//! changes its own field through `event.value`, never through the field, as the reference's event
//! model has it.
//!
//! Adobe's reference makes the document `this` in a field script; here the document's members are
//! the global object's, since a script's top-level `this` is that object. The members' meanings are
//! the reference's "Doc methods", "Field properties" and "event properties" pages, each a documented
//! choice (`doc/todo/56`).

use std::cell::RefCell;
use std::collections::BTreeMap;

use boa_engine::object::builtins::{JsArray, JsDate};
use boa_engine::object::{FunctionObjectBuilder, ObjectInitializer};
use boa_engine::property::PropertyDescriptor;
use boa_engine::{Context, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::aform::{
    Call, Function, Keyed, Keystroke, Literal, Trigger, extract_nums, make_number, merge_change,
    parse_date,
};
use pdf_model::view::{
    Alignment, BorderStyle, Colour, ConsoleCommand, Display, FieldState, FieldType, Glyph,
    Property, ScriptEdit, ScriptSite, TextFlag,
};

use super::{State, guard, members, refuse};
use crate::request::byte_offset;
use crate::surface::{EXCLUDED, Holder, NOT_BRIDGED, REFUSED};
use crate::{Outcome, RefusalKind, Request, viewer};

/// The objects a realm builds once and hands out many times.
#[derive(Debug)]
struct Objects {
    /// What every `Field` inherits its accessors from.
    prototype: JsObject,
    /// The `Field` handed out for each name, so that one name is one object.
    fields: RefCell<BTreeMap<String, JsObject>>,
}

/// Table 227 bit 1.
const READ_ONLY: u32 = 1;
/// Table 227 bit 2.
const REQUIRED: u32 = 1 << 1;
/// Table 231 bit 21, `FileSelect`: one of the three flags that must be clear for `Comb`.
const FILE_SELECT: u32 = 1 << 20;

/// Installs the guards and the host object model a realm keeps for its lifetime.
///
/// # Errors
///
/// The engine's, where a property cannot be defined — which, on a context just constructed, it
/// can.
pub(super) fn install(context: &mut Context) -> JsResult<()> {
    guard::install(context)?;
    let prototype = field_prototype(context)?;
    context.insert_data(Objects {
        prototype,
        fields: RefCell::new(BTreeMap::new()),
    });
    document(context)?;
    constants(context)?;
    Ok(())
}

/// Installs one run's `event`, and answers it.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn begin(context: &mut Context, request: &Request) -> JsResult<JsObject> {
    let event = ObjectInitializer::new(context).build();
    let text = |text: &str| JsValue::from(JsString::from(text));
    let (kind, name) = request.site.event_names(
        !request.field.is_empty() && matches!(request.site, ScriptSite::Annotation(_)),
    );
    // The reference's "Event type/name combinations" page makes the document the target of every
    // `Doc` event — the open and Table 200's five (ADR 1614).
    let target = if matches!(
        request.site,
        ScriptSite::Library | ScriptSite::OpenAction | ScriptSite::Document(_) | ScriptSite::Timer
    ) {
        JsValue::from(context.global_object())
    } else {
        field_object(context, &request.field).map_or_else(JsValue::null, JsValue::from)
    };
    let source =
        field_object(context, &request.event.source).map_or_else(JsValue::null, JsValue::from);
    let target_name = if request.site == ScriptSite::Library {
        request.label.as_str()
    } else {
        request.field.as_str()
    };
    for (key, value, writable) in [
        ("value", text(&request.event.value), true),
        ("change", text(&request.event.change), true),
        ("rc", JsValue::from(true), true),
        (
            "selStart",
            JsValue::from(request.event.selection_start),
            true,
        ),
        ("selEnd", JsValue::from(request.event.selection_end), true),
        (
            "willCommit",
            JsValue::from(request.event.will_commit),
            false,
        ),
        // The reference's "event properties" page makes these three read-only (ADR 1626).
        ("commitKey", JsValue::from(request.event.commit_key), false),
        ("fieldFull", JsValue::from(request.event.field_full), false),
        ("changeEx", text(&request.event.change_ex), false),
        // The reference's "event properties" page makes the three read-only; the host told the
        // view state its keys (ADR 1762).
        ("shift", JsValue::from(request.event.shift), false),
        ("modifier", JsValue::from(request.event.modifier), false),
        ("keyDown", JsValue::from(request.event.key_down), false),
        ("target", target, false),
        ("source", source, false),
        ("targetName", text(target_name), false),
        ("name", text(name), false),
        ("type", text(kind), false),
    ] {
        data(&event, key, value, writable, context)?;
    }
    if matches!(request.site, ScriptSite::Field(_)) && !request.event.rich_value.is_empty() {
        rich_pair(&event, request, context)?;
    }
    refusers(&event, Holder::Event, context)?;
    let global = context.global_object();
    global.define_property_or_throw(
        JsString::from("event"),
        PropertyDescriptor::builder()
            .value(event.clone())
            .writable(false)
            .enumerable(true)
            .configurable(true)
            .build(),
        context,
    )?;
    Ok(event)
}

/// `event.richValue` and `event.richChange` at a rich text field's event, both read-only as RFC
/// 0008 section 4.2 admits them (ADR 1762).
///
/// The value is the field's rich text string read into the reference's `Span` objects by
/// `pdf_model::span::spans` — in this process, so the document's markup is parsed where nothing
/// else is reached — at the reference's 12 points where the string states no absolute size. The
/// change is one span of `event.change`, styled as the span the selection starts in: the
/// reference makes a keystroke a single-member array, and what style a typed character takes is
/// the run it is typed into, a documented choice. A string the reader does not take leaves the
/// pair undefined, as on a field that is not rich text.
fn rich_pair(event: &JsObject, request: &Request, context: &mut Context) -> JsResult<()> {
    let Some(spans) = pdf_model::span::spans(&request.event.rich_value, None, 12.0) else {
        return Ok(());
    };
    let mut objects = Vec::with_capacity(spans.len());
    for span in &spans {
        objects.push(JsValue::from(super::util::span_object(span, context)?));
    }
    let value = JsArray::from_iter(objects, context);
    data(event, "richValue", JsValue::from(value), false, context)?;
    let mut units = 0_u64;
    let start = u64::from(request.event.selection_start);
    let typed_into = spans
        .iter()
        .find(|span| {
            let length = u64::try_from(span.text.encode_utf16().count()).unwrap_or(u64::MAX);
            units = units.saturating_add(length);
            units >= start
        })
        .or_else(|| spans.last());
    let change = pdf_model::span::Span {
        text: request.event.change.clone(),
        ..typed_into.cloned().unwrap_or_default()
    };
    let change = JsArray::from_iter(
        [JsValue::from(super::util::span_object(&change, context)?)],
        context,
    );
    data(event, "richChange", JsValue::from(change), false, context)?;
    Ok(())
}

/// The global object as the document: its methods, `app`, `util`, `console`, the library, and
/// every name the surface refuses.
fn document(context: &mut Context) -> JsResult<()> {
    let global = context.global_object();
    let methods: [(&str, NativeFunction); 4] = [
        ("getField", NativeFunction::from_fn_ptr(get_field)),
        (
            "getNthFieldName",
            NativeFunction::from_fn_ptr(nth_field_name),
        ),
        ("calculateNow", NativeFunction::from_fn_ptr(calculate_now)),
        ("resetForm", NativeFunction::from_fn_ptr(reset_form)),
    ];
    for (name, native) in methods {
        let callable = function(context, name, native);
        data(&global, name, JsValue::from(callable), false, context)?;
    }
    for (name, read) in [
        ("numFields", Document::Fields),
        ("numPages", Document::Pages),
    ] {
        let getter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, _arguments, context| {
                Ok(document_number(read, context))
            }),
        );
        let setter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, _arguments, context| {
                Err(refuse(
                    format!("this.{name}="),
                    RefusalKind::Unreachable(
                        "the document's page count and field count are the document's to state, \
                         and a script reads them"
                            .to_owned(),
                    ),
                    context,
                ))
            }),
        );
        accessor(&global, name, getter, setter, context)?;
    }
    page_num(&global, context)?;
    members::document(&global, context)?;
    super::pages::document(&global, context)?;
    super::window::install(&global, context)?;
    super::annotations::install(&global, context)?;
    refusers(&global, Holder::Doc, context)?;
    refusers(&global, Holder::Global, context)?;

    let app = ObjectInitializer::new(context).build();
    identity(&app, context)?;
    members::app(&app, context)?;
    super::pages::app(&app, context)?;
    refusers(&app, Holder::App, context)?;
    data(&global, "app", JsValue::from(app), false, context)?;
    let util = ObjectInitializer::new(context).build();
    let methods: [(&str, NativeFunction); 2] = [
        ("printx", NativeFunction::from_fn_ptr(print_mask)),
        ("printd", NativeFunction::from_fn_ptr(print_date)),
    ];
    for (name, native) in methods {
        let callable = function(context, name, native);
        data(&util, name, JsValue::from(callable), false, context)?;
    }
    members::util(&util, context)?;
    super::util::install(&util, context)?;
    refusers(&util, Holder::Util, context)?;
    data(&global, "util", JsValue::from(util), false, context)?;
    let console = console_object(context)?;
    data(&global, "console", JsValue::from(console), false, context)?;

    for library in Function::ALL {
        let native = NativeFunction::from_copy_closure(move |_this, arguments, context| {
            call_library(library, arguments, context)
        });
        let callable = function(context, library.name(), native);
        data(
            &global,
            library.name(),
            JsValue::from(callable),
            false,
            context,
        )?;
    }
    members::library(&global, context)?;
    Ok(())
}

/// `console`: `println`, which logs a line, and `show`, `hide` and `clear`, each a request to the
/// host's console (ADR 1762).
fn console_object(context: &mut Context) -> JsResult<JsObject> {
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
    for (name, command) in [
        ("show", ConsoleCommand::Show),
        ("hide", ConsoleCommand::Hide),
        ("clear", ConsoleCommand::Clear),
    ] {
        let native = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, _arguments, context| {
                ask_console(command, context);
                Ok(JsValue::undefined())
            }),
        );
        data(&console, name, JsValue::from(native), false, context)?;
    }
    refusers(&console, Holder::Console, context)?;
    Ok(console)
}

/// `console.show`, `hide` or `clear`: a request to the host's console, which shows what scripts
/// log (ADR 1762). Adobe's "console methods" page is the meaning of each, a documented choice.
///
/// A `clear` also drops the lines this run logged before it, which the host has not been handed
/// yet, so that what the console shows after the run is what the script logged after its clear.
fn ask_console(command: ConsoleCommand, context: &mut Context) {
    State::with(context, |record| {
        if command == ConsoleCommand::Clear {
            record.clear_log();
        }
        record.push_edit(ScriptEdit::Console(command));
    });
}

/// `app`'s six properties that say which viewer a script runs in, each this program's own answer
/// ([`crate::viewer`], ADR 1615), read-only as the reference's "app properties" page has them.
fn identity(app: &JsObject, context: &mut Context) -> JsResult<()> {
    let answers: [(&str, Answer); 6] = [
        ("viewerType", Answer::Text(viewer::VIEWER_TYPE)),
        ("viewerVariation", Answer::Text(viewer::VIEWER_VARIATION)),
        ("viewerVersion", Answer::Number(viewer::version())),
        ("formsVersion", Answer::Number(viewer::version())),
        ("platform", Answer::Text(viewer::platform())),
        ("language", Answer::Text(viewer::LANGUAGE)),
    ];
    for (name, answer) in answers {
        let getter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, _arguments, _context| {
                Ok(match answer {
                    Answer::Text(text) => JsValue::from(JsString::from(text)),
                    Answer::Number(number) => JsValue::from(number),
                })
            }),
        );
        let setter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, _arguments, context| {
                Err(refuse(
                    format!("app.{name}="),
                    RefusalKind::Unreachable(
                        "which viewer a script runs in is the viewer's to state, and a script \
                         reads it"
                            .to_owned(),
                    ),
                    context,
                ))
            }),
        );
        accessor(app, name, getter, setter, context)?;
    }
    Ok(())
}

/// What one of `app`'s identity properties answers.
#[derive(Debug, Clone, Copy)]
enum Answer {
    /// A string.
    Text(&'static str),
    /// A number.
    Number(f64),
}

/// `util.printx(cFormat, cSource)`: the source written through the mask, by the very function
/// `AFSpecial_Format` writes through (`pdf_model::aform::print_mask`, ADR 1578 section 7).
fn print_mask(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    // The mask copies each source character once at most, so what it writes is never longer than
    // the two strings the script already holds, and no budget is asked.
    let mask = text_argument(arguments, 0, context)?;
    let source = text_argument(arguments, 1, context)?;
    Ok(JsValue::from(JsString::from(
        pdf_model::aform::print_mask(&mask, &source).as_str(),
    )))
}

/// `util.printd(cFormat, oDate, bXFAPicture)`: a `Date` written in a picture, or in one of the
/// reference's three numbered formats.
///
/// The picture language is `pdf_model::aform::print_date`'s, the one `AFDate_FormatEx` writes
/// through, and the date's fields are read in local time at the request's offset. The numbered
/// formats are the reference's "util methods" page's, a documented choice each (ADR 1615): `0` is
/// §7.9.4's date string in local time with its offset — the reference's example for it is
/// `D:20000801145605+07'00'`; `1`, which the reference calls *universal*, is the same moment in
/// Universal Time, written with §7.9.4's `Z`, since its example repeats format 0's; and `2` is the
/// reference's example's own shape, `yyyy/mm/dd HH:MM:ss`, because this program has one locale.
/// An XFA picture clause — `bXFAPicture` true — is XFA's, which Annex K permits a processor not to
/// implement, and is refused by name.
fn print_date(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let refused = |why: &str, context: &mut Context| {
        refuse(
            "util.printd".to_owned(),
            RefusalKind::Library(why.to_owned()),
            context,
        )
    };
    if arguments.get(2).is_some_and(JsValue::to_boolean) {
        return Err(refused(
            "its third argument asks for an XFA picture clause, which is XFA's, and Annex K \
             permits a processor not to implement XFA",
            context,
        ));
    }
    let Some(date) = arguments
        .get(1)
        .and_then(JsValue::as_object)
        .and_then(|object| JsDate::from_object(object).ok())
    else {
        return Err(refused("its second argument is not a Date", context));
    };
    let format = arguments.first().cloned().unwrap_or_default();
    let numbered = match format.as_number() {
        None => None,
        Some(number)
            if number.fract().abs() < f64::EPSILON && (0..=2).contains(&integral(number)) =>
        {
            Some(integral(number))
        }
        Some(_) => {
            return Err(refused(
                "its first argument is a number, and the numbered formats are 0, 1 and 2",
                context,
            ));
        }
    };
    let universal = numbered == Some(1);
    let Some(moment) = moment_of(&date, universal, context)? else {
        return Err(refused(
            "its date is not a moment: the Date holds no time value",
            context,
        ));
    };
    let picture = match numbered {
        Some(0 | 1) => "yyyymmddHHMMss".to_owned(),
        Some(_) => "yyyy/mm/dd HH:MM:ss".to_owned(),
        None => format.to_string(context)?.to_std_string_lossy(),
    };
    // A place-holder writes at most nine characters for its four — `mmmm` is `September` — so the
    // output is held to the string budget before it is built.
    let units = u64::try_from(picture.encode_utf16().count()).unwrap_or(u64::MAX);
    guard::string_units(units.saturating_mul(9) / 4, context)?;
    let body = pdf_model::aform::print_date(&picture, &moment)
        .map_err(|refusal| refused(refusal.sentence(), context))?;
    let written = match numbered {
        Some(0) => {
            // `getTimezoneOffset` is minutes *west* of Universal Time, as ECMA-262 defines it.
            let west = date.get_timezone_offset(context)?.to_number(context)?;
            format!("D:{body}{}", pdf_offset(integral(west).saturating_neg()))
        }
        Some(1) => format!("D:{body}Z"),
        _ => body,
    };
    Ok(JsValue::from(JsString::from(written.as_str())))
}

/// A `Date`'s fields as the library's moment, in local time or in Universal Time; `None` for a
/// `Date` that holds no time value, or one whose year the library's moment cannot hold.
fn moment_of(
    date: &JsDate,
    universal: bool,
    context: &mut Context,
) -> JsResult<Option<pdf_model::aform::DateTime>> {
    let fields = if universal {
        [
            date.get_utc_full_year(context)?,
            date.get_utc_month(context)?,
            date.get_utc_date(context)?,
            date.get_utc_hours(context)?,
            date.get_utc_minutes(context)?,
            date.get_utc_seconds(context)?,
        ]
    } else {
        [
            date.get_full_year(context)?,
            date.get_month(context)?,
            date.get_date(context)?,
            date.get_hours(context)?,
            date.get_minutes(context)?,
            date.get_seconds(context)?,
        ]
    };
    let mut numbers = [0.0_f64; 6];
    for (slot, field) in numbers.iter_mut().zip(fields) {
        let number = field.to_number(context)?;
        if !number.is_finite() {
            return Ok(None);
        }
        *slot = number;
    }
    let [year, month, day, hour, minute, second] = numbers;
    let small = |value: f64| u8::try_from(integral(value)).ok();
    let Ok(year) = i32::try_from(integral(year)) else {
        return Ok(None);
    };
    let (Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        small(month),
        small(day),
        small(hour),
        small(minute),
        small(second),
    ) else {
        return Ok(None);
    };
    Ok(Some(pdf_model::aform::DateTime {
        year,
        // ECMA-262 counts months from zero.
        month: month.saturating_add(1),
        day,
        hour,
        minute,
        second,
    }))
}

/// A number as an integer, towards zero: the `Date` getters answer integral values, and a
/// numbered format is checked to be one before it is read.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the values are integral where they are read, and `as` saturates out of range"
)]
pub(super) fn integral(value: f64) -> i64 {
    value as i64
}

/// §7.9.4's `O HH ' mm` for an offset in minutes east of Universal Time: `Z` for none.
fn pdf_offset(minutes: i64) -> String {
    if minutes == 0 {
        return "Z".to_owned();
    }
    let sign = if minutes < 0 { '-' } else { '+' };
    let absolute = minutes.unsigned_abs();
    format!("{sign}{:02}'{:02}", absolute / 60, absolute % 60)
}

/// One argument's text, `undefined` where it was not passed.
pub(super) fn text_argument(
    arguments: &[JsValue],
    index: usize,
    context: &mut Context,
) -> JsResult<String> {
    Ok(arguments
        .get(index)
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_lossy())
}

/// The reference's `font` object: its keys and the §9.6.2.2 standard font each names, which
/// `Field.textFont` is set with (ADR 1762).
fn font_constants(global: &JsObject, context: &mut Context) -> JsResult<()> {
    let font = ObjectInitializer::new(context).build();
    for (key, base) in [
        ("Times", "Times-Roman"),
        ("TimesB", "Times-Bold"),
        ("TimesI", "Times-Italic"),
        ("TimesBI", "Times-BoldItalic"),
        ("Helv", "Helvetica"),
        ("HelvB", "Helvetica-Bold"),
        ("HelvI", "Helvetica-Oblique"),
        ("HelvBI", "Helvetica-BoldOblique"),
        ("Cour", "Courier"),
        ("CourB", "Courier-Bold"),
        ("CourI", "Courier-Oblique"),
        ("CourBI", "Courier-BoldOblique"),
        ("Symbol", "Symbol"),
        ("ZapfD", "ZapfDingbats"),
    ] {
        data(
            &font,
            key,
            JsValue::from(JsString::from(base)),
            false,
            context,
        )?;
    }
    data(global, "font", JsValue::from(font), false, context)
}

/// The reference's `display`, `border` and `color` objects: constants a script compares and
/// assigns, and `color.equal`.
fn constants(context: &mut Context) -> JsResult<()> {
    let global = context.global_object();
    font_constants(&global, context)?;
    let display = ObjectInitializer::new(context).build();
    for (name, constant) in [
        ("visible", Display::Visible),
        ("hidden", Display::Hidden),
        ("noPrint", Display::NoPrint),
        ("noView", Display::NoView),
    ] {
        data(
            &display,
            name,
            JsValue::from(constant.number()),
            false,
            context,
        )?;
    }
    data(&global, "display", JsValue::from(display), false, context)?;

    let border = ObjectInitializer::new(context).build();
    for (name, style) in [
        ("s", BorderStyle::Solid),
        ("b", BorderStyle::Beveled),
        ("d", BorderStyle::Dashed),
        ("i", BorderStyle::Inset),
        ("u", BorderStyle::Underline),
    ] {
        data(
            &border,
            name,
            JsValue::from(JsString::from(style.adobe())),
            false,
            context,
        )?;
    }
    data(&global, "border", JsValue::from(border), false, context)?;

    // The "Field properties" page's `style` table names six glyph styles of a check box or radio
    // button, each with its keyword, and types the property a string; that the string is the
    // style's own name in the table — `style.ci` is `"circle"` — is a documented choice, since the
    // page states the keywords and not their values (ADR 1652). `Field.style` reads and writes
    // them as the glyph Table 192's `/CA` draws (ADR 1665).
    let glyph = ObjectInitializer::new(context).build();
    for (name, style) in [
        ("ch", Glyph::Check),
        ("cr", Glyph::Cross),
        ("di", Glyph::Diamond),
        ("ci", Glyph::Circle),
        ("st", Glyph::Star),
        ("sq", Glyph::Square),
    ] {
        data(
            &glyph,
            name,
            JsValue::from(JsString::from(style.adobe())),
            false,
            context,
        )?;
    }
    data(&global, "style", JsValue::from(glyph), false, context)?;

    // The "FullScreen properties" page's `cursor` table names three pointer behaviours and types
    // the property a number without stating one; they are numbered in the table's own order, a
    // documented choice (ADR 1652). Nothing here reads them back: `app.fs` is no member RFC 0008
    // section 4.2 admits to `app`, and it is refused by name (`crate::surface::EXCLUDED`, ADR
    // 1665).
    let cursor = ObjectInitializer::new(context).build();
    for (name, number) in [("hidden", 0), ("delay", 1), ("visible", 2)] {
        data(&cursor, name, JsValue::from(number), false, context)?;
    }
    data(&global, "cursor", JsValue::from(cursor), false, context)?;

    let color = ObjectInitializer::new(context).build();
    for (name, constant) in [
        ("transparent", Colour::Transparent),
        ("black", Colour::Gray(0.0)),
        ("white", Colour::Gray(1.0)),
        ("red", Colour::Rgb([1.0, 0.0, 0.0])),
        ("green", Colour::Rgb([0.0, 1.0, 0.0])),
        ("blue", Colour::Rgb([0.0, 0.0, 1.0])),
        ("cyan", Colour::Cmyk([1.0, 0.0, 0.0, 0.0])),
        ("magenta", Colour::Cmyk([0.0, 1.0, 0.0, 0.0])),
        ("yellow", Colour::Cmyk([0.0, 0.0, 1.0, 0.0])),
        ("dkGray", Colour::Gray(0.25)),
        ("gray", Colour::Gray(0.5)),
        ("ltGray", Colour::Gray(0.75)),
    ] {
        let array = colour_array(constant, context);
        data(&color, name, array, true, context)?;
    }
    let equal = function(
        context,
        "equal",
        NativeFunction::from_copy_closure(|_this, arguments, context| {
            let first = colour_of(&arguments.first().cloned().unwrap_or_default(), context)?;
            let second = colour_of(&arguments.get(1).cloned().unwrap_or_default(), context)?;
            Ok(JsValue::from(first.is_some() && first == second))
        }),
    );
    data(&color, "equal", JsValue::from(equal), false, context)?;
    refuser(&color, "color.", "convert", None, context)?;
    data(&global, "color", JsValue::from(color), false, context)?;
    Ok(())
}

/// What the document's three number properties read.
#[derive(Debug, Clone, Copy)]
enum Document {
    /// `numFields`.
    Fields,
    /// `numPages`.
    Pages,
    /// `pageNum`.
    Page,
}

/// One of the document's number properties, from the realm's table.
fn document_number(read: Document, context: &Context) -> JsValue {
    State::table(context, |table| match read {
        Document::Fields => JsValue::from(u32::try_from(table.fields.len()).unwrap_or(u32::MAX)),
        Document::Pages => JsValue::from(table.pages),
        Document::Page => JsValue::from(table.page),
    })
    .unwrap_or_default()
}

/// `this.pageNum`: read from the realm's table, and written as a page turn ([`write_page_num`]).
fn page_num(global: &JsObject, context: &mut Context) -> JsResult<()> {
    let getter = function(
        context,
        "pageNum",
        NativeFunction::from_copy_closure(|_this, _arguments, context| {
            Ok(document_number(Document::Page, context))
        }),
    );
    let setter = function(
        context,
        "pageNum",
        NativeFunction::from_fn_ptr(write_page_num),
    );
    accessor(global, "pageNum", getter, setter, context)?;
    Ok(())
}

/// `this.pageNum = n`: a page turn the host performs, recorded as [`ScriptEdit::GoTo`] (ADR 1640).
///
/// Adobe's "Doc properties" page makes `pageNum` the document's current page, zero-based, read and
/// written, with `this.pageNum = 0` and `this.pageNum++` as its examples. It states nothing for a
/// value that names no page, so this is a documented choice: the value is read as ECMAScript's
/// `ToNumber` and truncated toward zero, and a value that is not finite or falls outside
/// `0..numPages` turns no page — the run says so in its notes, and the script goes on, as a turn a
/// person asks for past the last page goes nowhere. The script reads back the page it turned to for
/// the rest of the run; the latest turn of a run is the one carried.
fn write_page_num(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let asked = arguments
        .first()
        .cloned()
        .unwrap_or_default()
        .to_number(context)?;
    let pages = State::table(context, |table| table.pages).unwrap_or(0);
    let Some(page) = page_named(asked, pages) else {
        State::with(context, |record| {
            record.note(&format!(
                "the script set this.pageNum to {asked}, which names no page of this document's \
                 {pages}; no page is turned (ADR 1640)"
            ));
        });
        return Ok(JsValue::undefined());
    };
    State::table(context, |table| table.page = page);
    State::with(context, |record| {
        if let Some(earlier) = record
            .edits
            .iter_mut()
            .find(|edit| matches!(edit, ScriptEdit::GoTo { .. }))
        {
            *earlier = ScriptEdit::GoTo { page };
        } else if record.edits.len() < super::MAX_EDITS {
            record.edits.push(ScriptEdit::GoTo { page });
        }
    });
    Ok(JsValue::undefined())
}

/// The zero-based page `asked` names among `pages`, truncated toward zero: `None` for a value that
/// is not a number, is infinite, or falls outside `0..pages`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is held inside `0..pages`, a `u32`'s range, and already whole before it is \
              converted, so the conversion is exact"
)]
fn page_named(asked: f64, pages: u32) -> Option<u32> {
    let truncated = asked.trunc();
    (truncated >= 0.0 && truncated < f64::from(pages)).then_some(truncated as u32)
}

/// `this.getField(cName)`: the field of that name, or of every field below it, or `null`.
///
/// A name that is not a terminal field but has fields below it answers a `Field` for the whole
/// subtree, which is how the reference lets a script hide a group; a name nothing answers to is
/// `null`, as the reference has it.
///
/// **A name the document holds is always matched as written**, and only a name that matches
/// nothing is read a second time, as [`spoken_name`] cuts it (ADR 1652). A second reading that
/// finds a field is said in the run's notes, so a reader is told which field the script was given.
fn get_field(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let name = arguments
        .first()
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_lossy();
    if let Some(field) = field_object(context, &name) {
        return Ok(JsValue::from(field));
    }
    if let Some((field, index)) = widget_address(&name)
        && let Some(widgets) = State::table(context, |table| {
            table.fields.get(field).map(|state| state.widgets.len())
        })
        .flatten()
    {
        // A widget the field does not have is no object of the realm, as a name no field has is
        // not: the reference's index counts the field's widgets and stops there.
        let held = usize::try_from(index).is_ok_and(|index| index < widgets);
        return Ok(if held {
            widget_object(context, &name, field, index).map_or_else(JsValue::null, JsValue::from)
        } else {
            JsValue::null()
        });
    }
    let Some(spoken) = spoken_name(&name) else {
        return Ok(JsValue::null());
    };
    let Some(field) = field_object(context, spoken) else {
        return Ok(JsValue::null());
    };
    State::with(context, |record| {
        record.note(&format!(
            "the script asked for the field {name:?}, which this document does not have, and was \
             given {spoken:?}, the name without the white space around it or the periods after \
             it (ADR 1652)"
        ));
    });
    Ok(JsValue::from(field))
}

/// The terminal field a name addresses one widget of, and the widget's index, the reference's
/// `name.N`: the name before a final PERIOD and a run of decimal digits, `None` for any other name.
///
/// The "Field" page of Adobe's reference has `getField` answer, for a field's name, a PERIOD and a
/// widget's index from zero, a `Field` of that one widget: its widget-level members are that
/// widget's, and its field-level members — the value among them — the field's (ADR 1664). A field
/// whose own name ends that way is found by the exact reading first, and an index too long for a
/// `u32` names no widget any field has.
fn widget_address(asked: &str) -> Option<(&str, u32)> {
    let (field, index) = asked.rsplit_once('.')?;
    if field.is_empty() || index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some((field, index.parse().unwrap_or(u32::MAX)))
}

/// The name `getField` reads a second time where `asked` names no field: without the white space
/// at either end and the PERIODs after it — `None` where that leaves it unchanged or empty.
///
/// The PERIOD is ISO 32000-2 §12.7.4.2's own separator, and the clause rules it out of a name's
/// parts:
///
/// > Because the PERIOD is used as a separator for fully qualified names, a partial name shall not
/// > contain a PERIOD character.
///
/// so a period that ends a name separates its last part from nothing, and a name of this document
/// that ends that way would be a field stating an empty `/T` — which the first, exact reading
/// already finds. White space is a character a partial name may hold, and is cut only because the
/// exact reading has already failed: a name this document holds with its spaces is never reached
/// here. The cut is the one Tier 0 makes of `AFSimple_Calculate`'s list of names, so that one
/// spelling of a name is read alike by the library and by a script (ADR 1652).
fn spoken_name(asked: &str) -> Option<&str> {
    let cut = asked
        .trim()
        .trim_end_matches(|character: char| character == '.' || character.is_whitespace());
    (!cut.is_empty() && cut != asked).then_some(cut)
}

/// `this.getNthFieldName(nIndex)`: the field names in §12.7.4.2's spelling, sorted, which is the
/// order the realm's table keeps — a documented choice, since the reference does not say which
/// order (ADR 1603).
fn nth_field_name(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let index = arguments
        .first()
        .cloned()
        .unwrap_or_default()
        .to_length(context)?;
    let name = State::table(context, |table| {
        usize::try_from(index)
            .ok()
            .and_then(|index| table.fields.keys().nth(index).cloned())
    })
    .flatten();
    Ok(name.map_or_else(JsValue::null, |name| {
        JsValue::from(JsString::from(name.as_str()))
    }))
}

/// `this.calculateNow()`: Table 224's `/CO` walked once more after the script.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function's signature is the engine's, and every native answers a result"
)]
fn calculate_now(
    _this: &JsValue,
    _arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    State::note(context, ScriptEdit::Calculate);
    Ok(JsValue::undefined())
}

/// `this.resetForm(aFields)`: §12.7.6.3's reset over the fields named, or every field.
fn reset_form(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let mut fields = Vec::new();
    if let Some(text) = arguments.first().and_then(JsValue::as_string) {
        fields.push(text.to_std_string_lossy());
    } else if let Some(list) = arguments.first().and_then(JsValue::as_object) {
        let length = list
            .get(JsString::from("length"), context)?
            .to_length(context)?;
        for index in 0..length.min(u64::from(u16::MAX)) {
            fields.push(
                list.get(index, context)?
                    .to_string(context)?
                    .to_std_string_lossy(),
            );
        }
    }
    State::note(context, ScriptEdit::Reset { fields });
    Ok(JsValue::undefined())
}

/// The `Field` for a name the realm knows, or for a name with known fields below it; `None`
/// otherwise and for the empty name.
fn field_object(context: &mut Context, name: &str) -> Option<JsObject> {
    if name.is_empty() || terminals(context, name).is_empty() {
        return None;
    }
    let objects = context.get_data::<Objects>()?;
    if let Some(held) = objects.fields.borrow().get(name) {
        return Some(held.clone());
    }
    let prototype = objects.prototype.clone();
    let object = ObjectInitializer::new(context).build();
    object.set_prototype(Some(prototype));
    data(
        &object,
        "name",
        JsValue::from(JsString::from(name)),
        false,
        context,
    )
    .ok()?;
    if let Some(objects) = context.get_data::<Objects>() {
        objects
            .fields
            .borrow_mut()
            .insert(name.to_owned(), object.clone());
    }
    Some(object)
}

/// The `Field` of one widget of a terminal field, held under the name a script asked by.
///
/// Its `name` is the field's, as the reference's is, and its `widget` the index that every
/// widget-level member reads and writes through ([`widget_of`]).
fn widget_object(context: &mut Context, asked: &str, field: &str, index: u32) -> Option<JsObject> {
    let objects = context.get_data::<Objects>()?;
    if let Some(held) = objects.fields.borrow().get(asked) {
        return Some(held.clone());
    }
    let prototype = objects.prototype.clone();
    let object = ObjectInitializer::new(context).build();
    object.set_prototype(Some(prototype));
    data(
        &object,
        "name",
        JsValue::from(JsString::from(field)),
        false,
        context,
    )
    .ok()?;
    object
        .define_property_or_throw(
            JsString::from(WIDGET),
            PropertyDescriptor::builder()
                .value(JsValue::from(index))
                .writable(false)
                .enumerable(false)
                .configurable(false)
                .build(),
            context,
        )
        .ok()?;
    if let Some(objects) = context.get_data::<Objects>() {
        objects
            .fields
            .borrow_mut()
            .insert(asked.to_owned(), object.clone());
    }
    Some(object)
}

/// The own property a `Field` of one widget carries its index in. Not a member of the reference:
/// a name no script of the census spells, so that it shadows nothing a document reads.
const WIDGET: &str = "__widget";

/// The widget a `Field` stands for, or `None` for one that stands for every widget of its field.
pub(super) fn widget_of(this: &JsValue, context: &mut Context) -> JsResult<Option<u32>> {
    let Some(object) = this.as_object() else {
        return Ok(None);
    };
    if !object.has_own_property(JsString::from(WIDGET), context)? {
        return Ok(None);
    }
    let index = object
        .get(JsString::from(WIDGET), context)?
        .to_u32(context)?;
    Ok(Some(index))
}

/// The terminal fields a name stands for: itself where the realm knows it, every field below it
/// where it does not.
pub(super) fn terminals(context: &Context, name: &str) -> Vec<String> {
    State::table(context, |table| {
        if table.fields.contains_key(name) {
            return vec![name.to_owned()];
        }
        let below = format!("{name}.");
        table
            .fields
            .range(below.clone()..)
            .take_while(|(held, _)| held.starts_with(&below))
            .map(|(held, _)| held.clone())
            .collect()
    })
    .unwrap_or_default()
}

/// The prototype every `Field` inherits from: an accessor per property the bridge carries, and the
/// refusers for the rest.
fn field_prototype(context: &mut Context) -> JsResult<JsObject> {
    let prototype = ObjectInitializer::new(context).build();
    for property in FieldProperty::ALL {
        let name = property.name();
        let getter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |this, _arguments, context| {
                read_property(property, this, context)
            }),
        );
        let setter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |this, arguments, context| {
                let value = arguments.first().cloned().unwrap_or_default();
                write_property(property, this, &value, context)?;
                Ok(JsValue::undefined())
            }),
        );
        accessor(&prototype, name, getter, setter, context)?;
    }
    let methods: [(&str, NativeFunction); 2] = [
        ("getArray", NativeFunction::from_fn_ptr(get_array)),
        ("setFocus", NativeFunction::from_fn_ptr(set_focus)),
    ];
    for (name, native) in methods {
        let callable = function(context, name, native);
        data(&prototype, name, JsValue::from(callable), false, context)?;
    }
    members::field(&prototype, context)?;
    super::choices::field(&prototype, context)?;
    refusers(&prototype, Holder::Field, context)?;
    Ok(prototype)
}

/// The `Field` properties the bridge carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldProperty {
    /// `value`.
    Value,
    /// `valueAsString`.
    ValueAsString,
    /// `type`.
    Type,
    /// `display`.
    Display,
    /// `hidden`.
    Hidden,
    /// `readonly`.
    ReadOnly,
    /// `required`.
    Required,
    /// `textColor`.
    TextColor,
    /// `fillColor`.
    FillColor,
    /// `strokeColor`.
    StrokeColor,
    /// `borderStyle`.
    BorderStyle,
    /// `alignment`.
    Alignment,
    /// `charLimit`.
    CharLimit,
    /// One of Table 231's text field flags.
    Flag(TextFlag),
    /// `page`.
    Page,
    /// `rect`.
    Rect,
    /// `doc`.
    Doc,
    /// `style`: a check box's or radio button's glyph (ADR 1665).
    Style,
    /// `lineWidth`: Table 168's `/W` (ADR 1762).
    LineWidth,
    /// `textSize`: the `/DA`'s `Tf` size (ADR 1762).
    TextSize,
    /// `textFont`: the `/DA`'s `Tf` font (ADR 1762).
    TextFont,
}

impl FieldProperty {
    /// Every property, in the order the prototype defines them.
    const ALL: [Self; 24] = [
        Self::Value,
        Self::ValueAsString,
        Self::Type,
        Self::Display,
        Self::Hidden,
        Self::ReadOnly,
        Self::Required,
        Self::TextColor,
        Self::FillColor,
        Self::StrokeColor,
        Self::BorderStyle,
        Self::Alignment,
        Self::CharLimit,
        Self::Flag(TextFlag::Multiline),
        Self::Flag(TextFlag::Password),
        Self::Flag(TextFlag::DoNotScroll),
        Self::Flag(TextFlag::Comb),
        Self::Page,
        Self::Rect,
        Self::Doc,
        Self::Style,
        Self::LineWidth,
        Self::TextSize,
        Self::TextFont,
    ];

    /// The reference's spelling.
    fn name(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::ValueAsString => "valueAsString",
            Self::Type => "type",
            Self::Display => "display",
            Self::Hidden => "hidden",
            Self::ReadOnly => "readonly",
            Self::Required => "required",
            Self::TextColor => "textColor",
            Self::FillColor => "fillColor",
            Self::StrokeColor => "strokeColor",
            Self::BorderStyle => "borderStyle",
            Self::Alignment => "alignment",
            Self::CharLimit => "charLimit",
            Self::Flag(flag) => flag.adobe(),
            Self::Page => "page",
            Self::Rect => "rect",
            Self::Doc => "doc",
            Self::Style => "style",
            Self::LineWidth => "lineWidth",
            Self::TextSize => "textSize",
            Self::TextFont => "textFont",
        }
    }
}

/// The field name a `Field` object carries, read from its own `name`.
pub(super) fn field_name(this: &JsValue, context: &mut Context) -> JsResult<String> {
    let Some(object) = this.as_object() else {
        return Err(JsNativeError::typ()
            .with_message("a Field property was read from something that is not a Field")
            .into());
    };
    Ok(object
        .get(JsString::from("name"), context)?
        .to_string(context)?
        .to_std_string_lossy())
}

/// One property of a field, read from the realm's table: the first terminal field's, for a name
/// that stands for a subtree, and a widget-level member from the widget the `Field` stands for —
/// its first, for a `Field` of every widget, as the reference's "Field" page has it (ADR 1664).
fn read_property(
    property: FieldProperty,
    this: &JsValue,
    context: &mut Context,
) -> JsResult<JsValue> {
    let name = field_name(this, context)?;
    if property == FieldProperty::Doc {
        return Ok(JsValue::from(context.global_object()));
    }
    let widget = widget_of(this, context)?;
    let Some(first) = terminals(context, &name).into_iter().next() else {
        return Ok(JsValue::undefined());
    };
    let Some(state) = State::table(context, |table| table.fields.get(&first).cloned()).flatten()
    else {
        return Ok(JsValue::undefined());
    };
    let text = |text: &str| JsValue::from(JsString::from(text));
    let shown = state.widget(widget).cloned().unwrap_or_default();
    Ok(match property {
        FieldProperty::Value => value_of(&state, context),
        FieldProperty::ValueAsString => text(&state.value),
        FieldProperty::Type => text(state.kind.adobe()),
        FieldProperty::Display => JsValue::from(shown.display.number()),
        FieldProperty::Hidden => JsValue::from(shown.display == Display::Hidden),
        FieldProperty::ReadOnly => JsValue::from(state.flags & READ_ONLY != 0),
        FieldProperty::Required => JsValue::from(state.flags & REQUIRED != 0),
        FieldProperty::TextColor => {
            colour_array(shown.text_color.unwrap_or(Colour::Gray(0.0)), context)
        }
        FieldProperty::FillColor => {
            colour_array(shown.fill_color.unwrap_or(Colour::Transparent), context)
        }
        FieldProperty::StrokeColor => {
            colour_array(shown.stroke_color.unwrap_or(Colour::Transparent), context)
        }
        FieldProperty::BorderStyle => text(shown.border_style.adobe()),
        FieldProperty::Alignment => text(shown.alignment.adobe()),
        FieldProperty::CharLimit => JsValue::from(state.char_limit.unwrap_or(0)),
        FieldProperty::Flag(flag) => JsValue::from(state.flags & flag.bit() != 0),
        FieldProperty::Page => state.page.map_or_else(|| JsValue::from(-1), JsValue::from),
        FieldProperty::Rect => {
            // The reference's rectangle is upper-left then lower-right; Table 166's `/Rect` is any
            // two opposite corners, normalised here.
            let [x0, y0, x1, y1] = shown.rect;
            let corners = [x0.min(x1), y0.max(y1), x0.max(x1), y0.min(y1)];
            JsValue::from(JsArray::from_iter(
                corners.into_iter().map(JsValue::from),
                context,
            ))
        }
        FieldProperty::Doc => JsValue::from(context.global_object()),
        // The style whose code the widget's normal caption holds; a caption that is none of the
        // six's codes is no style the reference names, and reads as `undefined` (ADR 1665).
        FieldProperty::Style => shown
            .captions
            .first()
            .and_then(|caption| Glyph::of_caption(caption))
            .map_or_else(JsValue::undefined, |glyph| text(glyph.adobe())),
        FieldProperty::LineWidth => JsValue::from(shown.line_width),
        // A `/DA` with no `Tf` states no size, which is §12.7.4.3's auto-size as well as the
        // reference's zero.
        FieldProperty::TextSize => JsValue::from(shown.text_size.unwrap_or(0.0)),
        FieldProperty::TextFont => text(&shown.text_font),
    })
}

/// `Field.value` as the reference reads it: a number where a text or combo-box field's text is
/// one, the text otherwise — the reference's documented behaviour, which is why its own library
/// reads values through `AFMakeNumber`.
fn value_of(state: &FieldState, context: &mut Context) -> JsValue {
    let text = JsValue::from(JsString::from(state.value.as_str()));
    if matches!(state.kind, FieldType::Text | FieldType::ComboBox)
        && !state.value.trim().is_empty()
        && let Ok(number) = text.to_number(context)
        && number.is_finite()
    {
        return JsValue::from(number);
    }
    text
}

/// Writes one property of a field — every terminal field a subtree's name stands for — into the
/// realm's table, and records each edit: a widget-level member on the one widget a `Field` of one
/// widget stands for, every other member on the field (ADR 1664).
fn write_property(
    property: FieldProperty,
    this: &JsValue,
    value: &JsValue,
    context: &mut Context,
) -> JsResult<()> {
    let name = field_name(this, context)?;
    let widget = widget_of(this, context)?;
    let member = property.name();
    let refused = |why: &str, context: &mut Context| {
        refuse(
            format!("Field.{member}="),
            RefusalKind::Unreachable(why.to_owned()),
            context,
        )
    };
    let change = match property {
        FieldProperty::Value => return write_value(&name, value, context),
        FieldProperty::Display => {
            let number = value.to_number(context)?;
            let Some(display) = Display::from_number(number) else {
                return Err(refused(
                    "display takes one of the display constants, 0 to 3",
                    context,
                ));
            };
            Property::Display(display)
        }
        FieldProperty::Hidden => Property::Display(if value.to_boolean() {
            Display::Hidden
        } else {
            Display::Visible
        }),
        FieldProperty::ReadOnly => Property::ReadOnly(value.to_boolean()),
        FieldProperty::Required => Property::Required(value.to_boolean()),
        FieldProperty::TextColor | FieldProperty::FillColor | FieldProperty::StrokeColor => {
            let Some(colour) = colour_of(value, context)? else {
                return Err(refused(
                    "a colour is an array of a colour space's name and its components: [\"T\"], \
                     [\"G\", g], [\"RGB\", r, g, b] or [\"CMYK\", c, m, y, k]",
                    context,
                ));
            };
            match property {
                FieldProperty::TextColor => Property::TextColor(colour),
                FieldProperty::FillColor => Property::FillColor(colour),
                _ => Property::StrokeColor(colour),
            }
        }
        FieldProperty::BorderStyle => {
            let text = value.to_string(context)?.to_std_string_lossy();
            let Some(style) = BorderStyle::from_adobe(&text) else {
                return Err(refused(
                    "borderStyle takes one of the border constants",
                    context,
                ));
            };
            Property::BorderStyle(style)
        }
        FieldProperty::Alignment => {
            let text = value.to_string(context)?.to_std_string_lossy();
            let Some(alignment) = Alignment::from_adobe(&text) else {
                return Err(refused(
                    "alignment takes \"left\", \"center\" or \"right\"",
                    context,
                ));
            };
            Property::Alignment(alignment)
        }
        FieldProperty::CharLimit => {
            let limit = value.to_length(context)?;
            Property::CharLimit(u32::try_from(limit).unwrap_or(u32::MAX))
        }
        FieldProperty::Style => style_of(&name, value, context)?,
        FieldProperty::LineWidth | FieldProperty::TextSize | FieldProperty::TextFont => {
            typographic(property, value, context)?
        }
        FieldProperty::ValueAsString
        | FieldProperty::Type
        | FieldProperty::Page
        | FieldProperty::Rect
        | FieldProperty::Doc => {
            return Err(refused("the reference makes it read-only", context));
        }
        FieldProperty::Flag(flag) => return write_flag(&name, flag, value.to_boolean(), context),
    };
    let widget = widget.filter(|_| change.is_widget_level());
    for field in terminals(context, &name) {
        let edit = ScriptEdit::Property {
            field: field.clone(),
            widget,
            property: change.clone(),
        };
        let applied = change.clone();
        State::edit(context, &field, edit, move |state| {
            state.apply(widget, &applied);
        });
    }
    Ok(())
}

/// `lineWidth`, `textSize` or `textFont` set: the property each writes, or the refusal of a value
/// the reference's "Field properties" page does not admit (ADR 1762).
fn typographic(
    property: FieldProperty,
    value: &JsValue,
    context: &mut Context,
) -> JsResult<Property> {
    let refused = |why: &str, context: &mut Context| {
        refuse(
            format!("Field.{}=", property.name()),
            RefusalKind::Unreachable(why.to_owned()),
            context,
        )
    };
    Ok(match property {
        // Any integer, 0 for none, widths past 5 distorting the field; a negative or non-finite
        // width is no width at all.
        FieldProperty::LineWidth => {
            let width = value.to_number(context)?;
            if !width.is_finite() || width < 0.0 {
                return Err(refused(
                    "lineWidth takes a width in points, 0 or more",
                    context,
                ));
            }
            Property::LineWidth(width)
        }
        // 0 to 32767 inclusive, 0 the auto-size.
        FieldProperty::TextSize => {
            let size = value.to_number(context)?;
            if !(0.0..=32767.0).contains(&size) {
                return Err(refused("textSize takes a size from 0 to 32767", context));
            }
            Property::TextSize(size)
        }
        // The view state finds the font among the form's `/DR` resources, by the name given or
        // the `/BaseFont` it names; the realm reads back what it wrote.
        _ => Property::TextFont(pdf_model::view::FontName {
            base: value.to_string(context)?.to_std_string_lossy(),
            resource: String::new(),
        }),
    })
}

/// `Field.style` set: one of the style constants, on a field the name stands for that is a check
/// box or a radio button, whose Table 192 `/CA` is the glyph (ADR 1665).
fn style_of(name: &str, value: &JsValue, context: &mut Context) -> JsResult<Property> {
    let refused = |why: &str, context: &mut Context| {
        refuse(
            "Field.style=".to_owned(),
            RefusalKind::Unreachable(why.to_owned()),
            context,
        )
    };
    let text = value.to_string(context)?.to_std_string_lossy();
    let Some(glyph) = Glyph::from_adobe(&text) else {
        return Err(refused("style takes one of the style constants", context));
    };
    for field in terminals(context, name) {
        let kind = State::table(context, |table| {
            table.fields.get(&field).map(|state| state.kind)
        })
        .flatten();
        if !matches!(kind, Some(FieldType::CheckBox | FieldType::RadioButton)) {
            return Err(refused(
                "style is a check box's or a radio button's glyph, and Table 192's /CA is that \
                 glyph only for a toggling button",
                context,
            ));
        }
    }
    Ok(Property::Style(glyph))
}

/// `Field.multiline`, `password`, `doNotScroll` or `comb` set: Table 231's flag on every terminal
/// field the name stands for, each a text field.
///
/// `comb` is held to the clause that defines it — the flag "[m]ay be set only if the `MaxLen`
/// entry is present in the text field dictionary … and if the Multiline, Password, and
/// `FileSelect` flags are clear" — and setting it sets `doNotScroll` too, the side effect the
/// reference's "Field properties" page states (ADR 1615). A refused write changes no field.
fn write_flag(name: &str, flag: TextFlag, on: bool, context: &mut Context) -> JsResult<()> {
    let member = flag.adobe();
    let refused = |why: String, context: &mut Context| {
        refuse(
            format!("Field.{member}="),
            RefusalKind::Unreachable(why),
            context,
        )
    };
    let fields = terminals(context, name);
    let states: Vec<FieldState> = State::table(context, |table| {
        fields
            .iter()
            .filter_map(|field| table.fields.get(field).cloned())
            .collect()
    })
    .unwrap_or_default();
    for state in &states {
        if state.kind != FieldType::Text {
            return Err(refused(
                format!(
                    "{member} is a text field's flag, one of Table 231's, and {} is a {} field",
                    state.name,
                    state.kind.adobe()
                ),
                context,
            ));
        }
        let excluding = TextFlag::Multiline.bit() | TextFlag::Password.bit() | FILE_SELECT;
        if flag == TextFlag::Comb
            && on
            && (state.flags & excluding != 0 || state.char_limit.is_none())
        {
            return Err(refused(
                format!(
                    "Table 231 lets comb be set only where the field states a /MaxLen and its \
                     multiline, password and file-select flags are clear, and {} does not meet that",
                    state.name
                ),
                context,
            ));
        }
    }
    for field in fields {
        let mut set = vec![flag];
        if flag == TextFlag::Comb && on {
            set.push(TextFlag::DoNotScroll);
        }
        for flag in set {
            let property = Property::TextFlag(flag, on);
            let edit = ScriptEdit::Property {
                field: field.clone(),
                widget: None,
                property: property.clone(),
            };
            State::edit(context, &field, edit, |state| state.apply(None, &property));
        }
    }
    Ok(())
}

/// `field.getArray()`: the terminal fields below this one, each a `Field` — the field itself where
/// it is terminal, a documented choice where the reference speaks only of a parent's terminal
/// children (ADR 1615).
fn get_array(this: &JsValue, _arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let name = field_name(this, context)?;
    let fields: Vec<JsValue> = terminals(context, &name)
        .iter()
        .filter_map(|field| field_object(context, field))
        .map(JsValue::from)
        .collect();
    Ok(JsValue::from(JsArray::from_iter(fields, context)))
}

/// `field.setFocus()`: the keyboard focus asked for on this field — its first terminal field, for a
/// name that stands for a subtree — as an edit the host carries out (ADR 1615).
///
/// The reference makes `setFocus` a widget's, so a `Field` of one widget asks for that widget and
/// a `Field` of every widget for the first; the host focuses the widget the edit names (ADR 1688).
fn set_focus(this: &JsValue, _arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let name = field_name(this, context)?;
    let widget = widget_of(this, context)?;
    if let Some(field) = terminals(context, &name).into_iter().next() {
        State::note(context, ScriptEdit::Focus { field, widget });
    }
    Ok(JsValue::undefined())
}

/// `Field.value = …`: every terminal field the name stands for takes the value's text.
///
/// A keystroke, format or validate script's own field is refused, because those three change their
/// field through `event.value`.
pub(super) fn write_value(name: &str, value: &JsValue, context: &mut Context) -> JsResult<()> {
    let (own, site) =
        State::with(context, |record| (record.field.clone(), record.site)).unwrap_or_default();
    if own == name
        && matches!(
            site,
            Some(ScriptSite::Field(
                Trigger::Keystroke | Trigger::Format | Trigger::Validate
            ))
        )
    {
        return Err(refuse(
            "Field.value=".to_owned(),
            RefusalKind::Unreachable(
                "a keystroke, format or validate script changes its own field through \
                 event.value (ADR 1603)"
                    .to_owned(),
            ),
            context,
        ));
    }
    let text = value.to_string(context)?.to_std_string_lossy();
    for field in terminals(context, name) {
        let edit = ScriptEdit::Value {
            field: field.clone(),
            value: text.clone(),
        };
        let written = text.clone();
        State::edit(context, &field, edit, move |state| state.value = written);
    }
    Ok(())
}

/// A colour as the reference's array.
fn colour_array(colour: Colour, context: &mut Context) -> JsValue {
    let mut items = Vec::new();
    let space = |name: &str| JsValue::from(JsString::from(name));
    match colour {
        Colour::Transparent => items.push(space("T")),
        Colour::Gray(gray) => items.extend([space("G"), JsValue::from(gray)]),
        Colour::Rgb(rgb) => {
            items.push(space("RGB"));
            items.extend(rgb.into_iter().map(JsValue::from));
        }
        Colour::Cmyk(cmyk) => {
            items.push(space("CMYK"));
            items.extend(cmyk.into_iter().map(JsValue::from));
        }
    }
    JsValue::from(JsArray::from_iter(items, context))
}

/// A colour from the reference's array, `None` where it is not one.
fn colour_of(value: &JsValue, context: &mut Context) -> JsResult<Option<Colour>> {
    let Some(array) = value.as_object().filter(JsObject::is_array) else {
        return Ok(None);
    };
    let space = array
        .get(0, context)?
        .to_string(context)?
        .to_std_string_lossy();
    let count = match space.as_str() {
        "T" => 0,
        "G" => 1,
        "RGB" => 3,
        "CMYK" => 4,
        _ => return Ok(None),
    };
    let mut components = [0.0_f64; 4];
    for (index, slot) in components.iter_mut().enumerate().take(count) {
        *slot = array
            .get(index.saturating_add(1), context)?
            .to_number(context)?;
    }
    let [first, second, third, _] = components;
    Ok(Some(match count {
        0 => Colour::Transparent,
        1 => Colour::Gray(first),
        3 => Colour::Rgb([first, second, third]),
        _ => Colour::Cmyk(components),
    }))
}

/// Reads the event back into `outcome` after a run that finished.
///
/// The event's properties are data properties that cannot be redefined, so reading them runs no
/// script; a value the script left as an object is not converted, because converting it would.
/// `value` is read back at a field's trigger, `change` at a keystroke; no other site takes either
/// back.
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
    let ScriptSite::Field(trigger) = request.site else {
        return;
    };
    let mut taken = vec![("value", &request.event.value, &mut outcome.value)];
    if trigger == Trigger::Keystroke {
        taken.push(("change", &request.event.change, &mut outcome.change));
    }
    let mut refusals = Vec::new();
    for (name, was, into) in taken {
        let Some(left) = read(name, context) else {
            continue;
        };
        if left.is_object() {
            refusals.push(crate::Refusal {
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
    outcome.refusals.extend(refusals);
}

/// The current run's `event`, as the global object holds it.
fn current_event(context: &mut Context) -> JsResult<JsObject> {
    context
        .global_object()
        .get(JsString::from("event"), context)?
        .as_object()
        .ok_or_else(|| {
            JsNativeError::typ()
                .with_message("the library was called with no event")
                .into()
        })
}

/// One `AF*` function called from a script: its arguments read as `pdf_model::aform`'s literals and
/// the call run against the event, or a helper answered directly.
fn call_library(
    library: Function,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let event = current_event(context)?;
    if let Some(answer) = helper(library, arguments, &event, context)? {
        return Ok(answer);
    }
    let call = Call {
        function: library,
        arguments: literals(library, arguments, context)?,
    };
    let state = EventText::read(&event, context)?;
    let refused = |sentence: &str, context: &mut Context| {
        refuse(
            library.name().to_owned(),
            RefusalKind::Library(sentence.to_owned()),
            context,
        )
    };
    match library.trigger() {
        Some(Trigger::Format) => match call.format(&state.value) {
            Ok(formatted) => set(&event, "value", &formatted.text, context)?,
            Err(refusal) => return Err(refused(refusal.sentence(), context)),
        },
        Some(Trigger::Keystroke) => match call.keystroke(&state.keystroke()) {
            Ok(Keyed::Accepted { value: Some(value) }) => set(&event, "value", &value, context)?,
            Ok(Keyed::Accepted { value: None }) => {}
            Ok(Keyed::Rejected { message }) => {
                reject(&event, library, message.as_deref(), context)?;
            }
            Err(refusal) => return Err(refused(refusal.sentence(), context)),
        },
        Some(Trigger::Validate) => match call.validate(&state.value) {
            Ok(Some(message)) => reject(&event, library, Some(&message), context)?,
            Ok(None) => {}
            Err(refusal) => return Err(refused(refusal.sentence(), context)),
        },
        Some(Trigger::Calculate) => {
            let result = {
                let mut values = |listed: &str| -> Vec<String> {
                    let names = terminals(context, listed);
                    State::table(context, |table| {
                        names
                            .iter()
                            .filter_map(|name| table.fields.get(name))
                            .map(|field| field.value.clone())
                            .collect()
                    })
                    .unwrap_or_default()
                };
                call.calculate(&mut values)
            };
            match result {
                Ok(total) => set(&event, "value", &total, context)?,
                Err(refusal) => return Err(refused(refusal.sentence(), context)),
            }
        }
        None => {
            return Err(refuse(
                library.name().to_owned(),
                RefusalKind::Unreachable("it is a helper with no trigger of its own".to_owned()),
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
pub(super) fn data(
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

/// Defines `holder[name]` as an accessor that cannot be redefined.
pub(super) fn accessor(
    holder: &JsObject,
    name: &str,
    getter: JsObject,
    setter: JsObject,
    context: &mut Context,
) -> JsResult<()> {
    holder.define_property_or_throw(
        JsString::from(name),
        PropertyDescriptor::builder()
            .get(getter)
            .set(setter)
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
pub(super) fn function(context: &Context, name: &str, native: NativeFunction) -> JsObject {
    FunctionObjectBuilder::new(context.realm(), native)
        .name(JsString::from(name))
        .build()
        .into()
}

/// Defines every member [`crate::surface`] lists for `holder` on `object`, each an accessor that
/// throws on read and on write.
pub(super) fn refusers(object: &JsObject, holder: Holder, context: &mut Context) -> JsResult<()> {
    let prefix = holder.prefix();
    for row in EXCLUDED.iter().filter(|row| row.holder == holder) {
        for member in row.members {
            refuser(
                object,
                prefix,
                member,
                Some(Refused::Excluded(row.reason)),
                context,
            )?;
        }
    }
    for row in REFUSED.iter().filter(|row| row.holder == holder) {
        for member in row.members {
            refuser(
                object,
                prefix,
                member,
                Some(Refused::Kept(row.reason)),
                context,
            )?;
        }
    }
    for (_, members) in NOT_BRIDGED.iter().filter(|(listed, _)| *listed == holder) {
        for member in *members {
            refuser(object, prefix, member, None, context)?;
        }
    }
    Ok(())
}

/// Why a member [`crate::surface`] lists is refused: Tier 2's reason, or Tier 1's member this
/// program keeps out with a reason of its own (ADR 1724).
#[derive(Debug, Clone, Copy)]
pub(super) enum Refused {
    /// RFC 0008 section 4.3's row.
    Excluded(&'static str),
    /// [`crate::surface::REFUSED`]'s row.
    Kept(&'static str),
}

/// One refused member: a getter and a setter that each throw its `NotAllowedError` — Tier 2's or
/// a kept member's where `reason` says which, one not yet bridged where it is `None`.
pub(super) fn refuser(
    object: &JsObject,
    prefix: &'static str,
    member: &'static str,
    reason: Option<Refused>,
    context: &mut Context,
) -> JsResult<()> {
    let throws = move |_this: &JsValue, _arguments: &[JsValue], context: &mut Context| {
        let kind = match reason {
            None => RefusalKind::NotBridged,
            Some(Refused::Excluded(reason)) => RefusalKind::Excluded(reason.to_owned()),
            Some(Refused::Kept(reason)) => RefusalKind::Unreachable(format!("it {reason}")),
        };
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
