//! The five answers that needed a handle of their own, and why each is the shape it is.
//!
//! **This module is the seven-hundred-and-ninth session's half of `doc/todo/30` item 5.** Eleven of
//! [`viewer_core::Query`]'s variants reached no symbol at all; six of them fitted a shape this ABI
//! already had — a panel's rows, a list of quadrilaterals, a string, a pair of numbers — and five
//! did not. What is here is those five, and the argument for each shape sits on the type rather
//! than in an ADR, because **a C entry point cannot change shape once a caller exists** and the
//! reason has to be readable where the accessor is.
//!
//! Everything here is **owned**, for [`crate::panels`]'s reason restated once: an
//! [`viewer_core::Answer`] may borrow the viewer, a C caller holding that borrow while it calls
//! back in is an aliasing hazard nothing on this side would notice, and the allocation is a
//! panel's rather than a pointer's.
//!
//! # The four shapes, and the one rule that produced them
//!
//! A C caller reads a structure by **index into a handle**, never by following a pointer this
//! library owns. So every answer here becomes one of four things:
//!
//! - a **flat list**, where the answer is one — [`Popups`];
//! - a **list of lists**, read with two indices — [`Matches`], where each occurrence of a search
//!   term is several quadrilaterals, and [`Structure`], where each page on the screen has its own
//!   tree;
//! - a **tree flattened depth first with a depth on each row**, which is what [`crate::panels`]
//!   already does for §12.3.3 and what [`Collection`]'s folders take;
//! - a **buffer the caller copies out of**, for the one answer that is pixels — [`Miniature`].
//!
//! Nothing here is a struct passed by value, which is the one kind of change
//! [`crate::abi::PDFV_ABI_VERSION`] exists to protect and which this round therefore adds none of.

use viewer_core::{AccessibilityNode, PageStructure, PopupWindow};

use crate::kinds::{
    BoxKind, ColumnKind, ColumnTextKind, ElementKind, FolderTextKind, NoteKind, ScopeKind,
};
use crate::status::Status;

/// An indirect reference as this boundary carries one: §7.3.10's two numbers.
///
/// Both, always, because "an indirect reference shall consist of the object number, the
/// generation number and the keyword `R`" — a viewer that dropped the second would name a
/// different object in a file that reused a number.
pub type ObjectRef = (u32, u16);

/// Every occurrence of a string on the page being shown, as shapes to draw over it.
///
/// **The sharpest of the eleven, and ADR 0509 said so**: a C caller could start Annex O's
/// document-wide search with `pdfv_find_start` and could not draw one match, because
/// [`viewer_core::Query::Find`]'s per-page geometry reached no symbol. `Event::Searched` says
/// *which page* a match is on and this says *where on it*, and a find bar needs both.
///
/// **A list of lists rather than one flat list**, and the difference is load-bearing rather than
/// tidy: [`viewer_core::Answer::Found`] merges a single occurrence's shapes per run of a line, so
/// a term broken across two lines is one match drawn as two quadrilaterals. A caller stepping
/// through matches counts [`Self::len`]; a caller drawing them walks both indices. Flattening
/// would have made *next match* mean *next quadrilateral*, which on a wrapped line is half a step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Matches {
    /// One entry per occurrence, in the order the page shows them.
    occurrences: Vec<Vec<[f32; 8]>>,
}

impl Matches {
    /// Takes what [`viewer_core::Answer::Found`] held.
    #[must_use]
    pub fn new(occurrences: Vec<Vec<[f32; 8]>>) -> Self {
        Self { occurrences }
    }

    /// How many occurrences there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.occurrences.len()
    }

    /// Whether there are none, which is what a term the page does not carry answers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.occurrences.is_empty()
    }

    /// The shapes covering one occurrence, copied so that the caller owns them.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such occurrence.
    pub fn quads(&self, index: usize) -> Result<Vec<[f32; 8]>, Status> {
        self.occurrences
            .get(index)
            .cloned()
            .ok_or(Status::OutOfRange)
    }
}

