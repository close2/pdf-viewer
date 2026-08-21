//! Annotations, and where their appearance streams land on the page.
//!
//! An annotation is a dictionary in the page's `/Annots` array describing something drawn
//! *over* the page content: a form field, a highlight, a stamp, a signature. Almost all of
//! clause 12 is about how one behaves — what it does when clicked, what value it holds, how
//! it participates in a form. None of that is needed to *draw* one, and this module is
//! deliberately only about drawing.
//!
//! What draws is the appearance stream: `/AP /N` is a form `XObject`, and the interpreter
//! already runs those. So the work here is selection and placement — which annotations are
//! visible, which of an appearance dictionary's states applies, and the matrix ISO 32000-2
//! §12.5.5 defines from the stream's `/BBox` and `/Matrix` to the annotation's `/Rect`.
//!
//! An annotation with *no* appearance stream is handed to [`crate::appearance`], which
//! constructs one out of the entries its subtype's clause states. That module holds every
//! per-subtype rule; this one does not know a `Square` from a `Widget` except to ask.
//!
//! # A stored appearance is self-contained
//!
//! §12.5.2's closing sentence — quoted in full in [`crate::appearance`], as Errata Collection 3
//! leaves it — has a reader "ignore the values of the C, IC, Border, BS, BE, CA, ca, H, DA, Q,
//! DS, LE, LL, MK, LLE, and Sy keys" when an appearance dictionary is present, and Table 166
//! says of `/CA` and `/ca` that each "shall not be used if the annotation has an appearance
//! stream ... in that case, the appearance stream shall specify any transparency". So the two
//! opacities below are read for a *constructed* appearance and left at their defaults for a
//! stored one.
//!
//! §12.5.5 states the opposite in one sentence — the appearance's group "shall be composited
//! ... using the values of the BM, ca and CA entries in the annotation dictionary" — and this
//! tree followed that reading until the twenty-first session. Two statements against one for
//! the opacities, and the two explain themselves: the entries are what an appearance is
//! *regenerated* from, and a stream that carries its own `/ExtGState` would otherwise have the
//! same opacity applied twice. `highlight.pdf` is exactly that file: `/CA 0.8` on the
//! annotation, `ca 0.8` inside the stream.
//!
//! **`/BM` is no longer one of the three, and that is an erratum rather than a re-reading.** EC3
//! struck it out of §12.5.2's list, leaving §12.5.5 and Table 166's own `/BM` row agreeing that
//! the blend mode applies whenever the annotation is painted onto the page — see [`blend_mode`],
//! which both paths now use.

use pdf_render::{Transform, geom::Point};
use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Name, Object, Stream};

/// What an appearance's content is: a stream the file stored, or one this crate wrote.
#[derive(Debug, Clone)]
pub(crate) enum Content {
    /// `/AP /N`, a form `XObject` (§12.5.5).
    Stored(Arc<Stream>),
    /// A content stream constructed from the annotation's appearance characteristics
    /// (§12.7.4.3, [`crate::appearance`]), with the resources it may name.
    ///
    /// The resources are Table 224's `/DR`, which §12.7.4.3 makes the `/Resources` of an
    /// appearance a processor builds. They are empty for a construction that only draws
    /// paths, and they carry the `/DA` string's font for one that draws text.
    Constructed {
        bytes: Vec<u8>,
        resources: Dictionary,
    },
}

/// An appearance stream, resolved and placed.
#[derive(Debug, Clone)]
pub(crate) struct Appearance {
    /// The content stream to run.
    pub content: Content,
    /// `AA` from §12.5.5: maps the appearance's own coordinates into the page's default
    /// user space, so that its bounding box covers the annotation's `/Rect`.
    pub transform: Transform,
    /// The appearance's `/BBox`, in the appearance's own coordinates, which §8.10.2 makes
    /// the clip for a form `XObject`'s content — where there is one.
    ///
    /// `None` for a construction whose geometry the clause states in *default user space* rather
    /// than inside a box — see `crate::appearance::Constructed::bounded`. A stored appearance
    /// always has one, because §12.5.5 makes it a form `XObject` and §8.10.2 gives every form a
    /// `/BBox`.
    pub bbox: Option<[f32; 4]>,
    /// Table 166's `/ca`, the opacity "used for all nonstroking operations on all visible
    /// elements of the annotation", defaulting to `/CA` and then to 1. Only a constructed
    /// appearance has one.
    pub fill_alpha: f32,
    /// Table 166's `/CA`, the same for stroking operations. Only a constructed appearance has
    /// one.
    pub stroke_alpha: f32,
    /// The annotation's blend mode name, from `/BM`, if it names one and the appearance is
    /// constructed.
    pub blend: Option<String>,
    /// Where the stored stream this appearance came from stopped decoding, if it did.
    ///
    /// §12.5.5 makes an appearance a form `XObject`, which §7.8.2 makes a content stream, so the
    /// rule for a damaged one reaches it and the prefix is drawn. The
    /// shortfall is carried here rather than reported at the decode because §12.7.4.3's
    /// regeneration reads these bytes and hands back a *spliced copy* of them: a report taken
    /// where the drawn stream is finally decoded would miss exactly the annotations whose
    /// variable text a reader has changed. `None` for every construction, which is written
    /// here rather than read from a file.
    pub damaged: Option<crate::content::DamagedStream>,
}

/// What an entry in `/Annots` asks the page to draw.
#[derive(Debug, Clone)]
pub(crate) enum Decision {
    /// Run this appearance stream, and report `owed` if the construction fell short of what
    /// the clause asks for — a widget's background is drawable where its field's text is not,
    /// and losing either statement would be worse than making both.
    Draw {
        appearance: Box<Appearance>,
        owed: Option<String>,
        /// §12.5.6.19's `/H`, where a press asks for a mark over the appearance.
        highlight: Option<Mark>,
        /// §12.5.3's `NoZoom` and `NoRotate`, as a transform in default user space.
        ///
        /// Beside the appearance rather than inside it, because §12.5.5 applies it to the
        /// annotation rather than to its stream:
        ///
        /// > The annotation may be further scaled and rotated if either the NoZoom or NoRotate
        /// > flag is set (see 12.5.3, "Annotation flags"). Any transformation applied to the
        /// > annotation as a whole shall be applied to the appearance within it.
        ///
        /// §12.5.6.19's highlight is part of that whole, and it is drawn from the same
        /// transform for that reason. The identity for every annotation that sets neither flag,
        /// which is all but 124 in the corpus.
        adjust: ViewAdjust,
    },
    /// Draw nothing, and say nothing — the document asked for nothing to be drawn.
    Nothing,
    /// Draw nothing, because this crate cannot. The string says what, for the report.
    Unsupported(String),
}

