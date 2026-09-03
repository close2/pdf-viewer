//! What a host asks of the viewer.
//!
//! One enum, and every variant is something a person did or something a worker finished. There
//! is deliberately no command that means "recompute": what has to be redrawn is this crate's
//! deduction, not the host's, because a host that had to know would be reimplementing the
//! scheduler.

use pdf_render::Raster;
use pdf_syntax::ObjectId;

use crate::secret::Secret;
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
    ///
    /// The fragment is Annex O's, and it is the text after `#` in the URI the bytes came from,
    /// exactly as the URI spells it. **Undecoded**: percent-decoding is done per argument after
    /// the fragment is split on its own separators, because a `%26` inside a destination's name is
    /// data rather than a separator. Splitting the *URI* is the host's — RFC 3986 is not this
    /// crate's business and a host that opened a file from a path has no fragment to give.
    Open {
        /// What the host will call this document.
        id: DocumentId,
        /// The file, whole.
        bytes: Vec<u8>,
        /// §7.6.4.1's user or owner password, where one is already known.
        ///
        /// A [`Secret`] rather than a `String` because this enumeration derives [`Debug`] and two
        /// hosts trace a command by printing one — see [`crate::secret`] for what that cost and
        /// what the type carries instead.
        password: Option<Secret>,
        /// Annex O's fragment identifier — the text after `#`, or nothing.
        fragment: Option<String>,
    },
    /// §12.4.4: the host has entered or left presentation mode.
    ///
    /// **The third host-supplied policy value, beside [`Self::Restrict`] and [`Self::Delegate`],
    /// and it is here because §12.4.4.2 makes a *state machine* depend on it.** That clause's
    /// NOTE 3 says an interactive processor "needs to respect navigation nodes only when in
    /// presentation mode", so whether an arrow key steps through a page's own states or turns the
    /// page is decided by something no state machine over a file can see: full screen is chrome
    /// and chrome is the host's (rule 5), so the host is the only party that knows.
    ///
    /// **This is not a second way to say something the host can already say.** A host that drives
    /// [`Self::Tick`] is running §12.4.4.1's clock, and this crate went on deducing "a
    /// presentation is running" from that until §12.4.4.2 was read properly (ADR 0135, amended by
    /// ADR 0316): a presentation a person steps through by hand sends no ticks at all, so the
    /// deduction answers *no* for exactly the case the clause is about.
    ///
    /// Two things happen when it arrives, both the clause's:
    ///
    /// - Entering makes the page's `/PresSteps` node current, in every open document, and saves
    ///   §8.11's group states. NOTE 2: "[i]nteractive PDF processors need to save the state of
    ///   optional content groups when a user enters presentation mode and restore it when
    ///   presentation mode ends."
    /// - Leaving restores them and leaves no current node.
    ///
    /// Applies to every open document and to every one opened afterwards, until it is sent again,
    /// which is [`Self::Delegate`]'s rule and for [`Self::Delegate`]'s reason: it is a fact about
    /// the *window* rather than about any one file.
    Present(PresentationMode),
    /// Time has passed, in milliseconds — ISO 32000-2 §12.4.4.1's `/Dur`, driven by the host.
    ///
    /// **Rule 3: this crate has no clock.** A presentation advances by itself, and the only way
    /// a state machine with no clock can know that a second went by is to be told. A host
    /// showing a presentation sends these; a host reading a document sends none, and nothing
    /// advances. What a tick does *not* say is whether a presentation is running — that is
    /// [`Self::Present`], because a person stepping through a slide show by hand sends no ticks.
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
    /// Put the focused document back at a view [`crate::Query::View`] answered with.
    ///
    /// **The absolute half of a vocabulary that is otherwise relative, and it exists because a
    /// host cannot reconstruct a view out of the commands that made it.** [`Self::GoTo`] states a
    /// page and [`Self::Zoom`] states a magnification, but the third part of a view — how far the
    /// page is scrolled under the viewport — has only [`Self::Scroll`], which is a *delta* the
    /// viewer clamps. A host that had watched every scroll it sent still could not say where the
    /// reader is, because what it sent and what the clamp kept are two different numbers.
    ///
    /// **A host does not compose one of these; it echoes one.** The value comes from
    /// [`crate::Answer::View`] and goes back unchanged, which is what makes the restore exact
    /// rather than close: the same `f32`s in both directions, with no arithmetic in between to
    /// round. Replaying the three commands instead — the page, then the magnification, then a
    /// scroll of the difference — reaches the same place only to within a rounding of the
    /// subtraction, and it requires a host to know which of the three resets the others.
    ///
    /// The three parts are applied as one: the page moves first, because a turn is what zeroes
    /// the scroll; then the magnification, because it is what the scroll is measured in; then the
    /// scroll itself. A page that moved raises [`crate::Event::PageChanged`] exactly as
    /// [`Self::GoTo`] does, because the reader is looking at a different page and §12.6.3's
    /// notification does not care which command moved them.
    ///
    /// **What it is for**, and the reason it is a command rather than an instrument: a confined
    /// worker that dies is one page's breach rather than the document's end (ADR 0734), and the
    /// host that starts another one has to put the reader back where they were. Nothing on this
    /// boundary could say where that was.
    View(Viewing),
    /// How much of what a document asserts about its reader this viewer obeys.
    ///
    /// **The one policy value in this crate, and the host is the only place it can come from.**
    /// Rule 2's principle exactly: a document's restrictions — §7.6.4.2's Table 22, §12.8.2.2's
    /// `/DocMDP` — are the *reader's* to set, and how much a person's own program obeys somebody
    /// else's file is not something a state machine over the file can know. `CLAUDE.md` states
    /// it: "**it shall always be possible to turn them off**".
    ///
    /// Applies to every open document and to every operation after it, until it is sent again.
    /// [`RestrictionLevel::On`] is the default, because a reader that ignored a document's
    /// restrictions without being asked would be making the choice on the person's behalf in the
    /// other direction.
    Restrict(RestrictionLevel),
    /// The person's answer to [`crate::Event::Asking`].
    ///
    /// **The command that makes [`RestrictionLevel::Ask`] a level rather than a variant nothing
    /// answers**, and it has the shape [`crate::Event::PasswordRequired`] answered by
    /// [`Self::Open`] has: the viewer holds the edit it was asked about — resolved, so that what
    /// goes ahead is what was asked for at the moment it was asked (see `open::Done`) — and this
    /// carries the one bit only a person can supply. `true` performs the edit exactly as
    /// [`RestrictionLevel::Off`] would have; `false` forgets it, and says nothing, because a
    /// question declined is not a document doing something and not this program refusing.
    ///
    /// One question is outstanding per document at a time: an edit that needs asking while one is
    /// held replaces it, on the reading that a person who moved on to a second edit has left the
    /// first unanswered, and a `No` to nothing outstanding does nothing. ADR 0814.
    Answer {
        /// Which document the question was about.
        document: DocumentId,
        /// Whether to go ahead.
        proceed: bool,
    },
    /// Table 29's `/PageLayout`: how the pages are arranged in the window.
    ///
    /// **The fourth host-supplied policy value, and it passes `doc/ui-boundary.md`'s test for the
    /// same reason [`Self::Present`] does.** The entry states the layout that "shall be used when
    /// the document is opened" — an *initial* state, which this crate reads for itself out of the
    /// catalog when a document opens and needs no host for. What no state machine over a file can
    /// know is what the person reading it has since chosen, and `CLAUDE.md`'s rule about a
    /// document's assertions over its reader applies with full force here: an arrangement a
    /// reader cannot leave is an arrangement somebody else's file imposed on them.
    ///
    /// **It is not a second way to say something a host can already say.** A layout decides which
    /// pages are interpreted, where each one's raster sits and what a point on the screen lands
    /// on, and every one of those is this crate's — a host holds no page sizes, no display lists
    /// and none of ADR 0118's arithmetic. It is exactly the division rule 5 draws.
    ///
    /// Applies to the **focused document** rather than to every one, which is where it differs
    /// from [`Self::Restrict`], [`Self::Present`] and [`Self::Delegate`]: those three are facts
    /// about the window and this is a way of looking at one file, which Table 29 makes a
    /// *document's* entry and which a person may reasonably want differently for two documents
    /// open side by side.
    Layout(pdf_model::viewer_preferences::PageLayout),
    /// Who draws §12.7's form widgets: this crate, or the host that placed real controls.
    ///
    /// **The second host-supplied policy value, and a different kind from [`Self::Restrict`].**
    /// That one says how much of what a *document* asserts over its reader this program obeys;
    /// this one says which half of the page the *host* has undertaken to draw. Both are asked
    /// once and answered by the only party that knows — §6.3.2.2's "unless otherwise instructed"
    /// is addressed to a processor, and only a host knows whether it has instructed.
    ///
    /// Applies to every open document and to every one opened afterwards, until it is sent again.
    /// [`pdf_model::view::WidgetAppearances::Drawn`] is the default, so a host that never sends
    /// this gets the page §6.3.2.2 describes and the display list this crate has always produced.
    ///
    /// **Not on [`Self::Open`] and not on [`crate::RenderRequest`]**, and neither is arbitrary. A
    /// render request is this crate's *output* — it carries a display list that has already been
    /// interpreted — so a flag there would arrive after the decision it governs. And a host may
    /// change its mind: `viewer-gtk` can show the same document with its controls or with the
    /// document's own pictures, which is how ADR 0245's evidence was taken.
    ///
    /// Changing it re-interprets the page, because §12.5.5's appearance streams are drawing
    /// commands and not pixels.
    Delegate(pdf_model::view::WidgetAppearances),
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
    /// The name is the key §7.11.4.1's `/EmbeddedFiles` tree filed the file under, which is
    /// [`pdf_model::attachment::Attachment::name`] and is what [`crate::Query::Attachments`]
    /// answered with. A name the tree does not hold extracts nothing and says so; a name it
    /// holds twice extracts the first, because the clause makes a name tree's keys unique and a
    /// file that broke that has not said which it meant.
    ///
    /// **The middle of that first sentence was a quotation, and it was wrong twice over.** Until
    /// the seven-hundred-and-fiftieth session it put *the tree shall map name strings to file
    /// specifications* inside quotation marks: the clause opened the sentence with *the
    /// associated name tree*, so the quoted words are not ISO 32000-2's, and the sentence itself
    /// was struck out with the two bullets around it by Errata Collection 3 (Issue #481, `/State`
    /// `Review` `Completed`). `pdf_model::attachment` was corrected for that erratum in the
    /// four-hundred-and-eighteenth session and `viewer_host::panel` in the
    /// four-hundred-and-twenty-ninth; **this third copy outlived both because the paraphrase hid
    /// it** — `spec-errata check` matches a quotation against the struck text, and one that
    /// misquotes matches nothing. Nothing about the command moves: what the key is stays what
    /// §7.7.4's name tree makes it.
    ///
    /// A file §12.5.6.15's annotation attaches is *not* named here and needs no command at all:
    /// it has no key in any tree, and a click on the annotation is what extracts it — the same
    /// [`crate::Event::Extracted`], reached through [`Self::Pointer`]. See
    /// [`crate::Query::Attachments`] for why the two routes are not one list.
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
    /// Take one step of a search across the whole document.
    ///
    /// **The document-wide half of a text search, and a command rather than a question for two
    /// reasons.** [`crate::Query::Find`] answers for the page being shown, from a readback that
    /// is already there, and that is what a highlight over the screen needs. Annex O asks the
    /// other one — Table Annex O.4's `search`, "[o]pen the document and search for one or more
    /// words, **selecting the first matching word in the document**" — and answering it means
    /// interpreting pages that are not on the screen and then *changing the view*: the page it
    /// lands on is shown and the occurrence becomes the selection, which is the annex's own verb.
    /// A `&self` query could do neither.
    ///
    /// **One page per step**, and the count still to read comes back on
    /// [`crate::Event::Searched`]. Rule 4 forbids blocking and rule 3 leaves this crate no clock
    /// to spend a time budget with, so the unit of work is the smallest honest one and the host
    /// decides how many it takes before it repaints — the same division that makes
    /// [`crate::Event::NeedsRender`] a request rather than a finished page. On ISO 32000-2's own
    /// 1023 pages a whole sweep is 5.84 s, which is what a host would otherwise have blocked for.
    Find(Find),
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
/// Six variants, and they are the two halves of `CLAUDE.md`'s amended exclusion: a value put into
/// something the document already holds, and an object added to it — an annotation, or since the
/// eight-hundred-and-eighty-fifth session §7.11.4's embedded file. All are a log beside an
/// immutable document, and all leave through §7.5.6's incremental update.
///
/// **Two of them are one subtype's**, which is what §12.5.6.6 costs and nothing else here did:
/// a free text annotation's text *is* the annotation, so creating one and then saying what it says
/// are two different things, where [`Self::Markup`] takes its whole meaning from the selection it
/// arrives beside. [`Self::FreeText`] draws the box, [`Self::SetFreeText`] fills it, and the
/// second names its target by object for [`Self::SetField`]'s reason turned around: a field has
/// §12.7.4.2's qualified name and an annotation has none.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// §12.7.4: put a value into a field, by the fully qualified name §12.7.4.2 gives it.
    ///
    /// A name rather than a widget, because §12.7.4.1 lets one field own several widgets and
    /// "a field's value" is the field's: typing into one of them changes all of them.
    /// [`crate::Query::FieldAt`] is where a host gets the name from a point.
    ///
    /// [`pdf_model::view::Entered::Cleared`] clears the field, which is what a person deleting the
    /// contents of one does and is a different state from never having touched it.
    ///
    /// **The value used to be `Option<String>`, and §12.7.5.4's list box is what changed it** in
    /// the four-hundred-and-twelfth session. Table 233 bit 22 — "(PDF 1.4) If set, more than one of
    /// the field's option items may be selected simultaneously" — lets a choice field hold several
    /// items at once, and one string could not say which several. Three hosts built a list box over
    /// [`crate::Query::Fields`] and all three asked their toolkit for single selection *because of
    /// this variant*; the shape ADRs 0166, 0167 and 0247 established is that the variant changes
    /// and every consumer fails to compile until it says what it does. ADR 0248.
    SetField {
        /// §12.7.4.2's fully qualified name.
        field: String,
        /// The new value: characters, §12.7.5.4's chosen options, or nothing.
        value: pdf_model::view::Entered,
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
    /// §12.5.6.6: put an empty free text annotation over a rectangle a person **drew**.
    ///
    /// > A free text annotation ( PDF 1.3 ) displays text directly on the page.
    ///
    /// The geometry is a drag rather than a selection, which is the whole difference from
    /// [`Self::Markup`] and follows from the clause: §12.5.6.10's four "appear as highlights,
    /// underlines, strikeouts … **in the text of a document**" and this one displays text of its
    /// own, so there is nothing on the page for it to be over.
    ///
    /// The two corners are in **device pixels of the viewport**, in either order, exactly as
    /// [`Command::Pointer`]'s point and [`crate::Query::Offset`]'s two are: what a host has is
    /// where the pointer went down and where it came up. The viewer maps them back through the
    /// transform the frame on the screen was drawn with and records what was *done* — the page and
    /// the rectangle in default user space — because the zoom and the scroll that made the mapping
    /// are not part of what a replay must reproduce.
    ///
    /// The colour is the text's, and it goes into Table 177's Required `/DA` rather than into
    /// Table 166's `/C`: that entry is an icon's background, a popup's title bar and a link's
    /// border, and this subtype has none of the three. Nothing happens for a rectangle with no
    /// area, which is a press that never moved.
    ///
    /// Answering *which* annotation it made is [`crate::Query::FreeTextAt`]'s, asked at a point
    /// inside the rectangle — an event carrying it would be a second way to say something a host
    /// can ask.
    FreeText {
        /// Where the drag began, in device pixels from the viewport's top-left corner.
        from: (f32, f32),
        /// Where it ended.
        to: (f32, f32),
        /// The colour of the text, as `DeviceRGB` components in 0..=1.
        colour: [f32; 3],
    },
    /// §12.5.6.6: say what a free text annotation a person added says, as Table 166's `/Contents`.
    ///
    /// That entry is the text to be displayed for the annotation, and on this subtype the text is
    /// the whole of what there is to display. The annotation is named by object because it has no other name — §12.7.4.2 gives a *field*
    /// a qualified name and nothing gives an annotation one — and a host gets the object from
    /// [`crate::Query::FreeTextAt`], which is the same shape [`Command::Activate`] and
    /// [`Command::SetGroup`] already take.
    ///
    /// **Only an annotation this session added takes it.** A free text annotation the file itself
    /// states is the producer's object, and replacing one is a different kind of writing from
    /// appending one; `doc/todo/33` carries what it would cost. Nothing happens, and the log stays
    /// as it was.
    SetFreeText {
        /// The object [`crate::Query::FreeTextAt`] answered with.
        annotation: ObjectId,
        /// What the annotation now says. Empty is an annotation with nothing in it, which is what
        /// one a person has just drawn is.
        text: String,
    },
    /// §7.11.4: put a file into the document, in one of §7.11.4.1's two homes.
    ///
    /// > Embedded file streams ( PDF 1.3 ) address this problem by allowing the contents of
    /// > referenced files to be embedded directly within the body of the PDF file.
    ///
    /// The bytes cross whole, for [`Command::Open`]'s reason: rule 2 says the host owns the
    /// filesystem, so what arrives here is the file and never a path. The name is what the file
    /// is filed under — §7.7.4's tree key where the home is the document, and Table 43's `/F`
    /// and `/UF` either way — and it is one namespace for both homes, so that [`Self::Detach`]
    /// by name is never ambiguous: a name already embedded, by the document or by this session,
    /// is refused with [`crate::Event::Reported`], because §7.9.6's keys "shall not overlap".
    ///
    /// What is done is an entry in the log like every other edit: the list
    /// [`crate::Query::Attachments`] answers with holds the file at once, the page draws its
    /// icon at once where the home is a page, [`Command::Undo`] forgets it, and the file on disk
    /// changes only when [`Command::Save`] writes §7.5.6's update — three objects and the tree,
    /// through the same writer `pdf-transform attachments --attach` uses. Which of Table 22's
    /// bits governs it depends on the home, and [`AttachHome`] says which. ADR 0814.
    Attach {
        /// The file, whole. A [`pdf_model::attachment::filing::Payload`] rather than a `Vec<u8>`
        /// for [`Secret`]'s reason one field up: this enumeration derives `Debug` and two hosts
        /// print a command, and a payload prints as its length.
        bytes: pdf_model::attachment::filing::Payload,
        /// The name it is filed under.
        name: String,
        /// Table 43's `/Desc` — and Table 166's `/Contents` where the home is a page, because
        /// §12.5.6.15 makes that the text a reader shows.
        description: Option<String>,
        /// Table 44's `/Subtype`, the media type, where the host knows one.
        mime: Option<String>,
        /// Which of §7.11.4.1's two homes.
        home: AttachHome,
    },
    /// §7.11.4: take an embedded file out of the document, by the name it is filed under.
    ///
    /// The name is [`pdf_model::attachment::Attachment::name`], which is what
    /// [`crate::Query::Attachments`] answered with. A file this session attached is forgotten —
    /// the same outcome as undoing its attach, reached from the list rather than from the log —
    /// and one the document's own `/EmbeddedFiles` tree names leaves the list at once and the
    /// tree when the next save rewrites it, with what the entry alone reached marked free in
    /// §7.5.6's new cross-reference section. A name nothing files is [`crate::Event::Reported`].
    ///
    /// **A file §12.5.6.15's annotation carries in the file's own `/Annots` is not among what
    /// this reaches**: taking it out would be deleting the producer's annotation, which is a
    /// different kind of writing from appending and is `doc/todo/33`'s.
    Detach {
        /// The `/EmbeddedFiles` key.
        name: String,
    },
}

