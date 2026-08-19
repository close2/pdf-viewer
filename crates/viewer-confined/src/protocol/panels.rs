//! The eleven answers a panel is made of, as bytes.
//!
//! §12.3.3's outline, §8.11.4.3's layer order, §7.11.4's embedded files, §12.3.5's portable
//! collection, §12.4.3's article threads, §12.3.4's thumbnail, §14.3.3's document information and
//! §14.3.2's metadata beside it, Table 29's opening pair, Table 147 whole, §12.5.6.14's popups and
//! §14.7's structure. Each of them answers with a `pdf-model` type rather than with a number, which
//! is why they were the half of this boundary ADR 0218 left; this module is that half.
//!
//! # The property every encoder here holds, and how
//!
//! **A field added to one of these types must fail to compile rather than stop crossing.** The
//! parent module gets that from `match` arms over enums nobody made `#[non_exhaustive]`; a
//! *struct* has no such match, so every encoder here opens with a `let` that names every field:
//!
//! ```ignore
//! let Item { id, title, destination, open, italic, bold, colour, children } = item;
//! ```
//!
//! A pattern with no `..` is exhaustive in exactly the way an arm list is, so a field added in
//! `pdf-model` breaks this build. That is the whole reason the destructuring is written out
//! instead of the fields being read through `item.id`, and it is worth the extra line: an encoding
//! that quietly dropped a field would be a panel showing less on the confined path than off it,
//! and no gate in this tree would see it.
//!
//! # What does not cross, and where it is said
//!
//! One thing is deliberately not carried and it is named in the *type* rather than in a comment:
//! an [`crate::Attachment`] has no stream. §7.11.4's bytes cross by
//! [`viewer_core::Command::Extract`] and come back as [`viewer_core::Event::Extracted`], which is
//! a channel that already existed — and a panel that listed five attachments would otherwise have
//! pulled every one of their payloads across the pipe to draw five rows.
//!
//! Two things are refused rather than approximated, each with the variant's name in the refusal:
//! a §7.11.6 collection value whose object is not one of Table 47's three kinds — see
//! [`encode_collection_data`] — and an [`XmpError`] variant this build does not name, `XmpError`
//! being the one `#[non_exhaustive]` type in this vocabulary.

use pdf_model::article::{Bead, Thread};
use pdf_model::attachment::{Attachment as FileAttachment, Relationship};
use pdf_model::collection::{
    Collection, Colours, Field, FieldKind, Folder, Initial, Item as CollectionItem, Layout,
    Navigator, Sort, Split, SplitDirection, Value as CollectionValue, View as CollectionView,
};
use pdf_model::destination::{Destination, Target, View};
use pdf_model::form::{Choice, ChoiceControl, Control, TextControl};
use pdf_model::metadata::{Information, Trapped};
use pdf_model::outline::{Item as OutlineItem, Outline};
use pdf_model::page::Boundary;
use pdf_model::structure::HeaderScope;
use pdf_model::thumbnail::Thumbnail;
use pdf_model::view::{FieldName, ShownValue};
use pdf_model::viewer_preferences::{
    Direction, Duplex, Opening, PageLayout, PageMode, PrintScaling, ViewerPreferences,
};
use pdf_model::xmp::{Name as XmpName, Value as XmpValue, Xmp, XmpError};
use pdf_render::{Color, Image};
use pdf_syntax::{Name, Object};
use viewer_core::{
    AccessibilityNode, Character, FormField, FormWidget, Layer, PopupWindow, TextLine,
};

use super::{ProtocolError, Reader, Uncarried, Writer};
use crate::Attachment;

/// How deep a tree from the other side of the pipe is followed.
///
/// Four of these answers nest — an outline, a layer order, a folder hierarchy — and each of the
/// readers that *produces* one bounds itself well inside this: `pdf_model::outline`'s walk stops
/// at 32, and §8.11.4.3's `/Order` and §12.3.5.2's folders are bounded the same way. So this is a
/// bound on a *message*, not on a document, and reaching it means the bytes did not come from one
/// of those readers. Sixty-four stack frames of decoding is nothing; the number a hostile message
/// would otherwise choose is what this is here for.
pub(super) const MAX_TREE_DEPTH: usize = 64;

