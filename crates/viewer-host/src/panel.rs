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
use pdf_model::collection::{Collection, Field, FieldKind, Initial, Layout, SplitDirection, View};
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
    /// Table 154's schema for this row, one cell per field, in Table 155's `/O` order.
    ///
    /// Empty for every panel but §12.3.5's, where Table 153's `/View D` asks for "all information
    /// in the Schema dictionary presented in a multi-column format" and `/View T` for "a subset of
    /// information from the Schema dictionary". A toolkit with columns draws these in them; one
    /// without draws [`PanelRow::detail`], which carries the same cells joined. Both come from one
    /// reading, so two windows cannot disagree about what a file's fields say (ADR 1215).
    pub cells: Vec<Cell>,
    /// The picture this row carries, where [`Mode`] draws one for it.
    ///
    /// `None` in every panel but §12.3.5's, and in that one wherever the layout draws no picture
    /// for the row. [`Picture`] says *how large* it is and *where it comes from*; the clause
    /// states no artwork for any of them, so each toolkit draws its own (ADRs 1215, 1251).
    pub picture: Option<Picture>,
    /// The rows underneath it.
    pub children: Vec<PanelRow>,
}

/// One of Table 154's schema fields, as it stands against one file.
///
/// Table 155's `/N` is "[t]he textual field name that shall be presented to the user by the
/// interactive PDF processor", so the heading is the document's own word and never this
/// program's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// Table 155's `/N`.
    pub heading: String,
    /// What this file says for that field, with Table 47's `/P` prefix already on it.
    pub value: String,
}

/// Which presentation a collection is shown in: Table 153's two views, and §12.3.6's three
/// further named layouts.
///
/// Table 153 states each `/View` value as its own `shall`, and two of them describe a *list of
/// files*: `H` is the sidebar being closed until a person opens it, and `C` defers to §12.3.6's
/// navigator, which [`presentation`] resolves before answering. §12.3.6's `Tree` is what
/// [`collection_rows`] builds by construction and shares [`Mode::Details`]'s arrangement; the
/// other three describe surfaces of their own and are the three variants after it (ADR 1251).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `D`: "The collection view shall be presented in details mode, with all information in the
    /// Schema dictionary presented in a multi- column format."
    Details,
    /// `T`: "The collection view shall be presented in tile mode, with each file in the collection
    /// denoted by a small icon and a subset of information from the Schema dictionary."
    Tile,
    /// §12.3.6's `FilmStrip`: "a strip of thumbnails, providing an index to the file attachments
    /// within the collection. The selected attachment should be previewed alongside the index."
    ///
    /// One run rather than a tree, because a strip is one run — and it indexes the folders as
    /// well, which the clause states outright: "[t]hese thumbnails provide an index into the
    /// files and folders present within the collection."
    FilmStrip,
    /// §12.3.6's `FreeForm`: "thumbnails for each item in the collection contents are displayed
    /// at a random location on the view."
    ///
    /// Where each thumbnail lands is the *host's*, because a random location is not a fact about
    /// the document and ADR 1168's property is what decides which of those cross.
    FreeForm,
    /// §12.3.6's `Linear`: "a large size preview of one file attachment in the collection and
    /// displays alongside the preview the metadata for the file attachment, including the name,
    /// description and other collection schema entries."
    Linear,
}

impl Mode {
    /// Whether this layout arranges the files flat rather than in §12.3.5.2's folder tree.
    ///
    /// Table 153's two views draw the tree: §12.3.5.2 is a separate `shall` about where a file
    /// sits, and ADR 1215 keeps the folders in both of them. The three §12.3.6 layouts each
    /// describe a surface with no nesting in it — a strip, a scatter, a single preview — so the
    /// folder a file sits in is said on the row instead of drawn around it.
    #[must_use]
    pub const fn is_flat(self) -> bool {
        matches!(self, Self::FilmStrip | Self::FreeForm | Self::Linear)
    }
}

/// What kind of thing a tile's icon stands for, which is all this crate decides about it.
///
/// Table 153's `/View T` asks for "a small icon" and states nothing about its artwork, so this is
/// the `Text` annotation's situation one clause over (`CLAUDE.md` principle 5): a deliberate
/// choice, documented as one. What the choice *is*: the icon names the file's kind, taken from
/// Table 44's `/Subtype` where the document states one, because that is the only thing about the
/// file the standard puts in this program's hands. Each toolkit then draws its own picture — GTK
/// and Qt from the icon theme, `viewer-ui` from its own ink (ADR 1215).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// §12.3.5.2's folder, which is a node of the tree rather than a file in it.
    Folder,
    /// A media type whose type is `image`.
    Image,
    /// A media type whose type is `audio`.
    Audio,
    /// A media type whose type is `video`.
    Video,
    /// A media type whose type is `text`, and `application/pdf` with it.
    Document,
    /// A file whose media type the document does not state, or states as something else.
    File,
}