/// Where [`Edit::Attach`] puts a file: one of §7.11.4.1's two homes.
///
/// > An embedded file stream shall be included in a PDF file in one of the following ways:
///
/// — "[a]ny file specification dictionary in the document may have an EF entry", which "shall be
/// used for file attachment annotations … which associate the embedded file with a location on a
/// page in the document", or the `EmbeddedFiles` entry of the name dictionary, which associates
/// it "with the document as a whole".
///
/// **The two homes read different bits of §7.6.4.2's Table 22, and that is decided from the
/// table's text.** A file filed in the tree is bit 4's — "[m]odify the contents of the document
/// by operations other than those controlled by bits 6, 9, and 11", the residual no other bit
/// carves out (ADR 0802). A file filed by an annotation is bit 6's, "[a]dd or modify text
/// annotations": §12.5.6.15 makes the file part of the annotation — it "contains a reference to a
/// file, which typically shall be embedded" — and bit 4's own row hands whatever bit 6 controls
/// to bit 6, so the annotation and the file it carries are one act of annotating, exactly as a
/// markup and the appearance stream it carries are. The consequence is stated rather than hidden:
/// a certification at §12.8.2.2's level 3, or a Table 22 word granting bit 6 and not 4, permits a
/// file on a page and withholds one in the tree. ADR 0814.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttachHome {
    /// §7.7.4's `/EmbeddedFiles` tree: the document as a whole. Bit 4,
    /// [`pdf_model::restriction::Operation::Modify`].
    Document,
    /// §12.5.6.15's annotation on the page under a point of the viewport, in device pixels from
    /// its top-left corner exactly as [`Command::Pointer`]'s point is. Bit 6,
    /// [`pdf_model::restriction::Operation::Annotate`].
    ///
    /// The icon is a 20-unit square centred on the point in the page's user space — a stated
    /// choice, `pdf_model::attachment::filing::rect_around` — with Table 187's default `PushPin`.
    /// A point on no page attaches nothing, which is a host's mistake rather than a document's.
    Page {
        /// Where, in device pixels from the viewport's top-left corner.
        at: (f32, f32),
    },
}

