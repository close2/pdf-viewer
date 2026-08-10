//! Three of `viewer_core`'s answers turned into one shape a platform tree can hold.
//!
//! **Toolkit-free on purpose**, which is why it is in this crate and not in either host: a
//! `GtkListView` and a `QTreeView` want the same rows and want them in different objects, so the
//! mapping is written once and the widget twice. Keeping the two apart is also what makes the
//! mapping testable with no display, which is the only part of a native host a workspace test
//! suite can see.
//!
//! The three answers are §12.3.3's outline ([`viewer_core::Query::Outline`]), §8.11.4.3's `/Order`
//! ([`viewer_core::Query::Layers`]) and §7.11.4's embedded files
//! ([`viewer_core::Query::Attachments`]). They arrive as three different types, because they *are*
//! three different things, and a platform tree wants one row type — so this is where a host pays
//! for that, once.

use pdf_model::attachment::Attachment;
use pdf_model::outline::{Item, Outline};
use pdf_syntax::ObjectId;
use viewer_core::Layer;

/// What a row does when a person acts on it.
///
/// Every one of these is a `viewer_core` message and none of them is a payload: §12.3.3's own
/// sentence is that clicking an item causes the processor "to jump to a destination or trigger an
/// action associated with the item", and which of the two it is belongs to the *document*. That is
/// why an outline row carries an object and not a page number (ADR 0144).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowAction {
    /// §12.3.3: activate this outline item — [`viewer_core::Command::Activate`].
    Activate(ObjectId),
    /// §8.11.4.3: an optional content group, whose switch is
    /// [`viewer_core::Command::SetGroup`].
    Toggle {
        /// The group's object.
        group: ObjectId,
        /// Whether it is on now.
        on: bool,
        /// Table 99's `/Locked`: "[t]he state of a locked group cannot be changed through the
        /// user interface of an interactive PDF processor." A locked row's switch is built
        /// insensitive, which is the platform's own way of saying the same thing.
        locked: bool,
    },
    /// §7.11.4: take this embedded file out — [`viewer_core::Command::Extract`].
    Extract {
        /// The `/EmbeddedFiles` key, which is what the command names.
        name: String,
    },
    /// A row that does nothing when acted on.
    ///
    /// §8.11.4.3's leading text string is exactly this: an array *with* one is "a collection of
    /// related groups" under a heading, and the heading is not a layer. A tree that let a person
    /// click it would be telling them it is.
    Inert,
}

/// One row of a platform tree, whatever the answer it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelRow {
    /// What the row says.
    pub label: String,
    /// A second line, where the answer carries something worth showing beside the label.
    pub detail: Option<String>,
    /// Whether the document asked for this row to start expanded.
    ///
    /// §12.3.3 gives an outline item's `/Count` a sign for it — "[i]f the outline item is open,
    /// Count is the sum of the number of visible descendent outline items" — so a tree that
    /// opened everything, or nothing, would be discarding a statement the file made.
    pub expanded: bool,
    /// What acting on the row does.
    pub action: RowAction,
    /// The rows underneath it.
    pub children: Vec<PanelRow>,
}

/// §12.3.3's outline, as rows.
#[must_use]
pub fn outline_rows(outline: &Outline) -> Vec<PanelRow> {
    outline.items.iter().map(row_of_item).collect()
}

/// One outline item and everything under it.
fn row_of_item(item: &Item) -> PanelRow {
    PanelRow {
        label: item.title.clone(),
        detail: None,
        expanded: item.open,
        action: RowAction::Activate(item.id),
        children: item.children.iter().map(row_of_item).collect(),
    }
}

/// §8.11.4.3's `/Order`, as rows.
///
/// The clause's two shapes stay two shapes: a group is a row with a switch, and a collection with
/// a leading string is a heading with children and no switch.
#[must_use]
pub fn layer_rows(layers: &[Layer]) -> Vec<PanelRow> {
    layers.iter().map(row_of_layer).collect()
}

/// One entry of `/Order`.
fn row_of_layer(layer: &Layer) -> PanelRow {
    match layer {
        Layer::Group {
            group,
            name,
            on,
            locked,
        } => PanelRow {
            // Table 96's `/Name` is "suitable for presentation in an interactive PDF processor's
            // user interface", and a group that states none still has to be shown: a row with no
            // text would be a switch nobody can name, so the object stands in for it.
            label: name
                .clone()
                .unwrap_or_else(|| format!("group {} {}", group.number, group.generation)),
            detail: None,
            expanded: true,
            action: RowAction::Toggle {
                group: *group,
                on: *on,
                locked: *locked,
            },
            children: Vec::new(),
        },
        Layer::Collection { label, children } => PanelRow {
            label: label.clone().unwrap_or_default(),
            detail: None,
            expanded: true,
            action: RowAction::Inert,
            children: children.iter().map(row_of_layer).collect(),
        },
    }
}

/// §7.11.4's embedded files, as rows.
///
/// Flat, because the `/EmbeddedFiles` name tree is a mapping rather than a hierarchy — §12.3.5's
/// `/Collection` is what arranges files into folders, and it is a different answer
/// ([`viewer_core::Query::Collection`]) that this host does not yet ask.
#[must_use]
pub fn attachment_rows(attachments: &[Attachment]) -> Vec<PanelRow> {
    attachments
        .iter()
        .map(|attachment| PanelRow {
            // Table 43's `/UF` is the file's own name and the tree's key need not be one, so the
            // file name is what a person is shown and the key is what the command carries.
            //
            // The middle of that sentence was a quotation of §7.11.4.1 — "shall map name strings
            // to file specifications" — until the four-hundred-and-twenty-ninth session. Errata
            // Collection 3 replaces the two bullets it came from outright (Issue #481, `/State`
            // `Review` `Completed`), and the replacement drops the sentence: an `/EmbeddedFiles`
            // association "is not required unless stated otherwise". What the key is stays what
            // §7.7.4 makes it, a name string in a name tree, which is why nothing here moves.
            // `pdf_model::attachment` carries the same correction, made one round after the
            // erratum was read and three before this copy of it was found (ADR 0254).
            label: attachment
                .file_name
                .clone()
                .unwrap_or_else(|| attachment.name.clone()),
            detail: detail_of(attachment),
            expanded: false,
            action: RowAction::Extract {
                name: attachment.name.clone(),
            },
            children: Vec::new(),
        })
        .collect()
}

/// Table 44's `/Subtype` and Table 45's `/Size`, where the file states them.
///
/// Both or either or neither: `/Desc` is "a textual description of the embedded file, which can
/// be displayed in the user interface" and takes precedence over both, because it is the entry
/// the clause wrote *for* this purpose.
fn detail_of(attachment: &Attachment) -> Option<String> {
    if let Some(description) = attachment.description.clone() {
        return Some(description);
    }
    match (&attachment.media_type, attachment.size) {
        (Some(media), Some(size)) => Some(format!("{media}, {size} bytes")),
        (Some(media), None) => Some(media.clone()),
        (None, Some(size)) => Some(format!("{size} bytes")),
        (None, None) => None,
    }
}
