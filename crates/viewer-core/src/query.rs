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
    /// §12.3.5's portable collection, where the catalog states one.
    ///
    /// The clause is a `shall` on a *viewer*: "[i]f this dictionary is present in a PDF document,
    /// the interactive PDF processor shall present the document as a portable collection". What
    /// crosses is Table 153 whole — the schema's columns, §12.3.5.2's folder tree, the initial
    /// view — and presenting it is a host's, in the same sense §12.3.3's outline is.
    ///
    /// [`Answer::None`] for the 974 corpus documents that state none, which is all of them.
    Collection,
    /// §12.4.3's article threads, as a panel would list them.
    ///
    /// The clause states the *structure* and leaves the way in to a viewer — "[i]nteractive PDF
    /// processors **may** provide navigation facilities to allow the user to follow a thread from
    /// one bead to the next" — so what crosses is the list and the choosing is a host's.
    /// [`crate::Command::Activate`] on a thread's object follows it, which is the same message an
    /// outline row sends and for the same reason: the *document* decides what activating a thing
    /// means.
    ///
    /// **Read on demand rather than when the document opens.** An article is a list nothing else
    /// consults, and `CLAUDE.md`'s "nothing eager" applies to the two documents in a thousand that
    /// would pay for it at launch — the same decision `interact`'s thread action already makes.
    Articles,
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
    /// Where the caret sits in the field at a point, given how far into the value it is.
    ///
    /// The other half of typing, and the same shape [`Query::Focus`] takes: **the standard states
    /// no caret at all**, so where a text cursor goes is derived from where §12.7.4.3 puts the
    /// next glyph and what it *looks like* — its width, its colour, its blink — is the host's, in
    /// its platform's own convention. §12.5.6.11's caret *annotation* is a different object and
    /// nothing to do with this one. ADR 0211.
    ///
    /// The point names the field, exactly as [`Query::FieldAt`] does and for the reason ADR 0201
    /// gives: a host holds the place it clicked rather than the text it typed, because a field
    /// does not move and §12.7.5.3's truncation means the text can. The offset is a byte offset
    /// into the value [`Answer::Field`] answered with, clamped to its length.
    ///
    /// [`Answer::None`] where no field is there, where its value is not text §12.7.4.3 lays out,
    /// and where the value could not be laid out at all — the last is the same condition that
    /// makes the page report the field, and a caret drawn from a layout that is not on the screen
    /// would be a lie about where a character is going.
    Caret {
        /// Device pixels from the viewport's top-left corner, as [`Query::FieldAt`] takes.
        at: (f32, f32),
        /// How far into the field's value the caret is, in bytes.
        offset: usize,
    },
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
    /// §14.8.2.5's logical content order for what is selected, where the page states one.
    ///
    /// [`Query::Selection`] answers in *page content order*, which is the order the stream showed
    /// the glyphs in and the order the shapes are in — so it is the right answer for drawing and
    /// the wrong one for copying off a page whose producer wrote its columns out of order.
    ///
    /// **A second query rather than a second field on [`Selected`]**, and the reason is the
    /// measurement in [`Query::Selection`]'s own note: a drag asks that one sixty times a second
    /// and this one walks the structure tree. A host asks this when a person presses copy.
    ///
    /// [`Answer::None`] where the document has no structure tree, where nothing is selected, or
    /// where the tree does not reach every byte of the selection — the last deliberately, because
    /// a copy that silently dropped the part of a drag the tree missed would be worse than one
    /// that hands back the order it already had. See `pdf_model::structure::Tree::logical_range`.
    LogicalSelection,
    /// Which annotation holds the input focus, and where it is on the screen.
    ///
    /// The other half of [`crate::Command::Focused`]: §12.5.1 says a processor may let a person
    /// walk the annotations with the tab key and says nothing whatever about showing which one
    /// they are on — so the *ring* is a host's, drawn in its platform's focus colour, and this is
    /// the geometry it needs. Device pixels of the viewport, like every other shape this crate
    /// answers with.
    ///
    /// [`Answer::None`] where nothing is focused, or where the focused annotation states no
    /// usable `/Rect`.
    Focus,
    /// §12.5.6.14's popup windows that are open on the page being shown.
    ///
    /// The clause makes a popup "a window … for entry and editing" with "no appearance stream", so
    /// it is the one annotation subtype whose picture is not the page's: a host draws it as chrome,
    /// in its platform's window furniture, which is why this answers text and a rectangle rather
    /// than pixels. Device pixels of the viewport, like [`Query::Selection`] and [`Query::Focus`].
    ///
    /// Only the open ones: Table 186's `/Open` says which start that way and
    /// [`crate::Command::Activate`] on the parent annotation changes it, which is §12.5.1's
    /// "[w]hen the user activates the annotation by clicking it, it exhibits its associated
    /// object, such as by opening a popup window displaying a text note".
    Popups,
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
    /// §12.4.3's article threads, in the `/Threads` array's own order.
    Articles(Vec<pdf_model::article::Thread>),
    /// §12.3.5's collection dictionary, read.
    Collection(pdf_model::collection::Collection),
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
    Field {
        /// The two names §14.9.3 makes a processor distinguish.
        name: pdf_model::view::FieldName,
        /// What the field says **now**, as §12.7.4.3 would lay it out.
        ///
        /// **`None` and `Some("")` are different answers.** `None` is a field whose value is not
        /// text at all — a button selects an appearance, a signature holds a dictionary, a list
        /// box states which items are selected — and `Some("")` is a text field with nothing in
        /// it. A host deciding where to send the keyboard needs exactly that distinction, and
        /// `viewer-ui` uses it for exactly that.
        ///
        /// **Not the file's `/V`**: whichever of the four statements about a value is current,
        /// which is what a host has to append to. Since ADR 0197 a field carrying §12.7.5.3's
        /// `DoNotScroll` takes only as much of a value as fits its rectangle, so a host keeping
        /// its own buffer would diverge from the field on the first character past the edge —
        /// reading this back after every keystroke is what makes that impossible.
        ///
        /// A password field answers with Table 231 bit 14's bullets rather than with its
        /// characters: a host is allowed to draw them and not to know them.
        value: Option<String>,
    },
    /// Where the caret is, in device pixels of the viewport.
    ///
    /// Two points and not a rectangle, because a caret has no width: how thick a text cursor is
    /// drawn is a platform's convention rather than a fact about the document, and a widget's
    /// `/R`, its appearance's `/Matrix` or its `/DA`'s `Tm` can turn it off the axes anyway.
    Caret {
        /// The end on the descent side of the line's baseline — the *bottom* on an upright page.
        from: (f32, f32),
        /// The end on the ascent side.
        to: (f32, f32),
    },
    /// Where a string occurs, one entry per occurrence in the order they are shown.
    ///
    /// Each is the shapes covering one occurrence, merged per run of a line, in device pixels of
    /// the viewport — the same form [`Selected::quads`] takes, because a host draws them the same
    /// way and in its own colour.
    Found(Vec<Vec<[f32; 8]>>),
    /// Whether anything has been edited.
    Dirty(bool),
    /// The focused annotation and the quadrilateral covering it, in device pixels.
    Focus {
        /// The annotation itself, which is what §12.6.3's `/Fo` was raised against.
        object: ObjectId,
        /// Its `/Rect` on the screen, `[x0, y0, … x3, y3]`, y downwards.
        quad: [f32; 8],
    },
    /// §14.8.2.5's logical content order for the selection, a rearrangement of the same bytes.
    LogicalSelection(String),
    /// §12.5.6.14's open popup windows, in the `/Annots` array's order.
    Popups(Vec<PopupWindow>),
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