/// §12.3.4's thumbnail for one page, decoded.
///
/// **One page's picture and nothing else, which is the whole design.** The
/// seven-hundred-and-fourth session found `viewer-ui` building the entire list of miniatures the
/// first time §12.3.4's panel was shown, and Table 29's `/PageMode /UseThumbs` opens that panel as
/// the document opens — 121 ms of a 156 ms first present on a thousand-page document, which is
/// `CLAUDE.md` section 2's forbidden thumbnail generation on the launch path reached by a road
/// nobody had checked. [`viewer_core::Query::Thumbnail`] is shaped one page at a time so that a
/// host can obey the rule, and **this ABI offers no other shape**: there is no
/// `pdfv_thumbnails_read`, deliberately, because a list-valued entry point is a loop a caller would
/// not have to write.
///
/// The pixels are copied into a buffer the caller owns, exactly as a frame is, for
/// [`crate::abi`]'s reason: no pointer into this library's memory is handed out, so there is no
/// lifetime for a C program to get wrong.
#[derive(Debug, Clone, PartialEq)]
pub struct Miniature {
    /// The decoded picture.
    image: pdf_render::Image,
    /// Whether `/ColorSpace` is one of the three forms §12.3.4 permits.
    permitted_colour_space: bool,
    /// Whether a `/Subtype`, if stated, is `Image`.
    permitted_subtype: bool,
}

impl Miniature {
    /// Takes what [`viewer_core::Answer::Thumbnail`] held.
    #[must_use]
    pub fn new(thumbnail: pdf_model::thumbnail::Thumbnail) -> Self {
        Self {
            image: thumbnail.image,
            permitted_colour_space: thumbnail.permitted_colour_space,
            permitted_subtype: thumbnail.permitted_subtype,
        }
    }

    /// The picture's size, how many bytes [`Self::copy`] writes, and the clause's two
    /// producer-side constraints.
    ///
    /// **Both flags are carried rather than enforced**, which is §12.3.4 read the way trap 5 asks:
    /// a `/ColorSpace` outside the three the clause permits and a stated `/Subtype` that is not
    /// `Image` make the *file* wrong, and the picture is still what the file says. The image is
    /// drawn either way and a caller with somewhere to put a note can say so.
    #[must_use]
    pub fn info(&self) -> (u32, u32, usize, bool, bool) {
        (
            self.image.width,
            self.image.height,
            self.image.data.len(),
            self.permitted_colour_space,
            self.permitted_subtype,
        )
    }

    /// Copies the samples into the caller's buffer, RGBA8 and row-major with no padding.
    ///
    /// # Errors
    ///
    /// [`Status::BufferTooSmall`] where the buffer is shorter than [`Self::info`]'s byte count.
    /// Nothing is written in that case.
    pub fn copy(&self, into: &mut [u8]) -> Result<usize, Status> {
        let data: &[u8] = &self.image.data;
        let Some(room) = into.get_mut(..data.len()) else {
            return Err(Status::BufferTooSmall);
        };
        room.copy_from_slice(data);
        Ok(data.len())
    }
}

/// §12.5.6.14's open popup windows on the page being shown.
///
/// **The one annotation subtype whose picture is not the page's.** The clause makes a popup "a
/// window … for entry and editing" with "no appearance stream", so a host draws it as *chrome* in
/// its platform's own window furniture — which is why this answers text, a rectangle and a colour
/// rather than pixels, and why it is one of the two things the seven-hundred-and-fourth session
/// named as reachable by no window but `viewer-ui`.
///
/// A flat list with no page on it, which is the rule [`viewer_core::Query::Fields`] follows and
/// ADR 0509 section 4 named: a quadrilateral already in the viewport's own device pixels needs no page,
/// because that is where a caller draws.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Popups {
    /// One entry per open window, in the `/Annots` array's order.
    windows: Vec<Note>,
}

/// One popup window, flattened.
#[derive(Debug, Clone, PartialEq)]
struct Note {
    /// The popup annotation, which `pdfv_activate` closes.
    annotation: ObjectRef,
    /// Table 186's `/Parent`, the markup annotation whose text this is.
    parent: Option<ObjectRef>,
    /// Its `/Rect` on the screen, `[x0, y0, … x3, y3]`, y downwards.
    quad: [f32; 8],
    /// §12.5.6.2's `/T`, Table 166's `/Contents`, and Table 166's `/M`, in that order.
    text: [Option<String>; 3],
    /// Table 166's `/C` as `DeviceRGB`, where the annotation states one.
    colour: Option<[f32; 3]>,
}

