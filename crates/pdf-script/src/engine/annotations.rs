//! `this.getAnnots`, `this.getAnnot`, `this.syncAnnotScan` and the `Annotation` object (ADR 1700).
//!
//! Each member's meaning is Adobe's *JavaScript for Acrobat API Reference* — "Doc methods" for the
//! three calls, "Annotation" and "Annotation properties" for the object — cited and never quoted,
//! a documented choice each under principle 5. What each property reads is the entry ISO 32000-2
//! states for it, which `pdf_model::view::AnnotationState` names; the realm holds the document's
//! annotations as the view state told them, and a write is an edit the view state makes as a
//! person's would be made.
//!
//! The choices, each named where it is made:
//!
//! - **`getAnnots` answers in page order**, each page's annotations in its `/Annots` order with a
//!   person's additions after them — the order §12.5.2 draws them in, which is what the
//!   reference's default `ANSB_None` leaves unsorted. `ANSB_Page` is the same order; the other
//!   three sorts are stable over it.
//! - **The sort and filter constants are numbered from zero in the order the reference lists
//!   them**; the reference names them and publishes no number, so a script that spells the name
//!   reads the same constant either way.
//! - **Only `ANFB_ShouldNone` filters**: the other six are Acrobat's rules for its comments panel,
//!   summary and export, which the reference names and does not define; each is refused by name.
//! - **`syncAnnotScan()` does nothing**: the reference makes it wait for a background scan of
//!   every page, and a realm is told every page's annotations before its first script runs.
//! - **Three properties are written, the three a person's own edit reaches** (RFC 0008 section
//!   4.2): `hidden`, `popupOpen` and `contents` — the last on a free text annotation alone, whose
//!   `/Contents` a person retypes. Every other property refuses a write by name.

use boa_engine::object::ObjectInitializer;
use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::view::{AnnotationChange, AnnotationState, ScriptEdit};

use super::bridge::{accessor, data, function, integral};
use super::members::named;
use super::{State, refuse};
use crate::RefusalKind;

/// The reference's sort methods for `getAnnots`, in its order: each one's place is its number.
const SORTS: [&str; 5] = [
    "ANSB_None",
    "ANSB_Page",
    "ANSB_Author",
    "ANSB_ModDate",
    "ANSB_Type",
];

/// The reference's filters for `getAnnots`, in its order: each one's place is its number.
const FILTERS: [&str; 7] = [
    "ANFB_ShouldNone",
    "ANFB_ShouldPrint",
    "ANFB_ShouldView",
    "ANFB_ShouldEdit",
    "ANFB_ShouldAppearInPanel",
    "ANFB_ShouldSummarize",
    "ANFB_ShouldExport",
];

/// Installs the three calls and the sort and filter constants on the document.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn install(global: &JsObject, context: &mut Context) -> JsResult<()> {
    let methods: [(&str, NativeFunction); 3] = [
        ("getAnnots", NativeFunction::from_fn_ptr(get_annots)),
        ("getAnnot", NativeFunction::from_fn_ptr(get_annot)),
        (
            "syncAnnotScan",
            NativeFunction::from_fn_ptr(|_this, _arguments, _context| Ok(JsValue::undefined())),
        ),
    ];
    for (name, native) in methods {
        let callable = function(context, name, native);
        data(global, name, JsValue::from(callable), false, context)?;
    }
    for list in [&SORTS[..], &FILTERS[..]] {
        for (place, name) in (0_u32..).zip(list) {
            data(global, name, JsValue::from(place), false, context)?;
        }
    }
    Ok(())
}

/// A zero-based page argument, `None` where it is absent.
///
/// A number that names no page names no annotation, and answers as a page with none does.
fn page_argument(value: &JsValue, context: &mut Context) -> JsResult<Option<i64>> {
    if value.is_null_or_undefined() {
        return Ok(None);
    }
    Ok(Some(integral(value.to_number(context)?)))
}

/// A choice from `list` passed as its number, the first where it is absent; `None` where the
/// number names none.
fn choice(value: &JsValue, list: &[&str], context: &mut Context) -> JsResult<Option<usize>> {
    if value.is_null_or_undefined() {
        return Ok(Some(0));
    }
    let number = integral(value.to_number(context)?);
    Ok(usize::try_from(number)
        .ok()
        .filter(|place| *place < list.len()))
}

