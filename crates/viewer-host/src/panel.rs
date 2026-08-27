//! `viewer_core`'s panel answers turned into one shape a platform tree can hold, and the list of
//! panels itself.
//!
//! **Toolkit-free on purpose**, which is why it is in this crate and not in either host: a
//! `GtkListView` and a `QTreeView` want the same rows and want them in different objects, so the
//! mapping is written once and the widget twice. Keeping the two apart is also what makes the
//! mapping testable with no display, which is the only part of a native host a workspace test
//! suite can see.
//!
//! The answers are §12.3.3's outline ([`viewer_core::Query::Outline`]), §8.11.4.3's `/Order`
//! ([`viewer_core::Query::Layers`]), §7.11.4's embedded files
//! ([`viewer_core::Query::Attachments`]), §12.3.5's portable collection
//! ([`viewer_core::Query::Collection`], which is those same files *arranged*), §12.4.3's article
//! threads ([`viewer_core::Query::Articles`]) and §14.3.3's document information with §14.3.2's
//! metadata beside it ([`viewer_core::Query::Properties`]). They arrive as six different types,
//! because they *are* six different things, and a platform tree wants one row type — so this is
//! where a host pays for that, once.
//!
//! # §12.3.4's miniatures are the one answer that is not a row
//!
//! A thumbnail is a picture, and a picture is a `gdk::Texture`, a `QPixmap` or a
//! [`pdf_render::Image`] depending on who is drawing it — which is precisely the line this crate
//! draws in its own documentation ("no widget … and no pixel format"). So what is shared is
//! [`page_entry`], which asks the two queries a page's row needs and settles the label a page with
//! no §12.4.2 label gets; **each host holds the decoded picture in its own toolkit's type**, and
//! each holds only the ones it is drawing.
//!
//! That last sentence is a requirement rather than a preference. `CLAUDE.md` section 2 forbids thumbnail
//! generation on the launch path by name, and [`viewer_core::Query::Thumbnail`] answers one page
//! at a time for the same reason: *"a thousand-page document that carried one for every page would
//! decode a thousand images to draw eight. The panel knows which eight it is showing; this crate
//! does not."* A host that asked this function in a loop over its page count would have moved the
//! eager work rather than removed it.

use pdf_model::article::Thread;
use pdf_model::attachment::Attachment;
use pdf_model::collection::{Collection, Field, FieldKind, Initial};
use pdf_model::metadata::{Information, Trapped};
use pdf_model::outline::{Item, Outline};
use pdf_model::viewer_preferences::PageMode;
use pdf_model::xmp::{Xmp, XmpError};
use pdf_render::Image;
use pdf_syntax::ObjectId;
use viewer_core::{Answer, Layer, Query, Viewer};

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
    /// Whether the row is a sentence *about* the document rather than a thing *in* it.
    ///
    /// §14.3.2's heading, the count of XMP properties nobody named, and "this document states no
    /// article threads" are all rows a list has to hold and none of them is an item. Three
    /// toolkits have three ways of saying so — a `dim-label` class, [`Qt::ItemIsEnabled`] cleared,
    /// a paler ink — and *which rows they are* is one decision, which is why it is here rather
    /// than inferred from [`RowAction::Inert`]: §8.11.4.3's collection heading is inert and is
    /// very much a thing in the document.
    ///
    /// [`Qt::ItemIsEnabled`]: https://doc.qt.io/qt-6/qt.html#ItemFlag-enum
    pub note: bool,
    /// Whether this row is the one the *document* asked to be presented first.
    ///
    /// §12.3.5.1's `/D` is the only thing in this file that sets it: Table 153's entry
    /// "identif[ies] an entry in the `EmbeddedFiles` name tree, determining the document that
    /// shall be initially presented in the user interface", and a panel over a page obeys that
    /// by setting one row apart rather than by opening a second window. **The clause states no
    /// appearance for it**, so which mark each toolkit makes is that toolkit's — a bold label
    /// here, a bold [`Qt::FontRole`] there — and *which row it is* is the one decision, which is
    /// why it is a field rather than a rule each host applies to [`RowAction::Extract`].
    ///
    /// [`Qt::FontRole`]: https://doc.qt.io/qt-6/qt.html#ItemDataRole-enum
    pub emphasis: bool,
    /// The rows underneath it.
    pub children: Vec<PanelRow>,
}

impl PanelRow {
    /// A plain row for a thing in the document: no detail, closed, inert and childless.
    #[must_use]
    pub fn item(label: String) -> Self {
        Self {
            label,
            detail: None,
            expanded: false,
            action: RowAction::Inert,
            note: false,
            emphasis: false,
            children: Vec::new(),
        }
    }

