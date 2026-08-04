//! What a host may ask the viewer, synchronously, without producing an event.
//!
//! The second channel, and it exists for one reason: **interaction happens faster than
//! rendering**. Hit-testing a point, laying a selection highlight over a page or drawing a
//! caret are things a host does between two frames, and a host that had to post a command and
//! wait for an event to come back would feel every one of them.
//!
//! Everything here reads. Nothing here changes anything, which is what [`crate::Viewer::query`]
//! taking `&self` states in the type system rather than in a comment.

use pdf_render::Raster;
use pdf_syntax::ObjectId;

use crate::viewer::DocumentId;

/// A question about the viewer's state.
///
/// The lifetime is [`Query::Find`]'s alone: a search takes a string a host already has, and
/// copying it to ask a question about a page would be the one allocation on a path a person
/// drives from a keyboard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Query<'a> {
    /// How many pages the focused document has.
    PageCount,
    /// Which page is showing, and what it is called.
    CurrentPage,
    /// Where a page sits on the screen and how large it is drawn.
    ///
    /// The index is zero-based. A page that is not the one showing has no place on the screen,
    /// so this answers [`Answer::None`] for it — the geometry is a property of the view, not of
    /// the page.
    PageGeometry(usize),
    /// §12.3.3's outline, as a panel would show it.
    Outline,
    /// §8.11.4.3's `/Order`, as a layer panel would show it.
    Layers,
    /// §7.11.4's embedded files, listed rather than extracted.
    Attachments,
    /// §12.4.2's page label for one page, where the document states one.
    ///
    /// [`Query::CurrentPage`] answers it for the page being shown, which is what a title bar
    /// needs; a list of pages needs it for every row, and the clause is the reason a number will
    /// not do: "[e]ach page in a PDF document shall be identified by an integer page index …
    /// [i]t may also be identified by a page label", and a document that labels its front matter
    /// in roman numerals has said its third page is called `iii`.
    PageLabel(usize),
    /// §12.3.4's thumbnail image for one page, decoded, where the page states one.
    ///
    /// **One page at a time and nothing cached here**, which is principle 2 rather than an
    /// oversight: §12.3.4's NOTE says thumbnails "are not required, and can be included for some
    /// pages and not for others", and a thousand-page document that carried one for every page
    /// would decode a thousand images to draw eight. The panel knows which eight it is showing;
    /// this crate does not, and a host that scrolls one keeps what it has already asked for.
    Thumbnail(usize),
    /// Whether activating at this viewport point would follow a §12.5.6.5 link.
    ///
    /// What a host needs to choose a cursor, which it does on every pointer move — so this is a
    /// query and not a command, and it is the reason the second channel exists. Device pixels
    /// from the viewport's top-left corner.
    LinkAt((f32, f32)),
    /// What the form field at a point is called, where there is one.
    ///
    /// What a host needs before it can send [`crate::Edit::SetField`], and it asks on a click.
    /// Two names come back: [`pdf_model::view::FieldName::qualified`] addresses the field and
    /// [`pdf_model::view::FieldName::shown`] is what §14.9.3 requires a user interface to show.
    FieldAt((f32, f32)),
    /// Whether anything has been edited since the document opened.
    Dirty,
    /// §14.3.3's document information dictionary, and §14.3.2's metadata stream beside it.
    ///
    /// What a document-properties panel shows. The second half is not decoration: §12.2's
    /// `/DisplayDocTitle` names XMP's `dc:title` and nothing else, so a host that titles a window
    /// from a document asks this and not [`Query::Preferences`] alone.
    ///
    /// **Decodes and parses the stream, so it is not free** — 78 KiB at the corpus's worst. A
    /// host asks it when a document opens and when a properties panel is drawn, which is what it
    /// is for; it is deliberately not on the render path.
    Properties,
    /// Table 29's `/PageMode` and `/PageLayout`: what the *catalog* asks of the window opening it.
    ///
    /// Beside [`Query::Preferences`] rather than folded into it, because they are two tables and
    /// a struct holding both would be a claim that they are one. §7.7.2's pair says which panel
    /// is open and how the pages are laid out; Table 147's says everything else about a window.
    Opening,
    /// §12.2's viewer preferences: what the document asks of the window showing it.
    ///
    /// Handed over rather than acted on, because every entry of Table 147 is about chrome the
    /// *host* owns — a tool bar, a menu bar, where the window sits on the screen — and this
    /// crate has none of those by construction. A host that has them honours what it can.
    Preferences,
    /// Every occurrence of a string on the page being shown, as shapes to draw over it.
    ///
    /// Case-insensitive, over the same readback [`Query::Selection`] answers with — which is why
    /// search cost this crate one function: the text layer built for selection is the same
    /// artefact, and §14.9's accessibility consumer will be the third.
    Find(&'a str),
    /// What is selected: the text, and the shapes to draw over it.
    ///
    /// Asked whenever a host repaints, which during a drag is every frame — so it is a query
    /// rather than an event carrying a payload nobody may want.
    Selection,
    /// The pixels the viewer is holding for the focused document, if any.
    ///
    /// [`Answer::None`] for a tier-2 host, which draws its own and hands the viewer nothing.
    Frame,
    /// §14.7's logical structure for the page being shown, as an accessibility API takes it.
    ///
    /// The last of the five items `doc/HANDOVER.md`'s section 0 listed as blocked on this interface, and
    /// the first consumer of six sessions' reading of §14.7 and §14.9. Answers with an empty list
    /// for an untagged document, which is an answer: the page says nothing about its own
    /// structure, and §14.7 leaves it free to.
    AccessibilityTree,
    /// Everything the focused document's current page could not draw.
    ///
    /// The same sentences [`crate::Event::Reported`] carried, kept so that a host which cleared
    /// its status bar can ask again rather than remembering.
    Reports,
}

