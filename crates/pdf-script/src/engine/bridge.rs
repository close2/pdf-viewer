//! The host object model a document's realm runs against (ADRs 1591, 1602, 1603).
//!
//! What is carried: the document as the global object — `getField` of any field, `getNthFieldName`,
//! `numFields`, `calculateNow`, `resetForm`, `pageNum` and `numPages`; a `Field` for every field the
//! realm was told of, with the properties RFC 0008 section 4.2 admits for a field's value and its
//! appearance (`value`, `valueAsString`, `name`, `type`, `display`, `hidden`, `readonly`,
//! `required`, `textColor`, `fillColor`, `strokeColor`, `borderStyle`, `alignment`, `charLimit`,
//! the flags `multiline`, `password`, `comb`, `doNotScroll`, and `page`, `rect`, `doc`); `event`
//! with what each site raises; the reference's `display`, `border` and `color` constants;
//! `console.println`; and the `AF*` library, each function a native that hands its arguments to
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

use boa_engine::object::builtins::JsArray;
use boa_engine::object::{FunctionObjectBuilder, ObjectInitializer};
use boa_engine::property::PropertyDescriptor;
use boa_engine::{Context, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::aform::{
    Call, Function, Keyed, Keystroke, Literal, Trigger, extract_nums, make_number, merge_change,
    parse_date,
};
use pdf_model::view::{
    Alignment, BorderStyle, Colour, Display, FieldState, FieldType, Property, ScriptEdit,
    ScriptSite,
};

use super::{State, guard, refuse};
use crate::request::byte_offset;
use crate::surface::{EXCLUDED, Holder, NOT_BRIDGED};
use crate::{Outcome, RefusalKind, Request};

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
/// The flags a script reads and this bridge does not let it write, with Table 231's bit for each.
const READ_FLAGS: [(&str, u32); 4] = [
    ("multiline", 1 << 12),
    ("password", 1 << 13),
    ("doNotScroll", 1 << 23),
    ("comb", 1 << 24),
];

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
    let target = field_object(context, &request.field).map_or_else(JsValue::null, JsValue::from);
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
        ("target", target, false),
        ("source", source, false),
        ("targetName", text(target_name), false),
        ("name", text(name), false),
        ("type", text(kind), false),
    ] {
        data(&event, key, value, writable, context)?;
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
        ("pageNum", Document::Page),
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
                        "the document's page count, field count and page are the document's and \
                         the viewer's to state, and a script reads them"
                            .to_owned(),
                    ),
                    context,
                ))
            }),
        );
        accessor(&global, name, getter, setter, context)?;
    }
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
    Ok(())
}