    /// A row that is a sentence about the document rather than a thing in it.
    ///
    /// Every list here has one for the case where the document states nothing, because an empty
    /// panel and a panel this program failed to fill look the same and only one of them is
    /// somebody else's file being quiet.
    #[must_use]
    pub fn saying(text: &str) -> Self {
        Self {
            note: true,
            ..Self::item(text.to_owned())
        }
    }
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
        note: false,
        emphasis: false,
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
            note: false,
            emphasis: false,
            children: Vec::new(),
        },
        Layer::Collection { label, children } => PanelRow {
            label: label.clone().unwrap_or_default(),
            detail: None,
            expanded: true,
            action: RowAction::Inert,
            note: false,
            emphasis: false,
            children: children.iter().map(row_of_layer).collect(),
        },
    }
}

/// §7.11.4's embedded files, as rows.
///
/// Flat, because the `/EmbeddedFiles` name tree is a mapping rather than a hierarchy — §12.3.5's
/// `/Collection` is what arranges files into folders, and it is a different answer
/// ([`viewer_core::Query::Collection`]) that [`collection_rows`] presents. A host asks that
/// query first and falls back to this one, because a collection is *these same files* arranged
/// rather than a second population of them.
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
            // §7.7.4 makes it, a string in a name tree, which is why nothing here moves — *name
            // string* being the term Issue #214 takes out of that clause.
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
            note: false,
            emphasis: false,
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

/// §12.3.5's portable collection, as rows: the same files, in folders, with the schema's columns.
///
/// > If this dictionary is present in a PDF document, the interactive PDF processor shall present
/// > the document as a portable collection.
///
/// That sentence is a `shall` addressed to a *viewer*, and it is why this function exists at all:
/// [`viewer_core::Query::Collection`] has carried Table 153 whole since the
/// three-hundred-and-fifty-second session and no native host asked it, so two of the three windows
/// showed a collection as a flat list of its files — the arrangement the document states, dropped.
///
/// **It is the files tab rather than a seventh panel.** A collection is how a document says its
/// embedded files are arranged, not a second population of them, so the tab a person already looks
/// in for [`attachment_rows`]'s list is where the arrangement belongs.
///
/// **The container's own pages stay on the screen**, which is the one decision the clause leaves
/// open: it says to present the document as a collection and does not say *instead of what*.
/// §7.6.7's unencrypted wrapper settles it — a wrapper's whole purpose is a page saying the
/// payload is encrypted, and Table 153's `/View H` is how such a document asks for the list to
/// start hidden — so a window that replaced the page with a file browser would hide the sentence
/// the wrapper exists to show. ADR 0202 took that decision for `viewer-ui`; this is the same
/// decision reaching the other two windows.
///
/// # Which file is in which folder
///
/// §12.3.5.2 gives the association no entry: it is written into the name-tree *key*, so
/// `<3>report.pdf` is *report.pdf* in folder 3 and [`pdf_model::collection::folder_of`] reads it.
/// A key that does not conform names no folder, and the clause says such files "shall be treated
/// as associated with the root folder" — so they are the top-level rows, above the folders, which
/// is where the root's own files belong. The row still carries the *tree's* key in its
/// [`RowAction::Extract`], folder number and all, because that is what
/// [`viewer_core::Command::Extract`] names a file by.
///
/// **Every embedded file is on the screen, and the clause says so twice.** A file cannot fall out
/// of this panel by being filed oddly:
///
/// - a document that states no `/Folders` gets a flat list of all of them — "[i]f no folder
///   structure is specified, interactive PDF processors should show all files in the collection in
///   a flat list";
/// - and a key naming a folder identifier the tree does not state is drawn at the root, because
///   "[w]hen folders are used, all files in the `EmbeddedFiles` name tree … shall be treated as
///   members of the folder structure by an interactive PDF processor". The key conforms to the
///   naming rules, so the clause's own "shall be treated as associated with the root folder" does
///   not reach it; what it contradicts is "[t]he value shall correspond to a folder ID", which is
///   a requirement on the *producer*. The clause states no remedy, and the root is the one place
///   in the structure that admits a file no folder claims — so this is a documented choice for a
///   malformed file, made to keep the `shall` above rather than to invent an arrangement.
///
/// Both cases lost the file entirely until the seven-hundred-and-seventy-second session (ADR
/// 0711), which is a panel drawing less than the document embeds.
///
/// # The columns
///
/// Table 155's `/O` is "[t]he relative order of the field name in the user interface", and `/V`
/// its "initial visibility" — so the hidden fields are dropped and the rest sort by `/O`, with a
/// field stating none after those that do, by key, which is the only order left when the file
/// states nothing. They become the row's one detail line, which is what a two-line tree row has
/// in place of a table's columns.
///
/// Only Table 155's **file-related** subtypes are answered. The other three read §7.11.6's
/// collection item on the file specification's `/CI`, which [`Attachment`] does not carry; that
/// is a gap recorded here rather than papered over, and it is the same gap `viewer_ui::chrome`
/// records.
///
/// # `/D`, and what "presented first" means for a panel over a page
///
/// [`pdf_model::collection::Initial`] is §12.3.5.1's four outcomes, resolved by `viewer-core`
/// because the name tree is the document's. A panel over a page obeys them the only way it can:
/// the initial document's row carries [`PanelRow::emphasis`], an empty tree says so instead of
/// drawing nothing, and the container case marks no row — the container is what is already on the
/// screen.
///
/// **This is written twice in this tree and that is deliberate**, on the same division as
/// [`outline_rows`] and [`layer_rows`]: `viewer_ui::chrome` builds its own rows for every panel
/// whose row is a *widget* — an expander, a switch, a miniature — and shares the ones that are
/// text. The decisions are the clause's and are stated in both places; what differs is the row
/// type each toolkit draws.
#[must_use]
pub fn collection_rows(
    collection: &Collection,
    initial: &Initial,
    attachments: &[Attachment],
) -> Vec<PanelRow> {
    let mut columns: Vec<(&String, &Field)> = collection
        .schema
        .iter()
        .filter(|(_, field)| field.visible)
        .collect();
    columns.sort_by_key(|(key, field)| (field.order.unwrap_or(i64::MAX), (*key).clone()));

    let mut rows = match collection.folders.as_ref() {
        Some(root) => {
            let stated = &stated_ids(root);
            let mut rows = files_where(attachments, &columns, &|id| {
                id.is_none_or(|id| !stated.contains(&id))
            });
            rows.push(folder_row(root, attachments, &columns));
            rows
        }
        None => files_where(attachments, &columns, &|_| true),
    };

    mark_initial(&mut rows, initial);

    if rows.is_empty() {
        // §12.3.5.1's "an empty preview window", and the case a document with a `/Collection` and
        // no `/EmbeddedFiles` tree is in. An empty panel and a panel this program failed to fill
        // look identical; only one of them is the file being quiet.
        rows.push(PanelRow::saying("This collection lists no files."));
    }
    rows
}