/// One of §12.5.6.14's popup windows, placed on the screen.
///
/// **Geometry and text, not pixels**, for the reason [`Selected`] gives: a window belongs to the
/// host's platform. What this crate owns is which windows are open, where the page puts them and
/// what the document says goes in them; what a title bar looks like is the host's, and a native
/// one would draw a real window rather than a rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct PopupWindow {
    /// The popup annotation, which is what [`crate::Command::Activate`] closes.
    pub annotation: ObjectId,
    /// The markup annotation whose text this is, where Table 186's `/Parent` names one.
    pub parent: Option<ObjectId>,
    /// The window's `/Rect` on the screen, `[x0, y0, … x3, y3]` clockwise from the top-left,
    /// y downwards — the same form [`Answer::Focus`] and [`Selected::quads`] take.
    pub quad: [f32; 8],
    /// §12.5.6.2's `/T`: "[t]he text label that shall be displayed in the title bar".
    pub title: Option<String>,
    /// Table 166's `/Contents`: the text in the window.
    pub text: Option<String>,
    /// Table 166's `/M`, as the file spells it — the table makes displaying it in *any* format a
    /// `shall`, so this crate does not narrow it to the dates §7.9.4 states.
    pub modified: Option<String>,
    /// Table 166's `/C`: "[t]he title bar of the annotation's popup window".
    pub colour: Option<pdf_render::Color>,
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