/// Refuses a tree that nests past [`MAX_TREE_DEPTH`].
fn depth(what: &'static str, at: usize) -> Result<(), ProtocolError> {
    if at > MAX_TREE_DEPTH {
        return Err(ProtocolError::TooDeep {
            what,
            limit: MAX_TREE_DEPTH,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// §12.3.3, the outline
// ---------------------------------------------------------------------------------------------

/// Encodes §12.3.3's outline, whole.
pub(super) fn encode_outline(writer: &mut Writer, outline: &Outline) {
    let Outline {
        items,
        stated_count,
    } = outline;
    writer.usize(items.len());
    for item in items {
        encode_outline_item(writer, item);
    }
    writer.option_i64(*stated_count);
}

/// Reads §12.3.3's outline.
pub(super) fn decode_outline(reader: &mut Reader<'_>) -> Result<Outline, ProtocolError> {
    let what = "an outline";
    Ok(Outline {
        items: reader.list(what, |reader| decode_outline_item(reader, 0))?,
        stated_count: reader.option_i64("an outline's stated count")?,
    })
}

fn encode_outline_item(writer: &mut Writer, item: &OutlineItem) {
    let OutlineItem {
        id,
        title,
        destination,
        open,
        italic,
        bold,
        colour,
        children,
    } = item;
    writer.object(*id).str(title);
    encode_option_destination(writer, *destination);
    writer
        .bool(*open)
        .bool(*italic)
        .bool(*bold)
        .numbers(colour)
        .usize(children.len());
    for child in children {
        encode_outline_item(writer, child);
    }
}

fn decode_outline_item(reader: &mut Reader<'_>, at: usize) -> Result<OutlineItem, ProtocolError> {
    let what = "an outline item";
    depth(what, at)?;
    Ok(OutlineItem {
        id: reader.object(what)?,
        title: reader.string("an outline item's title")?,
        destination: decode_option_destination(reader)?,
        open: reader.bool("an outline item's open flag")?,
        italic: reader.bool("an outline item's italic flag")?,
        bold: reader.bool("an outline item's bold flag")?,
        colour: reader.colour("an outline item's colour")?,
        children: reader.list(what, |reader| {
            decode_outline_item(reader, at.saturating_add(1))
        })?,
    })
}

// ---------------------------------------------------------------------------------------------
// §12.3.2's destinations, which an outline item and an article bead both reach for
// ---------------------------------------------------------------------------------------------

fn encode_option_destination(writer: &mut Writer, destination: Option<Destination>) {
    match destination {
        Some(destination) => {
            writer.u8(1);
            encode_destination(writer, destination);
        }
        None => {
            writer.u8(0);
        }
    }
}

fn decode_option_destination(
    reader: &mut Reader<'_>,
) -> Result<Option<Destination>, ProtocolError> {
    if reader.bool("a destination")? {
        Ok(Some(decode_destination(reader)?))
    } else {
        Ok(None)
    }
}

fn encode_destination(writer: &mut Writer, destination: Destination) {
    let Destination { target, view } = destination;
    match target {
        Target::Object(id) => {
            writer.u8(0).object(id);
        }
        Target::Number(number) => {
            writer.u8(1).i64(number);
        }
    }
    match view {
        View::Xyz { left, top, zoom } => {
            writer
                .u8(0)
                .option_f32(left)
                .option_f32(top)
                .option_f32(zoom);
        }
        View::Fit => {
            writer.u8(1);
        }
        View::FitH { top } => {
            writer.u8(2).option_f32(top);
        }
        View::FitV { left } => {
            writer.u8(3).option_f32(left);
        }
        View::FitR { rect } => {
            writer.u8(4).numbers(&rect);
        }
        View::FitB => {
            writer.u8(5);
        }
        View::FitBH { top } => {
            writer.u8(6).option_f32(top);
        }
        View::FitBV { left } => {
            writer.u8(7).option_f32(left);
        }
    }
}

fn decode_destination(reader: &mut Reader<'_>) -> Result<Destination, ProtocolError> {
    let what = "a destination's target";
    let target = match reader.u8(what)? {
        0 => Target::Object(reader.object(what)?),
        1 => Target::Number(reader.i64(what)?),
        value => return Err(unrecognised(what, value)),
    };
    let what = "a destination's view";
    let view = match reader.u8(what)? {
        0 => View::Xyz {
            left: reader.option_f32(what)?,
            top: reader.option_f32(what)?,
            zoom: reader.option_f32(what)?,
        },
        1 => View::Fit,
        2 => View::FitH {
            top: reader.option_f32(what)?,
        },
        3 => View::FitV {
            left: reader.option_f32(what)?,
        },
        4 => View::FitR {
            rect: reader.rect(what)?,
        },
        5 => View::FitB,
        6 => View::FitBH {
            top: reader.option_f32(what)?,
        },
        7 => View::FitBV {
            left: reader.option_f32(what)?,
        },
        value => return Err(unrecognised(what, value)),
    };
    Ok(Destination { target, view })
}

// ---------------------------------------------------------------------------------------------
// §8.11.4.3, the layer order
// ---------------------------------------------------------------------------------------------

/// Encodes §8.11.4.3's `/Order`, as a layer panel would show it.
pub(super) fn encode_layers(writer: &mut Writer, layers: &[Layer]) {
    writer.usize(layers.len());
    for layer in layers {
        encode_layer(writer, layer);
    }
}

/// Reads §8.11.4.3's `/Order`.
pub(super) fn decode_layers(reader: &mut Reader<'_>) -> Result<Vec<Layer>, ProtocolError> {
    reader.list("a layer order", |reader| decode_layer(reader, 0))
}

fn encode_layer(writer: &mut Writer, layer: &Layer) {
    match layer {
        Layer::Group {
            group,
            name,
            on,
            locked,
        } => {
            writer.u8(0).object(*group);
            writer.option_str(name.as_deref()).bool(*on).bool(*locked);
        }
        Layer::Collection { label, children } => {
            writer
                .u8(1)
                .option_str(label.as_deref())
                .usize(children.len());
            for child in children {
                encode_layer(writer, child);
            }
        }
    }
}

fn decode_layer(reader: &mut Reader<'_>, at: usize) -> Result<Layer, ProtocolError> {
    let what = "a layer";
    depth(what, at)?;
    Ok(match reader.u8(what)? {
        0 => Layer::Group {
            group: reader.object("an optional content group")?,
            name: reader.option_string("a group's name")?,
            on: reader.bool("a group's state")?,
            locked: reader.bool("a group's lock")?,
        },
        1 => Layer::Collection {
            label: reader.option_string("a layer collection's label")?,
            children: reader.list(what, |reader| decode_layer(reader, at.saturating_add(1)))?,
        },
        value => return Err(unrecognised(what, value)),
    })
}

// ---------------------------------------------------------------------------------------------
// §7.11.4, the embedded files
// ---------------------------------------------------------------------------------------------

/// Encodes §7.11.4's embedded files, as a panel lists them.
///
/// **The stream does not cross, and the asymmetry of this pair is where that is said.** What goes
/// in is [`pdf_model::attachment::Attachment`], whose last field is the bytes; what comes out is
/// [`crate::Attachment`], which has no such field. §7.11.4's payload leaves the confinement by
/// [`viewer_core::Command::Extract`] and [`viewer_core::Event::Extracted`], a channel that
/// already existed and that a host asks for one file at a time — where listing five attachments
/// this way would have pulled all five payloads across the pipe to draw five rows.
pub(super) fn encode_attachments(writer: &mut Writer, attachments: &[FileAttachment]) {
    writer.usize(attachments.len());
    for attachment in attachments {
        let FileAttachment {
            name,
            file_name,
            description,
            media_type,
            size,
            created,
            modified,
            checksum,
            relationship,
            // The one field this encoding does not carry, named so that the pattern stays
            // exhaustive: a field added to `Attachment` still fails to compile here.
            stream: _,
        } = attachment;
        writer
            .str(name)
            .option_str(file_name.as_deref())
            .option_str(description.as_deref())
            .option_str(media_type.as_deref())
            .option_i64(*size)
            .option_str(created.as_deref())
            .option_str(modified.as_deref())
            .option_bytes(checksum.as_deref());
        encode_relationship(writer, relationship);
    }
}

/// Reads §7.11.4's embedded files.
pub(super) fn decode_attachments(
    reader: &mut Reader<'_>,
) -> Result<Vec<Attachment>, ProtocolError> {
    reader.list("an attachment list", |reader| {
        Ok(Attachment {
            name: reader.string("an attachment's name")?,
            file_name: reader.option_string("an attachment's file name")?,
            description: reader.option_string("an attachment's description")?,
            media_type: reader.option_string("an attachment's media type")?,
            size: reader.option_i64("an attachment's size")?,
            created: reader.option_string("an attachment's creation date")?,
            modified: reader.option_string("an attachment's modification date")?,
            checksum: reader.option_bytes("an attachment's checksum")?,
            relationship: decode_relationship(reader)?,
        })
    })
}

/// Table 43's `/AFRelationship`, §14.13.
fn encode_relationship(writer: &mut Writer, relationship: &Relationship) {
    match relationship {
        Relationship::Source => writer.u8(0),
        Relationship::Data => writer.u8(1),
        Relationship::Alternative => writer.u8(2),
        Relationship::Supplement => writer.u8(3),
        Relationship::EncryptedPayload => writer.u8(4),
        Relationship::FormData => writer.u8(5),
        Relationship::Schema => writer.u8(6),
        Relationship::Unspecified => writer.u8(7),
        Relationship::Other(name) => writer.u8(8).str(name),
    };
}

fn decode_relationship(reader: &mut Reader<'_>) -> Result<Relationship, ProtocolError> {
    let what = "a file relationship";
    Ok(match reader.u8(what)? {
        0 => Relationship::Source,
        1 => Relationship::Data,
        2 => Relationship::Alternative,
        3 => Relationship::Supplement,
        4 => Relationship::EncryptedPayload,
        5 => Relationship::FormData,
        6 => Relationship::Schema,
        7 => Relationship::Unspecified,
        8 => Relationship::Other(reader.string(what)?),
        value => return Err(unrecognised(what, value)),
    })
}

// ---------------------------------------------------------------------------------------------
// §12.4.3, the article threads
// ---------------------------------------------------------------------------------------------

/// Encodes §12.4.3's article threads, in the `/Threads` array's own order.
pub(super) fn encode_articles(writer: &mut Writer, threads: &[Thread]) {
    writer.usize(threads.len());
    for thread in threads {
        let Thread { id, title, beads } = thread;
        writer
            .object(*id)
            .option_str(title.as_deref())
            .usize(beads.len());
        for bead in beads {
            let Bead { id, page, rect } = bead;
            writer.object(*id).option_object(*page);
            match rect {
                Some(rect) => {
                    writer.u8(1).numbers(rect);
                }
                None => {
                    writer.u8(0);
                }
            }
        }
    }
}

/// Reads §12.4.3's article threads.
pub(super) fn decode_articles(reader: &mut Reader<'_>) -> Result<Vec<Thread>, ProtocolError> {
    reader.list("an article thread list", |reader| {
        Ok(Thread {
            id: reader.object("a thread")?,
            title: reader.option_string("a thread's title")?,
            beads: reader.list("a bead list", |reader| {
                Ok(Bead {
                    id: reader.object("a bead")?,
                    page: reader.option_object("a bead's page")?,
                    rect: reader.option_rect("a bead's rectangle")?,
                })
            })?,
        })
    })
}

// ---------------------------------------------------------------------------------------------
// §12.3.5, the portable collection
// ---------------------------------------------------------------------------------------------

/// Encodes §12.3.5's collection dictionary, whole.
///
/// # Errors
///
/// [`Uncarried`] where a §7.11.6 collection value carries an object Table 47 does not describe.
pub(super) fn encode_collection(
    writer: &mut Writer,
    collection: &Collection,
) -> Result<(), Uncarried> {
    let Collection {
        schema,
        initial,
        view,
        sort,
        navigator,
        colours,
        split,
        folders,
    } = collection;

    writer.usize(schema.len());
    for (key, field) in schema {
        let Field {
            kind,
            name,
            order,
            visible,
            editable,
        } = field;
        writer.str(key);
        encode_field_kind(writer, kind);
        writer
            .str(name)
            .option_i64(*order)
            .bool(*visible)
            .bool(*editable);
    }

    writer.option_str(initial.as_deref());
    encode_collection_view(writer, *view);

    match sort {
        Some(Sort { fields, ascending }) => {
            writer.u8(1).strings(fields).usize(ascending.len());
            for flag in ascending {
                writer.bool(*flag);
            }
        }
        None => {
            writer.u8(0);
        }
    }

    match navigator {
        Some(Navigator { layouts }) => {
            writer.u8(1).usize(layouts.len());
            for layout in layouts {
                encode_layout(writer, layout);
            }
        }
        None => {
            writer.u8(0);
        }
    }

    let Colours {
        background,
        card_background,
        card_border,
        primary_text,
        secondary_text,
    } = colours;
    writer
        .option_numbers(background.as_ref().map(<[f32; 3]>::as_slice))
        .option_numbers(card_background.as_ref().map(<[f32; 3]>::as_slice))
        .option_numbers(card_border.as_ref().map(<[f32; 3]>::as_slice))
        .option_numbers(primary_text.as_ref().map(<[f32; 3]>::as_slice))
        .option_numbers(secondary_text.as_ref().map(<[f32; 3]>::as_slice));

    match split {
        Some(Split {
            direction,
            position,
        }) => {
            writer.u8(1).u8(match direction {
                SplitDirection::Horizontal => 0,
                SplitDirection::Vertical => 1,
                SplitDirection::None => 2,
            });
            writer.option_f32(*position);
        }
        None => {
            writer.u8(0);
        }
    }

    match folders {
        Some(folder) => {
            writer.u8(1);
            encode_folder(writer, folder)?;
        }
        None => {
            writer.u8(0);
        }
    }
    Ok(())
}

/// Reads §12.3.5's collection dictionary.
pub(super) fn decode_collection(reader: &mut Reader<'_>) -> Result<Collection, ProtocolError> {
    let schema = reader
        .list("a collection schema", |reader| {
            let key = reader.string("a schema key")?;
            let field = Field {
                kind: decode_field_kind(reader)?,
                name: reader.string("a column's name")?,
                order: reader.option_i64("a column's order")?,
                visible: reader.bool("a column's visibility")?,
                editable: reader.bool("a column's editability")?,
            };
            Ok((key, field))
        })?
        .into_iter()
        .collect();

    let initial = reader.option_string("a collection's initial document")?;
    let view = decode_collection_view(reader)?;

    let sort = if reader.bool("a collection's sort")? {
        Some(Sort {
            fields: reader.strings("a sort's fields")?,
            ascending: reader.list("a sort's directions", |reader| {
                reader.bool("a sort's direction")
            })?,
        })
    } else {
        None
    };

    let navigator = if reader.bool("a collection's navigator")? {
        Some(Navigator {
            layouts: reader.list("a navigator's layouts", decode_layout)?,
        })
    } else {
        None
    };

    let colours = Colours {
        background: reader.option_colour("a collection's colour")?,
        card_background: reader.option_colour("a collection's colour")?,
        card_border: reader.option_colour("a collection's colour")?,
        primary_text: reader.option_colour("a collection's colour")?,
        secondary_text: reader.option_colour("a collection's colour")?,
    };

    let split = if reader.bool("a collection's split")? {
        let what = "a split's direction";
        Some(Split {
            direction: match reader.u8(what)? {
                0 => SplitDirection::Horizontal,
                1 => SplitDirection::Vertical,
                2 => SplitDirection::None,
                value => return Err(unrecognised(what, value)),
            },
            position: reader.option_f32("a split's position")?,
        })
    } else {
        None
    };

    let folders = if reader.bool("a collection's folders")? {
        Some(decode_folder(reader, 0)?)
    } else {
        None
    };

    Ok(Collection {
        schema,
        initial,
        view,
        sort,
        navigator,
        colours,
        split,
        folders,
    })
}

/// Encodes §12.3.5.1's resolved `/D`: which document the collection opens on.
///
/// Four instructions rather than an optional name, because the clause states four outcomes for
/// one entry and a host that received `None` would have to consult the `/EmbeddedFiles` tree to
/// tell "no `/D`" from "a `/D` naming nothing" — which is the tree a confined host does not have.
pub(super) fn encode_initial(writer: &mut Writer, initial: &Initial) {
    match initial {
        Initial::Container => {
            writer.u8(0);
        }
        Initial::Embedded(name) => {
            writer.u8(1);
            writer.str(name);
        }
        Initial::FirstFile => {
            writer.u8(2);
        }
        Initial::Empty => {
            writer.u8(3);
        }
    }
}

/// Reads §12.3.5.1's resolved `/D`.
pub(super) fn decode_initial(reader: &mut Reader<'_>) -> Result<Initial, ProtocolError> {
    let what = "a collection's initial document";
    Ok(match reader.u8(what)? {
        0 => Initial::Container,
        1 => Initial::Embedded(reader.string("an initial document's name")?),
        2 => Initial::FirstFile,
        3 => Initial::Empty,
        value => return Err(unrecognised(what, value)),
    })
}

/// Table 155's `/Subtype`.
fn encode_field_kind(writer: &mut Writer, kind: &FieldKind) {
    match kind {
        FieldKind::Text => writer.u8(0),
        FieldKind::Date => writer.u8(1),
        FieldKind::Number => writer.u8(2),
        FieldKind::FileName => writer.u8(3),
        FieldKind::Description => writer.u8(4),
        FieldKind::ModificationDate => writer.u8(5),
        FieldKind::CreationDate => writer.u8(6),
        FieldKind::Size => writer.u8(7),
        FieldKind::CompressedSize => writer.u8(8),
        FieldKind::Other(name) => writer.u8(9).str(name),
    };
}

fn decode_field_kind(reader: &mut Reader<'_>) -> Result<FieldKind, ProtocolError> {
    let what = "a collection column's subtype";
    Ok(match reader.u8(what)? {
        0 => FieldKind::Text,
        1 => FieldKind::Date,
        2 => FieldKind::Number,
        3 => FieldKind::FileName,
        4 => FieldKind::Description,
        5 => FieldKind::ModificationDate,
        6 => FieldKind::CreationDate,
        7 => FieldKind::Size,
        8 => FieldKind::CompressedSize,
        9 => FieldKind::Other(reader.string(what)?),
        value => return Err(unrecognised(what, value)),
    })
}

/// Table 153's `/View`.
fn encode_collection_view(writer: &mut Writer, view: CollectionView) {
    writer.u8(match view {
        CollectionView::Details => 0,
        CollectionView::Tile => 1,
        CollectionView::Hidden => 2,
        CollectionView::Navigator => 3,
    });
}

fn decode_collection_view(reader: &mut Reader<'_>) -> Result<CollectionView, ProtocolError> {
    let what = "a collection's view";
    Ok(match reader.u8(what)? {
        0 => CollectionView::Details,
        1 => CollectionView::Tile,
        2 => CollectionView::Hidden,
        3 => CollectionView::Navigator,
        value => return Err(unrecognised(what, value)),
    })
}

/// Table 160's named layouts.
fn encode_layout(writer: &mut Writer, layout: &Layout) {
    match layout {
        Layout::View(view) => {
            writer.u8(0);
            encode_collection_view(writer, *view);
        }
        Layout::FilmStrip => {
            writer.u8(1);
        }
        Layout::FreeForm => {
            writer.u8(2);
        }
        Layout::Linear => {
            writer.u8(3);
        }
        Layout::Tree => {
            writer.u8(4);
        }
        Layout::Custom(name) => {
            writer.u8(5).str(name);
        }
    }
}

fn decode_layout(reader: &mut Reader<'_>) -> Result<Layout, ProtocolError> {
    let what = "a collection layout";
    Ok(match reader.u8(what)? {
        0 => Layout::View(decode_collection_view(reader)?),
        1 => Layout::FilmStrip,
        2 => Layout::FreeForm,
        3 => Layout::Linear,
        4 => Layout::Tree,
        5 => Layout::Custom(reader.string(what)?),
        value => return Err(unrecognised(what, value)),
    })
}

/// Table 159's folder, and the ones below it.
fn encode_folder(writer: &mut Writer, folder: &Folder) -> Result<(), Uncarried> {
    let Folder {
        id,
        name,
        description,
        item,
        has_thumbnail,
        children,
    } = folder;
    writer.u32(*id).str(name).option_str(description.as_deref());
    encode_collection_item(writer, item)?;
    writer.bool(*has_thumbnail).usize(children.len());
    for child in children {
        encode_folder(writer, child)?;
    }
    Ok(())
}

fn decode_folder(reader: &mut Reader<'_>, at: usize) -> Result<Folder, ProtocolError> {
    let what = "a collection folder";
    depth(what, at)?;
    Ok(Folder {
        id: reader.u32("a folder's identifier")?,
        name: reader.string("a folder's name")?,
        description: reader.option_string("a folder's description")?,
        item: decode_collection_item(reader)?,
        has_thumbnail: reader.bool("a folder's thumbnail flag")?,
        children: reader.list(what, |reader| decode_folder(reader, at.saturating_add(1)))?,
    })
}

/// §7.11.6's collection item: Table 46's user-defined columns.
fn encode_collection_item(writer: &mut Writer, item: &CollectionItem) -> Result<(), Uncarried> {
    let CollectionItem { values } = item;
    writer.usize(values.len());
    for (key, value) in values {
        let CollectionValue { data, prefix } = value;
        writer.str(key);
        encode_collection_data(writer, data)?;
        writer.option_str(prefix.as_deref());
    }
    Ok(())
}

fn decode_collection_item(reader: &mut Reader<'_>) -> Result<CollectionItem, ProtocolError> {
    Ok(CollectionItem {
        values: reader
            .list("a collection item", |reader| {
                let key = reader.string("a collection item's key")?;
                let value = CollectionValue {
                    data: decode_collection_data(reader)?,
                    prefix: reader.option_string("a collection value's prefix")?,
                };
                Ok((key, value))
            })?
            .into_iter()
            .collect(),
    })
}

/// Table 47's `/D`, which the clause bounds to three kinds.
///
/// [`CollectionValue::data`] is Table 47's `/D` — `pdf_model::collection::item` unwraps a
/// collection subitem dictionary into its `/D` and its `/P` before this ever sees it, so the
/// dictionary Table 46 permits as an *entry* is already gone. What is left is the subitem's own
/// data, whose type ISO 32000-2 §7.11.6, Table 47 gives as "text string, date, or number":
///
/// > The type of data shall match the data type identified by the corresponding collection field
/// > dictionary.
///
/// and Table 155's three item subtypes are `S` a text string, `D` a date string and `N` a number.
/// So the scalars are what this carries; an array, a dictionary, a stream or a reference is a file
/// that has written something the clause does not describe, and it is **refused by name** rather
/// than flattened into a string that would then be indistinguishable from one the file wrote.
fn encode_collection_data(writer: &mut Writer, data: &Object) -> Result<(), Uncarried> {
    match data {
        Object::Null => {
            writer.u8(0);
        }
        Object::Boolean(flag) => {
            writer.u8(1).bool(*flag);
        }
        Object::Integer(number) => {
            writer.u8(2).i64(*number);
        }
        Object::Real(number) => {
            writer.u8(3).f64(*number);
        }
        Object::String(bytes) => {
            writer.u8(4).bytes(bytes);
        }
        Object::Name(name) => {
            writer.u8(5).bytes(name.as_bytes());
        }
        Object::Array(_) | Object::Dictionary(_) | Object::Stream(_) | Object::Reference(_) => {
            return Err(Uncarried {
                message: "Answer::Collection",
                reason: "a collection subitem's /D is not the text string, date or number \
                         Table 47 describes",
            });
        }
    }
    Ok(())
}

fn decode_collection_data(reader: &mut Reader<'_>) -> Result<Object, ProtocolError> {
    let what = "a collection value";
    Ok(match reader.u8(what)? {
        0 => Object::Null,
        1 => Object::Boolean(reader.bool(what)?),
        2 => Object::Integer(reader.i64(what)?),
        3 => Object::Real(reader.f64(what)?),
        4 => Object::String(reader.bytes(what)?.into()),
        5 => Object::Name(Name::new(reader.bytes(what)?.to_vec())),
        value => return Err(unrecognised(what, value)),
    })
}

// ---------------------------------------------------------------------------------------------
// §12.3.4, a page's thumbnail
// ---------------------------------------------------------------------------------------------

/// Encodes §12.3.4's thumbnail, decoded.
pub(super) fn encode_thumbnail(writer: &mut Writer, thumbnail: &Thumbnail) {
    let Thumbnail {
        image,
        permitted_colour_space,
        permitted_subtype,
    } = thumbnail;
    let Image {
        width,
        height,
        data,
        interpolate,
    } = image;
    writer
        .u32(*width)
        .u32(*height)
        .bytes(data)
        .bool(*interpolate)
        .bool(*permitted_colour_space)
        .bool(*permitted_subtype);
}

/// Reads §12.3.4's thumbnail.
pub(super) fn decode_thumbnail(reader: &mut Reader<'_>) -> Result<Thumbnail, ProtocolError> {
    let width = reader.u32("a thumbnail's width")?;
    let height = reader.u32("a thumbnail's height")?;
    let data = reader.bytes("a thumbnail's samples")?;
    // The confined side is the untrusted side, so its dimensions are checked against the samples
    // it actually sent — the same rule `Answer::Frame` applies to a page's raster, and for the
    // same reason: a host that indexed a short buffer by a stated width would read whatever
    // followed it.
    let expected = usize::try_from(width)
        .ok()
        .zip(usize::try_from(height).ok())
        .and_then(|(width, height)| width.checked_mul(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ProtocolError::Overlong {
            what: "a thumbnail's samples",
            claimed: usize::MAX,
            available: data.len(),
        })?;
    if data.len() != expected {
        return Err(ProtocolError::Overlong {
            what: "a thumbnail's samples",
            claimed: expected,
            available: data.len(),
        });
    }
    Ok(Thumbnail {
        image: Image {
            width,
            height,
            data: data.into(),
            interpolate: reader.bool("a thumbnail's interpolation")?,
        },
        permitted_colour_space: reader.bool("a thumbnail's colour space flag")?,
        permitted_subtype: reader.bool("a thumbnail's subtype flag")?,
    })
}

// ---------------------------------------------------------------------------------------------
// §14.3.3 and §14.3.2, what a properties panel shows
// ---------------------------------------------------------------------------------------------

/// Encodes Table 349's information dictionary and §14.3.2's metadata beside it.
///
/// # Errors
///
/// [`Uncarried`] for an [`XmpError`] this build does not name. `XmpError` is the one
/// `#[non_exhaustive]` type in this vocabulary, so its `match` cannot be made exhaustive from
/// here — and a refusal is the honest answer, in the same shape `Answer::Frame` gives for a
/// raster format this build cannot name.
pub(super) fn encode_properties(
    writer: &mut Writer,
    information: &Information,
    metadata: Option<&Result<Xmp, XmpError>>,
) -> Result<(), Uncarried> {
    let Information {
        title,
        author,
        subject,
        keywords,
        creator,
        producer,
        created,
        modified,
        trapped,
    } = information;
    writer
        .option_str(title.as_deref())
        .option_str(author.as_deref())
        .option_str(subject.as_deref())
        .option_str(keywords.as_deref())
        .option_str(creator.as_deref())
        .option_str(producer.as_deref())
        .option_str(created.as_deref())
        .option_str(modified.as_deref())
        .u8(match trapped {
            Trapped::Fully => 0,
            Trapped::NotYet => 1,
            Trapped::Unknown => 2,
        });

    match metadata {
        None => {
            writer.u8(0);
        }
        Some(Ok(xmp)) => {
            writer.u8(1).usize(xmp.properties().len());
            for (name, value) in xmp.properties() {
                let XmpName { namespace, local } = name;
                writer.str(namespace).str(local);
                encode_xmp_value(writer, value);
            }
        }
        Some(Err(error)) => {
            writer.u8(2);
            encode_xmp_error(writer, error)?;
        }
    }
    Ok(())
}

/// Reads Table 349's information dictionary and §14.3.2's metadata.
pub(super) fn decode_properties(
    reader: &mut Reader<'_>,
) -> Result<(Information, Option<Result<Xmp, XmpError>>), ProtocolError> {
    let what = "a document's trapping";
    let information = Information {
        title: reader.option_string("a title")?,
        author: reader.option_string("an author")?,
        subject: reader.option_string("a subject")?,
        keywords: reader.option_string("keywords")?,
        creator: reader.option_string("a creator")?,
        producer: reader.option_string("a producer")?,
        created: reader.option_string("a creation date")?,
        modified: reader.option_string("a modification date")?,
        trapped: match reader.u8(what)? {
            0 => Trapped::Fully,
            1 => Trapped::NotYet,
            2 => Trapped::Unknown,
            value => return Err(unrecognised(what, value)),
        },
    };

    let what = "a metadata stream";
    let metadata = match reader.u8(what)? {
        0 => None,
        1 => Some(Ok(Xmp::from_properties(reader.list(what, |reader| {
            let name = XmpName {
                namespace: reader.string("a property's namespace")?,
                local: reader.string("a property's local name")?,
            };
            Ok((name, decode_xmp_value(reader)?))
        })?))),
        2 => Some(Err(decode_xmp_error(reader)?)),
        value => return Err(unrecognised(what, value)),
    };
    Ok((information, metadata))
}

/// ISO 16684-1's three shapes of value, and the structured one this reader does not enter.
fn encode_xmp_value(writer: &mut Writer, value: &XmpValue) {
    match value {
        XmpValue::Text(text) => {
            writer.u8(0).str(text);
        }
        XmpValue::Alt(items) => {
            writer.u8(1).usize(items.len());
            for (language, text) in items {
                writer.option_str(language.as_deref()).str(text);
            }
        }
        XmpValue::Seq(items) => {
            writer.u8(2).strings(items);
        }
        XmpValue::Bag(items) => {
            writer.u8(3).strings(items);
        }
        XmpValue::Structure => {
            writer.u8(4);
        }
    }
}

fn decode_xmp_value(reader: &mut Reader<'_>) -> Result<XmpValue, ProtocolError> {
    let what = "a metadata value";
    Ok(match reader.u8(what)? {
        0 => XmpValue::Text(reader.string(what)?),
        1 => XmpValue::Alt(reader.list(what, |reader| {
            let language = reader.option_string("a language")?;
            Ok((language, reader.string(what)?))
        })?),
        2 => XmpValue::Seq(reader.strings(what)?),
        3 => XmpValue::Bag(reader.strings(what)?),
        4 => XmpValue::Structure,
        value => return Err(unrecognised(what, value)),
    })
}

fn encode_xmp_error(writer: &mut Writer, error: &XmpError) -> Result<(), Uncarried> {
    match error {
        XmpError::Undecodable => {
            writer.u8(0);
        }
        XmpError::TooLarge { bytes } => {
            writer.u8(1).usize(*bytes);
        }
        XmpError::NotText => {
            writer.u8(2);
        }
        XmpError::Malformed {
            line,
            column,
            detail,
        } => {
            writer.u8(3).u32(*line).u32(*column).str(detail);
        }
        XmpError::Unbalanced { detail } => {
            writer.u8(4).str(detail);
        }
        XmpError::TooMuch { what } => {
            writer.u8(5).str(what);
        }
        // `XmpError` is `#[non_exhaustive]`, so a variant can arrive without this crate being
        // rebuilt against it. Refusing names the answer rather than turning an error this build
        // cannot spell into one it can.
        _ => {
            return Err(Uncarried {
                message: "Answer::Properties",
                reason: "the metadata stream failed in a way this build does not name",
            });
        }
    }
    Ok(())
}

fn decode_xmp_error(reader: &mut Reader<'_>) -> Result<XmpError, ProtocolError> {
    let what = "a metadata failure";
    Ok(match reader.u8(what)? {
        0 => XmpError::Undecodable,
        1 => XmpError::TooLarge {
            bytes: reader.usize(what)?,
        },
        2 => XmpError::NotText,
        3 => XmpError::Malformed {
            line: reader.u32(what)?,
            column: reader.u32(what)?,
            detail: reader.string(what)?,
        },
        4 => XmpError::Unbalanced {
            detail: reader.string(what)?,
        },
        // `TooMuch`'s `what` is a `&'static str` naming one of `pdf_model::xmp`'s four budgets,
        // and a `&'static str` cannot be made from bytes that arrived at run time. The four are
        // therefore named here rather than reconstructed, which also means a fifth budget is a
        // refusal rather than a leaked allocation.
        5 => XmpError::TooMuch {
            what: match reader.string(what)?.as_str() {
                "nesting depth" => "nesting depth",
                "properties" => "properties",
                "array items" => "array items",
                "value length" => "value length",
                _ => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a metadata budget",
                        value: u32::MAX,
                    });
                }
            },
        },
        value => return Err(unrecognised(what, value)),
    })
}

// ---------------------------------------------------------------------------------------------
// Table 29 and Table 147, what the document asks of the window
// ---------------------------------------------------------------------------------------------

/// One wire byte per value of Table 29's `/PageLayout`, in the order the table states them.
///
/// Named rather than written twice since `Command::Layout` crossed: the entry travels as an
/// *answer* about what the document asked for and as a *command* saying what the reader chose,
/// and two codings of one table would be two chances to disagree.
pub(super) const fn layout_code(layout: PageLayout) -> u8 {
    match layout {
        PageLayout::SinglePage => 0,
        PageLayout::OneColumn => 1,
        PageLayout::TwoColumnLeft => 2,
        PageLayout::TwoColumnRight => 3,
        PageLayout::TwoPageLeft => 4,
        PageLayout::TwoPageRight => 5,
    }
}

/// That coding read back, refusing a byte Table 29 does not define.
pub(super) fn layout_of(value: u8) -> Result<PageLayout, ProtocolError> {
    Ok(match value {
        0 => PageLayout::SinglePage,
        1 => PageLayout::OneColumn,
        2 => PageLayout::TwoColumnLeft,
        3 => PageLayout::TwoColumnRight,
        4 => PageLayout::TwoPageLeft,
        5 => PageLayout::TwoPageRight,
        value => return Err(unrecognised("a page layout", value)),
    })
}

/// Encodes Table 29's `/PageMode` and `/PageLayout`.
pub(super) fn encode_opening(writer: &mut Writer, opening: Opening) {
    let Opening { mode, layout } = opening;
    encode_page_mode(writer, mode);
    writer.u8(layout_code(layout));
}

/// Reads Table 29's two display entries.
pub(super) fn decode_opening(reader: &mut Reader<'_>) -> Result<Opening, ProtocolError> {
    let mode = decode_page_mode(reader)?;
    Ok(Opening {
        mode,
        layout: layout_of(reader.u8("a page layout")?)?,
    })
}

fn encode_page_mode(writer: &mut Writer, mode: PageMode) {
    writer.u8(match mode {
        PageMode::UseNone => 0,
        PageMode::UseOutlines => 1,
        PageMode::UseThumbs => 2,
        PageMode::UseOptionalContent => 3,
        PageMode::FullScreen => 4,
        PageMode::UseAttachments => 5,
    });
}

fn decode_page_mode(reader: &mut Reader<'_>) -> Result<PageMode, ProtocolError> {
    let what = "a page mode";
    Ok(match reader.u8(what)? {
        0 => PageMode::UseNone,
        1 => PageMode::UseOutlines,
        2 => PageMode::UseThumbs,
        3 => PageMode::UseOptionalContent,
        4 => PageMode::FullScreen,
        5 => PageMode::UseAttachments,
        value => return Err(unrecognised(what, value)),
    })
}

/// Encodes §12.2's Table 147, whole.
pub(super) fn encode_preferences(writer: &mut Writer, preferences: &ViewerPreferences) {
    let ViewerPreferences {
        hide_toolbar,
        hide_menubar,
        hide_window_ui,
        fit_window,
        center_window,
        display_doc_title,
        non_full_screen_page_mode,
        direction,
        view_area,
        view_clip,
        print_area,
        print_clip,
        print_scaling,
        duplex,
        pick_tray_by_pdf_size,
        print_page_range,
        num_copies,
        enforce_print_scaling,
    } = preferences;
    writer
        .bool(*hide_toolbar)
        .bool(*hide_menubar)
        .bool(*hide_window_ui)
        .bool(*fit_window)
        .bool(*center_window)
        .bool(*display_doc_title);
    encode_page_mode(writer, *non_full_screen_page_mode);
    writer.u8(match direction {
        Direction::LeftToRight => 0,
        Direction::RightToLeft => 1,
    });
    for boundary in [view_area, view_clip, print_area, print_clip] {
        encode_boundary(writer, *boundary);
    }
    writer.u8(match print_scaling {
        PrintScaling::AppDefault => 0,
        PrintScaling::NoScaling => 1,
    });
    match duplex {
        Some(duplex) => {
            writer.u8(1).u8(match duplex {
                Duplex::Simplex => 0,
                Duplex::FlipShortEdge => 1,
                Duplex::FlipLongEdge => 2,
            });
        }
        None => {
            writer.u8(0);
        }
    }
    writer
        .option_bool(*pick_tray_by_pdf_size)
        .usize(print_page_range.len());
    for (first, last) in print_page_range {
        writer.i64(*first).i64(*last);
    }
    writer.option_i64(*num_copies).bool(*enforce_print_scaling);
}

/// Reads §12.2's Table 147.
pub(super) fn decode_preferences(
    reader: &mut Reader<'_>,
) -> Result<ViewerPreferences, ProtocolError> {
    let hide_toolbar = reader.bool("a preference")?;
    let hide_menubar = reader.bool("a preference")?;
    let hide_window_ui = reader.bool("a preference")?;
    let fit_window = reader.bool("a preference")?;
    let center_window = reader.bool("a preference")?;
    let display_doc_title = reader.bool("a preference")?;
    let non_full_screen_page_mode = decode_page_mode(reader)?;
    let what = "a reading direction";
    let direction = match reader.u8(what)? {
        0 => Direction::LeftToRight,
        1 => Direction::RightToLeft,
        value => return Err(unrecognised(what, value)),
    };
    let view_area = decode_boundary(reader)?;
    let view_clip = decode_boundary(reader)?;
    let print_area = decode_boundary(reader)?;
    let print_clip = decode_boundary(reader)?;
    let what = "a print scaling";
    let print_scaling = match reader.u8(what)? {
        0 => PrintScaling::AppDefault,
        1 => PrintScaling::NoScaling,
        value => return Err(unrecognised(what, value)),
    };
    let what = "a duplex setting";
    let duplex = if reader.bool(what)? {
        Some(match reader.u8(what)? {
            0 => Duplex::Simplex,
            1 => Duplex::FlipShortEdge,
            2 => Duplex::FlipLongEdge,
            value => return Err(unrecognised(what, value)),
        })
    } else {
        None
    };
    Ok(ViewerPreferences {
        hide_toolbar,
        hide_menubar,
        hide_window_ui,
        fit_window,
        center_window,
        display_doc_title,
        non_full_screen_page_mode,
        direction,
        view_area,
        view_clip,
        print_area,
        print_clip,
        print_scaling,
        duplex,
        pick_tray_by_pdf_size: reader.option_bool("a paper tray choice")?,
        print_page_range: reader.list("a print page range", |reader| {
            Ok((reader.i64("a page number")?, reader.i64("a page number")?))
        })?,
        num_copies: reader.option_i64("a copy count")?,
        enforce_print_scaling: reader.bool("an enforced preference")?,
    })
}

/// Table 31's five page boundaries.
fn encode_boundary(writer: &mut Writer, boundary: Boundary) {
    writer.u8(match boundary {
        Boundary::Media => 0,
        Boundary::Crop => 1,
        Boundary::Bleed => 2,
        Boundary::Trim => 3,
        Boundary::Art => 4,
    });
}

fn decode_boundary(reader: &mut Reader<'_>) -> Result<Boundary, ProtocolError> {
    let what = "a page boundary";
    Ok(match reader.u8(what)? {
        0 => Boundary::Media,
        1 => Boundary::Crop,
        2 => Boundary::Bleed,
        3 => Boundary::Trim,
        4 => Boundary::Art,
        value => return Err(unrecognised(what, value)),
    })
}

// ---------------------------------------------------------------------------------------------
// §12.5.6.14's popups and §14.7's structure
// ---------------------------------------------------------------------------------------------

/// Encodes §12.5.6.14's open popup windows, placed on the screen.
pub(super) fn encode_popups(writer: &mut Writer, popups: &[PopupWindow]) {
    writer.usize(popups.len());
    for popup in popups {
        let PopupWindow {
            annotation,
            parent,
            quad,
            title,
            text,
            modified,
            colour,
        } = popup;
        writer
            .object(*annotation)
            .option_object(*parent)
            .quad(*quad);
        writer
            .option_str(title.as_deref())
            .option_str(text.as_deref())
            .option_str(modified.as_deref());
        match colour {
            Some(Color { r, g, b, a }) => {
                writer.u8(1).f32(*r).f32(*g).f32(*b).f32(*a);
            }
            None => {
                writer.u8(0);
            }
        }
    }
}

/// Reads §12.5.6.14's open popup windows.
pub(super) fn decode_popups(reader: &mut Reader<'_>) -> Result<Vec<PopupWindow>, ProtocolError> {
    reader.list("a popup list", |reader| {
        Ok(PopupWindow {
            annotation: reader.object("a popup")?,
            parent: reader.option_object("a popup's parent")?,
            quad: reader.quad("a popup's rectangle")?,
            title: reader.option_string("a popup's title")?,
            text: reader.option_string("a popup's text")?,
            modified: reader.option_string("a popup's modification date")?,
            colour: if reader.bool("a popup's colour")? {
                Some(Color {
                    r: reader.f32("a popup's colour")?,
                    g: reader.f32("a popup's colour")?,
                    b: reader.f32("a popup's colour")?,
                    a: reader.f32("a popup's colour")?,
                })
            } else {
                None
            },
        })
    })
}

/// Encodes §14.7's structure for one page, parent-first.
pub(super) fn encode_accessibility(writer: &mut Writer, nodes: &[AccessibilityNode]) {
    writer.usize(nodes.len());
    for node in nodes {
        let AccessibilityNode {
            parent,
            role,
            name,
            substituted,
            language,
            quads,
            header_scope,
            bounds,
            control,
            annotation,
            headers,
            lines,
        } = node;
        writer
            .option_usize(*parent)
            .str(role)
            .str(name)
            .bool(*substituted)
            .option_str(language.as_deref())
            .u8(scope_kind(*header_scope));
        match bounds {
            Some(rect) => {
                writer.u8(1).numbers(rect);
            }
            None => {
                writer.u8(0);
            }
        }
        // §12.7's control for the widget behind §14.7.5.3's object reference, where the element
        // names one. `control_kind::NONE` is the absence, so the discriminant carries the option
        // rather than a flag byte in front of it.
        match control {
            Some(control) => encode_control(writer, control),
            None => {
                writer.u8(control_kind::NONE);
            }
        }
        // §12.5's annotation the element's own object reference names, which is what says the
        // element *is* one — the fact an action request needs and neither the rectangle nor the
        // control carries.
        writer.option_object(*annotation);
        writer.usize(headers.len());
        for header in headers {
            writer.usize(*header);
        }
        writer.usize(quads.len());
        for quad in quads {
            writer.quad(*quad);
        }
        // Each line's text, then one character at a time: how many bytes of that text it produced
        // and where it is. The invariant the far side checks is that the two agree — see
        // `read_lines`.
        writer.usize(lines.len());
        for line in lines {
            writer.str(&line.text).usize(line.characters.len());
            for character in &line.characters {
                writer.usize(character.bytes).numbers(&character.bounds);
            }
        }
    }
}

/// Reads §14.7's structure for one page.
///
/// The parent links are checked against the list they index: a node whose parent is itself, or is
/// a node that has not been read yet, is not a tree, and a host walking one would loop. The answer
/// is parent-first by construction on the confined side, which is what makes the check a single
/// comparison rather than a graph traversal.
pub(super) fn decode_accessibility(
    reader: &mut Reader<'_>,
) -> Result<Vec<AccessibilityNode>, ProtocolError> {
    let what = "an accessibility tree";
    let mut at = 0usize;
    reader.list(what, |reader| {
        let parent = reader.option_usize("a node's parent")?;
        if let Some(parent) = parent
            && parent >= at
        {
            return Err(ProtocolError::Unrecognised {
                what: "a node's parent, which must be a node already read",
                value: u32::try_from(parent).unwrap_or(u32::MAX),
            });
        }
        let index = at;
        at = at.saturating_add(1);
        Ok(AccessibilityNode {
            parent,
            role: reader.string("a node's role")?,
            name: reader.string("a node's name")?,
            substituted: reader.bool("a node's substitution flag")?,
            language: reader.option_string("a node's language")?,
            header_scope: read_scope(reader)?,
            bounds: reader.option_rect("a node's stated bounding box")?,
            control: decode_optional_control(reader)?,
            annotation: reader.option_object("a node's annotation")?,
            // §14.8.4.8.3's header cells, checked the same way the parent link is: a header is a
            // cell the search walked *out to*, so it is always a node already read, and one that
            // is not would be a confined side pointing a host at something it has not been given.
            headers: reader.list("a node's header cells", |reader| {
                let header = reader.usize("a node's header cell")?;
                if header >= index {
                    return Err(ProtocolError::Unrecognised {
                        what: "a node's header cell, which must be a node already read",
                        value: u32::try_from(header).unwrap_or(u32::MAX),
                    });
                }
                Ok(header)
            })?,
            quads: super::read_quads(reader, "a node's shapes")?,
            lines: read_lines(reader)?,
        })
    })
}

/// Reads the lines of one element's own text, with the invariant a text interface indexes by.
///
/// [`viewer_core::TextLine`] states that its text is exactly the readback of its characters, and
/// every platform's text interface turns an offset into the string into an index into the
/// characters by walking their byte counts. A confined side that sent the two out of step would
/// put a host's caret arithmetic past the end of its own string, so the sum is checked here —
/// which is the side of the wire where the producer is not to be trusted.
fn read_lines(reader: &mut Reader<'_>) -> Result<Vec<TextLine>, ProtocolError> {
    reader.list("a node's lines", |reader| {
        let text = reader.string("a line's text")?;
        let characters = reader.list("a line's characters", |reader| {
            Ok(Character {
                bytes: reader.usize("a character's bytes")?,
                bounds: reader.rect("a character's box")?,
            })
        })?;
        let stated = characters
            .iter()
            .try_fold(0usize, |sum, character| sum.checked_add(character.bytes));
        if stated != Some(text.len()) {
            return Err(ProtocolError::Unrecognised {
                what: "a line whose characters and text disagree",
                value: u32::try_from(stated.unwrap_or(usize::MAX)).unwrap_or(u32::MAX),
            });
        }
        Ok(TextLine { text, characters })
    })
}

/// Table 384's `/Scope`, as one byte.
///
/// A discriminant of its own rather than an `Option` of a name: the three values are the
/// standard's closed set, and a name would let the confined side send a fourth.
fn scope_kind(scope: Option<HeaderScope>) -> u8 {
    match scope {
        None => 0,
        Some(HeaderScope::Row) => 1,
        Some(HeaderScope::Column) => 2,
        Some(HeaderScope::Both) => 3,
    }
}

/// Reads what [`scope_kind`] wrote, refusing a value this build does not define.
fn read_scope(reader: &mut Reader<'_>) -> Result<Option<HeaderScope>, ProtocolError> {
    Ok(match reader.u8("a header cell's axis")? {
        0 => None,
        1 => Some(HeaderScope::Row),
        2 => Some(HeaderScope::Column),
        3 => Some(HeaderScope::Both),
        value => return Err(unrecognised("a header cell's axis", value)),
    })
}

/// §12.7's form fields on the page being shown, with their widgets placed.
///
/// The twelfth answer, and the one ADR 0235's audit called the gap. Every field of every type is
/// named in a destructuring pattern for this module's stated reason: a field added in
/// `pdf-model` or `viewer-core` has to fail to compile rather than stop crossing, and a control
/// that lost its `/MaxLen` on the confined path would be a form a host built differently
/// depending on which side of the pipe it was on.
/// §12.7.4.3's value, with Table 231 bit 14's answer to whether it *is* the value.
///
/// One function for both places a field's value crosses — [`viewer_core::Answer::Field`] and this
/// module's `Fields` — so that the two cannot come to disagree about the flag. ADR 0247.
pub(super) fn encode_shown(writer: &mut Writer, value: Option<&ShownValue>) {
    match value {
        Some(shown) => {
            writer.bool(true).str(&shown.text).bool(shown.obscured);
        }
        None => {
            writer.bool(false);
        }
    }
}

/// Reads one back.
///
/// # Errors
///
/// [`ProtocolError::Truncated`] where the message ends inside it.
pub(super) fn decode_shown(reader: &mut Reader<'_>) -> Result<Option<ShownValue>, ProtocolError> {
    if !reader.bool("whether a field has a text value")? {
        return Ok(None);
    }
    Ok(Some(ShownValue {
        text: reader.string("a field's value")?,
        obscured: reader.bool("whether a field's value is Table 231 bit 14's echo")?,
    }))
}

pub(super) fn encode_fields(writer: &mut Writer, fields: &[FormField]) {
    writer.usize(fields.len());
    for field in fields {
        let FormField {
            name,
            partial,
            control,
            value,
            read_only,
            required,
            no_export,
            widgets,
        } = field;
        writer
            .str(&name.qualified)
            .option_str(name.alternative.as_deref())
            .str(partial);
        encode_shown(writer, value.as_ref());
        writer.bool(*read_only).bool(*required).bool(*no_export);
        encode_control(writer, control);
        writer.usize(widgets.len());
        for widget in widgets {
            let FormWidget {
                annotation,
                quad,
                on_state,
                export,
                on,
            } = widget;
            writer.object(*annotation).quad(*quad);
            writer
                .option_str(on_state.as_deref())
                .option_str(export.as_deref())
                .bool(*on);
        }
    }
}

/// Reads them back.
pub(super) fn decode_fields(reader: &mut Reader<'_>) -> Result<Vec<FormField>, ProtocolError> {
    reader.list("a form field list", |reader| {
        Ok(FormField {
            name: FieldName {
                qualified: reader.string("a field's qualified name")?,
                alternative: reader.option_string("a field's alternative name")?,
            },
            partial: reader.string("a field's partial name")?,
            value: decode_shown(reader)?,
            read_only: reader.bool("a field's ReadOnly flag")?,
            required: reader.bool("a field's Required flag")?,
            no_export: reader.bool("a field's NoExport flag")?,
            control: decode_control(reader)?,
            widgets: reader.list("a field's widgets", |reader| {
                Ok(FormWidget {
                    annotation: reader.object("a widget")?,
                    quad: reader.quad("a widget's rectangle")?,
                    on_state: reader.option_string("a widget's on state")?,
                    export: reader.option_string("a widget's export value")?,
                    on: reader.bool("a widget's state")?,
                })
            })?,
        })
    })
}

/// Which of §12.7.5's types the field is, and its own table's flags.
fn encode_control(writer: &mut Writer, control: &Control) {
    match control {
        Control::PushButton => {
            writer.u8(control_kind::PUSH_BUTTON);
        }
        Control::CheckBox { on } => {
            writer.u8(control_kind::CHECK_BOX).bool(*on);
        }
        Control::RadioButton {
            on,
            no_toggle_to_off,
            in_unison,
        } => {
            writer
                .u8(control_kind::RADIO_BUTTON)
                .bool(*on)
                .bool(*no_toggle_to_off)
                .bool(*in_unison);
        }
        Control::Text(text) => {
            let TextControl {
                multiline,
                password,
                file_select,
                do_not_spell_check,
                do_not_scroll,
                comb,
                max_len,
                rich_text,
            } = text;
            writer
                .u8(control_kind::TEXT)
                .bool(*multiline)
                .bool(*password)
                .bool(*file_select)
                .bool(*do_not_spell_check)
                .bool(*do_not_scroll)
                .bool(*rich_text);
            encode_count(writer, *comb);
            encode_count(writer, *max_len);
        }
        Control::Choice(choice) => {
            let ChoiceControl {
                combo,
                editable,
                multi_select,
                do_not_spell_check,
                commit_on_selection,
                options,
                selected,
                top,
            } = choice;
            writer
                .u8(control_kind::CHOICE)
                .bool(*combo)
                .bool(*editable)
                .bool(*multi_select)
                .bool(*do_not_spell_check)
                .bool(*commit_on_selection)
                .usize(*top);
            writer.usize(options.len());
            for option in options {
                let Choice { export, label } = option;
                writer.option_str(export.as_deref()).str(label);
            }
            writer.usize(selected.len());
            for index in selected {
                writer.usize(*index);
            }
        }
        Control::Signature => {
            writer.u8(control_kind::SIGNATURE);
        }
        Control::Unstated => {
            writer.u8(control_kind::UNSTATED);
        }
    }
}

/// Reads one back.
fn decode_control(reader: &mut Reader<'_>) -> Result<Control, ProtocolError> {
    let kind = reader.u8("a form control")?;
    decode_control_kind(reader, kind)?.ok_or_else(|| unrecognised("a form control", kind))
}

/// Reads a control that may be absent, which is what an element naming no widget encodes as.
///
/// `control_kind::NONE` is the absence rather than a flag byte in front of the discriminant: the
/// enumeration already needs one byte and a field's control is never optional on the wire, so the
/// two callers differ only in whether zero is a value they accept.
fn decode_optional_control(reader: &mut Reader<'_>) -> Result<Option<Control>, ProtocolError> {
    let what = "a structure element's form control";
    let kind = reader.u8(what)?;
    if kind == control_kind::NONE {
        return Ok(None);
    }
    match decode_control_kind(reader, kind)? {
        Some(control) => Ok(Some(control)),
        None => Err(unrecognised(what, kind)),
    }
}

/// One control's body, given the discriminant already read.
///
/// `None` for a discriminant this build does not define, which each caller reports in its own
/// words — the two describe different things and a shared message would name the wrong one.
fn decode_control_kind(
    reader: &mut Reader<'_>,
    kind: u8,
) -> Result<Option<Control>, ProtocolError> {
    Ok(Some(match kind {
        control_kind::PUSH_BUTTON => Control::PushButton,
        control_kind::CHECK_BOX => Control::CheckBox {
            on: reader.bool("a check box's state")?,
        },
        control_kind::RADIO_BUTTON => Control::RadioButton {
            on: reader.bool("a radio set's state")?,
            no_toggle_to_off: reader.bool("a radio set's NoToggleToOff flag")?,
            in_unison: reader.bool("a radio set's RadiosInUnison flag")?,
        },
        control_kind::TEXT => Control::Text(TextControl {
            multiline: reader.bool("a text field's Multiline flag")?,
            password: reader.bool("a text field's Password flag")?,
            file_select: reader.bool("a text field's FileSelect flag")?,
            do_not_spell_check: reader.bool("a text field's DoNotSpellCheck flag")?,
            do_not_scroll: reader.bool("a text field's DoNotScroll flag")?,
            rich_text: reader.bool("a text field's RichText flag")?,
            comb: decode_count(reader, "a comb field's cell count")?,
            max_len: decode_count(reader, "a text field's /MaxLen")?,
        }),
        control_kind::CHOICE => Control::Choice(ChoiceControl {
            combo: reader.bool("a choice field's Combo flag")?,
            editable: reader.bool("a choice field's Edit flag")?,
            multi_select: reader.bool("a choice field's MultiSelect flag")?,
            do_not_spell_check: reader.bool("a choice field's DoNotSpellCheck flag")?,
            commit_on_selection: reader.bool("a choice field's CommitOnSelChange flag")?,
            top: reader.usize("a list box's /TI")?,
            options: reader.list("a choice field's /Opt", |reader| {
                Ok(Choice {
                    export: reader.option_string("an option's export value")?,
                    label: reader.string("an option's label")?,
                })
            })?,
            selected: reader.list("a choice field's selection", |reader| {
                reader.usize("a selected option")
            })?,
        }),
        control_kind::SIGNATURE => Control::Signature,
        control_kind::UNSTATED => Control::Unstated,
        _ => return Ok(None),
    }))
}

/// An optional count, as a flag and a fixed-width number.
///
/// `Option<u32>` rather than `Option<usize>`, because Table 232's `/MaxLen` and Table 231 bit 25's
/// cell count are what the file states and both are bounded by the entry's own type. A flag beside
/// a fixed-width number rather than a length prefix keeps the absent case one byte and the present
/// case five, with no arithmetic on either side.
fn encode_count(writer: &mut Writer, count: Option<u32>) {
    writer.bool(count.is_some()).u32(count.unwrap_or_default());
}

/// Reads one back.
fn decode_count(reader: &mut Reader<'_>, what: &'static str) -> Result<Option<u32>, ProtocolError> {
    let stated = reader.bool(what)?;
    let count = reader.u32(what)?;
    Ok(stated.then_some(count))
}

/// [`Control`]'s discriminants: §12.7.5's four types, with buttons split as §12.7.5.2 splits them.
mod control_kind {
    /// No control at all, which is every structure element that names no widget annotation.
    ///
    /// Never written for a [`super::Control`] itself: a field always has one of the seven below,
    /// `UNSTATED` included.
    pub(super) const NONE: u8 = 0;
    pub(super) const PUSH_BUTTON: u8 = 1;
    pub(super) const CHECK_BOX: u8 = 2;
    pub(super) const RADIO_BUTTON: u8 = 3;
    pub(super) const TEXT: u8 = 4;
    pub(super) const CHOICE: u8 = 5;
    pub(super) const SIGNATURE: u8 = 6;
    /// Table 226 makes `/FT` required and this is a field that states none.
    pub(super) const UNSTATED: u8 = 7;
}

/// A discriminant this build does not define.
fn unrecognised(what: &'static str, value: u8) -> ProtocolError {
    ProtocolError::Unrecognised {
        what,
        value: u32::from(value),
    }
}