/// Every folder identifier the tree states, which is what decides whether a key names one.
fn stated_ids(root: &pdf_model::collection::Folder) -> std::collections::BTreeSet<u32> {
    let mut ids = std::collections::BTreeSet::new();
    let mut stack = vec![root];
    while let Some(folder) = stack.pop() {
        ids.insert(folder.id);
        stack.extend(folder.children.iter());
    }
    ids
}

/// The files whose name-tree key names `folder`, as rows.
fn files_in(
    folder: u32,
    attachments: &[Attachment],
    columns: &[(&String, &Field)],
) -> Vec<PanelRow> {
    files_where(attachments, columns, &|id| id == Some(folder))
}

/// The files whose key's folder identifier — `None` where the key names none — `wanted` admits.
fn files_where(
    attachments: &[Attachment],
    columns: &[(&String, &Field)],
    wanted: &dyn Fn(Option<u32>) -> bool,
) -> Vec<PanelRow> {
    attachments
        .iter()
        .filter_map(|attachment| {
            let (id, name) = match pdf_model::collection::folder_of(&attachment.name) {
                Some((id, name)) => (Some(id), name),
                None => (None, attachment.name.as_str()),
            };
            if !wanted(id) {
                return None;
            }
            Some(PanelRow {
                detail: columns_of(columns, attachment).or_else(|| detail_of(attachment)),
                action: RowAction::Extract {
                    name: attachment.name.clone(),
                },
                ..PanelRow::item(
                    attachment
                        .file_name
                        .clone()
                        .unwrap_or_else(|| name.to_owned()),
                )
            })
        })
        .collect()
}

/// One folder, with its files and its child folders under it.
///
/// Open, because a collection that arrived closed would be a folder tree presented as one row —
/// which is §12.3.5's `shall` obeyed in the letter and not at all in what a person sees. The
/// nesting is bounded by [`pdf_model::collection::Collection::read`], which walks `/Child` and
/// `/Next` forward only and visits each object once.
fn folder_row(
    folder: &pdf_model::collection::Folder,
    attachments: &[Attachment],
    columns: &[(&String, &Field)],
) -> PanelRow {
    let mut children = files_in(folder.id, attachments, columns);
    children.extend(
        folder
            .children
            .iter()
            .map(|child| folder_row(child, attachments, columns)),
    );
    PanelRow {
        detail: folder.description.clone(),
        expanded: true,
        // A folder is not a file: it has no bytes to take out, so its row acts through its
        // children.
        action: RowAction::Inert,
        children,
        ..PanelRow::item(folder.name.clone())
    }
}

/// Sets §12.3.5.1's initial document apart, in the order a person reads the tree.
///
/// Depth first, because that is the order the rows are shown in and therefore what the clause's
/// "the first item from the list of files to display in its user interface" points at.
fn mark_initial(rows: &mut [PanelRow], initial: &Initial) {
    let wanted = match initial {
        Initial::Embedded(name) => Some(name.clone()),
        Initial::FirstFile => None,
        // The container's own pages are on the screen already, so there is no row to mark; an
        // empty tree has no rows at all and says so instead.
        Initial::Container | Initial::Empty => return,
    };
    let mut done = false;
    mark_first(rows, wanted.as_deref(), &mut done);
}