impl Popups {
    /// Takes what [`viewer_core::Answer::Popups`] held.
    #[must_use]
    pub fn new(windows: &[PopupWindow]) -> Self {
        Self {
            windows: windows
                .iter()
                .map(|window| Note {
                    annotation: (window.annotation.number, window.annotation.generation),
                    parent: window
                        .parent
                        .map(|parent| (parent.number, parent.generation)),
                    quad: window.quad,
                    text: [
                        window.title.clone(),
                        window.text.clone(),
                        window.modified.clone(),
                    ],
                    colour: window.colour.map(|colour| [colour.r, colour.g, colour.b]),
                })
                .collect(),
        }
    }

    /// How many windows are open.
    #[must_use]
    pub fn len(&self) -> usize {
        self.windows.len()
    }

    /// Whether there are none, which is every page of almost every document.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// The popup annotation and Table 186's `/Parent`, where it names one.
    ///
    /// Both, because they answer two different questions: the first is what `pdfv_activate` closes
    /// the window with, and the second is the markup annotation the note *belongs to* — which is
    /// what a host highlights when the pointer is over the window.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such window.
    pub fn objects(&self, index: usize) -> Result<(ObjectRef, Option<ObjectRef>), Status> {
        self.windows
            .get(index)
            .map(|note| (note.annotation, note.parent))
            .ok_or(Status::OutOfRange)
    }

    /// The window's rectangle on the screen, in device pixels of the viewport.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such window.
    pub fn quad(&self, index: usize) -> Result<[f32; 8], Status> {
        self.windows
            .get(index)
            .map(|note| note.quad)
            .ok_or(Status::OutOfRange)
    }

    /// One of the window's three strings, and `""` for one the annotation does not state.
    ///
    /// **An empty string rather than a refusal**, which is the opposite of what
    /// [`crate::form::Form`] does for a field's value and is right for the opposite reason: a
    /// field distinguishes "no text value at all" from "an empty one" because a host decides where
    /// to send the keyboard on that difference, and a note with no title is a note a host draws
    /// with no title. Table 166 makes none of the three required.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such window.
    pub fn text(&self, index: usize, which: NoteKind) -> Result<&str, Status> {
        let note = self.windows.get(index).ok_or(Status::OutOfRange)?;
        let text = match which {
            NoteKind::Title => note.text.first(),
            NoteKind::Contents => note.text.get(1),
            NoteKind::Modified => note.text.get(2),
        };
        // Unreachable: the array has three entries and `NoteKind` has three variants. Written as a
        // refusal rather than an index so that no length is trusted twice.
        Ok(text
            .ok_or(Status::OutOfRange)?
            .as_deref()
            .unwrap_or_default())
    }

    /// Table 166's `/C`, "[t]he title bar of the annotation's popup window", as `DeviceRGB`.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such window, [`Status::NoAnswer`] where it states
    /// no colour — which is a different thing from black and is why the two are told apart.
    pub fn colour(&self, index: usize) -> Result<[f32; 3], Status> {
        self.windows
            .get(index)
            .ok_or(Status::OutOfRange)?
            .colour
            .ok_or(Status::NoAnswer)
    }
}

/// What [`Structure::shape`] answers: the parent index, §14.9.3's substitution, and Table 384's
/// `/Scope` for a header cell.
pub type NodeFacts = (Option<usize>, bool, Option<ScopeKind>);