/// The answer to a [`Query`].
///
/// Borrowed where the viewer already holds the answer and owned where it has to build one, so
/// that asking a question at pointer speed does not allocate at pointer speed.
#[derive(Debug)]
pub enum Answer<'a> {
    /// There is nothing to answer with: no document is focused, or the question named a page
    /// that is not showing.
    None,
    /// §14.7's structure, in §14.8.2.5's logical order, parent-first.
    Accessibility(Vec<crate::AccessibilityNode>),
    /// A count of pages.
    Count(usize),
    /// Which page is showing.
    Page {
        /// Which document.
        document: DocumentId,
        /// The zero-based index.
        index: usize,
        /// §12.4.2's label, where the document states one.
        label: Option<String>,
        /// How many pages there are.
        of: usize,
    },
    /// Where the page sits and how large it is drawn.
    Geometry(PageGeometry),
    /// §12.3.3's outline items, in the order the document's linked list holds them.
    Outline(&'a pdf_model::outline::Outline),
    /// §8.11.4.3's layers, in `/Order`.
    Layers(Vec<Layer>),
    /// §7.11.4's embedded files.
    Attachments(Vec<pdf_model::attachment::Attachment>),
    /// §12.4.2's label for the page asked about, or [`Answer::None`] where it states none.
    Label(String),
    /// §12.3.4's thumbnail for the page asked about, decoded.
    ///
    /// [`Answer::None`] for a page that states none, which is most pages in most documents and
    /// is not a defect — and for one whose `/Thumb` this crate cannot decode, which is reported
    /// through the same channel any other undecodable image is.
    ///
    /// The two flags on it are the clause's *producer-side* constraints, carried rather than
    /// enforced: a `/ColorSpace` outside the three §12.3.4 permits, and a `/Subtype` that is
    /// stated and is not `Image`. The image is drawn either way — the file is wrong and the
    /// picture is still what the file says — and a host with somewhere to put a note can say so.
    Thumbnail(pdf_model::thumbnail::Thumbnail),
    /// Whether a link is under the point asked about.
    Link(bool),
    /// What is selected.
    Selected(Selected<'a>),
    /// What the field is called: §12.7.4.2's fully qualified name, and Table 226's `/TU`.
    ///
    /// Both, because §14.9.3 says the second "shall be used in place of the actual field name
    /// when an interactive PDF processor identifies the field in a user-interface" while the
    /// first is what [`crate::Edit::SetField`] addresses. A host needs each for a different job
    /// and this crate cannot choose between them on its behalf.
    Field(pdf_model::view::FieldName),
    /// Where a string occurs, one entry per occurrence in the order they are shown.
    ///
    /// Each is the shapes covering one occurrence, merged per run of a line, in device pixels of
    /// the viewport — the same form [`Selected::quads`] takes, because a host draws them the same
    /// way and in its own colour.
    Found(Vec<Vec<[f32; 8]>>),
    /// Whether anything has been edited.
    Dirty(bool),
    /// §14.3.3's Table 349, and §14.3.2's metadata stream beside it.
    Properties {
        /// What the trailer's `/Info` says.
        information: pdf_model::metadata::Information,
        /// The catalog's `/Metadata`, read — `None` where the document names none.
        ///
        /// Three states rather than two, deliberately: no stream, a stream this reader refused,
        /// and a stream it read. A host wording a properties panel needs the middle one, because
        /// "this document states no metadata" and "this document states metadata I could not
        /// read" are different sentences and only the second is about us.
        ///
        /// **Was `metadata_stream: bool` until the two-hundred-and-ninety-fourth session**, when
        /// `pdf_model::xmp` gave the crate something to put here. Nothing in this vocabulary is
        /// `#[non_exhaustive]` exactly so that a change of this shape breaks every consumer's
        /// build rather than being ignored in one of them.
        metadata: Option<Result<pdf_model::xmp::Xmp, pdf_model::xmp::XmpError>>,
    },
    /// Table 29's two display entries.
    Opening(pdf_model::viewer_preferences::Opening),
    /// §12.2's Table 147, whole.
    Preferences(pdf_model::viewer_preferences::ViewerPreferences),
    /// The pixels the viewer holds, and where they belong on the screen.
    Frame(FrameView<'a>),
    /// What the current page could not draw.
    Reports(&'a [String]),
}

/// Where the page sits on the screen, and how large.
///
/// Everything a host needs to map between the page and the viewport in both directions: chrome
/// it draws over the page goes through `origin` and `scale`, and a pointer position comes back
/// the same way. Device pixels throughout, because that is what a host has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageGeometry {
    /// The page's extent in PDF user space units after §7.7.3.3's `/Rotate`, scaled by
    /// `/UserUnit`.
    pub page: pdf_render::Size,
    /// Device pixels per user space unit, which is the zoom and the display's scale together.
    pub scale: f32,
    /// The rasterised page's width in device pixels.
    pub width: u32,
    /// The rasterised page's height in device pixels.
    pub height: u32,
    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    ///
    /// Negative when the page is scrolled, positive when the page is smaller than the viewport
    /// and centred in it. A host adds this to a point in raster space to get a point on the
    /// screen, and subtracts it to go the other way.
    pub origin: (f32, f32),
}

/// The pixels the viewer is holding, and where they belong.
#[derive(Debug)]
pub struct FrameView<'a> {
    /// Which page these pixels are of.
    ///
    /// Worth checking: a frame outlives the page turn that made it stale by exactly as long as
    /// the new page takes to render, and showing the old pixels in the meantime is better than
    /// showing nothing — but only if the host knows which it has.
    pub page: usize,
    /// Row-major RGBA, no padding.
    pub raster: &'a Raster,
    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    pub origin: (f32, f32),
}

/// What is selected, and where it is on the screen.
///
/// **Geometry, not pixels.** A selection highlight changes at pointer speed and must not force a
/// page to be drawn again; and a native host draws it in macOS's selection colour, KDE's accent
/// or the Windows highlight brush, with its own caret and focus ring. Handing over finished
/// pixels would make all of that impossible, which is why this crate's chrome crosses as shapes.
#[derive(Debug)]
pub struct Selected<'a> {
    /// The selected text, as the page reads back.
    pub text: &'a str,
    /// The shapes covering it, in **device pixels of the viewport**, one per run of a line.
    ///
    /// `[x0, y0, … x3, y3]` round each quadrilateral, in the order the runs were shown. Device
    /// pixels rather than the page's own units because the host has no transform of its own and
    /// asking it to compose one would be asking it to re-derive the magnification, the centring
    /// and the y flip — which is exactly the arithmetic that was wrong for seventy-five sessions
    /// (ADR 0118).
    pub quads: Vec<[f32; 8]>,
}