/// The first extractable row matching `wanted`, or the first of any where it is `None`.
fn mark_first(rows: &mut [PanelRow], wanted: Option<&str>, done: &mut bool) {
    for row in rows {
        if *done {
            return;
        }
        if let RowAction::Extract { name } = &row.action
            && wanted.is_none_or(|asked| asked == name)
        {
            row.emphasis = true;
            *done = true;
            return;
        }
        mark_first(&mut row.children, wanted, done);
    }
}

/// The schema's columns for one file, as `name: value` joined — the detail line of its row.
///
/// Table 47's `/P` prefix is concatenated with the *value* and not with the name, which is what
/// the table says it is for: "[a] prefix string that shall be concatenated with the text string
/// presented to the user".
fn columns_of(columns: &[(&String, &Field)], attachment: &Attachment) -> Option<String> {
    let shown: Vec<String> = columns
        .iter()
        .filter_map(|(_, field)| {
            let value = column_value(field, attachment)?;
            Some(format!("{}: {value}", field.name))
        })
        .collect();
    (!shown.is_empty()).then(|| shown.join("  ·  "))
}

/// One column's value for one file.
///
/// Table 155's `/Subtype` decides *where the value lives*, which is the distinction
/// [`pdf_model::collection::FieldKind`] exists for: the first three kinds read §7.11.6's
/// collection item and the file-related ones read the file specification a host already has.
/// Only the second group can be answered from an [`Attachment`].
fn column_value(field: &Field, attachment: &Attachment) -> Option<String> {
    match field.kind {
        FieldKind::FileName => attachment.file_name.clone(),
        FieldKind::Description => attachment.description.clone(),
        FieldKind::Size => attachment.size.map(|size| format!("{size}")),
        FieldKind::ModificationDate => attachment.modified.clone(),
        FieldKind::CreationDate => attachment.created.clone(),
        _ => None,
    }
}

/// §12.4.3's article threads, as rows.
///
/// The clause states the structure and leaves the way in to a viewer — "[i]nteractive PDF
/// processors **may** provide navigation facilities to allow the user to follow a thread from one
/// bead to the next" — so what a panel owes is the list and a way to start following it.
///
/// A thread's title comes from Table 162's `/I`, whose contents "shall conform to the syntax for
/// the document information dictionary (see 14.3.3)", and a thread that states none is still a
/// thread: it gets the clause's own noun and its place in the `/Threads` array, whose order
/// §12.6.4.7 makes load-bearing.
#[must_use]
pub fn article_rows(threads: &[Thread]) -> Vec<PanelRow> {
    if threads.is_empty() {
        return vec![PanelRow::saying("This document states no article threads.")];
    }
    threads
        .iter()
        .enumerate()
        .map(|(index, thread)| PanelRow {
            detail: Some(match thread.beads.len() {
                1 => "1 bead".to_owned(),
                count => format!("{count} beads"),
            }),
            // The same message an outline row sends, and for the same reason: the *document*
            // decides what activating an object means. `viewer_core::interact` composes
            // §12.6.4.7's own thread action out of it, so following a thread lands on Table 163's
            // `/R` rather than on the page the first bead happens to sit on.
            action: RowAction::Activate(thread.id),
            ..PanelRow::item(
                thread
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("Article {}", index.saturating_add(1))),
            )
        })
        .collect()
}

/// §14.3.3's Table 349, with §14.3.2's metadata stream under it, as rows.
///
/// A label and a value apiece, and nothing here is clickable. **Both tables are shown rather than
/// merged**: Table 349's every text entry carries a NOTE pointing at an XMP counterpart, §12.2
/// ranks `dc:title` above `/Title` and nothing ranks the rest, and §14.3.4 leaves the disagreement
/// "at the discretion of the PDF processor" — so a panel that merged them would be hiding a
/// disagreement rather than resolving one.
///
/// `metadata` has three states and all three are said, which is what
/// [`viewer_core::Answer::Properties`] carries them for: no stream, a stream this reader refused,
/// and a stream it read.
#[must_use]
pub fn property_rows(
    information: &Information,
    metadata: Option<&Result<Xmp, XmpError>>,
) -> Vec<PanelRow> {
    let stated: [(&str, Option<String>); 9] = [
        ("Title", information.title.clone()),
        ("Author", information.author.clone()),
        ("Subject", information.subject.clone()),
        ("Keywords", information.keywords.clone()),
        ("Created in", information.creator.clone()),
        ("Converted by", information.producer.clone()),
        (
            "Created",
            stamp(information.created_date(), information.created.as_ref()),
        ),
        (
            "Modified",
            stamp(information.modified_date(), information.modified.as_ref()),
        ),
        (
            "Trapped",
            // Table 349's stated default is `Unknown`, so a document that says nothing about
            // trapping and one that says `Unknown` are the same statement and neither is shown.
            match information.trapped {
                Trapped::Unknown => None,
                other => Some(format!("{other:?}")),
            },
        ),
    ];
    let mut out: Vec<PanelRow> = stated
        .into_iter()
        .filter_map(|(label, value)| {
            value.map(|value| PanelRow {
                detail: Some(value),
                ..PanelRow::item(format!("{label}:"))
            })
        })
        .collect();
    if out.is_empty() {
        out.push(PanelRow::saying(
            "This document states no §14.3.3 information.",
        ));
    }
    metadata_rows(metadata, &mut out);
    out
}