/// §14.7's logical structure for every page the arrangement is showing.
///
/// **Two indices, and the standard is why.** [`viewer_core::PageStructure`] carries one tree per
/// page because §14.7.5.2's marked-content identifier "uniquely identifies the marked-content
/// sequence within its content stream" and §14.7.5.4 keys the route in from *that page's*
/// `/StructParents` — so two pages' trees share no numbering, and a flat list across the screen
/// would have had to renumber one page's tree against another's with no order to follow. A caller
/// walks pages, then nodes, and every index a node carries — its parent, its headers — is into
/// that page's own list.
///
/// Within a page the tree is a flat list with a parent index, which is not this ABI's flattening
/// at all: it is the shape [`viewer_core::AccessibilityNode`] already has, because that is what
/// AccessKit and AT-SPI want.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Structure {
    /// One entry per page on the screen, in page order.
    pages: Vec<PageStructure>,
}

impl Structure {
    /// Takes what [`viewer_core::Answer::Accessibility`] held.
    #[must_use]
    pub fn new(pages: Vec<PageStructure>) -> Self {
        Self { pages }
    }

    /// How many pages the arrangement is showing.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pages.len()
    }

    /// Whether there are none, which is a viewer with nothing open.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    /// Which page an entry is about, and how many nodes its tree has.
    ///
    /// Zero nodes is an answer rather than a silence: §14.7 leaves a producer free to state no
    /// structure, and a reader that invented a reading order for an untagged page would be
    /// presenting a guess where a person is entitled to the author's answer.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such entry.
    pub fn page(&self, entry: usize) -> Result<(usize, usize), Status> {
        self.pages
            .get(entry)
            .map(|page| (page.page, page.nodes.len()))
            .ok_or(Status::OutOfRange)
    }

    /// One node, by page entry and index into that page's own list.
    fn node(&self, entry: usize, index: usize) -> Result<&AccessibilityNode, Status> {
        self.pages
            .get(entry)
            .ok_or(Status::OutOfRange)?
            .nodes
            .get(index)
            .ok_or(Status::OutOfRange)
    }

    /// A node's place in its page's tree and the two facts a screen reader decides on.
    ///
    /// `substituted` is §14.9.3's and §14.9.5's substitution: an element stating `/Alt` or `/E` has
    /// said what to speak *instead of* its content, so a client handing this to a platform API
    /// stops there rather than descending. It is not a nicety — descending anyway reads the
    /// element twice.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such page or node.
    pub fn shape(&self, entry: usize, index: usize) -> Result<NodeFacts, Status> {
        let node = self.node(entry, index)?;
        Ok((
            node.parent,
            node.substituted,
            node.header_scope.map(ScopeKind::of),
        ))
    }

    /// One of the node's three strings, and `""` for a language no ancestor states.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such page or node.
    pub fn text(&self, entry: usize, index: usize, which: ElementKind) -> Result<&str, Status> {
        let node = self.node(entry, index)?;
        Ok(match which {
            ElementKind::Role => node.role.as_str(),
            ElementKind::Name => node.name.as_str(),
            ElementKind::Language => node.language.as_deref().unwrap_or_default(),
        })
    }

    /// Where the element's own text was drawn, in device pixels of the viewport.
    ///
    /// Empty for an element whose content drew no text — a figure, a table cell holding an image
    /// — which is a statement about this program's text layer rather than about the element, and
    /// is why [`Self::rectangle`] exists beside it.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such page or node.
    pub fn quads(&self, entry: usize, index: usize) -> Result<Vec<[f32; 8]>, Status> {
        Ok(self.node(entry, index)?.quads.clone())
    }

    /// One of the node's two rectangles, `[x0, y0, x1, y1]` in the same device pixels.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such page or node, [`Status::NoAnswer`] where the
    /// node has no rectangle of that kind — which for [`BoxKind::Drawn`] is every element whose
    /// content is a picture, and is a fact a caller may want rather than a failure.
    pub fn rectangle(
        &self,
        entry: usize,
        index: usize,
        which: BoxKind,
    ) -> Result<[f32; 4], Status> {
        let node = self.node(entry, index)?;
        match which {
            BoxKind::Stated => node.bounds,
            BoxKind::Drawn => node.drawn,
        }
        .ok_or(Status::NoAnswer)
    }

    /// How many header cells §14.8.4.8.3 associates with this element.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such page or node.
    pub fn headers(&self, entry: usize, index: usize) -> Result<usize, Status> {
        Ok(self.node(entry, index)?.headers.len())
    }

    /// One of those header cells, as an index into **this page's** node list.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such page, node or header.
    pub fn header(&self, entry: usize, index: usize, header: usize) -> Result<usize, Status> {
        self.node(entry, index)?
            .headers
            .get(header)
            .copied()
            .ok_or(Status::OutOfRange)
    }

    /// How many lines of text the element's own content items drew.
    ///
    /// **What a platform text interface is built on**, and the half of this answer that did not
    /// cross until the seven-hundred-and-twenty-sixth session: `PDFV_ELEMENT_NAME` is what the
    /// element is *called* and this is what it *says*, with each character's place beside it, which
    /// is what `org.a11y.atspi.Text`'s `GetCharacterExtents` and `GetOffsetAtPoint` need and what
    /// no string can answer.
    ///
    /// Zero for an element stating §14.9.3's `/Alt` or §14.9.5's `/E` — the phrase substitutes for
    /// the whole element, which is what `pdfv_structure_node`'s `substituted` also says — and for
    /// one whose content drew no text, which is most of them.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such page or node.
    pub fn lines(&self, entry: usize, index: usize) -> Result<usize, Status> {
        Ok(self.node(entry, index)?.lines.len())
    }

    /// One line's text, and how many character codes produced it.
    ///
    /// The two come from one call because the invariant between them is what a text interface
    /// rests on: the sum of the characters' byte counts is this string's length, so an offset into
    /// the text and an index into the characters convert into each other without either side
    /// guessing.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such page, node or line.
    pub fn line(&self, entry: usize, index: usize, line: usize) -> Result<(&str, usize), Status> {
        self.node(entry, index)?
            .lines
            .get(line)
            .map(|line| (line.text.as_str(), line.characters.len()))
            .ok_or(Status::OutOfRange)
    }

    /// One character code's share of a line: how many bytes of the line's text it produced, and
    /// where its glyph is.
    ///
    /// **The unit is the *code* rather than the character**, which is `viewer_core::Character`'s
    /// own reading and the reason the byte count crosses at all: a code mapped through `/ToUnicode`
    /// to several characters — a ligature read back as `ffi` — drew one glyph in one place, and
    /// splitting its box into thirds would invent positions the file does not state.
    ///
    /// The rectangle is `[x0, y0, x1, y1]` in the device pixels of the viewport every other shape
    /// in this ABI is in.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such page, node, line or character.
    pub fn character(
        &self,
        entry: usize,
        index: usize,
        line: usize,
        character: usize,
    ) -> Result<(usize, [f32; 4]), Status> {
        self.node(entry, index)?
            .lines
            .get(line)
            .and_then(|line| line.characters.get(character))
            .map(|character| (character.bytes, character.bounds))
            .ok_or(Status::OutOfRange)
    }
}