/// What this reader does with the restrictions a document asserts over it.
///
/// The project owner's statement, in `CLAUDE.md`, names four levels: `off`, `on`, *ask before the
/// operation*, and *warn before the operation*. **All four are here since the
/// eight-hundred-and-eighty-fifth session**, and the two that arrived last did so on the
/// condition the two-hundred-and-twelfth set: not as variants nothing produces and nothing
/// answers, but with the event a window receives and the command that answers it —
/// [`crate::Event::Asking`] answered by [`Command::Answer`], and [`crate::Event::Warned`] after
/// an edit that went ahead — which is the shape [`crate::Event::PasswordRequired`] already had.
///
/// **What made them cheap to add is that nothing here is `#[non_exhaustive]`.** Every consumer
/// of this crate failed to compile until it said what it does about the two events, which is the
/// whole reason `doc/ui-boundary.md` keeps these enums exhaustive (ADR 0212, ADR 0814).
///
/// **The four levels themselves live in `pdf_model::restriction::Level` since the
/// eight-hundred-and-seventy-second session**, mapped by [`RestrictionLevel::level`], and this
/// type exists so that a host names the level in this crate's vocabulary rather than in
/// `pdf-model`'s. `Viewer::standing` asks `pdf_model::restriction::decide` once per edit and
/// matches its verdict exhaustively. ADR 0803.
///
/// **Which level a person is at is the host's to supply and never asked here**: `On` is the
/// default for the reason its own entry gives, and a window without a way to set the other three
/// is a window at `On` — or at `Off`, by the word `viewer_host::IGNORE_RESTRICTIONS`, because
/// `CLAUDE.md` says that one "shall always be possible".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RestrictionLevel {
    /// Obey what the document asserts: the operation is refused and the reason is said, as
    /// [`crate::Event::Refused`].
    ///
    /// The default, and §7.6.4.1's `shall` — "PDF readers shall respect the intent of the
    /// document creator by restricting user access to an encrypted PDF file according to the
    /// permissions contained in the file" — is kept by a reader who leaves it alone.
    #[default]
    On,
    /// Ignore what the document asserts and perform the operation.
    ///
    /// **This does not make the file say something untrue.** §12.8.2.3's withdrawal of a `/UR3`
    /// whose grant a save exceeds still happens, because that is not a restriction on the reader
    /// — it is a statement the *file* would be making about bytes nobody signed. Turning a
    /// restriction off is the reader's; making the file lie is not.
    Off,
    /// Put what the document asserts to the person and wait: [`crate::Event::Asking`], answered
    /// by [`Command::Answer`], with the edit held in between and nothing done until a `yes`.
    Ask,
    /// Perform the operation and then say what the document asserted: [`crate::Event::Warned`],
    /// after the edit and after the [`crate::Event::Dirty`] it caused.
    Warn,
}