/// What picture a row carries, and how much room the layout gives it.
///
/// Table 153 and §12.3.6 ask for three different pictures of the same file, and the difference
/// between them is size and source rather than artwork: `/View T` wants "a small icon",
/// `FilmStrip` and `FreeForm` want "thumbnails", `Linear` wants "a large size preview". Every one
/// of them carries an [`Icon`], because the *kind* of a file is what this crate decides about it
/// (ADR 1215); the two larger ones also say that a host should ask [`attachment_preview`] for the
/// attachment's own picture and fall back to the icon where the file states none (ADR 1251).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picture {
    /// Table 153's `/View T`: "each file in the collection denoted by a small icon".
    Icon(Icon),
    /// §12.3.6's thumbnail — `FilmStrip`'s "strip of thumbnails" and `FreeForm`'s scatter.
    Thumbnail(Icon),
    /// §12.3.6's "large size preview": `Linear`'s one attachment, and the one `FilmStrip` says
    /// "should be previewed alongside the index".
    Preview(Icon),
}

impl Picture {
    /// The kind of thing the row stands for, whatever size the layout draws it at.
    #[must_use]
    pub const fn kind(self) -> Icon {
        match self {
            Self::Icon(icon) | Self::Thumbnail(icon) | Self::Preview(icon) => icon,
        }
    }

    /// Whether a host should ask [`attachment_preview`] for the file's own picture first.
    ///
    /// False for [`Self::Icon`], which Table 153 makes an icon outright, and true for the two
    /// sizes §12.3.6 builds out of pictures of the attachments themselves.
    #[must_use]
    pub const fn wants_the_files_own_picture(self) -> bool {
        matches!(self, Self::Thumbnail(_) | Self::Preview(_))
    }
}

impl Icon {
    /// The icon-theme name a desktop toolkit looks this kind up by.
    ///
    /// These are the freedesktop icon-naming names, which a `GtkImage` and a `QIcon` both resolve
    /// from the running theme — so two of the three windows share one lookup and neither ships
    /// artwork. `viewer-ui` draws its own page and has no theme, so it does not ask this.
    ///
    /// **Not a decision the standard makes**, and not one either toolkit makes either: Table 153
    /// asks for "a small icon" and stops (ADR 1215).
    #[must_use]
    pub const fn theme_name(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::Image => "image-x-generic",
            Self::Audio => "audio-x-generic",
            Self::Video => "video-x-generic",
            Self::Document => "text-x-generic",
            Self::File => "application-x-generic",
        }
    }
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
            cells: Vec::new(),
            picture: None,
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
        cells: Vec::new(),
        picture: None,
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
            cells: Vec::new(),
            picture: None,
            children: Vec::new(),
        },
        Layer::Collection { label, children } => PanelRow {
            label: label.clone().unwrap_or_default(),
            detail: None,
            expanded: true,
            action: RowAction::Inert,
            note: false,
            emphasis: false,
            cells: Vec::new(),
            picture: None,
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
            // §12.3.5's schema is what fills these, and this list is the one a document with no
            // `/Collection` gets: the same files, unarranged.
            cells: Vec::new(),
            picture: None,
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
    order: &[String],
    attachments: &[Attachment],
) -> Vec<PanelRow> {
    let mode = presentation(collection);
    let mut columns: Vec<(&String, &Field)> = collection
        .schema
        .iter()
        .filter(|(_, field)| field.visible)
        .collect();
    columns.sort_by_key(|(key, field)| (field.order.unwrap_or(i64::MAX), (*key).clone()));
    // Table 153's `/View T` is "a subset of information from the Schema dictionary" against
    // `/View D`'s "all information in the Schema dictionary", so the tile takes the head of the
    // order the document itself stated and the details view takes the whole of it.
    if mode == Mode::Tile {
        columns.truncate(TILE_FIELDS);
    }

    // Table 153's `/Sort`, applied once and here: every list below is a *filter* of this one, so
    // sorting the files before they are split into folders puts each folder's own rows in the
    // stated order without any of those filters knowing there is an order at all.
    let sorted = in_sort_order(order, attachments);
    let attachments = sorted.as_slice();

    // §12.3.6's two layouts that show one attachment larger than the rest need to know which one
    // before a row is built, because it is the row that differs rather than the drawing.
    let previewed = match mode {
        Mode::FilmStrip | Mode::Linear => previewed_file(initial, attachments),
        Mode::Details | Mode::Tile | Mode::FreeForm => None,
    };
    let shape = Shape {
        mode,
        columns: &columns,
        previewed: previewed.as_deref(),
        folders: folder_names(collection),
    };

    let mut rows = if mode.is_flat() {
        flat_rows(collection, attachments, &shape)
    } else {
        match collection.folders.as_ref() {
            Some(root) => {
                let stated = &stated_ids(root);
                let mut rows = files_where(attachments, &shape, &|id| {
                    id.is_none_or(|id| !stated.contains(&id))
                });
                rows.push(folder_row(root, attachments, &shape));
                rows
            }
            None => files_where(attachments, &shape, &|_| true),
        }
    };

    mark_initial(&mut rows, initial);

    if rows.is_empty() {
        // §12.3.5.1's "an empty preview window", and the case a document with a `/Collection` and
        // no `/EmbeddedFiles` tree is in. An empty panel and a panel this program failed to fill
        // look identical; only one of them is the file being quiet.
        rows.push(PanelRow::saying("This collection lists no files."));
    }
    if let Some(sentence) = unsupported_presentation(collection) {
        rows.push(PanelRow::saying(&sentence));
    }
    if let Some(sentence) = unused_furniture(collection) {
        rows.push(PanelRow::saying(&sentence));
    }
    if let Some(sentence) = restricted_names(collection) {
        rows.push(PanelRow::saying(&sentence));
    }
    rows
}

