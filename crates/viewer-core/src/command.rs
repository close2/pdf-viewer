//! What a host asks of the viewer.
//!
//! One enum, and every variant is something a person did or something a worker finished. There
//! is deliberately no command that means "recompute": what has to be redrawn is this crate's
//! deduction, not the host's, because a host that had to know would be reimplementing the
//! scheduler.

use pdf_render::Raster;
use pdf_syntax::ObjectId;

use crate::viewer::{DocumentId, RenderToken};

/// One thing a host asks the viewer to do.
#[derive(Debug)]
pub enum Command {
    /// Open a document from bytes, under a host-chosen identity.
    ///
    /// Bytes rather than a path, because rule 2 says the host owns the filesystem. The identity
    /// is the host's so that it can name the same document in a later command without waiting
    /// for an event to tell it what the document was called.
    ///
    /// The password is §7.6.4.1's, where the host already has one. A document that needs one
    /// and was given none — or the wrong one — produces [`crate::Event::PasswordRequired`] and
    /// no open document, which is the prompt this project has owed since encryption landed.
    Open {
        /// What the host will call this document.
        id: DocumentId,
        /// The file, whole.
        bytes: Vec<u8>,
        /// §7.6.4.1's user or owner password, where one is already known.
        password: Option<String>,
    },
    /// Time has passed, in milliseconds — ISO 32000-2 §12.4.4.1's `/Dur`, driven by the host.
    ///
    /// **Rule 3: this crate has no clock.** A presentation advances by itself, and the only way
    /// a state machine with no clock can know that a second went by is to be told. A host
    /// showing a presentation sends these; a host reading a document sends none, and nothing
    /// advances — which is why *whether a presentation is running* is not a state this crate
    /// keeps. Full screen is chrome, and chrome is the host's (rule 5).
    ///
    /// > If no Dur entry is specified in the page object, the page shall not advance
    /// > automatically.
    ///
    /// So a page stating no `/Dur` swallows every tick, and NOTE 1's other half — "[t]he user
    /// can advance the page manually before the specified time has expired" — is
    /// [`Self::GoTo`], which resets the clock because it changes the page.
    Tick {
        /// How long since the last tick, in milliseconds.
        millis: u32,
    },
    /// Close a document and forget everything derived from it.
    Close(DocumentId),
    /// Make an already-open document the one commands apply to.
    Focus(DocumentId),
    /// The viewport changed size, or moved to a display with a different scale.
    ///
    /// Width and height are **device** pixels and `scale` is the ratio of device pixels to
    /// logical ones — 2.0 on a doubled display. Both, rather than logical pixels alone, because
    /// a page must be rasterised at the resolution it will be shown at: rendering at the
    /// logical size and letting the compositor scale it up is exactly the blur this project's
    /// first principle exists to avoid.
    Resize {
        /// Viewport width in device pixels.
        width: u32,
        /// Viewport height in device pixels.
        height: u32,
        /// Device pixels per logical pixel.
        scale: f32,
    },
    /// Show another page of the focused document.
    GoTo(PageTarget),
    /// Change the magnification of the focused document, holding one point of the viewport still.
    ///
    /// `at` is a point of the viewport in device pixels, and it is what makes a wheel zoom feel
    /// like magnification rather than like a jump: the page grows about the thing being pointed
    /// at instead of about the middle of the window. `None` is the viewport's centre, which is
    /// what a keyboard `+` has to mean because it names no point.
    ///
    /// Nothing in the standard decides this — §12.3.2.1's magnification is a *document's*
    /// opinion about where to look (ADR 0162) and this is a reader's — so it is a documented
    /// choice, argued in ADR 0166.
    Zoom {
        /// How large the page is to be drawn.
        zoom: Zoom,
        /// The viewport point to keep still, in device pixels; `None` for its centre.
        at: Option<(f32, f32)>,
    },
    /// Move the page under the viewport by a device-pixel delta.
    ///
    /// Positive `dy` moves the *content* up, which is what a wheel scrolling down does. Clamped
    /// to the rendered page: a viewer that let a page be scrolled out of sight would be
    /// answering a question nobody asked.
    Scroll {
        /// Horizontal delta in device pixels.
        dx: f32,
        /// Vertical delta in device pixels.
        dy: f32,
    },
    /// Change something about the document, and add it to the log.
    ///
    /// The document itself is never changed — `CLAUDE.md`'s rule 1 makes it immutable — so an
    /// edit is an entry in a log beside it, which is what makes [`Command::Undo`] a matter of
    /// forgetting the entry rather than of restoring anything.
    Edit(Edit),
    /// Undo the last edit.
    Undo,
    /// Redo the last undone edit.
    ///
    /// A new edit after an undo discards what was undone, which is what a single log with a
    /// cursor means and what every editor does.
    Redo,
    /// §7.11.4: take an embedded file's bytes out of the document.
    ///
    /// Answered with [`crate::Event::Extracted`] carrying the decoded bytes, or with
    /// [`crate::Event::Reported`] naming why not. The *host* writes them somewhere, for the same
    /// reason it writes [`Self::Save`]'s: rule 2 says this crate has no filesystem, and where a
    /// file taken out of a document should land is a policy rather than a rendering decision.
    ///
    /// The name is the key §7.11.4.1's `/EmbeddedFiles` tree filed the file under — "the tree
    /// shall map name strings to file specifications" — which is
    /// [`pdf_model::attachment::Attachment::name`] and is what [`crate::Query::Attachments`]
    /// answered with. A name the tree does not hold extracts nothing and says so; a name it
    /// holds twice extracts the first, because the clause makes a name tree's keys unique and a
    /// file that broke that has not said which it meant.
    Extract {
        /// The `/EmbeddedFiles` key.
        name: String,
    },
    /// Write §7.5.6's incremental update for everything the log holds.
    ///
    /// Answered with [`crate::Event::Saved`] carrying the bytes, or with
    /// [`crate::Event::Reported`] naming why not. The *host* writes them somewhere: rule 2 says
    /// this crate has no filesystem, which is also what lets a confined process with none still
    /// produce a saved file.
    Save,
    /// Select something, or stop selecting.
    ///
    /// A drag is [`Self::Pointer`]'s business; this is what a menu item or a keystroke asks for.
    Select(Selection),
    /// §12.5.1: move the input focus to the next or previous annotation on the page.
    ///
    /// > Interactive PDF processors may permit the user to navigate through the annotations on a
    /// > page by using the keyboard (in particular, the tab key).
    ///
    /// The *order* is the document's — Table 31's `/Tabs`, all five values, in
    /// `pdf_model::tab_order` — and the *key* is the host's, because the clause names a key and
    /// this crate has no keyboard. What crosses is the direction, which is the only part of it
    /// that is neither.
    ///
    /// Raises §12.6.3's `/Bl` on whatever held the focus and `/Fo` on whatever receives it, which
    /// is the same pair a press raises and in the same order — Table 197 states one thing losing
    /// the focus before the next receives it.
    ///
    /// **Widgets are not the only annotations this visits**, unlike a press. §12.5.1 says "the
    /// annotations on a page" without qualification and Table 31's `W` exists precisely to name
    /// the *narrower* order, so a tab that skipped a link would be reading a rule the clause does
    /// not state. Table 197's `/Fo` and `/Bl` still fire only for widgets, because that table
    /// says so.
    Focused(FocusMove),
    /// Activate an object the host is showing outside the page — §12.3.3's outline item.
    ///
    /// The clause: "[c]licking the text of any visible item activates the item, causing the
    /// interactive PDF processor to jump to a destination **or trigger an action** associated
    /// with the item." A panel row is not a point on a page, so [`Self::Pointer`] cannot express
    /// it; and a host cannot perform the action itself, because `/A` may be any of §12.6's
    /// types and a whole `/Next` chain of them. So it hands over the item's *object* and this
    /// crate reads what activating it means.
    ///
    /// An object that is not something this crate knows how to activate does nothing and says
    /// nothing, which is the right answer for a host that named the wrong thing —
    /// [`crate::Event::Reported`] is for what the *document* could not do, not for what a
    /// caller asked.
    ///
    Activate(ObjectId),
    /// §8.11: switch an optional content group on or off.
    ///
    /// The group is named by object identity because that is what §8.11.2.2's `/OCGs` and Table
    /// 99's `/Order` hold; [`crate::Query::Layers`] is where a host gets the identities and the
    /// names to show beside them.
    SetGroup {
        /// The optional content group's object.
        group: ObjectId,
        /// Whether it is on.
        on: bool,
    },
    /// The pointer moved, or a button went down or up, at a point in the viewport.
    ///
    /// Device pixels, measured from the viewport's top-left corner — what a host has, rather
    /// than what the document uses; the viewer maps it back through the transform the frame on
    /// the screen was drawn with.
    ///
    /// One button, because that is all §12.5.5 assumes. NOTE 2 of that clause: "the term mouse
    /// denotes a generic pointing device that controls the location of a cursor on the screen
    /// and has at least one button". A host with several sends the primary one and keeps the
    /// rest for its own menus.
    Pointer {
        /// Where, in device pixels from the viewport's top-left corner.
        at: (f32, f32),
        /// What the pointer did.
        action: PointerAction,
    },
    /// The bytes the viewer asked for with [`crate::Event::NeedsFile`].
    ///
    /// `None` is a host that will not or cannot supply them, which is a legitimate answer and
    /// not an error: the policy about which files a document may name belongs to whoever owns
    /// the filesystem, and by rule 2 that is never this crate. The refusal is said out loud
    /// rather than swallowed.
    Supply {
        /// What the bytes were wanted for.
        purpose: Purpose,
        /// The file, or nothing.
        bytes: Option<Vec<u8>>,
    },
    /// A worker finished — or failed — the request it was handed.
    ///
    /// A token that does not match the request outstanding is **dropped**, which is the whole
    /// reason the token exists: a page turned while a render was in flight must not be
    /// overwritten by the frame the previous page produced.
    RenderReady {
        /// The token from [`crate::Event::NeedsRender`].
        token: RenderToken,
        /// What the worker did with it.
        rendered: Rendered,
    },
}