impl RestrictionLevel {
    /// The same level as `pdf_model::restriction` names it.
    #[must_use]
    pub const fn level(self) -> pdf_model::restriction::Level {
        match self {
            Self::On => pdf_model::restriction::Level::On,
            Self::Off => pdf_model::restriction::Level::Off,
            Self::Ask => pdf_model::restriction::Level::Ask,
            Self::Warn => pdf_model::restriction::Level::Warn,
        }
    }
}

/// Whether §12.4.4's presentation is running, as the host has said.
///
/// Two states rather than four, and unlike [`RestrictionLevel`] that is not an instalment: the
/// clause has two — "when a user enters presentation mode" and "when presentation mode ends" —
/// and a third would be a state no sentence of §12.4.4 produces and no host answers, which is
/// what `doc/todo/38` forbids.
///
/// What this crate does with it is §12.4.4.2's, and full screen is deliberately not part of it.
/// §12.4.4.1 says a processor "may allow a document to be displayed in the form of a presentation
/// or slide show" and says nothing whatever about a window; what the clause states is the advance
/// timing, the transition and the current navigation node, and those are what change here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresentationMode {
    /// A document being read. The default, and what every host that never sends this gets.
    #[default]
    Off,
    /// A presentation is running: §12.4.4.2's nodes are respected and a page turn plays `/Trans`.
    On,
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