/// The reference's `display`, `border` and `color` objects: constants a script compares and
/// assigns, and `color.equal`.
fn constants(context: &mut Context) -> JsResult<()> {
    let global = context.global_object();
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

/// `this.getField(cName)`: the field of that name, or of every field below it, or `null`.
///
/// A name that is not a terminal field but has fields below it answers a `Field` for the whole
/// subtree, which is how the reference lets a script hide a group; a name nothing answers to is
/// `null`, as the reference has it.
fn get_field(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let name = arguments
        .first()
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_lossy();
    Ok(field_object(context, &name).map_or_else(JsValue::null, JsValue::from))
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

/// The terminal fields a name stands for: itself where the realm knows it, every field below it
/// where it does not.
fn terminals(context: &Context, name: &str) -> Vec<String> {
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
    /// One of [`READ_FLAGS`], by index.
    Flag(usize),
    /// `page`.
    Page,
    /// `rect`.
    Rect,
    /// `doc`.
    Doc,
}

impl FieldProperty {
    /// Every property, in the order the prototype defines them.
    const ALL: [Self; 20] = [
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
        Self::Flag(0),
        Self::Flag(1),
        Self::Flag(2),
        Self::Flag(3),
        Self::Page,
        Self::Rect,
        Self::Doc,
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
            Self::Flag(index) => READ_FLAGS.get(index).map_or("flag", |(name, _)| name),
            Self::Page => "page",
            Self::Rect => "rect",
            Self::Doc => "doc",
        }
    }
}

/// The field name a `Field` object carries, read from its own `name`.
fn field_name(this: &JsValue, context: &mut Context) -> JsResult<String> {
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
/// that stands for a subtree.
fn read_property(
    property: FieldProperty,
    this: &JsValue,
    context: &mut Context,
) -> JsResult<JsValue> {
    let name = field_name(this, context)?;
    if property == FieldProperty::Doc {
        return Ok(JsValue::from(context.global_object()));
    }
    let Some(first) = terminals(context, &name).into_iter().next() else {
        return Ok(JsValue::undefined());
    };
    let Some(state) = State::table(context, |table| table.fields.get(&first).cloned()).flatten()
    else {
        return Ok(JsValue::undefined());
    };
    let text = |text: &str| JsValue::from(JsString::from(text));
    Ok(match property {
        FieldProperty::Value => value_of(&state, context),
        FieldProperty::ValueAsString => text(&state.value),
        FieldProperty::Type => text(state.kind.adobe()),
        FieldProperty::Display => JsValue::from(state.display.number()),
        FieldProperty::Hidden => JsValue::from(state.display == Display::Hidden),
        FieldProperty::ReadOnly => JsValue::from(state.flags & READ_ONLY != 0),
        FieldProperty::Required => JsValue::from(state.flags & REQUIRED != 0),
        FieldProperty::TextColor => {
            colour_array(state.text_color.unwrap_or(Colour::Gray(0.0)), context)
        }
        FieldProperty::FillColor => {
            colour_array(state.fill_color.unwrap_or(Colour::Transparent), context)
        }
        FieldProperty::StrokeColor => {
            colour_array(state.stroke_color.unwrap_or(Colour::Transparent), context)
        }
        FieldProperty::BorderStyle => text(state.border_style.adobe()),
        FieldProperty::Alignment => text(state.alignment.adobe()),
        FieldProperty::CharLimit => JsValue::from(state.char_limit.unwrap_or(0)),
        FieldProperty::Flag(index) => JsValue::from(
            READ_FLAGS
                .get(index)
                .is_some_and(|(_, bit)| state.flags & bit != 0),
        ),
        FieldProperty::Page => state.page.map_or_else(|| JsValue::from(-1), JsValue::from),
        FieldProperty::Rect => {
            // The reference's rectangle is upper-left then lower-right; Table 166's `/Rect` is any
            // two opposite corners, normalised here.
            let [x0, y0, x1, y1] = state.rect;
            let corners = [x0.min(x1), y0.max(y1), x0.max(x1), y0.min(y1)];
            JsValue::from(JsArray::from_iter(
                corners.into_iter().map(JsValue::from),
                context,
            ))
        }
        FieldProperty::Doc => JsValue::from(context.global_object()),
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
/// realm's table, and records each edit.
fn write_property(
    property: FieldProperty,
    this: &JsValue,
    value: &JsValue,
    context: &mut Context,
) -> JsResult<()> {
    let name = field_name(this, context)?;
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
        FieldProperty::ValueAsString
        | FieldProperty::Type
        | FieldProperty::Page
        | FieldProperty::Rect
        | FieldProperty::Doc => {
            return Err(refused("the reference makes it read-only", context));
        }
        FieldProperty::Flag(_) => {
            return Err(refuse(
                format!("Field.{member}="),
                RefusalKind::NotBridged,
                context,
            ));
        }
    };
    for field in terminals(context, &name) {
        let edit = ScriptEdit::Property {
            field: field.clone(),
            property: change,
        };
        State::edit(context, &field, edit, |state| apply(state, change));
    }
    Ok(())
}

/// `Field.value = …`: every terminal field the name stands for takes the value's text.
///
/// A keystroke, format or validate script's own field is refused, because those three change their
/// field through `event.value`.
fn write_value(name: &str, value: &JsValue, context: &mut Context) -> JsResult<()> {
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

/// A property set, over the realm's record of a field.
fn apply(state: &mut FieldState, property: Property) {
    let set = |flags: u32, bit: u32, on: bool| if on { flags | bit } else { flags & !bit };
    match property {
        Property::Display(display) => state.display = display,
        Property::ReadOnly(on) => state.flags = set(state.flags, READ_ONLY, on),
        Property::Required(on) => state.flags = set(state.flags, REQUIRED, on),
        Property::TextColor(colour) => state.text_color = Some(colour),
        Property::FillColor(colour) => state.fill_color = Some(colour),
        Property::StrokeColor(colour) => state.stroke_color = Some(colour),
        Property::BorderStyle(style) => state.border_style = style,
        Property::Alignment(alignment) => state.alignment = alignment,
        Property::CharLimit(limit) => state.char_limit = Some(limit),
    }
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

/// Defines `holder[name]` as an accessor that cannot be redefined.
fn accessor(
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