/// §14.3.2's stream, under §14.3.3's dictionary and marked as the other place.
///
/// The four properties a person recognises, labelled with the names the stream itself uses, and a
/// count for everything else — a panel that listed every property of a large packet would be a
/// list of XML rather than a list of what the document says about itself.
fn metadata_rows(metadata: Option<&Result<Xmp, XmpError>>, out: &mut Vec<PanelRow>) {
    let Some(metadata) = metadata else {
        return;
    };
    let xmp = match metadata {
        Ok(xmp) => xmp,
        Err(error) => {
            out.push(PanelRow::saying(&format!(
                "§14.3.2's metadata stream could not be read: {error}"
            )));
            return;
        }
    };
    let stated: [(&str, Option<String>); 7] = [
        ("dc:title", xmp.title().map(str::to_owned)),
        (
            "dc:creator",
            xmp.authors().map(|authors| authors.join(", ")),
        ),
        ("dc:description", xmp.description().map(str::to_owned)),
        ("pdf:Producer", xmp.producer().map(str::to_owned)),
        ("xmp:CreatorTool", xmp.creator_tool().map(str::to_owned)),
        ("xmp:CreateDate", xmp.created().map(str::to_owned)),
        ("xmp:ModifyDate", xmp.modified().map(str::to_owned)),
    ];
    let shown = stated.iter().filter(|(_, value)| value.is_some()).count();
    out.push(PanelRow::saying("§14.3.2 (XMP):"));
    for (label, value) in stated {
        let Some(value) = value else { continue };
        out.push(PanelRow {
            detail: Some(value),
            ..PanelRow::item(format!("{label}:"))
        });
    }
    let rest = xmp.properties().len().saturating_sub(shown);
    if rest > 0 {
        out.push(PanelRow::saying(&format!("and {rest} other propert(ies).")));
    }
}

/// A §7.9.4 date as a person reads it, or the file's own string where it does not conform.
///
/// **The string is never dropped.** A producer that wrote a malformed date still wrote something,
/// and showing nothing would hide it — which is the same choice [`Information`] makes by keeping
/// the bytes beside the parse.
///
/// Public because §14.3.3's panel is not the only place a host shows one: Table 166's `/M` on a
/// §12.5.6.14 popup window is the same question, and two answers to it would be two formats in one
/// window.
#[must_use]
pub fn stamp(parsed: Option<pdf_syntax::Date>, written: Option<&String>) -> Option<String> {
    match parsed {
        Some(date) => Some(format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            date.year, date.month, date.day, date.hour, date.minute
        )),
        None => written.cloned(),
    }
}

/// One row of §12.3.4's panel: what the page is called, and its miniature where it states one.
///
/// The picture is [`pdf_render::Image`] rather than a toolkit's type for this crate's standing
/// reason — no pixel format here — and each host turns it into a `gdk::Texture`, a `QPixmap` or a
/// display list's paint.
#[derive(Debug, Clone)]
pub struct PageEntry {
    /// §12.4.2's label for the page, or `Page N` where the document states none.
    ///
    /// The fallback is a *choice* and the clause is why it cannot be dropped: "[e]ach page in a
    /// PDF document shall be identified by an integer page index … [i]t may also be identified by
    /// a page label", so a page with no label still has to be named, and the index plus one is
    /// what a person counts pages by.
    pub label: String,
    /// §12.3.4's `/Thumb`, decoded, or `None` for the pages that state none.
    ///
    /// Most pages of most documents, and not a defect: the clause's NOTE says thumbnails "are not
    /// required, and can be included for some pages and not for others". A page with none is
    /// still a row, because a panel that listed only the pages carrying a miniature would be a
    /// list of the document's *thumbnails* rather than of its pages.
    pub thumbnail: Option<Image>,
}

/// One page's row of §12.3.4's panel, asked for when a panel is about to draw it.
///
/// **One page at a time, and no cache here.** [`viewer_core::Query::Thumbnail`] is shaped this way
/// on purpose and this function keeps the shape: a host holds the miniatures of the rows it is
/// showing, in its own toolkit's image type, and a host that called this in a loop over its page
/// count would have moved `CLAUDE.md` section 2's eager work rather than removed it.
#[must_use]
pub fn page_entry(viewer: &Viewer, index: usize) -> PageEntry {
    PageEntry {
        label: match viewer.query(Query::PageLabel(index)) {
            Answer::Label(label) => label,
            _ => format!("Page {}", index.saturating_add(1)),
        },
        thumbnail: match viewer.query(Query::Thumbnail(index)) {
            Answer::Thumbnail(thumbnail) => Some(thumbnail.image),
            _ => None,
        },
    }
}