/// What a step of [`Command::Find`] is doing.
///
/// Three verbs rather than one, because "look for this string" and "keep looking" and "stop
/// looking" are three different things a find bar does and a host that had to infer which from
/// the string alone could not tell a re-typed needle from a request for the next occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Find {
    /// Look for a string, beginning at what is selected now, and take the first step.
    ///
    /// Sent again for every *next* and *previous*: the search starts from the current selection,
    /// so a second `Start` with the same string finds the occurrence after the one it is on.
    /// Whatever search was in progress is abandoned.
    Start {
        /// What to look for. Case is folded and a space matches whatever separated the words on
        /// the page — see [`crate::Query::Find`], which uses the same rule so that the page's
        /// highlights and the document's search cannot disagree.
        needle: String,
        /// Which way through the document.
        direction: FindDirection,
    },
    /// Read one more page of the search already in progress.
    ///
    /// What a host pumps until [`crate::Event::Searched`] says nothing is remaining. Nothing at
    /// all when no search is in progress, which is the right answer for a host that pumped once
    /// too often.
    Continue,
    /// Forget the search in progress.
    ///
    /// What closing a find bar sends. It does not clear the selection: what is selected is a
    /// separate statement and [`Command::Select`] is how a host clears it.
    Stop,
}

/// Which way through the document a [`Command::Find`] reads.
///
/// **Wrapping is this crate's choice and not the standard's**, which describes no find bar at
/// all: a search that runs off the end comes round to the beginning and stops where it started,
/// so every occurrence is reachable from anywhere and none is reported twice.
/// [`crate::Event::Searched`] says when it has wrapped, because a host that wants to tell a
/// person cannot work it out. Annex O's `search` does **not** wrap — it wants "the first matching
/// word in the document", so it begins at the beginning and stops at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindDirection {
    /// Down the document from where the selection is.
    Forward,
    /// Up it.
    Backward,
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
/// Four outcomes rather than a `Result<Raster, E>`, because a host that draws straight to its
/// own surface has no raster to hand back and has not failed either. That is the difference
/// between tier 1 and tier 2 of this project's three pixel tiers, and it is the only place in
/// the protocol where the two differ.
///
/// **Two of the four say the viewer holds no pixels, and they are not the same statement.**
/// [`Self::Presented`] is about the *viewer* — this host draws every page onto its own surface,
/// for good, at whatever size its window is — and [`Self::Listed`] is about *this page*. The
/// difference is what [`crate::MAX_PIXELS`] is for: a host that has presented once is asking for
/// no allocation here ever again and the budget stops applying to it, while a host that took one
/// page's list may hand the next one back as pixels and must go on being bounded.
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
    ///
    /// **A statement about the host and not about the page**, and it is remembered as one: after
    /// it, [`crate::Query::Frame`] answers [`crate::Answer::None`] for every page and
    /// [`crate::MAX_PIXELS`] stops bounding what this crate asks for, because nothing here will
    /// hold a whole-page raster again. A host that means it about one page wants [`Self::Listed`].
    Presented,
    /// The host took this request's own [`crate::RenderRequest::list`] and holds the page itself.
    ///
    /// **Per page, which is the whole of what distinguishes it from [`Self::Presented`].** The
    /// viewer keeps no pixels for this page and will not repaint it, exactly as for a presented
    /// one; what it does not do is conclude anything about the host. The page holds its place in
    /// the arrangement, so the scheduler does not ask for it again; [`crate::Query::Frame`] goes
    /// on answering for the pages whose pixels this crate *is* holding; and
    /// [`crate::MAX_PIXELS`] goes on bounding every request, because the next page may come back
    /// as [`Self::Raster`].
    ///
    /// It exists for a host that can afford the pixels of some pages and would rather have the
    /// marks of others — `viewer-confined`'s worker is the one in this tree, which compares the
    /// two byte counts per page and ships whichever is smaller. Answering [`Self::Presented`]
    /// there would take the raster budget off a process running under an address-space ceiling,
    /// where an unbounded allocation is a kill rather than a refusal (ADR 0640).
    ///
    /// [`crate::Event::Damage`] follows it, because taking a list is not the same as having
    /// drawn it: the host, or whoever the host is answering for, still has a page to put on a
    /// screen.
    Listed,
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

