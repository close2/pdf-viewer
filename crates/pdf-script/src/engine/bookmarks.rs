//! `this.bookmarkRoot` and the `Bookmark` object (ADRs 1922, 1934).
//!
//! Each member's meaning is Adobe's *JavaScript for Acrobat API Reference* — "Doc properties" for
//! `bookmarkRoot`, "Bookmark properties" and "Bookmark methods" for the object — cited and never
//! quoted, a documented choice each under principle 5. What each property reads is the entry
//! ISO 32000-2 states for it, which `pdf_model::view::BookmarkState` names: Table 151's `/Title`
//! is `name`, the sign of its `/Count` is `open`, its `/C` is `color`, and Table 152's two bits
//! are `style`.
//!
//! The choices, each named where it is made:
//!
//! - **The root is the outline dictionary**, Table 150, which states no title, no colour and no
//!   flags, and whose `/Count` "cannot be negative". So the root's `name` is the empty string, its
//!   `open` is true, its `color` Table 151's default black and its `style` 0, its `parent` null and
//!   its `doc` the document, whatever the outline holds (ADR 1922).
//! - **`children` is the outline the realm was told**, read on first use (ADR 1934): a script that
//!   reaches the outline without spelling `bookmarkRoot` meets a refusal that says so, rather than
//!   an outline that looks empty.
//! - **`open` is the file's**, the sign of `/Count`. §12.3.3 lets a person open and close items in
//!   the panel, and the panel is the host's; the realm is not told what a person did there.
//! - **Every write is refused as the document's structure** — `name`, `open`, `color` and `style`
//!   with the four methods that add, move, remove and re-script items — and `children`, `parent`
//!   and `doc` refuse as what a reader reads.
//! - **`execute()` is a person's click**: the item is handed to the host, which activates it as it
//!   activates a row of its panel (ADR 1904). The root has no action, and executing it does
//!   nothing.

use boa_engine::object::ObjectInitializer;
use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::view::{BookmarkState, Colour, ScriptEdit};

use super::bridge::{accessor, colour_array, data, function, refusers};
use super::members::read_only;
use super::{State, refuse};
use crate::RefusalKind;
use crate::surface::{EXCLUDED, Holder};

/// An outline item by its object, or the root where `None`.
type Key = Option<(u32, u16)>;

/// Which of a `Bookmark`'s properties an accessor reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Member {
    /// `children`.
    Children,
    /// `name`.
    Name,
    /// `open`.
    Open,
    /// `color`.
    Color,
    /// `style`.
    Style,
    /// `parent`.
    Parent,
}

impl Member {
    /// The six, with the reference's spelling of each.
    const ALL: [(&'static str, Self); 6] = [
        ("children", Self::Children),
        ("name", Self::Name),
        ("open", Self::Open),
        ("color", Self::Color),
        ("style", Self::Style),
        ("parent", Self::Parent),
    ];

    /// Whether a write would change what the document states of the item, rather than what a
    /// reader reads of the tree.
    fn states(self) -> bool {
        matches!(self, Self::Name | Self::Open | Self::Color | Self::Style)
    }
}

/// Installs `this.bookmarkRoot` on the document.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn install(global: &JsObject, context: &mut Context) -> JsResult<()> {
    let root = bookmark(None, context)?;
    data(global, "bookmarkRoot", JsValue::from(root), false, context)
}

/// `event.target` at an outline item's event: the item's `Bookmark`, or null where the realm was
/// not told of it — an item past the bound on the items a realm is told (ADR 1934).
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn target(number: u32, generation: u16, context: &mut Context) -> JsResult<JsValue> {
    let key = Some((number, generation));
    let told = State::table(context, |table| kids_of(table, key).is_some()).unwrap_or(false);
    if told {
        Ok(JsValue::from(bookmark(key, context)?))
    } else {
        Ok(JsValue::null())
    }
}

/// One `Bookmark` object: its six properties, each reading the realm's outline now, its `doc`,
/// `execute`, and the four methods the surface refuses.
fn bookmark(key: Key, context: &mut Context) -> JsResult<JsObject> {
    let object = ObjectInitializer::new(context).build();
    for (name, member) in Member::ALL {
        let getter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, _arguments, context| {
                read(key, member, context)
            }),
        );
        let setter = function(
            context,
            name,
            NativeFunction::from_copy_closure(move |_this, _arguments, context| {
                Err(if member.states() {
                    refuse(
                        format!("Bookmark.{name}="),
                        RefusalKind::Excluded(structure().to_owned()),
                        context,
                    )
                } else {
                    read_only(&format!("Bookmark.{name}="), context)
                })
            }),
        );
        accessor(&object, name, getter, setter, context)?;
    }
    let document = JsValue::from(context.global_object());
    data(&object, "doc", document, false, context)?;
    let execute = function(
        context,
        "execute",
        NativeFunction::from_copy_closure(move |_this, _arguments, context| {
            if let Some((number, generation)) = key {
                State::note(context, ScriptEdit::Bookmark { number, generation });
            }
            Ok(JsValue::undefined())
        }),
    );
    data(&object, "execute", JsValue::from(execute), false, context)?;
    refusers(&object, Holder::Bookmark, context)?;
    Ok(object)
}