/// How many of §12.3.4's miniatures a panel keeps once it has decoded them.
///
/// **A bound rather than a cache size, and `CLAUDE.md` section 2's memory high-water is why.** A
/// miniature is a decoded RGBA raster — Table 87's own examples are a few score samples on a side,
/// which is tens of kilobytes apiece — so a panel that kept every one it had scrolled past would
/// hold tens of megabytes of a thousand-page document that a reader looked at once. What a panel
/// needs is the rows it is drawing and enough either side that scrolling back does not decode
/// again; two hundred and fifty-six is a couple of screens' worth in every direction and about
/// fourteen megabytes at the size above.
///
/// The number is one number: [`Miniatures`] holds it for the two hosts written in Rust, and
/// `viewer-qt`'s C++ `QPixmap` cache asks for it across the bridge rather than writing a second
/// one down.
pub const KEPT_MINIATURES: usize = 256;

/// The miniatures a panel has decoded, bounded by [`KEPT_MINIATURES`].
///
/// Generic over the picture, because that is the one thing this crate will not name: `T` is a
/// `gdk::Texture` in one host and a [`pdf_render::Image`] in another. What is shared is the
/// *policy* — decode on demand, keep what is near, and drop what is far — which is the half that
/// decides whether §12.3.4's panel obeys `CLAUDE.md` section 2 or merely postpones disobeying it.
///
/// Eviction is by distance from the row last asked for rather than by age. A panel is scrolled,
/// so the rows it will want next are the rows next to the one it just wanted; a least-recently-used
/// order would throw away the row above the viewport in favour of one a reader has left behind.
#[derive(Debug)]
pub struct Miniatures<T> {
    /// Page index to what that row draws. Ordered, so that the furthest entries are at the ends.
    held: std::collections::BTreeMap<usize, std::rc::Rc<Held<T>>>,
}

/// One page's row, once it has been asked for.
#[derive(Debug)]
pub struct Held<T> {
    /// §12.4.2's label, or the fallback [`page_entry`] chose.
    pub label: String,
    /// The miniature in the host's own picture type, or [`None`] for a page stating no `/Thumb`.
    pub picture: Option<T>,
}

impl<T> Default for Miniatures<T> {
    fn default() -> Self {
        Self {
            held: std::collections::BTreeMap::new(),
        }
    }
}

impl<T> Miniatures<T> {
    /// An empty panel, which is what a document that has just opened has.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets everything, which is what a new document is.
    pub fn clear(&mut self) {
        self.held.clear();
    }

    /// How many rows are held, which is what a test asserts the bound on.
    #[must_use]
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether nothing has been decoded yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// This row where it is already held, and [`None`] where nothing has asked for it.
    ///
    /// The reading half of [`Self::row`], for a host that draws from a shared borrow after having
    /// filled what it is about to draw: a panel that fetched *while* drawing would be deciding
    /// which rows exist from inside the code that lays them out.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Held<T>> {
        self.held.get(&index).map(std::rc::Rc::as_ref)
    }

    /// This row, decoding it with `make` if it is not held, and dropping the furthest rows if
    /// holding it puts the panel over [`KEPT_MINIATURES`].
    ///
    /// `make` takes the [`PageEntry`] [`page_entry`] answered and turns its picture into the
    /// host's own type; it is called exactly once per row that is not already held, which is what
    /// makes the whole thing demand-driven rather than merely deferred.
    pub fn row(&mut self, index: usize, make: impl FnOnce() -> Held<T>) -> std::rc::Rc<Held<T>> {
        if let Some(held) = self.held.get(&index) {
            return std::rc::Rc::clone(held);
        }
        let held = std::rc::Rc::new(make());
        self.held.insert(index, std::rc::Rc::clone(&held));
        while self.held.len() > KEPT_MINIATURES {
            // The two ends of an ordered map are the two candidates for "furthest from here", and
            // one comparison picks between them. A map with neither end is empty, which the loop's
            // own condition has already excluded — and is answered by stopping rather than by an
            // assertion, because a bound that holds late is better than a bound that panics.
            let (Some(&first), Some(&last)) =
                (self.held.keys().next(), self.held.keys().next_back())
            else {
                break;
            };
            let furthest = if index.abs_diff(first) >= index.abs_diff(last) {
                first
            } else {
                last
            };
            self.held.remove(&furthest);
        }
        held
    }
}