/// What the pointer did.
///
/// Three, because §12.5.5 names three situations and two of them are the ends of a press: "[t]he
/// normal appearance shall be used when the annotation is not interacting with the user … [t]he
/// rollover appearance shall be used when the user moves the cursor into the annotation's active
/// area without pressing the mouse button … [t]he down appearance shall be used when the mouse
/// button is pressed or held down within the annotation's active area".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerAction {
    /// The pointer moved with no button down.
    Moved,
    /// The button went down.
    Pressed,
    /// The pointer moved with the button held.
    ///
    /// Distinct from [`Self::Moved`] because the two mean opposite things: a move is a person
    /// looking for something and a drag is a person choosing it. This one extends the selection
    /// and never changes an annotation's appearance.
    Dragged,
    /// The button came up.
    ///
    /// **This is what activates a link**, and only where the button went down on the same
    /// annotation it came up on. The clause states no rule for that — it describes appearances,
    /// not activation — so this is a choice, and it is the one every pointing interface makes:
    /// a press that is dragged away before release is a press the person changed their mind
    /// about.
    Released,
}

/// One change a person made.
///
/// Two variants, and they are the two halves of `CLAUDE.md`'s amended exclusion: a value put into
/// a field the document already holds, and an annotation added to it. Both are a log beside an
/// immutable document, and both leave through §7.5.6's incremental update.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// §12.7.4: put a value into a field, by the fully qualified name §12.7.4.2 gives it.
    ///
    /// A name rather than a widget, because §12.7.4.1 lets one field own several widgets and
    /// "a field's value" is the field's: typing into one of them changes all of them.
    /// [`crate::Query::FieldAt`] is where a host gets the name from a point.
    ///
    /// `None` clears the field, which is what a person deleting the contents of one does and is
    /// a different state from never having touched it.
    SetField {
        /// §12.7.4.2's fully qualified name.
        field: String,
        /// The new value, or nothing.
        value: Option<String>,
    },
    /// §12.5.6.10: mark up **what is selected**, in one of the clause's four ways.
    ///
    /// The first edit that adds an object to a document rather than changing one it already
    /// holds, and `CLAUDE.md` permits exactly it: what a *user* does to an open document is not
    /// authoring, and §7.5.6's incremental update writes it back with the producer's bytes
    /// untouched underneath.
    ///
    /// **The selection is the target, and it is resolved when the command arrives.** The clause
    /// defines these four over *text* — "text markup annotations shall appear as highlights,
    /// underlines, strikeouts … in the text of a document" — and this crate already has that
    /// geometry: [`crate::Query::Selection`]'s quadrilaterals, one per run of a line. Nothing
    /// happens where nothing is selected, which is a host's mistake rather than a document's.
    ///
    /// The colour is Table 166's `/C`, in `DeviceRGB` because that is what the entry's three
    /// components mean. What is *not* carried: Table 166's `/M`, because rule 3 gives this crate
    /// no clock, and `/T`, because a person's name is not something this program knows.
    Markup {
        /// Which of §12.5.6.10's four.
        kind: pdf_model::view::Markup,
        /// Table 166's `/C`, as `DeviceRGB` components in 0..=1.
        colour: [f32; 3],
    },
}