/// §12.3.5's portable collection: Table 153's columns, its folders, and where to open.
///
/// **The clause is a `shall` on a viewer** — "[i]f this dictionary is present in a PDF document,
/// the interactive PDF processor shall present the document as a portable collection" — so what a
/// caller needs is everything required to *arrange* the files it already reads with
/// `pdfv_attachments_read`, and nothing that is merely decoration.
///
/// Three things cross and the third is the one a caller could not compute. The **columns** are
/// Table 155's fields in `/O` order, which the table calls "[t]he relative order of the field name
/// in the user interface"; the **folders** are §12.3.5.2's tree, flattened depth first with a depth
/// on each row exactly as §12.3.3's outline is; and `folder_of` is §12.3.5.2's own grammar for an
/// `/EmbeddedFiles` key, which says which folder a file is in. Without the last one a caller holds
/// a folder tree and a file list and no way to put one inside the other.
///
/// `/Sort`, `/Navigator`, `/Colors` and `/Split` are deliberately not here, and this is a
/// statement rather than an omission: each of the four describes how a *particular* layout would
/// look, and this boundary's standing rule is that a look belongs to the platform — the same rule
/// that keeps a selection's colour out of [`crate::shapes::Quads`]. A caller that wants the
/// document's suggestions rather than its own is asking for something no host has yet wanted; it
/// would be four more accessors on this handle and no new decision.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Collection {
    /// Table 153's `/View`, as a number.
    view: crate::kinds::CollectionViewKind,
    /// §12.3.5.1's resolved outcome, and the `/EmbeddedFiles` key where it names one.
    initial: (crate::kinds::InitialKind, String),
    /// The schema's fields in `/O` order.
    columns: Vec<Column>,
    /// §12.3.5.2's folder tree, depth first.
    folders: Vec<FolderRow>,
}