/// Which of this program's panels a window is showing.
///
/// **A closed set, and that is what makes "all three hosts stay level" checkable.** `doc/todo/30`
/// states the rule — a feature lands on the boundary and `viewer-ui`, `viewer-gtk` and `viewer-qt`
/// all adopt it — and until this enumeration existed it was a rule about features with no purchase
/// on the panel a person actually clicks: the tier-2 host drew six lists and the two native hosts
/// drew three. Each host now carries a test that walks [`Tab::ALL`] through a match exhaustive over
/// this enumeration, so a seventh panel added here fails to compile in three hosts. It is
/// [`crate::keys::Key`]'s mechanism applied to the other thing a window shows (ADR 0526).
///
/// Deliberately **not** in [`viewer_core`], for that crate's rule 5: a panel is chrome. What the
/// core answers is five queries; which of them a window offers as a tab, and in what order, is a
/// host's — and it is one decision rather than three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tab {
    /// §12.3.3's outline.
    #[default]
    Contents,
    /// §12.3.4's thumbnail images, one row per page.
    Pages,
    /// §8.11.4.3's `/Order`.
    Layers,
    /// §7.11.4's embedded files, or §12.3.5's collection where the document states one.
    Files,
    /// §12.4.3's article threads.
    Articles,
    /// §14.3.3's document information dictionary, and §14.3.2's metadata stream under it.
    Document,
}

impl Tab {
    /// Every panel, in the order they are offered.
    ///
    /// §12.3.3's outline first because Table 29's `UseOutlines` is the page mode documents state
    /// by far the most often, and §12.3.4's pages beside it because those two are the ones a
    /// reader navigates with. The list is checked against the enumeration by
    /// `every_tab_is_in_the_list_a_host_is_held_to`.
    pub const ALL: &'static [Self] = &[
        Self::Contents,
        Self::Pages,
        Self::Layers,
        Self::Files,
        Self::Articles,
        Self::Document,
    ];

    /// What the tab says.
    ///
    /// One wording rather than three, for [`crate::status`]'s reason: the third copy of a sentence
    /// is where two hosts stop agreeing about what they are saying.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Contents => "Contents",
            Self::Pages => "Pages",
            Self::Layers => "Layers",
            Self::Files => "Files",
            // Short enough for a sixth tab in a narrow panel, and the clause's own noun: §12.4.3
            // calls the thing "an article thread" and its `/I` title is the article's.
            Self::Articles => "Read",
            Self::Document => "About",
        }
    }

    /// Which of [`Self::ALL`] this is, for a host that indexes its tabs by number.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Contents => 0,
            Self::Pages => 1,
            Self::Layers => 2,
            Self::Files => 3,
            Self::Articles => 4,
            Self::Document => 5,
        }
    }

    /// The tab at a position in [`Self::ALL`], or [`None`] past its end.
    #[must_use]
    pub fn at(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// Which panel Table 29's `/PageMode` asks a window to open on.
    ///
    /// §7.7.2 states "how the document shall be displayed when opened" and §12.2's
    /// `/NonFullScreenPageMode` states "how to display the document on exiting full-screen mode",
    /// so this is asked twice for two clauses and answered once.
    ///
    /// [`None`] for the two names that are not a panel at all: `UseNone` asks for neither an
    /// outline nor thumbnails, and `FullScreen` is [`crate::presentation`]'s. **`UseThumbs` is not
    /// among them and used to be** — both native hosts reported it as a mode they had no panel for
    /// until §12.3.4's list was built, which is trap 5 discharged by implementing the clause rather
    /// than by saying it was not.
    #[must_use]
    pub const fn of_page_mode(mode: PageMode) -> Option<Self> {
        match mode {
            PageMode::UseNone | PageMode::FullScreen => None,
            PageMode::UseOutlines => Some(Self::Contents),
            PageMode::UseThumbs => Some(Self::Pages),
            PageMode::UseOptionalContent => Some(Self::Layers),
            PageMode::UseAttachments => Some(Self::Files),
        }
    }
}

#[cfg(test)]
mod tests {
    use pdf_model::article::Thread;
    use pdf_model::metadata::Information;
    use pdf_model::viewer_preferences::PageMode;
    use pdf_syntax::ObjectId;

    use super::{Held, KEPT_MINIATURES, Miniatures, Tab, article_rows, property_rows};

    /// [`Tab::ALL`] is the list three hosts are held to, so it has to be the whole enumeration.
    ///
    /// The match is what makes this a check rather than a second hand-written list: a variant added
    /// above and forgotten here fails to compile. `Key::ALL`'s test, one panel over.
    #[test]
    fn every_tab_is_in_the_list_a_host_is_held_to() {
        for tab in Tab::ALL {
            // Exhaustive on purpose. A new panel lands here as a compile error.
            match tab {
                Tab::Contents
                | Tab::Pages
                | Tab::Layers
                | Tab::Files
                | Tab::Articles
                | Tab::Document => {}
            }
        }
        let mut seen: Vec<Tab> = Vec::new();
        for tab in Tab::ALL {
            assert!(!seen.contains(tab), "{tab:?} is in the list twice");
            seen.push(*tab);
        }
        assert_eq!(seen.len(), 6, "the list is the enumeration, and no shorter");
    }