/// RFC 0008 section 4.3's reason for the `Bookmark` methods that change the outline, which is the
/// reason for a write to what an item states.
fn structure() -> &'static str {
    EXCLUDED
        .iter()
        .find(|row| row.holder == Holder::Bookmark)
        .map_or("changes the document's structure", |row| row.reason)
}

/// The objects of an item's children, or of the root's, in the order the outline holds them;
/// `None` where the realm was not told the outline or the outline does not hold the item.
fn kids_of(table: &super::Table, key: Key) -> Option<Vec<(u32, u16)>> {
    let items = table.document.outline.as_ref()?;
    let parent = match key {
        None => None,
        Some(id) => {
            let place = items
                .iter()
                .position(|item| (item.number, item.generation) == id)?;
            Some(u32::try_from(place).unwrap_or(u32::MAX))
        }
    };
    Some(
        items
            .iter()
            .filter(|item| item.parent == parent)
            .map(|item: &BookmarkState| (item.number, item.generation))
            .collect(),
    )
}

/// One property's value.
fn read(key: Key, member: Member, context: &mut Context) -> JsResult<JsValue> {
    let text = |text: &str| JsValue::from(JsString::from(text));
    let Some(key) = key else {
        return Ok(match member {
            Member::Children => children(None, context)?,
            Member::Name => text(""),
            Member::Open => JsValue::from(true),
            Member::Color => colour_array(Colour::Rgb([0.0; 3]), context),
            Member::Style => JsValue::from(0),
            Member::Parent => JsValue::null(),
        });
    };
    let item = State::table(context, |table| {
        let items = table.document.outline.as_ref()?;
        items
            .iter()
            .find(|item| (item.number, item.generation) == key)
            .map(|item| {
                let parent = item
                    .parent
                    .and_then(|place| usize::try_from(place).ok())
                    .and_then(|place| items.get(place))
                    .map(|parent| (parent.number, parent.generation));
                (item.clone(), parent)
            })
    })
    .flatten();
    let Some((item, parent)) = item else {
        return Err(not_told(&format!("Bookmark.{}", name_of(member)), context));
    };
    Ok(match member {
        Member::Children => children(Some(key), context)?,
        Member::Name => text(&item.name),
        Member::Open => JsValue::from(item.open),
        Member::Color => colour_array(Colour::Rgb(item.colour), context),
        Member::Style => JsValue::from(item.style),
        Member::Parent => match parent {
            Some(parent) => JsValue::from(bookmark(Some(parent), context)?),
            None => root(item.parent.is_some(), context)?,
        },
    })
}

/// An item's parent where it names none the realm holds: the root for a top-level item, and null
/// for an item whose parent's place names no item — which an outline this crate's wire decoded
/// never states.
fn root(names_a_parent: bool, context: &mut Context) -> JsResult<JsValue> {
    if names_a_parent {
        return Ok(JsValue::null());
    }
    context
        .global_object()
        .get(JsString::from("bookmarkRoot"), context)
}

/// The reference's spelling of one member.
fn name_of(member: Member) -> &'static str {
    Member::ALL
        .iter()
        .find(|(_, listed)| *listed == member)
        .map_or("", |(name, _)| name)
}

/// The item's children as `Bookmark` objects, in the order the outline holds them; null where it
/// has none, as the reference has it.
fn children(key: Key, context: &mut Context) -> JsResult<JsValue> {
    let kids = State::table(context, |table| kids_of(table, key)).flatten();
    let Some(kids) = kids else {
        return Err(not_told("Bookmark.children", context));
    };
    if kids.is_empty() {
        return Ok(JsValue::null());
    }
    let mut objects = Vec::with_capacity(kids.len());
    for kid in kids {
        objects.push(JsValue::from(bookmark(Some(kid), context)?));
    }
    Ok(JsValue::from(JsArray::from_iter(objects, context)))
}

/// The refusal of a member that reads the outline where the realm was not told it.
fn not_told(member: &str, context: &mut Context) -> boa_engine::JsError {
    refuse(
        member.to_owned(),
        RefusalKind::Unreachable(
            "the realm is told the document's outline when a script spells bookmarkRoot or runs \
             at an outline item, and this document's scripts did neither before this one (ADR \
             1934)"
                .to_owned(),
        ),
        context,
    )
}