/// What [`Collection::column`] answers: the subtype, where its value lives, `/O`, `/V` and `/E`.
///
/// A named alias rather than a bare tuple because five values in a row is where a reader stops
/// counting — and deliberately not a `#[repr(C)]` struct, which is the one kind of change
/// [`crate::abi::PDFV_ABI_VERSION`] exists to guard: `pdfv_collection_column` writes five
/// out-parameters instead.
pub type ColumnFacts = (ColumnKind, bool, Option<i64>, bool, bool);

/// One column of a collection. Table 155.
#[derive(Debug, Clone, PartialEq)]
struct Column {
    /// Which subtype, and the name the file wrote for one this standard does not define.
    kind: (ColumnKind, String),
    /// `/N`, the name shown to a person.
    name: String,
    /// The `/Schema` key, which §7.11.6's collection item addresses values by.
    key: String,
    /// `/O`, where the file states one.
    order: Option<i64>,
    /// `/V`, "[t]he initial visibility of the field".
    visible: bool,
    /// `/E`, whether a processor "should provide support for editing the field value".
    editable: bool,
}

/// One folder of §12.3.5.2's tree.
#[derive(Debug, Clone, PartialEq)]
struct FolderRow {
    /// `/ID`, "a non-negative integer value representing the unique folder identification number",
    /// and the number an `/EmbeddedFiles` key refers to.
    id: u32,
    /// How far in it is, zero at the root.
    depth: u32,
    /// `/Name` and `/Desc`.
    text: [String; 2],
    /// Whether `/Thumb` is present.
    has_thumbnail: bool,
}

impl Collection {
    /// Takes what [`viewer_core::Answer::Collection`] held.
    #[must_use]
    pub fn new(
        collection: &pdf_model::collection::Collection,
        initial: &pdf_model::collection::Initial,
    ) -> Self {
        // Table 155's `/O` order, with a field stating none after every field that states one and
        // then by key — which is the only order left when the file states none, and is the order
        // `viewer-ui` already draws them in.
        let mut columns: Vec<Column> = collection
            .schema
            .iter()
            .map(|(key, field)| {
                let (kind, subtype) = ColumnKind::of(&field.kind);
                Column {
                    kind: (kind, subtype.to_owned()),
                    name: field.name.clone(),
                    key: key.clone(),
                    order: field.order,
                    visible: field.visible,
                    editable: field.editable,
                }
            })
            .collect();
        columns.sort_by(|left, right| {
            left.order
                .unwrap_or(i64::MAX)
                .cmp(&right.order.unwrap_or(i64::MAX))
                .then_with(|| left.key.cmp(&right.key))
        });
        let mut folders = Vec::new();
        if let Some(root) = collection.folders.as_ref() {
            push_folders(root, 0, &mut folders);
        }
        let (kind, name) = crate::kinds::InitialKind::of(initial);
        Self {
            view: crate::kinds::CollectionViewKind::of(collection.view),
            initial: (kind, name.to_owned()),
            columns,
            folders,
        }
    }

    /// Table 153's `/View`.
    #[must_use]
    pub fn view(&self) -> crate::kinds::CollectionViewKind {
        self.view
    }

    /// §12.3.5.1's outcome, and the `/EmbeddedFiles` key for [`crate::kinds::InitialKind::Embedded`].
    ///
    /// The key is `""` for the other three, which name no file: the container is the document
    /// already on the screen, the first file is whichever a caller lists first, and an empty
    /// collection has none.
    #[must_use]
    pub fn initial(&self) -> (crate::kinds::InitialKind, &str) {
        (self.initial.0, self.initial.1.as_str())
    }