/// `this.getAnnots(nPage, nSortBy, bReverse, nFilterBy)`: the document's annotations, or one
/// page's, as `Annotation` objects; `null` where there is none.
fn get_annots(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let values = named(
        arguments,
        &["nPage", "nSortBy", "bReverse", "nFilterBy"],
        context,
    )?;
    let value = |index: usize| values.get(index).cloned().unwrap_or_default();
    let page = page_argument(&value(0), context)?;
    let Some(sort) = choice(&value(1), &SORTS, context)? else {
        return Err(refuse(
            "this.getAnnots(nSortBy)".to_owned(),
            RefusalKind::Unreachable(
                "nSortBy names none of the reference's five sort methods".to_owned(),
            ),
            context,
        ));
    };
    let reverse = value(2).to_boolean();
    match choice(&value(3), &FILTERS, context)? {
        Some(0) => {}
        filter => {
            let name = filter.and_then(|place| FILTERS.get(place)).map_or_else(
                || "a number that names none of the reference's seven".to_owned(),
                |name| (*name).to_owned(),
            );
            return Err(refuse(
                format!("this.getAnnots(nFilterBy: {name})"),
                RefusalKind::Unreachable(
                    "the reference's filters other than ANFB_ShouldNone are Acrobat's rules for \
                     its comments panel, summaries and export, which it names and does not \
                     define; getAnnots with no filter answers every annotation (ADR 1700)"
                        .to_owned(),
                ),
                context,
            ));
        }
    }
    let mut annotations: Vec<AnnotationState> = State::table(context, |table| {
        table
            .document
            .annotations
            .iter()
            .filter(|annotation| page.is_none_or(|page| i64::from(annotation.page) == page))
            .cloned()
            .collect()
    })
    .unwrap_or_default();
    match SORTS.get(sort).copied() {
        Some("ANSB_Author") => annotations.sort_by(|left, right| left.author.cmp(&right.author)),
        Some("ANSB_ModDate") => annotations.sort_by_key(|annotation| annotation.modified),
        Some("ANSB_Type") => annotations.sort_by(|left, right| left.kind.cmp(&right.kind)),
        _ => {}
    }
    if reverse {
        annotations.reverse();
    }
    if annotations.is_empty() {
        return Ok(JsValue::null());
    }
    let mut objects = Vec::with_capacity(annotations.len());
    for annotation in &annotations {
        objects.push(JsValue::from(annotation_object(annotation, context)?));
    }
    Ok(JsValue::from(JsArray::from_iter(objects, context)))
}

/// `this.getAnnot(nPage, cName)`: the annotation on that page whose Table 166 `/NM` is the name,
/// the first in the page's order where two share one; `null` where none does.
fn get_annot(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let values = named(arguments, &["nPage", "cName"], context)?;
    let page = page_argument(&values.first().cloned().unwrap_or_default(), context)?;
    let name = values
        .get(1)
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_lossy();
    let found = State::table(context, |table| {
        table
            .document
            .annotations
            .iter()
            .find(|annotation| {
                page.is_some_and(|page| i64::from(annotation.page) == page)
                    && annotation.name.as_deref() == Some(name.as_str())
            })
            .cloned()
    })
    .flatten();
    match found {
        Some(annotation) => Ok(JsValue::from(annotation_object(&annotation, context)?)),
        None => Ok(JsValue::null()),
    }
}

/// Which of an `Annotation`'s properties an accessor reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Member {
    /// `type`.
    Kind,
    /// `page`.
    Page,
    /// `rect`.
    Rect,
    /// `name`.
    Name,
    /// `contents`.
    Contents,
    /// `author`.
    Author,
    /// `modDate`.
    Modified,
    /// `hidden`.
    Hidden,
    /// `readOnly`.
    ReadOnly,
    /// `popupOpen`.
    PopupOpen,
}

impl Member {
    /// The ten, with the reference's spelling of each.
    const ALL: [(&'static str, Self); 10] = [
        ("type", Self::Kind),
        ("page", Self::Page),
        ("rect", Self::Rect),
        ("name", Self::Name),
        ("contents", Self::Contents),
        ("author", Self::Author),
        ("modDate", Self::Modified),
        ("hidden", Self::Hidden),
        ("readOnly", Self::ReadOnly),
        ("popupOpen", Self::PopupOpen),
    ];
}

/// One `Annotation` object: ten accessors, each reading the realm's record of the annotation now,
/// so that a write is read back.
fn annotation_object(annotation: &AnnotationState, context: &mut Context) -> JsResult<JsObject> {
    let object = ObjectInitializer::new(context).build();
    let id = (annotation.number, annotation.generation);
    for (name, member) in Member::ALL {
        let getter =
            function(
                context,
                name,
                NativeFunction::from_copy_closure(move |_this, _arguments, context| {
                    match State::table(context, |table| held(table, id)).flatten() {
                        Some(annotation) => read(&annotation, member, context),
                        None => Ok(JsValue::undefined()),
                    }
                }),
            );
        let setter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, arguments, context| {
                let value = arguments.first().cloned().unwrap_or_default();
                write(id, name, member, &value, context)?;
                Ok(JsValue::undefined())
            }),
        );
        accessor(&object, name, getter, setter, context)?;
    }
    Ok(object)
}