/// What [`Command::Select`] asks for.
///
/// Not in ISO 32000-2, which says nothing about selecting text. Every entry here is a choice
/// about a user interface, and the choices are in `viewer-core`'s `select` module rather than in
/// a host so that two hosts cannot make them differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// Everything the page reads back as.
    All,
    /// Nothing.
    None,
}

/// Which way [`Command::Focused`] moves through §12.5.1's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusMove {
    /// The next annotation, wrapping to the first at the end. What the tab key means.
    Next,
    /// The previous one, wrapping to the last. What shift-tab means.
    Previous,
    /// Nothing focused, which is what a click outside every annotation already does.
    None,
}

/// What a file the viewer asks a host for is wanted for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// §12.7.6.4's import-data action: the file holds §12.7.8's form data.
    ImportData,
}

/// What a worker did with a [`crate::RenderRequest`].
///
/// Three outcomes rather than a `Result<Raster, E>`, because a host that draws straight to its
/// own surface has no raster to hand back and has not failed either. That is the difference
/// between tier 1 and tier 2 of this project's three pixel tiers, and it is the only place in
/// the protocol where the two differ.
#[derive(Debug)]
pub enum Rendered {
    /// Tier 1: the pixels, for the viewer to hold and the host to blit when it is told to.
    ///
    /// The viewer keeps them, so scrolling and exposing repaint from memory rather than
    /// re-rendering. 1920×1080 RGBA is 8.3 MB; one frame per open document is the whole cost.
    Raster(Raster),
    /// Tier 2: the host drew the request onto its own surface and presented it.
    ///
    /// The viewer holds no pixels, so it cannot repaint on the host's behalf: a tier-2 host is
    /// told what changed by [`crate::Event::Damage`] and re-renders. It keeps its own display
    /// list to do that with — the request it was handed is enough.
    Presented,
    /// The request could not be drawn, and this is why.
    ///
    /// Reported rather than swallowed: a viewer that silently shows the previous page when a
    /// render fails is telling the person something false about the document.
    Failed(String),
}