/// One entry of §8.11.4.3's `/Order`, as a layer panel would show it.
///
/// Two shapes, because the clause defines two and says what each means: a nested array *with* a
/// leading text string is a heading over related groups, and one *without* is genuine nesting
/// of content. A panel that drew both the same way would tell a person that a heading is a
/// layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layer {
    /// A group, whose `/Name` is what a panel shows and whose state a click changes.
    Group {
        /// The group's object, which is what [`crate::Command::SetGroup`] names.
        group: ObjectId,
        /// Table 98's `/Name`. ISO 32000-2 §8.11.2.1:
        ///
        /// > The name of the optional content group, suitable for presentation in an
        /// > interactive PDF processor's user interface.
        name: Option<String>,
        /// Whether the group is currently on.
        on: bool,
        /// Table 99's `/Locked`. ISO 32000-2 §8.11.4.3:
        ///
        /// > The state of a locked group cannot be changed through the user interface of an
        /// > interactive PDF processor.
        ///
        /// Carried rather than acted on: a panel that offers the switch anyway is the thing
        /// the clause forbids, and a panel is not this crate.
        locked: bool,
    },
    /// A collection of entries, with the optional label the clause allows.
    Collection {
        /// The non-selectable heading, where the array opens with one.
        label: Option<String>,
        /// What the collection holds.
        children: Vec<Layer>,
    },
}
