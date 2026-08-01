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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Query {
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
    /// Whether activating at this viewport point would follow a §12.5.6.5 link.
    ///
    /// What a host needs to choose a cursor, which it does on every pointer move — so this is a
    /// query and not a command, and it is the reason the second channel exists. Device pixels
    /// from the viewport's top-left corner.
    LinkAt((f32, f32)),
    /// The fully qualified name of the form field at a point, where there is one.
    ///
    /// What a host needs before it can send [`crate::Edit::SetField`], and it asks on a click.
    FieldAt((f32, f32)),
    /// Whether anything has been edited since the document opened.
    Dirty,
    /// What is selected: the text, and the shapes to draw over it.
    ///
    /// Asked whenever a host repaints, which during a drag is every frame — so it is a query
    /// rather than an event carrying a payload nobody may want.
    Selection,
    /// The pixels the viewer is holding for the focused document, if any.
    ///
    /// [`Answer::None`] for a tier-2 host, which draws its own and hands the viewer nothing.
    Frame,
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
    /// Whether a link is under the point asked about.
    Link(bool),
    /// What is selected.
    Selected(Selected<'a>),
    /// §12.7.4.2's fully qualified name of a field.
    Field(String),
    /// Whether anything has been edited.
    Dirty(bool),
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