    /// A tab's place in the list and its own index are one number, whichever way it is asked.
    ///
    /// Two ways of saying the same thing is two ways to disagree, and a host that indexes a
    /// notebook by [`Tab::index`] while translating a click through [`Tab::at`] would open the
    /// wrong panel silently.
    #[test]
    fn a_tabs_index_is_where_it_is_in_the_list() {
        for (place, tab) in Tab::ALL.iter().enumerate() {
            assert_eq!(tab.index(), place);
            assert_eq!(Tab::at(place), Some(*tab));
        }
        assert_eq!(Tab::at(Tab::ALL.len()), None);
    }

    /// Table 29's four panel names reach a panel, and its two non-panel names reach none.
    ///
    /// `UseThumbs` is the row this test exists for: both native hosts used to report it as a mode
    /// they had no panel for, which is trap 5 honestly discharged and still a clause unobeyed.
    #[test]
    fn every_page_mode_that_names_a_panel_reaches_one() {
        assert_eq!(
            Tab::of_page_mode(PageMode::UseOutlines),
            Some(Tab::Contents)
        );
        assert_eq!(Tab::of_page_mode(PageMode::UseThumbs), Some(Tab::Pages));
        assert_eq!(
            Tab::of_page_mode(PageMode::UseOptionalContent),
            Some(Tab::Layers)
        );
        assert_eq!(
            Tab::of_page_mode(PageMode::UseAttachments),
            Some(Tab::Files)
        );
        assert_eq!(Tab::of_page_mode(PageMode::UseNone), None);
        assert_eq!(Tab::of_page_mode(PageMode::FullScreen), None);
    }

    /// §12.4.3: a thread with no `/I` title is still a thread, and its beads are counted.
    #[test]
    fn a_thread_with_no_title_keeps_its_place_in_the_threads_array() {
        let threads = vec![
            Thread {
                id: ObjectId::new(4, 0),
                title: Some("Leading article".to_owned()),
                beads: Vec::new(),
            },
            Thread {
                id: ObjectId::new(9, 0),
                title: None,
                beads: Vec::new(),
            },
        ];
        let rows = article_rows(&threads);
        assert_eq!(rows[0].label, "Leading article");
        assert_eq!(rows[1].label, "Article 2");
        assert_eq!(rows[0].detail.as_deref(), Some("0 beads"));
        assert!(rows.iter().all(|row| !row.note));
    }

    /// A document with no threads gets the sentence rather than an empty list.
    ///
    /// Trap 5's shape for a panel: an empty list and a list this program failed to fill look
    /// identical, and only one of them is the document being quiet.
    #[test]
    fn a_document_with_no_threads_says_so() {
        let rows = article_rows(&[]);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].note);
    }

    /// §12.3.4's panel keeps what is near the row it is drawing and drops what is far.
    ///
    /// The bound is `CLAUDE.md` section 2's memory high-water reaching a panel: a thousand-page document
    /// scrolled end to end decodes a thousand miniatures and holds [`KEPT_MINIATURES`] of them.
    #[test]
    fn a_panel_holds_what_it_is_near_and_no_more_than_the_bound() {
        let mut held: Miniatures<usize> = Miniatures::new();
        let mut decoded = 0_usize;
        for index in 0..1000 {
            held.row(index, || {
                decoded += 1;
                Held {
                    label: format!("Page {}", index + 1),
                    picture: Some(index),
                }
            });
        }
        assert_eq!(decoded, 1000, "every row asked for is decoded exactly once");
        assert_eq!(held.len(), KEPT_MINIATURES);
        // Scrolled to the end, so what is kept is the end: the rows a reader is about to scroll
        // back through, not the ones they left a thousand pages ago.
        assert_eq!(
            held.row(999, || panic!("row 999 was just asked for"))
                .picture,
            Some(999)
        );
        assert_eq!(decoded, 1000);
        held.clear();
        assert!(held.is_empty());
    }

    /// §14.3.3: an absent `/Info` is not an error, and a panel says which of the two it is.
    #[test]
    fn a_document_stating_no_information_says_so_and_states_no_entries() {
        let rows = property_rows(&Information::default(), None);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].note);
    }

    /// Table 349's stated default for `/Trapped` is `Unknown`, so it is not a row.
    ///
    /// A document that says nothing about trapping and one that says `Unknown` have made the same
    /// statement, and a panel showing `Trapped: Unknown` for every file in the world would be
    /// reporting its own default back.
    #[test]
    fn the_entries_a_document_states_are_the_rows_and_a_default_is_not_one() {
        let information = Information {
            title: Some("A paper".to_owned()),
            ..Information::default()
        };
        let rows = property_rows(&information, None);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Title:");
        assert_eq!(rows[0].detail.as_deref(), Some("A paper"));
        assert!(!rows[0].note);
    }
}