/// Which page to show.
///
/// Relative and absolute in one enum because they are one question — "which page next" — and a
/// host that had to turn a key press into an index would be doing the clamping itself.
///
/// **There is deliberately no `Destination` variant.** One was added in the
/// hundred-and-sixty-sixth session for the outline panel and removed in the hundred-and-sixty-
/// eighth, when §12.3.3's other half was read properly: the clause asks a click to "jump to a
/// destination **or** trigger an action", so what a panel row sends is
/// [`Command::Activate`] — the item's object, from which this crate reads `/Dest` *and* `/A`.
/// A variant carrying only the jump would have been a path nobody takes, and `CLAUDE.md` forbids
/// shipping one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTarget {
    /// A zero-based index, which is the page tree's numbering and not a reader's.
    Index(usize),
    /// The first page.
    First,
    /// The last page.
    Last,
    /// The next page, or nowhere if this is the last.
    Next,
    /// The previous page, or nowhere if this is the first.
    Previous,
    /// A signed number of pages from here, clamped to the document.
    ///
    /// Clamped rather than wrapping: paging past the end and landing back at page one is
    /// disorienting, and the end of a document is information worth feeling.
    Relative(isize),
}

/// How large the page is drawn.
///
/// Two of these are *modes* rather than numbers — a page fitted to the viewport restays fitted
/// when the window is resized, which is the behaviour a fixed scale cannot express.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Zoom {
    /// The whole page, as large as fits.
    FitPage,
    /// The page's width, as large as fits, with the rest scrolled.
    FitWidth,
    /// The page's height, as large as fits, with the rest scrolled.
    ///
    /// Here because §12.3.2.2's `/FitV` asks for it — "the contents of the page magnified just
    /// enough to fit the entire height of the page within the window" — and a viewer that has
    /// fit-page and fit-width and not this one would have to answer that destination with a
    /// number rather than with a mode.
    FitHeight,
    /// A fixed magnification: logical pixels per PDF user space unit, where 1.0 is 72 dpi.
    Scale(f32),
    /// One step larger than whatever is showing now.
    In,
    /// One step smaller.
    Out,
}