/// Where the reader is looking: the page, the magnification, and the scroll.
///
/// **One fact rather than three**, and that is the whole reason it is a type. The scroll is
/// measured from the top of the current page's row, so a scroll paired with a different page is
/// not a smaller truth but a wrong one; and the magnification is what the scroll's device pixels
/// are pixels *of*. [`crate::Query::View`] answers with one and [`Command::View`] takes one back,
/// which is the only pair of directions this value is meant to travel in.
///
/// The fields are public because a host displays them — a window's title, a trace line, a
/// remembered position in a configuration file — and not so that a host can build one from
/// numbers of its own. A view composed rather than answered is a guess about where the clamp
/// would have left the reader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewing {
    /// Which page, zero-based — the page every question about "the page being shown" is answered
    /// for, and the page the scroll below is measured from.
    pub page: usize,
    /// How large the page is drawn, as [`Command::Zoom`] would state it.
    ///
    /// A *mode* where the reader chose one, because [`Zoom::FitWidth`] and its two neighbours
    /// survive a resize and the magnification they resolve to does not: restoring the number
    /// would restore this window's picture and lose the next window's. [`Zoom::In`] and
    /// [`Zoom::Out`] are steps rather than places, so this crate never answers with one — and
    /// [`Command::View`] carrying one nonetheless steps from wherever it lands, which is what
    /// those two mean everywhere else rather than a second meaning here.
    pub zoom: Zoom,
    /// How far the page is scrolled under the viewport, in device pixels.
    ///
    /// Positive means the content has moved up and left — the same sense [`Command::Scroll`]'s
    /// delta has, and the same units. **Already clamped**: this is where the viewer put the
    /// reader, not where a host asked for them to be.
    pub scroll: (f32, f32),
}