/// The realm's record of one annotation.
fn held(table: &super::Table, id: (u32, u16)) -> Option<AnnotationState> {
    table
        .document
        .annotations
        .iter()
        .find(|annotation| (annotation.number, annotation.generation) == id)
        .cloned()
}

/// One property's value.
fn read(annotation: &AnnotationState, member: Member, context: &mut Context) -> JsResult<JsValue> {
    let text = |text: &str| JsValue::from(JsString::from(text));
    Ok(match member {
        Member::Kind => text(&annotation.kind),
        Member::Page => JsValue::from(annotation.page),
        Member::Rect => {
            let corners = annotation.rect.map(JsValue::from);
            JsValue::from(JsArray::from_iter(corners, context))
        }
        Member::Name => annotation
            .name
            .as_deref()
            .map_or_else(JsValue::undefined, text),
        Member::Contents => text(&annotation.contents),
        Member::Author => annotation
            .author
            .as_deref()
            .map_or_else(JsValue::undefined, text),
        Member::Modified => match annotation.modified {
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
            None => JsValue::undefined(),
        },
        Member::Hidden => JsValue::from(annotation.hidden),
        Member::ReadOnly => JsValue::from(annotation.read_only),
        Member::PopupOpen => annotation
            .popup_open
            .map_or_else(JsValue::undefined, JsValue::from),
    })
}

/// One property written: the change recorded in the realm and handed to the view state as an
/// edit, or refused by name where a person's edit does not reach it.
fn write(
    id: (u32, u16),
    name: &'static str,
    member: Member,
    value: &JsValue,
    context: &mut Context,
) -> JsResult<()> {
    let Some(annotation) = State::table(context, |table| held(table, id)).flatten() else {
        return Ok(());
    };
    let unreachable = |reason: &str, context: &mut Context| {
        refuse(
            format!("Annotation.{name}="),
            RefusalKind::Unreachable(reason.to_owned()),
            context,
        )
    };
    let change = match member {
        Member::Hidden => AnnotationChange::Hidden(value.to_boolean()),
        Member::PopupOpen if annotation.popup_open.is_some() => {
            AnnotationChange::PopupOpen(value.to_boolean())
        }
        Member::PopupOpen => {
            return Err(unreachable(
                "the annotation has no popup window to open (ADR 1700)",
                context,
            ));
        }
        Member::Contents if annotation.kind == "FreeText" => {
            AnnotationChange::Contents(value.to_string(context)?.to_std_string_lossy())
        }
        Member::Contents => {
            return Err(unreachable(
                "a reader's edit retypes the contents of a free text annotation alone, and a \
                 script reaches what a reader's edit does (ADR 1700)",
                context,
            ));
        }
        _ => {
            return Err(unreachable(
                "a reader's edit of an annotation reaches its hidden, popupOpen and a free text \
                 annotation's contents, and a script reaches what a reader's edit does (ADR 1700)",
                context,
            ));
        }
    };
    let before = State::table(context, |table| {
        let before = table.document.annotations.clone();
        let held = table
            .document
            .annotations
            .iter_mut()
            .find(|annotation| (annotation.number, annotation.generation) == id)?;
        match &change {
            AnnotationChange::Hidden(hidden) => held.hidden = *hidden,
            AnnotationChange::PopupOpen(open) => held.popup_open = Some(*open),
            AnnotationChange::Contents(text) => held.contents.clone_from(text),
        }
        Some(before)
    })
    .flatten();
    let Some(before) = before else {
        return Ok(());
    };
    State::with(context, |record| {
        record.annotations_before.get_or_insert(before);
    });
    State::note(
        context,
        ScriptEdit::Annotation {
            number: id.0,
            generation: id.1,
            change,
        },
    );
    Ok(())
}