/// What every row of one collection has in common, which is everything but the file.
///
/// One value rather than four parameters threaded through five functions: the layout, the columns
/// it shows, which attachment it previews and what §12.3.5.2's folders are called are decided once
/// in [`collection_rows`] and read everywhere below it.
struct Shape<'a> {
    /// Which presentation, resolved by [`presentation`].
    mode: Mode,
    /// Table 154's visible schema fields in Table 155's `/O` order, already cut to what this mode
    /// shows.
    columns: &'a [(&'a String, &'a Field)],
    /// The `/EmbeddedFiles` key of the one attachment §12.3.6's `FilmStrip` and `Linear` show
    /// larger than the rest, or [`None`] in a layout that shows them all alike.
    previewed: Option<&'a str>,
    /// §12.3.5.2's folder identifiers and what each is called, for the layouts that draw no tree.
    folders: std::collections::BTreeMap<u32, String>,
}

impl Shape<'_> {
    /// The picture this row carries, at the size its layout draws.
    fn picture(&self, kind: Icon, previewed: bool) -> Option<Picture> {
        match self.mode {
            Mode::Details => None,
            Mode::Tile => Some(Picture::Icon(kind)),
            Mode::FilmStrip if previewed => Some(Picture::Preview(kind)),
            Mode::FilmStrip | Mode::FreeForm => Some(Picture::Thumbnail(kind)),
            Mode::Linear => previewed.then_some(Picture::Preview(kind)),
        }
    }

    /// Whether this row carries the schema's fields.
    ///
    /// Table 153's two views put them on every row. §12.3.6's `FilmStrip` puts them beside the one
    /// attachment it previews — "[t]he selected attachment should be previewed alongside the
    /// index" — and `Linear` likewise; `FreeForm` states nothing but thumbnails and a selection,
    /// so its items carry their names and no fields.
    const fn shows_fields(&self, previewed: bool) -> bool {
        match self.mode {
            Mode::Details | Mode::Tile => true,
            Mode::FilmStrip | Mode::Linear => previewed,
            Mode::FreeForm => false,
        }
    }

    /// Which of §12.3.5.2's folders a file sits in, where the layout draws no tree to put it in.
    ///
    /// §12.3.5.2 makes membership of the folder structure a `shall`, and the three flat layouts
    /// draw no folder around a file — so the arrangement is *said* on the row instead of lost.
    fn where_it_sits(&self, folder: Option<u32>) -> Option<String> {
        if !self.mode.is_flat() {
            return None;
        }
        self.folders.get(&folder?).map(|name| format!("in {name}"))
    }
}

/// §12.3.5.2's folders by identifier, for a layout that names them instead of nesting in them.
fn folder_names(collection: &Collection) -> std::collections::BTreeMap<u32, String> {
    let mut names = std::collections::BTreeMap::new();
    let mut stack: Vec<&pdf_model::collection::Folder> = collection.folders.iter().collect();
    while let Some(folder) = stack.pop() {
        names.insert(folder.id, folder.name.clone());
        stack.extend(folder.children.iter());
    }
    names
}

/// The one attachment §12.3.6's `FilmStrip` and `Linear` show larger than the rest.
///
/// `Linear` "provides a large size preview of **one** file attachment in the collection", and
/// `FilmStrip` previews "the selected attachment" — so both need one named before anybody has
/// selected anything, and §12.3.5.1's `/D` is the document's own answer to that question. Where
/// `/D` names the container, names nothing, or names a file the name tree does not hold, the
/// clause's own fallback applies: "the first item from the list of files to display in its user
/// interface", which in this list is the first in Table 153's `/Sort` order.
fn previewed_file(initial: &Initial, attachments: &[&Attachment]) -> Option<String> {
    if let Initial::Embedded(name) = initial
        && attachments.iter().any(|file| &file.name == name)
    {
        return Some(name.clone());
    }
    attachments.first().map(|file| file.name.clone())
}

/// §12.3.6's three flat layouts, which arrange the same files without §12.3.5.2's tree.
///
/// `FilmStrip` indexes the folders as well as the files, because the clause says its thumbnails
/// "provide an index into the files and folders present within the collection"; `FreeForm` and
/// `Linear` are stated over "the file attachments" alone, so their folders are named on each
/// row by [`Shape::where_it_sits`] rather than drawn.
///
/// The folders come first and in the tree's own order. Table 153's `/Sort` orders the *items* of
/// the collection and a folder is not one of them, so the standard states no order between the
/// two groups; putting the coarser half of the index first is this program's choice, made once
/// here (ADR 1251).
fn flat_rows(collection: &Collection, attachments: &[&Attachment], shape: &Shape) -> Vec<PanelRow> {
    let mut rows = Vec::new();
    if shape.mode == Mode::FilmStrip
        && let Some(root) = collection.folders.as_ref()
    {
        index_folders(root, shape, &mut rows);
    }
    rows.extend(files_where(attachments, shape, &|_| true));
    rows
}