/// Whether a press changes what this annotation looks like; see
/// [`crate::view::press_changes_appearance`].
pub(crate) fn press_changes(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> bool {
    let down = has_down(document, annotation);
    down || toggles_no_view(document, annotation, view)
        || !matches!(
            highlight(document, annotation, down),
            Highlight::None | Highlight::Push
        )
}

/// Whether the cursor arriving changes what this annotation looks like; see
/// [`crate::view::hover_changes_appearance`].
///
/// Two entries, one from each of the two clauses that make a picture depend on the pointer:
/// Table 170's `/R`, which §12.5.5 shows "when the user moves the cursor into the annotation's
/// active area without pressing the mouse button", and Table 167's `ToggleNoView`, which changes
/// whether the annotation is drawn at all.
pub(crate) fn hover_changes(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> bool {
    let appearances = document.get_key(annotation, "AP");
    let rollover = appearances
        .as_dict()
        .is_some_and(|appearances| !document.get_key(appearances, "R").is_null());
    rollover || toggles_no_view(document, annotation, view)
}

/// Whether §12.5.3's `ToggleNoView` reaches this annotation, through §12.7.8's `/F` if an FDF
/// file said anything about it.
///
/// The same two lines as [`decided`]'s, and for the same reason: Table 249's imported `/F`
/// "shall replace that of the F entry in the form's corresponding annotation dictionary", so a
/// flag an FDF set is a flag the picture depends on — and this function's whole job is to say
/// whether the picture depends on the pointer.
fn toggles_no_view(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> bool {
    let stated = document
        .get_key(annotation, "F")
        .as_integer()
        .unwrap_or_default();
    let flags = view
        .flags
        .map_or(stated, |change| change.applied_to(stated));
    flags & FLAG_TOGGLE_NO_VIEW != 0
}

/// Whether the annotation states Table 170's `/D`, which is the appearance `/H /P` displays.
fn has_down(document: &Document, annotation: &Dictionary) -> bool {
    let appearances = document.get_key(annotation, "AP");
    appearances
        .as_dict()
        .is_some_and(|appearances| !document.get_key(appearances, "D").is_null())
}

/// The mark a press asks for, where the pointer is down on this annotation.
///
/// `None` for every annotation nothing is pressing, which is every annotation on every page this
/// program has ever drawn until a person holds a button down on one — and for `/H /N`, `/H /P`
/// and `/H /T`, which ask for no mark of their own.
fn pressed_mark(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
    rect: [f32; 4],
    has_down: bool,
) -> Option<Mark> {
    if view.appearance != crate::view::Appearance::Down {
        return None;
    }
    match highlight(document, annotation, has_down) {
        Highlight::None | Highlight::Push => None,
        Highlight::Invert => Some(Mark::Rectangle(rect)),
        Highlight::Outline => Some(Mark::Border {
            rect,
            width: crate::appearance::border_width(document, annotation),
        }),
    }
}

/// What §12.5.6.19's `/H` asks to be drawn over an annotation while it is pressed.
///
/// The clause states the effect as an arithmetic one — "for each colour channel in the colour
/// space used for display of the annotation value, colour values shall be transformed by the
/// function f(x) = 1 - x" — which §11.3.5.2's Difference mode is, against white: `B(cb, cs) =
/// |cb - cs|` with every component of the source at 1 leaves `1 - cb`. So both modes are one
/// white shape under one blend mode, and neither needs a new command in the display list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Mark {
    /// `I`: the annotation's rectangle, filled.
    Rectangle([f32; 4]),
    /// `O`: its border, stroked at the width §12.5.4 gives it.
    Border {
        /// The annotation's rectangle, in default user space.
        rect: [f32; 4],
        /// The border's width, from `/BS` `/W` or Table 166's `/Border`.
        width: f32,
    },
}

/// Table 176's and Table 191's `/H`: what a press does to an annotation.
///
/// Table 176 says the same of a link, with four modes rather than five and the same default.
/// (**The second number said 192 until the four-hundred-and-thirteenth session**, which is the
/// `/MK` appearance characteristics dictionary and states no `/H` at all; the widget
/// annotation's own entry is Table 191's. It is the third place in this tree to carry that
/// exact pair — §12.5.6.19's ledger row was corrected in the three-hundred-and-eighty-seventh
/// and `highlight`'s doc comment below in the four-hundred-and-second — and two more of them
/// were inside `highlight`'s own body, which the round that corrected the comment above it did
/// not read. Five places, one entry, three rounds: `doc/todo/01`'s ninth sweep's own subject.)
/// ISO 32000-2 §12.5.6.19, of a widget:
///
/// > The annotation's highlighting mode , the visual effect that shall be used when the mouse
/// > button is pressed or held down inside its active area: N (None) No highlighting. I (Invert)
/// > Invert the colours used to display the contents of the annotation rectangle. O (Outline)
/// > Stroke the colours used to display the annotation border.
///
/// A clause that describes a *moment*, and one this program could not reach until it grew a
/// pointer in the hundred-and-thirty-second session (ADR 0122).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Highlight {
    /// `N`: nothing extra is drawn.
    None,
    /// `I`: the contents of the annotation rectangle are inverted.
    Invert,
    /// `O`: the border is inverted.
    Outline,
    /// `P` and `T`: §12.5.5's down appearance, which the appearance selection already does.
    Push,
}

/// What §12.5.6.19's `/H` asks for, with the reading its default needs.
///
/// **The default is `I` and this applies it only where the annotation states no down
/// appearance.** The two clauses genuinely disagree for an annotation that states a `/D` and no
/// `/H`: §12.5.5 says the down appearance "shall be used when the mouse button is pressed", and
/// Table 191 gives `/H` the default `I` and says "[a] highlighting mode other than P shall
/// override any down appearance". (**This comment said "Table 192" until the
/// four-hundred-and-second session**, which is the number §12.5.6.19's ledger row was corrected
/// for fifteen rounds earlier and this file was not: 192 is the `/MK` appearance characteristics
/// dictionary, and `/H` is the widget annotation's own entry. `doc/todo/01`'s ninth sweep.) Taking the default flatly makes the *file's own artwork*
/// unshowable — 95 of the corpus's page-one annotations state a `/D` and no `/H`, so 95 pieces
/// of artwork would exist for a moment that could never display them. Taking a stated `/D` as
/// the writer having said `P` leaves every stated entry meaning something, and loses only a
/// default the file did not write.
///
/// A **stated** mode is honoured exactly, including the override: `/H /I` beside a `/D` inverts
/// and the down appearance is not used. The corpus cannot decide between the two readings —
/// every one of the four annotations stating both states `/H P` — so the argument has to.
pub(crate) fn highlight(document: &Document, annotation: &Dictionary, has_down: bool) -> Highlight {
    // **Two tables define `/H` and no others do**: Table 176's, on a link annotation (§12.5.6.5),
    // and Table 191's, on a widget (§12.5.6.19). Both give it the default `I`, and a default
    // belongs to the entry rather than to annotations in general — so a subtype whose clause
    // states no `/H` has no highlighting mode to default, and a press on it draws no mark.
    //
    // Reachable only from the two-hundred-and-fifty-third session, and that is the whole reason
    // it was not caught: `viewer-core` took the pressed annotation from the *link* one, so the
    // default could only ever land on a subtype that does define the entry. Widening the region
    // to every annotation is what made a `Square` invert under the cursor, in a test written for
    // a different flag.
    let subtype = document.get_key(annotation, "Subtype");
    let subtype = subtype.as_name().map(Name::as_bytes);
    if !matches!(subtype, Some(b"Link" | b"Widget")) {
        return Highlight::None;
    }
    let stated = document.get_key(annotation, "H");
    let Some(name) = stated.as_name() else {
        return if has_down {
            Highlight::Push
        } else {
            Highlight::Invert
        };
    };
    match name.as_bytes() {
        b"N" => Highlight::None,
        b"O" => Highlight::Outline,
        // T is "[s]ame as P (which is preferred)".
        b"P" | b"T" => Highlight::Push,
        // I, and anything Table 191 does not define: the clause's own default for an entry
        // whose value it does not recognise is the default value, which is I.
        _ => Highlight::Invert,
    }
}

/// The annotation subtypes ISO 32000-2 Table 171 defines.
///
/// Used only for the `Invisible` flag, whose meaning in Table 167 is conditional on the
/// subtype being outside this list: "applies only to annotations which do not belong to one
/// of the standard annotation types and for which no annotation handler is available".
const STANDARD_SUBTYPES: [&[u8]; 28] = [
    b"3D",
    b"Caret",
    b"Circle",
    b"FileAttachment",
    b"FreeText",
    b"Highlight",
    b"Ink",
    b"Line",
    b"Link",
    b"Movie",
    b"Polygon",
    b"PolyLine",
    b"Popup",
    b"PrinterMark",
    b"Projection",
    b"Redact",
    b"RichMedia",
    b"Screen",
    b"Sound",
    b"Square",
    b"Squiggly",
    b"Stamp",
    b"StrikeOut",
    b"Text",
    b"TrapNet",
    b"Underline",
    b"Watermark",
    b"Widget",
];

/// `/F` bit 1: render nothing for an unknown subtype with no handler.
const FLAG_INVISIBLE: i64 = 1;
/// `/F` bit 2: render nothing, whatever the subtype.
const FLAG_HIDDEN: i64 = 1 << 1;
/// `/F` bit 6: render nothing *on screen*. A viewer is a screen.
const FLAG_NO_VIEW: i64 = 1 << 5;
/// `/F` bit 7: respond to nothing. About interaction alone, and ignored for a widget.
const FLAG_READ_ONLY: i64 = 1 << 6;
/// `/F` bit 4: keep the annotation's size on the screen whatever the magnification is.
const FLAG_NO_ZOOM: i64 = 1 << 3;
/// `/F` bit 5: keep the annotation's orientation whatever §7.7.3.3's `/Rotate` says.
const FLAG_NO_ROTATE: i64 = 1 << 4;
/// `/F` bit 9: read [`FLAG_NO_VIEW`] the other way round while the pointer is on this annotation.
const FLAG_TOGGLE_NO_VIEW: i64 = 1 << 8;

/// How large a text annotation's icon is, in default user space at magnification 1.
///
/// **A choice, and one this module is the right place for**, since `crate::icon` is already the
/// one piece of pure invention in the tree: §12.5.6.4 requires "predefined icon appearances" for
/// seven standard names and draws none of them, and this is the same silence one question along
/// — how big. Twenty units is about the height of two lines of ten-point text, which is the size
/// at which a note's icon is legible beside the text it annotates and small enough not to cover
/// it.
///
/// The clause states no size and the file states none either — see [`anchored_icon`] for why
/// `/Rect` is not one — so this number answers a silence rather than a rule, and it is the size
/// of every text icon this program draws.
const ICON_SIZE: f32 = 20.0;

/// A square for a text annotation's synthesised icon, or `None` for any other subtype.
///
/// §12.5.6.4 opens with the sentence that decides this:
///
/// > A text annotation represents a "sticky note" attached to a point in the PDF document. When
/// > closed, the annotation shall appear as an icon
///
/// **Attached to a point**, and a `shall` about the icon appearing — so a `/Rect` with no area is
/// not this annotation stating that it covers nothing, the way a `Square`'s or a `Highlight`'s
/// would be. The same clause's next sentence gives the size:
///
/// > Text annotations shall not scale and rotate with the page; they shall behave as if the
/// > NoZoom and NoRotate annotation flags (see "Table 167 -Annotation flags") were always set.
///
/// and §12.5.3 says what that means:
///
/// > If the NoZoom flag is set, the annotation shall always maintain the same fixed size on the
/// > screen and shall be unaffected by the magnification level at which the page itself is
/// > displayed.
///
/// A fixed size on the screen is by definition not `/Rect`'s, because `/Rect` is stated in
/// default user space and every extent in that space scales with the magnification.
///
/// The corner is Table 167's own, in the `NoZoom` row: "The location of the annotation on the
/// page (defined by the upper-left corner of its annotation rectangle) shall remain fixed,
/// regardless of the page magnification" — so the square hangs down and to the right of it.
///
/// # Why this is not conditioned on the rectangle
///
/// **It was, for the two hundred and seventy-five sessions between the two-hundred-and-sixty-fifth
/// and the six-hundred-and-fortieth**, which drew the icon on the largest square inside `/Rect`
/// wherever `/Rect` had an area and reached this square only where it had none. The derivation
/// above never needed that condition: none of the three sentences mentions the rectangle's size,
/// and Table 166 does not give `/Rect` one either — it is "defining the location of the annotation
/// on the page in default user space units".
///
/// What supplies a size instead is §12.5.5's algorithm, which maps the coordinate system of the
/// appearance form dictionary "to the annotation's rectangle in default user space" by scaling its
/// transformed `/BBox` onto `/Rect`'s corners — and an annotation with no appearance stream has no
/// such box to map, which is why this function is reached from the branch where
/// [`Normal::Absent`] was answered and from nowhere else. `1407194.pdf`'s note states `/Rect [0 542 400 792]` with no `/AP`, and a 250-unit icon
/// covered the top-left quarter of a book cover: −6.304 of 255 against three references agreeing
/// within 0.6, every one of them drawing a small note at the corner.
///
/// §12.5.6.15's file attachment and §12.5.6.16's sound are deliberately not here: neither clause
/// states either sentence, so neither annotation is attached to a point and neither is held at a
/// fixed size, and [`crate::appearance::symbol_icon`] still inscribes those in `/Rect`.
///
/// `rc_annotation.pdf` is the one corpus witness — `/Subtype /Text /Name /Note /Rect
/// [50 50 50 50]` — and it drew nothing here for as long as anybody looked, inside an `ambiguous`
/// verdict, with `poppler` and `mupdf` drawing an icon and `ghostscript` and `hayro` drawing
/// nothing. Found by `doc/todo/00`'s step 7 sweep at −1.783 of 255, which is the only instrument
/// that sees a page this tree is not drawing.
fn anchored_icon(subtype: &[u8], rect: [f32; 4]) -> Option<[f32; 4]> {
    if subtype != b"Text" {
        return None;
    }
    let left = rect[0].min(rect[2]);
    let top = rect[1].max(rect[3]);
    Some([left, top - ICON_SIZE, left + ICON_SIZE, top])
}

/// Whether §12.5.3's `NoView` applies to this annotation *right now*.
///
/// Table 167, bit 9:
///
/// > If set, invert the interpretation of the NoView flag for annotation selection and mouse
/// > hovering, causing the annotation to be visible when the mouse pointer hovers over the
/// > annotation or when the annotation is selected.
///
/// So the flag is not a second suppression but a *pointer-dependent reading* of the first, and
/// the exclusive-or is the sentence: `NoView` alone hides, `NoView` and `ToggleNoView` together
/// hide until the cursor arrives, and `ToggleNoView` alone hides only while it is there.
///
/// Table 170's appearance is what "hovering" and "selected" are here. §12.5.5 defines the
/// rollover as "when the user moves the cursor into the annotation's active area without
/// pressing the mouse button" and the down appearance as the button held inside it, so anything
/// but [`Appearance::Normal`] is the cursor being on this annotation — which is the condition
/// this clause states in prose and that one states in a table.
///
/// **Unreachable before the two-hundred-and-fifty-third session**, and not because of this
/// clause: `viewer-core` took the annotation under the pointer from the *link* one, so no
/// annotation that was not a link ever left [`Appearance::Normal`].
/// §12.5.3's two flags that say "not on this screen": `Hidden`, and `NoView` as read by
/// [`no_view`].
///
/// **One statement of it rather than two**, which is why it is a function: `crate::popup` asks
/// the same question of a subtype [`decided`] answers `Nothing` for before it gets this far, and
/// two copies of a flag test are how a clause comes to be honoured on one path and not the other.
pub(crate) fn displayed(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> bool {
    let flags = stated_flags(document, annotation, view);
    !((flags & FLAG_HIDDEN != 0 && view.hidden_by_action != Some(false))
        || no_view(flags, view.appearance))
}

/// The annotation's `/F`, with whatever §12.7.8 or §12.6.4.11 has said about it applied.
///
/// §12.7.8's Table 249: an imported `/F` "shall replace that of the F entry in the form's
/// corresponding annotation dictionary", and `/SetF` and `/ClrF` modify it — so where an FDF file
/// has said something about this widget's flags, that is the answer and the file's `/F` is what
/// it was applied to.
fn stated_flags(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> i64 {
    let stated = document
        .get_key(annotation, "F")
        .as_integer()
        .unwrap_or_default();
    view.flags
        .map_or(stated, |change| change.applied_to(stated))
}

fn no_view(flags: i64, appearance: crate::view::Appearance) -> bool {
    let stated = flags & FLAG_NO_VIEW != 0;
    let inverted =
        flags & FLAG_TOGGLE_NO_VIEW != 0 && appearance != crate::view::Appearance::Normal;
    stated != inverted
}

/// What §12.5.3's `NoZoom` and `NoRotate` need to know about the view they are drawn into.
///
/// Both flags make an appearance's placement depend on something outside the file, which is why
/// they are carried in rather than read: the page's `/Rotate` is the file's, but the
/// magnification is the *reader's*, and `CLAUDE.md`'s rule 1 keeps interpretation a pure
/// function of the document and the view state.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewAdjust {
    /// The similarity §12.5.3 asks for, in default user space, or the identity.
    pub transform: Transform,
    /// Whether this annotation's placement depends on the *magnification* (§12.5.3's `NoZoom`).
    ///
    /// Reported even where no magnification was supplied, because that is exactly when a caller
    /// needs to know: it says "this page would look different if you told me the zoom", which is
    /// what makes a re-interpretation on zoom cost nothing on the 923 documents with no such
    /// annotation. `NoRotate` is *not* here — the page's `/Rotate` is in the file, so an
    /// annotation that only sets that flag is a pure function of the document as everything else
    /// is.
    pub view_dependent: bool,
}

impl Default for ViewAdjust {
    fn default() -> Self {
        Self {
            transform: Transform::IDENTITY,
            view_dependent: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ViewGeometry {
    /// §7.7.3.3's `/Rotate`, normalised to 0, 90, 180 or 270.
    pub rotate: u16,
    /// Logical pixels per default user space unit — 1.0 at 100%.
    ///
    /// `None` is **not** 1.0: it is *nobody has said*, which is what every caller that renders a
    /// page at its own scale means, and under it `NoZoom` changes nothing. Treating an unstated
    /// magnification as 100% would make the corpus and the oracle assert a zoom nobody chose.
    pub magnification: Option<f32>,
}

impl ViewGeometry {
    /// §12.5.3's adjustment for one annotation, about the upper-left corner of its `/Rect`.
    ///
    /// > If the NoZoom flag is set, the annotation shall always maintain the same fixed size on
    /// > the screen and shall be unaffected by the magnification level at which the page itself
    /// > is displayed. Similarly, if the NoRotate flag is set, the annotation shall retain its
    /// > original orientation on the screen when the page is rotated (by changing the Rotate
    /// > entry in the page object; see 7.7.3, "Page tree").
    ///
    /// and the clause names the fixed point in the sentence after:
    ///
    /// > In either case, the annotation's position is defined by the coordinates of the
    /// > upper-left corner of its annotation rectangle, as defined by the Rect entry in the
    /// > annotation dictionary and interpreted in the default user space of the page.
    ///
    /// So this is a similarity about that corner, expressed in default user space and composed
    /// *before* the page's own transform — which is what makes it undo that transform's rotation
    /// and scale rather than adding to them. The `/Rect` itself is untouched, as the clause
    /// requires: "it shall not actually change the annotation's Rect entry".
    fn adjustment(self, flags: i64, rect: Option<[f32; 4]>) -> ViewAdjust {
        let no_zoom = flags & FLAG_NO_ZOOM != 0;
        let no_rotate = flags & FLAG_NO_ROTATE != 0;
        let unchanged = ViewAdjust {
            transform: Transform::IDENTITY,
            view_dependent: no_zoom,
        };
        if !no_zoom && !no_rotate {
            return ViewAdjust::default();
        }
        // No rectangle is no fixed point, and §12.5.3 defines the adjustment entirely in terms
        // of one — so an annotation with no usable `/Rect` gets no adjustment rather than one
        // pivoted about the origin, which would move it off the page.
        let Some(rect) = rect else {
            return unchanged;
        };
        // "the upper-left corner of its annotation rectangle": smallest x, greatest y, since
        // §7.9.5 lets a rectangle state its corners in either order.
        let (corner_x, corner_y) = (rect[0].min(rect[2]), rect[1].max(rect[3]));

        // The page turns default user space clockwise by `/Rotate`, so keeping the annotation
        // upright means turning it the same amount the other way. Written as matrices rather
        // than through a trigonometric function for the reason `base_transform` is: these four
        // are exact and a cosine of 90 degrees is not.
        let rotation = if no_rotate {
            match self.rotate {
                // (x, y) -> (-y, x)
                90 => Transform::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0),
                180 => Transform::new(-1.0, 0.0, 0.0, -1.0, 0.0, 0.0),
                // (x, y) -> (y, -x)
                270 => Transform::new(0.0, -1.0, 1.0, 0.0, 0.0, 0.0),
                _ => Transform::IDENTITY,
            }
        } else {
            Transform::IDENTITY
        };
        // A magnification of 2 draws every user unit twice as large, so an annotation that is to
        // come out its stated size is drawn half as large in the space that gets magnified.
        let scale = match self.magnification.filter(|_| no_zoom) {
            Some(magnification) if magnification.is_finite() && magnification > 0.0 => {
                Transform::scale(1.0 / magnification, 1.0 / magnification)
            }
            _ => Transform::IDENTITY,
        };
        ViewAdjust {
            transform: Transform::translate(-corner_x, -corner_y)
                .then(rotation)
                .then(scale)
                .then(Transform::translate(corner_x, corner_y)),
            view_dependent: no_zoom,
        }
    }
}

/// Whether §12.5.3's flags let the pointer reach this annotation at all.
///
/// **Three of Table 167's bits say it, and each says it differently.** ISO 32000-2 §12.5.3, on
/// Hidden:
///
/// > If set, do not render the annotation or allow it to interact with the user, regardless of
/// > its annotation type or whether an annotation handler is available.
///
/// on `NoView`:
///
/// > If set, do not render the annotation on the screen or allow it to interact with the user.
///
/// and on `ReadOnly`, which is about **nothing else**:
///
/// > If set, do not allow the annotation to interact with the user. The annotation may be
/// > rendered or printed (depending on the settings of the NoView and Print flags) but should
/// > not respond to mouse clicks or change its appearance in response to mouse motions. This
/// > flag shall be ignored for widget annotations; its function is subsumed by the ReadOnly flag
/// > of the associated form field
///
/// That last exception is stated and applied: a widget's own `/F` bit 7 means nothing, and what
/// governs a widget is Table 226's field flag.
///
/// **Deliberately not [`decide`]**, which answers a different question. An annotation may render
/// nothing because it has no appearance stream and no clause states one, and still have an
/// activation region a pointer enters: §12.5.6.5's link is the standing example, and Table 197's
/// events belong to every annotation dictionary. Gating §12.6.3 on *whether ink appeared* would
/// switch off a document's own trigger events for the annotations that are only regions.
///
/// `Invisible` is not here, and Table 167 is why: it governs an annotation whose type is not one
/// this document defines, and it is about rendering an unknown subtype rather than interaction.
#[must_use]
pub(crate) fn interacts(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> bool {
    let stated = document
        .get_key(annotation, "F")
        .as_integer()
        .unwrap_or_default();
    let flags = view
        .flags
        .map_or(stated, |change| change.applied_to(stated));
    // §12.6.4.11's hide action clears the Hidden bit for the session, exactly as it does for
    // rendering; `decide` states the argument.
    if flags & FLAG_HIDDEN != 0 && view.hidden_by_action != Some(false) {
        return false;
    }
    // **`NoView` and `ToggleNoView` together do not suppress interaction**, and that is a
    // derivation rather than a reading of a second sentence. Table 167 states the pair's effect
    // as "causing the annotation to be visible when the mouse pointer hovers over the
    // annotation" — an effect conditioned on the hover being *noticed*. An annotation the
    // pointer cannot land on never leaves `Appearance::Normal`, so `NoView` would never be
    // inverted and bit 9 could not mean anything at all. What the pair asks for is an annotation
    // that appears under the cursor, which is exactly an annotation whose region is live.
    if flags & FLAG_NO_VIEW != 0 && flags & FLAG_TOGGLE_NO_VIEW == 0 {
        return false;
    }
    let widget = document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|subtype| subtype.as_bytes() == b"Widget");
    flags & FLAG_READ_ONLY == 0 || widget
}

/// Decides what, if anything, an annotation contributes to the page.
///
/// `view` carries what the viewer's state says about this annotation and nothing about the
/// document; [`crate::view::AnnotationView::default`] is what every annotation gets in a
/// document nothing has interacted with, and produces the file's own answer to all four
/// questions.
///
/// `hidden_by_action` being `Some(false)` is §12.6.4.11's hide action with `/H false`. The
/// clause makes the two the same bit — the action "hides or shows one or more annotations on
/// the screen by setting or clearing their Hidden flags" — so an action that clears it beats
/// what the file wrote, and this program clears it for the session rather than in the file.
/// `NoView` is deliberately *not* covered: Table 167 makes it a separate bit about the device
/// rather than about the annotation's state, and §12.6.4.11 names only the Hidden flag.
pub(crate) fn decide(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
    geometry: ViewGeometry,
) -> Decision {
    let mut decision = decided(document, annotation, view);
    // §12.5.3's two view-dependent flags, applied once and to the whole annotation. Read here
    // rather than inside each construction because the clause's own sentence is about the
    // annotation rather than about its appearance, and because the fixed point it pivots about
    // is `/Rect`'s corner, which both paths already have.
    if let Decision::Draw { adjust, .. } = &mut decision {
        let stated = document
            .get_key(annotation, "F")
            .as_integer()
            .unwrap_or_default();
        let mut flags = view
            .flags
            .map_or(stated, |change| change.applied_to(stated));
        // §12.5.6.4, of a text annotation and of nothing else:
        //
        // > Text annotations shall not scale and rotate with the page; they shall behave as if
        // > the NoZoom and NoRotate annotation flags (see "Table 167 -Annotation flags") were
        // > always set.
        //
        // A `shall` about the *subtype* rather than about the file's `/F`, so it is applied to
        // an annotation that sets neither flag and cannot be cleared by one that clears them.
        // Unreachable until the two-hundred-and-seventeenth session gave the two flags a
        // meaning, and `icon.rs`'s module comment carried the blocker in prose the whole time.
        let subtype = document.get_key(annotation, "Subtype");
        let subtype = subtype.as_name().map(Name::as_bytes);
        if subtype == Some(b"Text".as_slice()) {
            flags |= FLAG_NO_ZOOM | FLAG_NO_ROTATE;
        }
        // §12.5.6.10, of the four text markup subtypes and of nothing else:
        //
        // > Text markup annotations shall appear as highlights, underlines, strikeouts (all
        // > PDF 1.3), or jagged ("squiggly") underlines ( PDF 1.4 ) in the text of a document.
        //
        // **Two `shall`s that cannot both hold, and the standard states no precedence.** At any
        // magnification but 1, §12.5.3's "the annotation shall always maintain the same fixed
        // size on the screen" moves a strike-out off the words it strikes out, and §12.5.6.10's
        // "in the text of a document" is then false. Table 182 is the second half of it: the
        // quadrilaterals are stated "in default user space" and each "shall encompasses a word
        // or group of contiguous words in the text underlying the annotation", so this
        // annotation's geometry is defined *by reference to the page's text* and cannot be held
        // still while the text moves.
        //
        // The choice, recorded as a choice (ADR 0172): §12.5.6.10 says what the annotation *is*
        // and §12.5.3 offers a display option that annotations have in general, so the general
        // option does not get to make the object stop being what its own subclause defines it
        // as. Counted before it was made: the corpus holds 511 text markup annotations across 34
        // documents, **211 of them carry `NoZoom` and all 211 are in one document** — every
        // strike-out of `ISO_32000-2_sponsored_EC3.pdf`, at one flag value, which is a
        // producer's habit rather than 211 decisions.
        if matches!(
            subtype,
            Some(b"Highlight" | b"Underline" | b"Squiggly" | b"StrikeOut")
        ) {
            flags &= !(FLAG_NO_ZOOM | FLAG_NO_ROTATE);
        }
        *adjust = geometry.adjustment(flags, rectangle(document, annotation, "Rect"));
    }
    decision
}

/// [`decide`] without §12.5.3's view-dependent flags, which it applies to whatever this returns.
fn decided(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> Decision {
    let subtype = document
        .get_key(annotation, "Subtype")
        .as_name()
        .map(|name| name.as_bytes().to_vec())
        .unwrap_or_default();

    // §12.5.6.14: a popup is the window belonging to some *other* annotation, opened in
    // response to that one. It is never part of the page's own rendering, so its absence
    // is not a gap.
    if subtype == b"Popup" {
        return Decision::Nothing;
    }

    if !displayed(document, annotation, view) {
        return Decision::Nothing;
    }
    let flags = stated_flags(document, annotation, view);
    if flags & FLAG_INVISIBLE != 0 && !STANDARD_SUBTYPES.contains(&subtype.as_slice()) {
        return Decision::Nothing;
    }

    // Table 166 makes `/Subtype` required, and a report that begins with an empty name reads as
    // a colon with nothing before it — which `issue7446.pdf` produced for four sessions.
    // Naming the absence is the whole of the fix: no subtype means no subtype clause, and a
    // clause is what every construction in `crate::appearance` reads.
    let name = if subtype.is_empty() {
        "an annotation with no /Subtype".to_owned()
    } else {
        String::from_utf8_lossy(&subtype).into_owned()
    };
    let stated_rect = rectangle(document, annotation, "Rect");

    // §12.5.6.6, and it is the one place a *reader's* statement outranks a stored appearance
    // stream. Table 177 makes the file's own `/AP` decisive over its `/DA` — "[t]he annotation
    // dictionary's AP entry, if present, shall take precedence over the DA entry" — which is a
    // precedence between two things the **file** says about the same text. A person who retyped
    // the note has changed that text, so the stored stream no longer describes this annotation at
    // all, and §12.5.6.6 states where its appearance comes from instead: "12.7.4.3, 'Variable
    // text', describes the process of using these entries to generate the appearance of the text
    // in these annotations". The construction is that process, so the stream is set aside and the
    // appearance is generated.
    if view.contents.is_some() && subtype == b"FreeText" {
        return match stated_rect {
            Some(rect) if !is_empty(rect) => {
                construct(document, annotation, &subtype, &name, rect, view)
            }
            Some(_) => Decision::Nothing,
            None => Decision::Unsupported(format!("{name}: no usable /Rect")),
        };
    }

    let stored = match stored_appearance(document, annotation, view) {
        Normal::Stream(stream) => stream,
        Normal::Absent => {
            // With no stored stream there is nothing whose own box could stand in for a missing
            // rectangle, and every construction in `crate::appearance` is written into one. So
            // this is where a `/Rect` Table 166 makes required is still refused.
            let Some(rect) = stated_rect else {
                return Decision::Unsupported(format!("{name}: no usable /Rect"));
            };
            let rect = match anchored_icon(&subtype, rect) {
                Some(square) => square,
                None if is_empty(rect) => return Decision::Nothing,
                None => rect,
            };
            return construct(document, annotation, &subtype, &name, rect, view);
        }
        Normal::StateNotDefined => return Decision::Nothing,
    };

    let damaged = appearance_damage(document, &stored, &name);

    // §12.5.5's algorithm maps the appearance's transformed bounding box onto `/Rect`, and the
    // two are the same kind of thing: a box in a coordinate space. **A missing operand makes
    // the map the identity, whichever operand it is.** The hundred-and-twenty-fifth session
    // established that in one direction — no `/BBox`, so §12.7.4.3's default box, which is
    // `/Rect`'s dimensions at the origin (ADR 0113) — and this is the same rule the other way:
    // no `/Rect`, so the appearance's own box, mapped through its `/Matrix`, is where it goes.
    //
    // `issue14438.pdf` is the witness, and what it settles is which entry to *name*: four of its
    // ink annotations state no `/Rect` at all and appearance streams whose `/BBox` is
    // `[0 0 0 0]`, so the file has stated an appearance covering no area. That draws nothing by
    // the file's own arithmetic, which is Table 166's excuse rather than a gap — and reporting a
    // missing rectangle for it named the one entry that could not have changed the picture.
    let matrix = matrix(document, &stored.dict);
    let stated_bbox = rectangle(document, &stored.dict, "BBox");
    let rect = match (stated_rect, stated_bbox) {
        (Some(rect), _) => rect,
        (None, Some(bbox)) => transformed(bbox, matrix),
        (None, None) => return Decision::Unsupported(format!("{name}: no usable /Rect")),
    };
    // An annotation covering no area cannot show anything, whether its appearance is stored or
    // constructed — and Table 166 excuses a writer from supplying one for exactly that shape.
    if is_empty(rect) {
        return Decision::Nothing;
    }

    // §8.10.2 makes `/BBox` required of a form `XObject`, and §12.5.5's algorithm starts by
    // transforming it — so a stream without one states no box to map onto `/Rect`, and the
    // only transform left is the identity.
    //
    // The box used instead is the one the standard itself states for an appearance stream, in
    // the one place it states any: §12.7.4.3, on the form dictionary a processor builds for a
    // field's appearance.
    //
    // > The lower-left corner of the bounding box ( BBox ) is set to coordinates (0, 0) in the
    // > form coordinate system. The box's top and right coordinates are taken from the
    // > dimensions of the annotation rectangle (the Rect entry in the widget annotation
    // > dictionary).
    //
    // That sentence is written for a *constructed* appearance and is applied here to a stored
    // one that omits the entry, which is an extension of it rather than a reading — recorded as
    // such, and reported, because §8.10.2 makes `/BBox` required and a stream whose marks lie
    // outside this box will still draw nothing. What makes it the right extension is that the
    // alternative is a refusal, and a refusal throws away the whole annotation — background,
    // border and all — over an entry §12.5.5 needs only for a *scale*.
    //
    // `checkbox-bad-appearance.pdf` is the corpus's one witness: its check box's `/AP` draws
    // `(4)` in ZapfDingbats at `0 0 Td`, which under this box is the tick's own corner and
    // under any other reading is the corner of the page.
    let bbox = stated_bbox.unwrap_or([0.0, 0.0, rect[2] - rect[0], rect[3] - rect[1]]);
    let missing_bbox = stated_bbox
        .is_none()
        .then(|| format!("{name}: appearance stream has no /BBox"));

    // §12.7.4.3 by way of Table 224's `/NeedAppearances`: for the field types whose text is
    // "not known until viewing time", the writer has said the stored stream may not match the
    // value, so the stream's `/Tx` marked-content region is rewritten in place. Everything
    // outside it is the file's own artwork and stays, which is what makes this a splice rather
    // than a second construction — see `crate::appearance::regenerate`.
    let mut owed = missing_bbox;
    let mut content = Content::Stored(Arc::clone(&stored));
    if crate::appearance::regenerates(document, annotation, &subtype, view.value)
        && let Some(regenerated) =
            crate::appearance::regenerate(document, annotation, &stored, bbox, view.value)
    {
        owed = owed.or_else(|| regenerated.report.map(|detail| format!("{name}: {detail}")));
        content = Content::Constructed {
            bytes: regenerated.content,
            resources: regenerated.resources,
        };
    }

    Decision::Draw {
        adjust: ViewAdjust::default(),
        highlight: pressed_mark(
            document,
            annotation,
            view,
            rect,
            has_down(document, annotation),
        ),
        appearance: Box::new(Appearance {
            transform: placement(bbox, matrix, rect),
            bbox: Some(bbox),
            // §12.5.2 and Table 166: a stored stream states its own *transparency*, so the
            // annotation's `/ca` and `/CA` are not applied to it — and a regenerated one is
            // still that stream, with one region of its marks rewritten. `/BM` is the entry
            // that is *not* like those two; see [`blend_mode`].
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend: blend_mode(document, annotation),
            content,
            damaged,
        }),
        owed,
    }
}

/// Constructs an appearance for an annotation that has none, and places it.
///
/// The constructed stream is written in the page's own default user space, so its `/BBox` *is*
/// the annotation's `/Rect` and §12.5.5's algorithm reduces to the identity — which is worth
/// stating because it is the reason this path needs no second placement rule. The `/BBox` still
/// clips, which is §8.10.2 doing what §12.5.5 relies on: an appearance is "a self-contained
/// content stream that shall be rendered inside the annotation rectangle".
fn construct(
    document: &Document,
    annotation: &Dictionary,
    subtype: &[u8],
    name: &str,
    rect: [f32; 4],
    view: crate::view::AnnotationView<'_>,
) -> Decision {
    // §12.5.6.14: a popup is the window belonging to some *other* annotation, and §12.5.6.24's
    // projection is a measurement inside an activated 3D model — clause 13, which principle 5
    // excludes. Table 166 names both, with `Link`, as the subtypes a writer need not give an
    // appearance dictionary at all.
    if subtype == b"Projection" {
        return Decision::Nothing;
    }

    let constructed = crate::appearance::construct(document, annotation, subtype, view, rect);
    let owed = constructed.report.map(|detail| format!("{name}: {detail}"));
    let Some(content) = constructed.content else {
        return match owed {
            Some(detail) => Decision::Unsupported(detail),
            None => Decision::Nothing,
        };
    };

    // Table 166: `/CA` is the opacity for stroking "all visible elements of the annotation in
    // its closed state, including its background and border", and `/ca` the same for
    // nonstroking — "If a ca entry is not present in this dictionary, then the value of this CA
    // entry shall also be used for nonstroking operations as well."
    let stroke_alpha = opacity(document, annotation, "CA");
    Decision::Draw {
        adjust: ViewAdjust::default(),
        // A constructed appearance is one the file did not state, so there is no `/D` to have
        // meant `P` by.
        highlight: pressed_mark(document, annotation, view, rect, false),
        appearance: Box::new(Appearance {
            transform: Transform::IDENTITY,
            // §8.10.2's box is a *form XObject's*, and a construction is not one: the clause
            // that states this subtype's geometry is what decides whether `/Rect` bounds it.
            bbox: constructed.bounded.then_some(rect),
            fill_alpha: opacity(document, annotation, "ca")
                .or(stroke_alpha)
                .unwrap_or(1.0),
            stroke_alpha: stroke_alpha.unwrap_or(1.0),
            blend: blend_mode(document, annotation),
            content: Content::Constructed {
                bytes: content,
                resources: constructed.resources,
            },
            // Written here from the annotation's own entries, so there is no stream that could
            // have been short of anything.
            damaged: None,
        }),
        owed,
    }
}

/// Where a stored appearance stream stopped decoding, if it did (§7.8.2, §12.5.5).
///
/// §12.5.5 says what the thing is, and that is the whole argument:
///
/// > Each appearance stream is a form XObject (see 8.10, "Form XObjects"): a self-contained
/// > content stream that shall be rendered inside the annotation rectangle.
///
/// So §7.8.2's rule reaches it: a prefix of a sequence of instructions is a shorter sequence of
/// the same kind, the prefix is drawn, and the shortfall is named. Asked *here*, at the only
/// point that still holds the stream — §12.7.4.3's regeneration replaces the appearance's content
/// with a spliced copy of these bytes, so a report taken where the drawn stream is decoded would
/// go quiet for exactly the fields whose variable text a reader has changed.
fn appearance_damage(
    document: &Document,
    stored: &Stream,
    name: &str,
) -> Option<crate::content::DamagedStream> {
    let detail = format!("a {name} annotation's appearance stream (§12.5.5)");
    // Asked through the same source the drawing reads (ADR 0427), so that a stream the memo
    // keeps is decoded exactly once for both questions and one it declines — a bomb, always —
    // is *read* for this answer rather than materialised for it.
    let content =
        crate::content::reader::NestedContent::of(document, stored, detail.clone()).ok()?;
    let (damage, kept) = content.damage()?;
    Some(crate::content::DamagedStream {
        detail,
        damage,
        kept,
    })
}

/// Table 166's `/BM`, which applies to a stored appearance as much as to a constructed one.
///
/// **The one annotation entry that is not like `/CA` and `/ca`**, and the reason is worth the
/// paragraph because this tree read it the other way for four hundred sessions. Table 166 states
/// each of the two opacities as the value "[w]hen regenerating the annotation's appearance
/// stream" and adds outright that it "shall not be used if the annotation has an appearance
/// stream". §12.5.2 states `/BM` with no such condition:
///
/// > (Optional; PDF 2.0) The blend mode that shall be used when painting the annotation onto the
/// > page (see 11.3.5, "Blend Mode" and 11.6.3, "Specifying Blending Colour Space and Blend
/// > Mode"). If this key is not present, blending shall take place using the Normal blend mode.
///
/// Painting the annotation onto the page is what happens to a stored stream too. §12.5.5 says
/// the same from its own side — the appearance's group "shall be composited ... using the values
/// of the BM, ca and CA entries in the annotation dictionary" — and the only sentence against
/// either was §12.5.2's list of entries a reader ignores.
///
/// **Errata Collection 3 struck `BM` out of that list** (Issue #23 and #34, `/State` `Review`
/// `Completed`), so the sentence now reads "When rendering the appearance dictionary, a PDF
/// reader shall ignore the values of the C, IC, Border, BS, BE, CA, ca, H, DA, Q, DS, LE, LL,
/// MK, LLE, and Sy keys" — `CA` and `ca` kept, `BM` gone, `MK` added. `doc/md/` carries the
/// unamended sentence because the sponsored copy records EC3 as annotations and the conversion
/// dropped every one of them (ADR 0252, ADR 0253). Nothing now contradicts Table 166's `/BM`
/// row, so it is read on both paths.
fn blend_mode(document: &Document, annotation: &Dictionary) -> Option<String> {
    document
        .get_key(annotation, "BM")
        .as_name()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
}

/// Reads one of Table 166's opacity entries, clamped to the range it states.
fn opacity(document: &Document, annotation: &Dictionary, key: &'static str) -> Option<f32> {
    document
        .get_key(annotation, key)
        .as_number()
        .map(|value| narrow(value).clamp(0.0, 1.0))
}

/// What `/AP /N` resolved to.
///
/// The two failures are different in kind, and conflating them is what a check box costs:
/// an annotation with no `/AP` at all is a gap in this crate, while one whose `/AS` selects
/// a state the appearance dictionary does not define is a document saying "draw nothing" —
/// which is exactly how an unchecked box with only an `On` appearance is written.
enum Normal {
    /// A stream to run.
    Stream(Arc<Stream>),
    /// No `/AP`, no `/N`, or a `/N` that is neither a stream nor a state dictionary.
    Absent,
    /// `/N` is a state dictionary and `/AS` selected nothing from it.
    StateNotDefined,
}

/// Resolves the appearance the annotation is showing to a single stream.
///
/// §12.5.5: an entry of Table 170's appearance dictionary is "a single appearance stream or
/// an appearance subdictionary", and where it is a subdictionary the `/AS` entry chooses
/// among the states. The clause also names the behaviour when that choice fails — "PDF
/// processors shall also attempt to provide reasonable behaviour (such as displaying nothing)
/// if an annotation's AS entry designates an appearance state for which no appearance is
/// defined in the appearance dictionary" — so displaying nothing there is the specified
/// answer rather than a shortfall.
///
/// `view.appearance` is which of the three the pointer's position asks for
/// (`crate::view::Appearance`). Table 170 requires only `/N` and makes `/R` and `/D` optional, so
/// an annotation with no entry for the state it is in has stated no special appearance for it and
/// shows its normal one — which is also what §12.5.3's *printing* path asks for, since "this
/// appearance is also used for printing the annotation".
///
/// **`view.value` is the second half of the choice, since the three-hundred-and-ninety-eighth
/// session.** §12.7.5.2.3 requires `/V` and `/AS` to agree, and a viewer that changes the first is
/// what has to carry the second: see `crate::appearance::appearance_state`, which answers `None`
/// for everything but a check box or radio button whose value this reader replaced.
fn stored_appearance(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> Normal {
    let showing = view.appearance;
    let appearances = document.get_key(annotation, "AP");
    let Some(appearances) = appearances.as_dict() else {
        return Normal::Absent;
    };
    let key = match showing {
        crate::view::Appearance::Normal => "N",
        crate::view::Appearance::Rollover => "R",
        crate::view::Appearance::Down => "D",
    };
    let normal = match document.get_key(appearances, key) {
        Object::Null => document.get_key(appearances, "N"),
        stated => stated,
    };

    if let Some(stream) = normal.as_stream() {
        return Normal::Stream(Arc::clone(stream));
    }
    let Some(states) = normal.as_dict() else {
        return Normal::Absent;
    };

    let selected =
        crate::appearance::appearance_state(document, annotation, view.value).or_else(|| {
            document
                .get_key(annotation, "AS")
                .as_name()
                .map(|name| name.as_bytes().to_vec())
        });
    // §12.5.5 keys `/AP`'s subdictionary by the appearance states the *file* names, so the
    // probe is §7.3.5's exact binary match on the bytes `/AS` states (ADR 0439).
    let resolved = selected
        .and_then(|name| states.get_by_name(&Name::new(name)).cloned())
        .map(|state| document.resolve(&state));
    match resolved.as_ref().and_then(|state| state.as_stream()) {
        Some(stream) => Normal::Stream(Arc::clone(stream)),
        None => Normal::StateNotDefined,
    }
}

/// The box a *stored* appearance's marks are written in, and §12.5.5's map from it to the page.
///
/// `None` where the file states no stream for the state the annotation is showing, which is the
/// case [`crate::appearance::construct`] answers instead — so the two answers here are the two
/// arms [`decided`] chooses between, asked by a caller that needs the *space* rather than the
/// picture. `crate::appearance::caret` is that caller, and it is why the box's two rules are
/// repeated in eight lines rather than shared: `decided` needs the stream, the report and the
/// two Decisions an absent stream can mean, and none of the three says where a point goes.
pub(crate) fn stored_frame(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> Option<([f32; 4], Transform)> {
    let Normal::Stream(stored) = stored_appearance(document, annotation, view) else {
        return None;
    };
    let matrix = matrix(document, &stored.dict);
    let stated_bbox = rectangle(document, &stored.dict, "BBox");
    // The same pair of defaults §12.5.5 is applied under in `decided`: a missing operand makes
    // the map the identity, whichever operand it is, and a stream with no `/BBox` gets
    // §12.7.4.3's — the annotation rectangle's dimensions at the origin.
    let rect = match (rectangle(document, annotation, "Rect"), stated_bbox) {
        (Some(rect), _) => rect,
        (None, Some(bbox)) => transformed(bbox, matrix),
        (None, None) => return None,
    };
    let bbox = stated_bbox.unwrap_or([0.0, 0.0, rect[2] - rect[0], rect[3] - rect[1]]);
    Some((bbox, placement(bbox, matrix, rect)))
}

/// Computes `AA`, the matrix ISO 32000-2 §12.5.5 defines.
///
/// The three steps of the clause, in order:
///
/// 1. Transform `/BBox` by `/Matrix` and take the smallest upright rectangle around the
///    result — the *transformed appearance box*. Taking the bounding box of the four
///    mapped corners is what makes this work for a rotating `/Matrix`, where the
///    quadrilateral is not upright and its extent is larger than the box it came from.
/// 2. Compute `A`, mapping that box's lower-left and upper-right corners onto `/Rect`'s.
/// 3. `AA = Matrix × A`.
///
/// A degenerate transformed box — zero width or height, which a `/BBox` of zero extent or a
/// singular `/Matrix` produces — has no scale onto `/Rect`. The scale is taken as 1 on that
/// axis, so the appearance is positioned rather than divided by zero into nothing.
fn placement(bbox: [f32; 4], matrix: Transform, rect: [f32; 4]) -> Transform {
    let corners = [
        Point::new(bbox[0], bbox[1]),
        Point::new(bbox[2], bbox[1]),
        Point::new(bbox[2], bbox[3]),
        Point::new(bbox[0], bbox[3]),
    ]
    .map(|corner| matrix.apply(corner));

    let (mut low, mut high) = (corners[0], corners[0]);
    for corner in corners {
        low = Point::new(low.x.min(corner.x), low.y.min(corner.y));
        high = Point::new(high.x.max(corner.x), high.y.max(corner.y));
    }

    let span = |from: f32, to: f32, onto: f32| {
        let extent = to - from;
        if extent.abs() > 1e-9 {
            onto / extent
        } else {
            1.0
        }
    };
    let scale = Transform::scale(
        span(low.x, high.x, rect[2] - rect[0]),
        span(low.y, high.y, rect[3] - rect[1]),
    );

    let align = Transform::translate(-low.x, -low.y)
        .then(scale)
        .then(Transform::translate(rect[0], rect[1]));
    matrix.then(align)
}

/// Whether a rectangle covers no area, in either axis.
fn is_empty(rect: [f32; 4]) -> bool {
    rect[2] - rect[0] <= 0.0 || rect[3] - rect[1] <= 0.0
}

/// The axis-aligned box around a box a transform has moved — §12.5.5's step 1.
pub(crate) fn transformed(box_: [f32; 4], matrix: Transform) -> [f32; 4] {
    let corners = [
        Point::new(box_[0], box_[1]),
        Point::new(box_[2], box_[1]),
        Point::new(box_[2], box_[3]),
        Point::new(box_[0], box_[3]),
    ]
    .map(|corner| matrix.apply(corner));
    let (mut low, mut high) = (corners[0], corners[0]);
    for corner in corners {
        low = Point::new(low.x.min(corner.x), low.y.min(corner.y));
        high = Point::new(high.x.max(corner.x), high.y.max(corner.y));
    }
    [low.x, low.y, high.x, high.y]
}

/// Reads a `/Matrix` entry, defaulting to the identity.
pub(crate) fn matrix(document: &Document, dict: &Dictionary) -> Transform {
    let Some(values) = numbers(document, dict, "Matrix") else {
        return Transform::IDENTITY;
    };
    let at = |index: usize| values.get(index).copied().unwrap_or_default();
    if values.len() < 6 {
        return Transform::IDENTITY;
    }
    Transform::new(at(0), at(1), at(2), at(3), at(4), at(5))
}

/// Reads a rectangle entry, normalised so the first corner is the lower left.
///
/// §7.9.5 describes a rectangle as four numbers giving "a pair of diagonally opposite
/// corners", and says the lower-left-then-upper-right order is only *typical*. It states no
/// requirement to normalise, and an earlier version of this comment attributed one to it.
///
/// The requirement is §12.5.5's, inside the algorithm that places an appearance, and it is
/// stated as a property of the corners rather than as an operation on the array:
///
/// > A maps the lower-left corner (the corner with the smallest x and y coordinates) and the
/// > upper-right corner (the corner with the greatest x and y coordinates) of the transformed
/// > appearance box to the corresponding corners of the annotation's rectangle.
///
/// A `/Rect` written the other way round is common enough that not normalising means
/// annotations placed off the page.
pub(crate) fn rectangle(
    document: &Document,
    dict: &Dictionary,
    key: &'static str,
) -> Option<[f32; 4]> {
    let values = numbers(document, dict, key)?;
    let at = |index: usize| values.get(index).copied().unwrap_or_default();
    if values.len() < 4 || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    Some([
        at(0).min(at(2)),
        at(1).min(at(3)),
        at(0).max(at(2)),
        at(1).max(at(3)),
    ])
}

/// Reads an array of numbers, resolving each entry.
pub(crate) fn numbers(
    document: &Document,
    dict: &Dictionary,
    key: &'static str,
) -> Option<Vec<f32>> {
    Some(
        document
            .get_key(dict, key)
            .as_array()?
            .iter()
            .filter_map(|item| document.resolve(item).as_number().map(narrow))
            .collect(),
    )
}

fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a coordinate outside f32's range cannot place anything on a page"
    )]
    {
        value as f32
    }
}

#[cfg(test)]
mod tests {
    use pdf_render::{Transform, geom::Point};

    /// An appearance whose `/BBox` already matches its `/Rect` must not be moved.
    ///
    /// The identity case, and the one that catches a translation applied in the wrong
    /// direction — which a case starting at the origin cannot, because there both
    /// directions agree.
    #[test]
    fn an_appearance_already_the_size_of_its_rectangle_is_only_translated() {
        let placement = super::placement(
            [0.0, 0.0, 100.0, 50.0],
            Transform::IDENTITY,
            [200.0, 300.0, 300.0, 350.0],
        );
        let corner = placement.apply(Point::new(0.0, 0.0));
        assert!((corner.x - 200.0).abs() < 1e-3, "{corner:?}");
        assert!((corner.y - 300.0).abs() < 1e-3, "{corner:?}");
        let far = placement.apply(Point::new(100.0, 50.0));
        assert!((far.x - 300.0).abs() < 1e-3, "{far:?}");
        assert!((far.y - 350.0).abs() < 1e-3, "{far:?}");
    }

    /// A `/BBox` away from the origin is translated to the rectangle, not scaled by its
    /// own offset.
    #[test]
    fn a_bounding_box_away_from_the_origin_lands_on_the_rectangle() {
        let placement = super::placement(
            [10.0, 20.0, 30.0, 60.0],
            Transform::IDENTITY,
            [0.0, 0.0, 40.0, 80.0],
        );
        let low = placement.apply(Point::new(10.0, 20.0));
        assert!(low.x.abs() < 1e-3 && low.y.abs() < 1e-3, "{low:?}");
        let high = placement.apply(Point::new(30.0, 60.0));
        assert!((high.x - 40.0).abs() < 1e-3, "{high:?}");
        assert!((high.y - 80.0).abs() < 1e-3, "{high:?}");
    }

    /// A rotating `/Matrix` is measured by the box around the rotated quadrilateral.
    ///
    /// This is step 1 of the algorithm and the only reason it is a *quadrilateral* rather
    /// than a rectangle. A 45° rotation makes a 100×100 box span 141.42 on both axes, so
    /// scaling to a 100×100 `/Rect` must shrink it by that ratio — not leave it at 1, which
    /// is what using the untransformed `/BBox` extent would give, and which would push half
    /// the appearance outside the annotation.
    #[test]
    fn a_rotating_matrix_is_measured_after_it_rotates() {
        let root_half = std::f32::consts::FRAC_1_SQRT_2;
        let rotate = Transform::new(root_half, root_half, -root_half, root_half, 0.0, 0.0);
        let placement =
            super::placement([0.0, 0.0, 100.0, 100.0], rotate, [0.0, 0.0, 100.0, 100.0]);

        // Every corner of the `/BBox` must land inside the rectangle, and the extremes must
        // land on its edges.
        let mapped = [
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
        ]
        .map(|corner| placement.apply(corner));
        for point in mapped {
            assert!(
                (-1e-3..=100.001).contains(&point.x) && (-1e-3..=100.001).contains(&point.y),
                "{point:?} escaped the annotation rectangle"
            );
        }
        let widest = mapped.iter().map(|point| point.x).fold(f32::MIN, f32::max)
            - mapped.iter().map(|point| point.x).fold(f32::MAX, f32::min);
        assert!((widest - 100.0).abs() < 1e-2, "{widest}");
    }

    /// A `/BBox` with no area is positioned rather than divided by zero.
    #[test]
    fn a_degenerate_bounding_box_does_not_produce_infinities() {
        let placement = super::placement(
            [5.0, 5.0, 5.0, 5.0],
            Transform::IDENTITY,
            [0.0, 0.0, 10.0, 10.0],
        );
        let point = placement.apply(Point::new(5.0, 5.0));
        assert!(point.x.is_finite() && point.y.is_finite(), "{point:?}");
    }
}