    /// How many columns the schema states.
    ///
    /// Zero is a permission rather than a gap: Table 153 says an absent schema lets a processor
    /// "choose useful defaults that are known to exist in a file specification dictionary, such as
    /// the file name, file size, and modified date", so an empty list is the document declining to
    /// choose and the caller choosing instead.
    #[must_use]
    pub fn columns(&self) -> usize {
        self.columns.len()
    }

    /// One column's subtype, its `/O`, and Table 155's two booleans.
    ///
    /// `in_the_item` is the clause's own division stated rather than derived: the first three
    /// subtypes "identify the types of fields in the collection item … dictionary" and the rest
    /// "identify the types of file-related fields", so it says whether a caller reads §7.11.6's
    /// `/CI` or the file specification it already has.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such column.
    pub fn column(&self, index: usize) -> Result<ColumnFacts, Status> {
        let column = self.columns.get(index).ok_or(Status::OutOfRange)?;
        let in_the_item = matches!(
            column.kind.0,
            ColumnKind::Text | ColumnKind::Date | ColumnKind::Number
        );
        Ok((
            column.kind.0,
            in_the_item,
            column.order,
            column.visible,
            column.editable,
        ))
    }

    /// One of a column's three strings.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such column.
    pub fn column_text(&self, index: usize, which: ColumnTextKind) -> Result<&str, Status> {
        let column = self.columns.get(index).ok_or(Status::OutOfRange)?;
        Ok(match which {
            ColumnTextKind::Name => column.name.as_str(),
            ColumnTextKind::Key => column.key.as_str(),
            ColumnTextKind::Subtype => column.kind.1.as_str(),
        })
    }

    /// How many folders §12.3.5.2's tree holds, counting every level.
    #[must_use]
    pub fn folders(&self) -> usize {
        self.folders.len()
    }

    /// One folder's `/ID`, its depth in the tree, and whether it states a `/Thumb`.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such folder.
    pub fn folder(&self, index: usize) -> Result<(u32, u32, bool), Status> {
        self.folders
            .get(index)
            .map(|folder| (folder.id, folder.depth, folder.has_thumbnail))
            .ok_or(Status::OutOfRange)
    }

    /// One of a folder's two strings, and `""` for a description it does not state.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such folder.
    pub fn folder_text(&self, index: usize, which: FolderTextKind) -> Result<&str, Status> {
        let folder = self.folders.get(index).ok_or(Status::OutOfRange)?;
        let text = match which {
            FolderTextKind::Name => folder.text.first(),
            FolderTextKind::Description => folder.text.get(1),
        };
        // Unreachable: two entries, two variants. A refusal rather than an index, so that no
        // length in this file is trusted twice.
        text.map(String::as_str).ok_or(Status::OutOfRange)
    }
}

/// Walks §12.3.5.2's tree depth first, recording each folder's depth.
///
/// The same flattening [`crate::panels`] does for §12.3.3, and for the same reason stated there: a
/// tree is the one shape a C ABI cannot hand over as itself.
fn push_folders(folder: &pdf_model::collection::Folder, depth: u32, into: &mut Vec<FolderRow>) {
    into.push(FolderRow {
        id: folder.id,
        depth,
        text: [
            folder.name.clone(),
            folder.description.clone().unwrap_or_default(),
        ],
        has_thumbnail: folder.has_thumbnail,
    });
    for child in &folder.children {
        push_folders(child, depth.saturating_add(1), into);
    }
}

#[cfg(test)]
mod tests {
    use super::{Collection, Matches, Popups};
    use crate::kinds::{ColumnTextKind, FolderTextKind, InitialKind, NoteKind};
    use crate::status::Status;