/// Every folder of the tree as one entry of a strip, depth first.
fn index_folders(folder: &pdf_model::collection::Folder, shape: &Shape, out: &mut Vec<PanelRow>) {
    out.push(PanelRow {
        detail: folder.description.clone(),
        // A folder is not a file: it has no bytes to take out, and in a strip it has no children
        // drawn under it either, so what it does is nothing until a person chooses it.
        action: RowAction::Inert,
        picture: shape.picture(Icon::Folder, false),
        ..PanelRow::item(folder.name.clone())
    });
    for child in &folder.children {
        index_folders(child, shape, out);
    }
}

/// The attachments in the order Table 153's `/Sort` states, or as they came where it states none.
///
/// §12.3.5.1's Table 153, on the entry:
///
/// > A collection sort dictionary, which specifies the order in which items in the collection
/// > shall be sorted in the user interface
///
/// The ordering itself is `pdf_model::collection::sorted_keys`, resolved before this crate sees
/// it and carried by `viewer_core::Answer::Collection` — what is left here is applying it, which
/// is a lookup rather than a comparison. A key the order does not name keeps its place after the
/// ones it does, so a list that disagrees with the order in any way still shows every file: this
/// panel's standing rule is that a file cannot fall out of it by being filed oddly (ADR 1168).
///
/// Public because `viewer-ui` builds its own rows — its row is a widget rather than text, which
/// is the division [`outline_rows`] already states — and *applying* an order is not a thing two
/// windows may answer differently.
#[must_use]
pub fn in_sort_order<'a>(order: &[String], attachments: &'a [Attachment]) -> Vec<&'a Attachment> {
    let mut sorted: Vec<&Attachment> = attachments.iter().collect();
    if order.is_empty() {
        return sorted;
    }
    let places: std::collections::HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(at, key)| (key.as_str(), at))
        .collect();
    // Stable, so the files the order does not name keep the order they arrived in — which is the
    // `/EmbeddedFiles` tree's, the one other order the document itself stated.
    sorted.sort_by_key(|attachment| {
        places
            .get(attachment.name.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    sorted
}

/// What a panel says about §12.3.5.2's restricted names, or nothing where the document breaks none.
///
/// §12.3.5.2 bounds a collection's names and then hands the reader a choice: "[a]n interactive
/// PDF processor may choose to support invalid names or not. If not, an appropriate error message
/// shall be provided." **This program supports them** — ADR 1050 — so the clause asks it for no
/// message at all, and this sentence is the project's own rule rather than the standard's: a name
/// the file got wrong is shown as the file wrote it, and a person is told that is what happened
/// instead of being left to wonder whether the folder is really called `a:b`.
///
/// One sentence for all three windows, for the reason ADR 0711 gives about the rest of this
/// clause: what a collection says is not one toolkit's to phrase.
#[must_use]
pub fn restricted_names(collection: &Collection) -> Option<String> {
    // The report carries one entry per restriction, and a person counts names.
    let names: std::collections::BTreeSet<&pdf_model::collection::Named> = collection
        .invalid_names
        .iter()
        .map(|defect| &defect.owner)
        .collect();
    match names.len() {
        0 => None,
        1 => Some(
            "One name here is not a valid file name; it is shown as the document wrote it."
                .to_owned(),
        ),
        many => Some(format!(
            "{many} names here are not valid file names; they are shown as the document wrote them."
        )),
    }
}

/// The named layouts these panels are capable of displaying, which is what §12.3.6's selection
/// rule has to be given.
///
/// §12.3.6 asks a processor for the first layout *it can draw*, so the answer is a property of
/// this program and not of the file:
///
/// - `Tree` is what [`collection_rows`] builds — the clause describes it as "a classic folder
///   view of the contents of a collection", with the folder structure as the nodes and the
///   attachments as the leaves, which is this panel by construction;
/// - `D` is Table 153's details view, the schema's visible columns as each row's detail line;
/// - `H` is met by the sidebar being closed until a person opens it.
///
/// - `T` is Table 153's tile view, each file under [`Icon`] with [`TILE_FIELDS`] of the schema's
///   own fields beside it;
/// - `FilmStrip` is the files and folders in one run of thumbnails with the previewed attachment's
///   fields beside it;
/// - `FreeForm` is the same thumbnails with the host scattering them;
/// - `Linear` is one attachment large, with the schema's fields and the file specification's
///   beside it.
///
/// **Every name Table 160 defines is here**, so §12.3.6's selection rule chooses from the whole
/// table and [`unsupported_presentation`] is left with the one case that remains: a navigator
/// naming only layouts this table does not define (ADR 1251).
pub const DRAWN_LAYOUTS: [Layout; 7] = [
    Layout::Tree,
    Layout::View(View::Details),
    Layout::View(View::Tile),
    Layout::View(View::Hidden),
    Layout::FilmStrip,
    Layout::FreeForm,
    Layout::Linear,
];

/// How many of Table 154's schema fields a tile shows, which is what "a subset" is decided to be.
///
/// Table 153 puts the two views on one axis and states no number for either: `D` has "all
/// information in the Schema dictionary" and "provides the most information to the user", `T` has
/// "a subset" and "provides top-level information about the file attachments". So what the
/// standard fixes is that the tile shows *fewer*, and the count is this program's — two, being
/// the most a row that also carries an icon holds at a panel's width, taken from the head of the
/// producer's own `/O` order so that the fields a document put first are the fields it keeps
/// (ADR 1215).
pub const TILE_FIELDS: usize = 2;

/// What a panel says when a document asks to be presented in a way it cannot draw, or nothing.
///
/// Two clauses meet here, and both are addressed to a processor rather than to a producer:
///
/// - §12.3.5.1 makes Table 153's `/View` the initial presentation — "[w]hen an interactive PDF
///   processor first opens a PDF document containing a collection, it shall display the contents
///   according to the View key of the collection dictionary";
/// - §12.3.6 says that "[w]hen a navigator dictionary is present, a PDF processor should use the
///   value of the Layout entry to present the collection to the user", and that where several are
///   named a processor "should present the first one it is capable of displaying in the order
///   present in the array".
///
/// The navigator is asked first because it is the more specific instruction and because Table 153
/// makes `/Navigator` the presentation when `/View` is `C`. [`pdf_model::collection::Navigator`]
/// answers the selection from [`DRAWN_LAYOUTS`], which is now every name Table 160 defines — so
/// `None` from it means the file names *only* layouts the table does not define, which §12.3.6
/// permits ("[t]his mechanism is inherently extensible and allows inclusion of custom named
/// layouts") while requiring a producer to name one of the seven as well.
///
/// **This sentence is the project's and not the standard's**, on ADR 0711's reason for the rest of
/// this clause: neither clause asks for a message. What it prevents is the failure trap 5 names —
/// a presentation the document asked for, silently replaced by a different one.
#[must_use]
pub fn unsupported_presentation(collection: &Collection) -> Option<String> {
    if let Some(navigator) = collection.navigator.as_ref() {
        if navigator.preferred(&DRAWN_LAYOUTS).is_some() {
            return None;
        }
        // Named, because a report that does not say what it matched cannot be checked against the
        // document (trap 11). A file naming no layout at all is Table 160's `/Layout` missing,
        // which is the entry it makes required.
        let named: Vec<String> = navigator.layouts.iter().map(layout_name).collect();
        return Some(match named.len() {
            0 => "This collection states a navigator that names no layout.".to_owned(),
            _ => format!(
                "This collection asks to be presented as {}, which Table 160 does not define and \
                 this panel does not draw; its files are shown as a tree.",
                named.join(" or ")
            ),
        });
    }
    match collection.view {
        // `/View C` with no `/Navigator` is a file contradicting §12.3.5.1's Table 153, which
        // makes the navigator "[r]equired if the value of View is C". Nothing to select from.
        View::Navigator => Some(
            "This collection asks to be presented by a navigator it does not state.".to_owned(),
        ),
        View::Details | View::Tile | View::Hidden => None,
    }
}

/// What a panel says about the presentation entries it reads and does not draw, or nothing.
///
/// Two entries, and neither carries a `shall` at a processor:
///
/// - Table 157's `/Colors` is "a suggested set of colours for use by a collection layout", whose
///   only sentence about using them is a NOTE — "[i]t is recommended that a layout use the colours
///   provided" — and a NOTE states no requirement;
/// - Table 158's `/Direction` `H` and `V` and its `/Position` describe "the orientation of the
///   splitter bar" of an initial view in which "the available display area **may** be divided by a
///   splitter bar into two areas", one of them "a preview of the initial or currently selected
///   document of the collection". This program's window divides the panel from the *container's*
///   own pages rather than from a preview of the selected attachment (ADR 0202), so a percentage
///   whose subject is the other pair of areas is not applied to this one. ADR 1252.
///
/// Table 158's `/Direction` `N` is not here: it says the window is not divided at all, which is a
/// statement about the window rather than about a bar, and [`whole_window`] obeys it.
///
/// **This sentence is the project's and not the standard's**, on ADR 0711's reason for the rest of
/// this clause: what the document asked for is said rather than dropped, because an entry read and
/// silently unused is indistinguishable from one nobody read (trap 5).
#[must_use]
pub fn unused_furniture(collection: &Collection) -> Option<String> {
    let colours = collection.colours != pdf_model::collection::Colours::default();
    let splitter = matches!(
        collection.split.as_ref().map(|split| &split.direction),
        Some(SplitDirection::Horizontal | SplitDirection::Vertical)
    );
    match (colours, splitter) {
        (false, false) => None,
        (true, false) => Some(
            "This collection suggests colours for a layout; this window draws its own.".to_owned(),
        ),
        (false, true) => Some(
            "This collection asks for a splitter bar between the file list and a preview; this \
             window puts the files beside the document's own pages."
                .to_owned(),
        ),
        (true, true) => Some(
            "This collection suggests colours for a layout and a splitter bar between the file \
             list and a preview; this window draws its own colours and puts the files beside the \
             document's own pages."
                .to_owned(),
        ),
    }
}

/// Table 158's `/Direction` `N`: whether the file navigation view takes the whole window.
///
/// §12.3.5.1, Table 158:
///
/// > N indicates that the window is not split. The entire window region shall be dedicated to the
/// > file navigation view.
///
/// A `shall` at a processor, and the one entry of the collection split dictionary that is not
/// about a bar: `H` and `V` state where a splitter goes, and `N` states that there is no second
/// area at all. A window with no splitter widget can still obey it, which is why it is answered
/// here rather than departed with the rest of the entry (ADR 1252).
///
/// **Not while Table 153's `/View` is `H`.** That value says "[t]he collection view shall be
/// initially hidden", and a view that is hidden cannot be the one the window is dedicated to; the
/// two entries would otherwise ask for opposite things at once.
#[must_use]
pub fn whole_window(collection: &Collection) -> bool {
    if collection.view == View::Hidden {
        return false;
    }
    matches!(
        collection.split.as_ref().map(|split| &split.direction),
        Some(SplitDirection::None)
    )
}

/// §12.3.6's preview picture for one attachment, asked for when a panel is about to draw it.
///
/// [`Picture::wants_the_files_own_picture`] says which rows have one. The picture is the
/// attachment's *own* first page's §12.3.4 `/Thumb`, which that clause makes the image `XObject`
/// the page's own entry names — so it is the miniature that file's producer wrote, and this
/// program invents none where a file states none.
///
/// [`None`] is the common answer and is not a defect: an attachment that is not a PDF, or whose
/// first page states no `/Thumb`, has no picture the standard defines. A host draws
/// [`Picture::kind`] in its place and the panel says so once, rather than passing an icon off as a
/// thumbnail (ADR 1251).
///
/// **One attachment at a time and no cache here**, exactly as [`page_entry`] is shaped: a host
/// holds the pictures of the rows it is showing, in its own toolkit's type, and [`Miniatures`] is
/// the policy for how many.
#[must_use]
pub fn attachment_preview(viewer: &Viewer, name: &str) -> Option<Image> {
    match viewer.query(Query::AttachmentPreview(name)) {
        Answer::Thumbnail(thumbnail) => Some(thumbnail.image),
        _ => None,
    }
}

/// One of Table 160's names, as the table spells it.
fn layout_name(layout: &Layout) -> String {
    match layout {
        Layout::View(View::Details) => "D".to_owned(),
        Layout::View(View::Tile) => "T".to_owned(),
        Layout::View(View::Hidden) => "H".to_owned(),
        Layout::View(View::Navigator) => "C".to_owned(),
        Layout::FilmStrip => "FilmStrip".to_owned(),
        Layout::FreeForm => "FreeForm".to_owned(),
        Layout::Linear => "Linear".to_owned(),
        Layout::Tree => "Tree".to_owned(),
        Layout::Custom(name) => name.clone(),
    }
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
fn files_in(folder: u32, attachments: &[&Attachment], shape: &Shape) -> Vec<PanelRow> {
    files_where(attachments, shape, &|id| id == Some(folder))
}

/// The files whose key's folder identifier — `None` where the key names none — `wanted` admits.
fn files_where(
    attachments: &[&Attachment],
    shape: &Shape,
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
            let previewed = shape.previewed == Some(attachment.name.as_str());
            let mut cells = if shape.shows_fields(previewed) {
                cells_of(shape.columns, attachment)
            } else {
                Vec::new()
            };
            if shape.mode == Mode::Linear && previewed {
                beside_the_preview(&mut cells, shape.columns, attachment);
            }
            let mut detail = joined(&cells).or_else(|| detail_of(attachment));
            if let Some(sits) = shape.where_it_sits(id) {
                detail = Some(match detail {
                    Some(line) => format!("{line}  ·  {sits}"),
                    None => sits,
                });
            }
            Some(PanelRow {
                detail,
                action: RowAction::Extract {
                    name: attachment.name.clone(),
                },
                picture: shape.picture(icon_of(attachment), previewed),
                cells,
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

/// §12.3.6's `Linear` metadata, which is the schema's fields *and* the file specification's.
///
/// The clause asks for both by name — "the metadata for the file attachment, including the name,
/// description and other collection schema entries", and "[a]n interactive PDF … should use the
/// file schema and file specification dictionary to provide information about the attachment" —
/// so a schema that names none of Table 155's file-related subtypes would otherwise leave the
/// layout's one attachment without the two entries the clause names first.
///
/// Only what the schema has not already asked for, so nothing is shown twice, and **the headings
/// are this program's words**: Table 155's `/N` is the document's name for a *schema* field, and
/// these are not schema fields, so the standard states no heading for them (ADR 1251).
fn beside_the_preview(
    cells: &mut Vec<Cell>,
    columns: &[(&String, &Field)],
    attachment: &Attachment,
) {
    let stated: Vec<FieldKind> = columns
        .iter()
        .map(|(_, field)| field.kind.clone())
        .collect();
    for (kind, heading, value) in [
        (
            FieldKind::FileName,
            "Name",
            attachment
                .file_name
                .clone()
                .or(Some(attachment.name.clone())),
        ),
        (
            FieldKind::Description,
            "Description",
            attachment.description.clone(),
        ),
        (
            FieldKind::Size,
            "Size",
            attachment.size.map(|size| format!("{size}")),
        ),
        (
            FieldKind::CreationDate,
            "Created",
            stamp(attachment.created_date(), attachment.created.as_ref()),
        ),
        (
            FieldKind::ModificationDate,
            "Modified",
            stamp(attachment.modified_date(), attachment.modified.as_ref()),
        ),
    ] {
        if stated.contains(&kind) {
            continue;
        }
        let Some(value) = value else { continue };
        cells.push(Cell {
            heading: heading.to_owned(),
            value,
        });
    }
}

/// One folder, with its files and its child folders under it.
///
/// Open, because a collection that arrived closed would be a folder tree presented as one row —
/// which is §12.3.5's `shall` obeyed in the letter and not at all in what a person sees. The
/// nesting is bounded by [`pdf_model::collection::Collection::read`], which walks `/Child` and
/// `/Next` forward only and visits each object once.
fn folder_row(
    folder: &pdf_model::collection::Folder,
    attachments: &[&Attachment],
    shape: &Shape,
) -> PanelRow {
    let mut children = files_in(folder.id, attachments, shape);
    children.extend(
        folder
            .children
            .iter()
            .map(|child| folder_row(child, attachments, shape)),
    );
    PanelRow {
        detail: folder.description.clone(),
        expanded: true,
        // A folder is not a file: it has no bytes to take out, so its row acts through its
        // children.
        action: RowAction::Inert,
        picture: shape.picture(Icon::Folder, false),
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

/// The schema's columns for one file, in the order [`collection_rows`] put them.
///
/// Table 47's `/P` prefix is concatenated with the *value* and not with the name, which is what
/// the table says it is for: "[a] prefix string that shall be concatenated with the text string
/// presented to the user".
/// **One cell per column, whether or not this file has a value for it.** A column format wants
/// the same headings in the same places on every row, and a field a file says nothing about is a
/// file saying nothing rather than a column that is not there — so the value is empty and the
/// heading stands. [`joined`] drops the empty ones, because a line is not a column.
fn cells_of(columns: &[(&String, &Field)], attachment: &Attachment) -> Vec<Cell> {
    columns
        .iter()
        .map(|(_, field)| Cell {
            heading: field.name.clone(),
            value: column_value(field, attachment).unwrap_or_default(),
        })
        .collect()
}

/// The same cells as one line, for a toolkit whose row has no columns to put them in.
///
/// Every window in this tree draws this today and two of them draw the columns as well, which is
/// why it is derived from the cells rather than built beside them: a detail line and a column
/// header that disagreed would be one reading of Table 155 presented twice (ADR 1215).
fn joined(cells: &[Cell]) -> Option<String> {
    let shown: Vec<String> = cells
        .iter()
        .filter(|cell| !cell.value.is_empty())
        .map(|cell| format!("{}: {}", cell.heading, cell.value))
        .collect();
    (!shown.is_empty()).then(|| shown.join("  ·  "))
}

/// Which of Table 153's two drawable presentations this collection asks for.
///
/// §12.3.5.1's `shall` is that a processor "shall display the contents according to the View key
/// of the collection dictionary", and §12.3.6 puts a navigator in front of it — "[w]hen a
/// navigator dictionary is present, a PDF processor should use the value of the Layout entry to
/// present the collection to the user". So the navigator is asked first, exactly as
/// [`unsupported_presentation`] asks it, and `/View` decides where it selects nothing this
/// program draws.
///
/// `H` and `C` are not modes of their own: `H` is the sidebar being closed until a person opens
/// it, and `C` is the navigator this function has already consulted. Both therefore answer
/// whatever the layout or the fallback says, which for a file list is the details view.
#[must_use]
pub fn presentation(collection: &Collection) -> Mode {
    if let Some(navigator) = collection.navigator.as_ref()
        && let Some(layout) = navigator.preferred(&DRAWN_LAYOUTS)
    {
        return match layout {
            Layout::View(View::Tile) => Mode::Tile,
            Layout::FilmStrip => Mode::FilmStrip,
            Layout::FreeForm => Mode::FreeForm,
            Layout::Linear => Mode::Linear,
            // `Tree`, `D`, `H`, and a `/View C` whose navigator selected nothing: the details
            // arrangement, which is §12.3.6's `Tree` by construction.
            _ => Mode::Details,
        };
    }
    match collection.view {
        View::Tile => Mode::Tile,
        View::Details | View::Hidden | View::Navigator => Mode::Details,
    }
}

/// Table 153's `/View T` icon for one file, from Table 44's `/Subtype` where the document has one.
///
/// The clause asks for "a small icon" and says nothing else, so this names a *kind* and no
/// picture; ADR 1215 records the choice and each toolkit draws its own. A media type this program
/// does not recognise, and a file stating none, are the same row: [`Icon::File`], which says only
/// that the thing is a file.
///
/// Public because `viewer_ui::chrome` draws its own rows — the division [`outline_rows`] already
/// states — and *which kind a file is* is not a thing two windows may answer differently.
#[must_use]
pub fn icon_of(attachment: &Attachment) -> Icon {
    let Some(media) = attachment.media_type.as_deref() else {
        return Icon::File;
    };
    let (kind, rest) = media.split_once('/').unwrap_or((media, ""));
    match kind.trim().to_ascii_lowercase().as_str() {
        "image" => Icon::Image,
        "audio" => Icon::Audio,
        "video" => Icon::Video,
        "text" => Icon::Document,
        "application" if rest.trim().eq_ignore_ascii_case("pdf") => Icon::Document,
        _ => Icon::File,
    }
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

/// §12.3.6's preview pictures a panel has decoded, keyed by `/EmbeddedFiles` key.
///
/// [`Miniatures`]'s sibling and its reasoning exactly, with a name for a key because a collection
/// addresses its files by §7.7.4's tree key rather than by an index: decode on demand, keep what
/// the panel is drawing, and drop the rest once there are more than [`KEPT_MINIATURES`] of them.
/// Generic over the picture for the same reason — `T` is a `gdk::Texture` in one host and a
/// [`pdf_render::Image`] in another, and this crate names no pixel format.
///
/// **A file with no picture is held as one**, which is what makes this a cache rather than a
/// retry loop: [`attachment_preview`] answers [`None`] for most attachments, and a map that held
/// only the successes would open the same embedded document again on every frame (ADR 1251).
#[derive(Debug)]
pub struct Previews<T> {
    /// The key, and the picture that file states or [`None`] where it states none.
    held: std::collections::BTreeMap<String, Option<T>>,
}

impl<T> Default for Previews<T> {
    fn default() -> Self {
        Self {
            held: std::collections::BTreeMap::new(),
        }
    }
}

impl<T> Previews<T> {
    /// An empty panel, which is what a document that has just opened has.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets everything, which is what a new document is.
    pub fn clear(&mut self) {
        self.held.clear();
    }

    /// How many files have been asked about, which is what a test asserts the bound on.
    #[must_use]
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether nothing has been asked about yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// This file's picture where it has already been asked about, and [`None`] otherwise.
    ///
    /// The reading half of [`Self::picture`], for a host that draws from a shared borrow after
    /// filling what it is about to draw — the division [`Miniatures::get`] already states.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Option<T>> {
        self.held.get(name)
    }

    /// This file's picture, decoding it with `make` if it has not been asked about.
    ///
    /// `make` is called exactly once per file, whatever it answers, so an attachment that states
    /// no `/Thumb` is opened once and not once a frame.
    pub fn picture(&mut self, name: &str, make: impl FnOnce() -> Option<T>) -> Option<&T> {
        if !self.held.contains_key(name) {
            let picture = make();
            // Dropped in key order rather than by distance, which is where this differs from
            // `Miniatures`: a collection's files have a stated order but no *position* a panel
            // scrolls through, so there is no "furthest" row to prefer. The bound is the same
            // number for the same reason — a decoded picture is tens of kilobytes.
            while self.held.len() >= KEPT_MINIATURES {
                let Some(oldest) = self.held.keys().next().cloned() else {
                    break;
                };
                self.held.remove(&oldest);
            }
            self.held.insert(name.to_owned(), picture);
        }
        self.held.get(name).and_then(Option::as_ref)
    }
}

/// Where a host puts one of §12.3.6's `FreeForm` thumbnails, as a fraction of the view.
///
/// > The FreeForm layout provides a simple layout, in which thumbnails for each item in the
/// > collection contents are displayed at a random location on the view.
///
/// **The place is the host's and the *stability* is the point.** A random location is not a fact
/// about the document — ADR 1168's property puts it on the platform's side — but a panel that
/// drew a new scatter on every frame would be unusable, so the place is derived from the file's
/// own `/EmbeddedFiles` key: the same file lands in the same spot for as long as the document is
/// open, and two files land in different spots. The three windows share this so that a person
/// moving between them sees one arrangement, which is `doc/todo/30`'s level-hosts rule rather
/// than anything the clause asks for (ADR 1251).
///
/// Both numbers are in `0.0 ..= 1.0`; a host multiplies them by the room it has left after the
/// thumbnail's own size.
#[must_use]
pub fn scattered(name: &str) -> (f32, f32) {
    // A 64-bit FNV-1a over the key, split into two halves. Any stable hash would do; this one is
    // four lines and brings in nothing.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a hash split into two 16-bit fractions, whose every value is exact in f32"
    )]
    let place = |bits: u64| (bits & 0xffff) as f32 / 65_535.0;
    (place(hash >> 16), place(hash >> 40))
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