    /// A match is a list of shapes, so *next match* is not *next quadrilateral*.
    ///
    /// The property the two-index shape exists for: a term wrapped across a line is one occurrence
    /// drawn as two quadrilaterals, and a caller stepping through hits must not visit it twice.
    #[test]
    fn an_occurrence_broken_across_two_lines_is_one_match_and_two_shapes() {
        let matches = Matches::new(vec![vec![[0.0; 8], [1.0; 8]], vec![[2.0; 8]]]);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches.quads(0).map(|quads| quads.len()), Ok(2));
        assert_eq!(matches.quads(1).map(|quads| quads.len()), Ok(1));
        assert_eq!(matches.quads(2), Err(Status::OutOfRange));
        assert!(Matches::default().is_empty());
    }

    /// A window with no title answers an empty string; one past the end is refused.
    #[test]
    fn a_note_that_is_not_there_is_refused_and_one_with_no_title_is_not() {
        let popups = Popups::default();
        assert!(popups.is_empty());
        assert_eq!(popups.text(0, NoteKind::Title), Err(Status::OutOfRange));
        assert_eq!(popups.quad(0), Err(Status::OutOfRange));
        assert_eq!(popups.colour(0), Err(Status::OutOfRange));
    }

    /// A document that states no collection still answers Table 153's defaults for one that does.
    ///
    /// What is asserted here is the *ordering* rule, because it is the one piece of policy this
    /// type applies: Table 155's `/O` is "[t]he relative order of the field name in the user
    /// interface", a field stating none has no place in that order, and a caller drawing columns
    /// in map order would put them in whatever order the keys sort in.
    #[test]
    fn columns_come_out_in_table_155s_own_order_and_a_field_without_one_comes_last() {
        let mut schema = std::collections::BTreeMap::new();
        for (key, name, order) in [
            ("aaa", "Third", None),
            ("zzz", "First", Some(1_i64)),
            ("mmm", "Second", Some(2_i64)),
        ] {
            schema.insert(
                key.to_owned(),
                pdf_model::collection::Field {
                    kind: pdf_model::collection::FieldKind::Text,
                    name: name.to_owned(),
                    order,
                    visible: true,
                    editable: false,
                },
            );
        }
        let collection = pdf_model::collection::Collection {
            schema,
            ..pdf_model::collection::Collection::default()
        };
        let flat = Collection::new(&collection, &pdf_model::collection::Initial::FirstFile);
        assert_eq!(flat.columns(), 3);
        assert_eq!(flat.column_text(0, ColumnTextKind::Name), Ok("First"));
        assert_eq!(flat.column_text(1, ColumnTextKind::Name), Ok("Second"));
        assert_eq!(flat.column_text(2, ColumnTextKind::Name), Ok("Third"));
        assert_eq!(flat.column_text(2, ColumnTextKind::Key), Ok("aaa"));
        assert_eq!(flat.column_text(0, ColumnTextKind::Subtype), Ok("S"));
        assert_eq!(
            flat.column_text(3, ColumnTextKind::Name),
            Err(Status::OutOfRange)
        );
        assert_eq!(flat.initial(), (InitialKind::FirstFile, ""));
        assert_eq!(flat.folders(), 0);
        assert_eq!(
            flat.folder_text(0, FolderTextKind::Name),
            Err(Status::OutOfRange)
        );
    }

    /// §12.3.5.2's tree is flattened parent-first with a depth on every row.
    #[test]
    fn a_folder_tree_crosses_depth_first_with_its_identification_numbers() {
        let leaf = pdf_model::collection::Folder {
            id: 2,
            name: "Inner".to_owned(),
            description: None,
            item: pdf_model::collection::Item::default(),
            has_thumbnail: false,
            children: Vec::new(),
        };
        let root = pdf_model::collection::Folder {
            id: 1,
            name: "Outer".to_owned(),
            description: Some("the root".to_owned()),
            item: pdf_model::collection::Item::default(),
            has_thumbnail: true,
            children: vec![leaf],
        };
        let collection = pdf_model::collection::Collection {
            folders: Some(root),
            ..pdf_model::collection::Collection::default()
        };
        let flat = Collection::new(&collection, &pdf_model::collection::Initial::Container);
        assert_eq!(flat.folders(), 2);
        assert_eq!(flat.folder(0), Ok((1, 0, true)));
        assert_eq!(flat.folder(1), Ok((2, 1, false)));
        assert_eq!(
            flat.folder_text(0, FolderTextKind::Description),
            Ok("the root")
        );
        assert_eq!(flat.folder_text(1, FolderTextKind::Description), Ok(""));
    }
}
