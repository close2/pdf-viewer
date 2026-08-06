//! Text and panels this program draws for itself.
//!
//! Everything here is **this host's own chrome**, and none of it belongs in `viewer-core`. A
//! native host draws §12.3.3's outline in a `QTreeView`, an `NSOutlineView` or a `GtkListView`,
//! with the platform's fonts, its focus ring and its scrollbars, and it gets the data from
//! [`viewer_core::Query::Outline`]. This program has no toolkit — winit is a window and an event
//! loop and nothing else — so it draws the panels itself, and that is the whole reason this
//! module exists.
//!
//! # Three lists, one shape
//!
//! [`Sidebar`] shows one of four things a document says about itself, chosen by a tab:
//! §12.3.3's outline, §8.11.4.3's `/Order` of optional content groups, §7.11.4's embedded files,
//! and §14.3.3's document information dictionary. They are one piece of code because they are
//! one shape — indented rows with a label, a marker at the left and something a click does — and
//! because the differences between them are the interesting part: an outline row *navigates*, a
//! layer row is a **switch** the clause may forbid, an attachment row hands a file over, and a
//! property is a statement with nothing to click.
//!
//! # What it is drawn with
//!
//! A [`pdf_render::DisplayList`] and [`pdf_font::LoadedFont::standard`], which means the panel
//! goes to whichever backend drew the page and looks the same on a machine with no fonts
//! installed. That is not a trick: §9.6.2.2 says the fourteen "shall be available to the PDF
//! processor", and since the hundred-and-forty-eighth session they are available as bytes in the
//! binary (ADR 0133), so an interface drawn in Helvetica is drawn in the same Helvetica
//! everywhere.
//!
//! # Coordinates
//!
//! Device pixels of the window, **y downwards**, which is the raster's space and not the page's
//! (trap 12a). The display list is handed to a [`pdf_render::TargetSpec`] whose transform is the
//! identity, so nothing here composes a page transform and nothing here has to think about
//! §7.7.3.3's rotation. Glyph outlines arrive from `pdf-font` in font units with y *upwards*, and
//! [`Chrome::text`] is the one place that flip happens.

use std::sync::Arc;

use pdf_render::{Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Transform};
use pdf_syntax::ObjectId;
use viewer_core::Layer;

/// How wide the panel is, in logical pixels.
///
/// A choice, and the only rule behind it is that an outline's titles are sentences: §12.3.3's
/// `/Title` is "the text that shall be displayed on the screen for this item", and documents
/// write whole clause headings there. Narrower than this and the specification's own outline is
/// all ellipsis; wider and the page it is beside stops being the thing on the screen. It grew
/// from 260 in the hundred-and-seventy-third session, when a fourth tab arrived and the strip
/// stopped fitting.
const PANEL_WIDTH: f32 = 300.0;

/// How tall one row is, as a multiple of the text size.
///
/// Wide enough that a click lands on the row a person aimed at, which is the only requirement a
/// list of clickable rows has.
const ROW_HEIGHT: f32 = 1.7;

/// The text size in logical pixels.
const TEXT_SIZE: f32 = 12.0;

/// How far one level of nesting indents, in logical pixels.
const INDENT: f32 = 14.0;

/// How much of the row's left edge the marker owns, in logical pixels.
///
/// A click inside it works the marker — discloses a subtree, throws a switch — instead of doing
/// what the rest of the row does. That is what every list of this shape does, and for the
/// outline it is what §12.3.3's `/Count` sign makes meaningful: a negative count is a closed
/// item, and closed items are the ones with something to disclose.
const MARKER: f32 = 14.0;

/// How tall a row carrying §12.3.4's thumbnail is, in row heights.
///
/// A choice: seven row heights is about 140 logical pixels of picture, which shows a portrait
/// page's miniature at roughly the size a producer writes one — Table 87's examples are a few
/// score samples on a side — with a line of text under it for the page's label.
const THUMBNAIL_UNITS: usize = 7;

/// How tall the tab strip is, in logical pixels.
const TABS: f32 = TEXT_SIZE * ROW_HEIGHT + 6.0;

/// The panel's background.
const BACKGROUND: Color = Color {
    r: 0.94,
    g: 0.94,
    b: 0.95,
    a: 1.0,
};

/// The line separating the panel from the page, and the marker's ink.
const EDGE: Color = Color {
    r: 0.78,
    g: 0.78,
    b: 0.80,
    a: 1.0,
};

/// The row under the pointer, and the tab in front.
const HOVER: Color = Color {
    r: 0.86,
    g: 0.88,
    b: 0.93,
    a: 1.0,
};

/// Text that states something rather than naming something: a media type, a size, "no layers".
const DIMMED: Color = Color {
    r: 0.42,
    g: 0.42,
    b: 0.45,
    a: 1.0,
};

/// How wide the box standing for a character §9.6.2.2's fourteen state no code for is, in ems.
///
/// **A choice, and it is this host's own** — the standard says nothing about an interface's own
/// text. The argument for the number is that §9.6.2.2's Courier advances *every* code by exactly
/// 0.6 em: a placeholder claims nothing about the character it stands for, so the one width the
/// fourteen state for everything is the honest width to give it. See [`Chrome::set`] for why
/// there is a box at all.
const MISSING_WIDTH: f32 = 0.6;

/// How tall that box is, in ems: a capital's height, so a run of them reads as a run of text.
const MISSING_HEIGHT: f32 = 0.62;

/// How thick its edge is, in ems.
const MISSING_EDGE: f32 = 0.06;

/// The fonts this program draws its own text with.
///
/// Four faces rather than one because §12.3.3's Table 152 gives an outline item a `/F` whose two
/// low bits are italic and bold — "[a] set of flags specifying style characteristics for
/// displaying the outline item's text" — so a document that asks for bold gets bold. Loaded once
/// and held: `LoadedFont` caches its own outlines, and re-deriving Liberation Sans' glyphs per
/// frame would be the one avoidable cost on a path that runs at pointer speed.
pub struct Chrome {
    /// Regular, bold, italic, bold italic — the order [`pdf_font::standard::face`] holds the
    /// same four in.
    faces: [pdf_font::LoadedFont; 4],
    /// Courier, for text whose *columns* mean something.
    ///
    /// `/NOTICE` is wrapped and aligned as characters, so setting it in a proportional face
    /// would turn its licence texts into ragged prose — and both licences it carries oblige a
    /// binary to reproduce them. §9.6.2.2 supplies a fixed-pitch face for exactly this.
    mono: pdf_font::LoadedFont,
}

impl std::fmt::Debug for Chrome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chrome").finish_non_exhaustive()
    }
}

/// Which of the four faces a run of chrome text is set in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    /// Table 152's `/F` bit 2.
    pub bold: bool,
    /// Table 152's `/F` bit 1.
    pub italic: bool,
}

impl Chrome {
    /// Loads the four faces, or says which one §9.6.2.2 could not supply.
    ///
    /// # Errors
    ///
    /// The sentence `pdf-font` produced. A compiled-in face that will not parse is a defect in
    /// this build rather than in any document, and `pdf-font`'s own
    /// `every_compiled_in_face_parses` is what should catch it first — but a program that drew
    /// no chrome and said nothing would be trap 5 in its own interface.
    pub fn new() -> Result<Self, String> {
        let named = |name: &str| {
            pdf_font::LoadedFont::standard(name)
                .map_err(|error| format!("the interface's own {name}: {error}"))
        };
        Ok(Self {
            faces: [
                named("Helvetica")?,
                named("Helvetica-Bold")?,
                named("Helvetica-Oblique")?,
                named("Helvetica-BoldOblique")?,
            ],
            mono: named("Courier")?,
        })
    }

    /// One character's advance in the fixed-pitch face, at a size.
    ///
    /// A fixed pitch is a fact about the face rather than about a string, which is why this is
    /// one number and [`Self::width`] is a sum: §9.6.2.2's Courier advances every code 600
    /// thousandths of an em.
    #[must_use]
    pub fn mono_advance(&self, size: f32) -> f32 {
        self.mono
            .code_for('0')
            .map_or(size * 0.6, |code| self.mono.advance(code) * size)
    }

    /// Draws a line in the fixed-pitch face, with its baseline at `at.1`.
    pub fn mono_text(&self, list: &mut DisplayList, text: &str, at: (f32, f32), size: f32) {
        let mut x = at.0;
        for character in text.chars() {
            let Some(code) = self.mono.code_for(character) else {
                // A code the face does not map still advances: the notice's columns are what
                // this face is for, and skipping the cell would shift the rest of the line.
                x += self.mono_advance(size);
                continue;
            };
            if let Some(path) = self.mono.outline(code) {
                list.push(Command::Fill {
                    path,
                    transform: Transform {
                        a: size,
                        b: 0.0,
                        c: 0.0,
                        d: -size,
                        e: x,
                        f: at.1,
                    },
                    fill_rule: FillRule::NonZero,
                    paint: Paint::Solid(Color::BLACK),
                    clip: None,
                    mask: None,
                    blend: pdf_render::BlendMode::Normal,
                });
            }
            x += self.mono.advance(code) * size;
        }
    }

    /// The face for a style.
    ///
    /// Destructured rather than indexed: four faces and four combinations, so the match is total
    /// and there is no arm for a case that cannot happen.
    fn face(&self, style: Style) -> &pdf_font::LoadedFont {
        let [regular, bold, italic, bold_italic] = &self.faces;
        match (style.bold, style.italic) {
            (false, false) => regular,
            (true, false) => bold,
            (false, true) => italic,
            (true, true) => bold_italic,
        }
    }

    /// What one character costs a line of chrome, and what stands for it.
    ///
    /// The one place [`Self::text`] and [`Self::width`] agree about a character, so that a string
    /// cannot be measured one way and drawn another — which is what elision, wrapping and the
    /// popup's title bar all depend on.
    ///
    /// **A character the face states no code for used to draw nothing and advance nothing**, so a
    /// row of Japanese was an empty row and a person had been told the document says nothing
    /// (trap 5, and `doc/todo/27`). It gets a box: the standard states no artwork for an
    /// interface's own text, so this is a documented choice and not a reading, and it is the one
    /// every text engine makes for the same reason.
    fn set(face: &pdf_font::LoadedFont, character: char) -> (Set, f32) {
        if let Some(code) = face.code_for(character) {
            return (Set::Glyph(code), face.advance(code));
        }
        if character.is_whitespace() {
            // A space this face cannot spell — U+00A0 and U+3000 are the ones documents write —
            // is still a space, and a box in place of one would be a claim about a character
            // nobody can see. It takes the width of the space the face *does* state.
            let blank = face.code_for(' ').map_or(0.25, |code| face.advance(code));
            return (Set::Blank, blank);
        }
        if character.is_control() {
            // Nor is a control character something a producer meant a person to see: it has no
            // visible form to be missing, so a box would be saying something untrue rather than
            // saying nothing. One `/Info` value in the corpus carries one.
            //
            // **U+FFFD is deliberately not in this arm.** §7.9.2.2's undefined code point is
            // exactly a byte that represents no character, and `pdf_syntax::text_string`'s own
            // comment says a caller "reports it rather than dropping it silently" — the box is
            // that report. `bug1146106.pdf` writes its layer names as UTF-16 **little**-endian,
            // which is none of the clause's three encodings, so 51 characters of one name are
            // that case and the panel says so.
            return (Set::Blank, 0.0);
        }
        (Set::Missing, MISSING_WIDTH)
    }

    /// How wide a string is at a size, in the same pixels [`Self::text`] draws it in.
    ///
    /// Character for character with [`Self::text`], through [`Self::set`]: a placeholder box has
    /// a width because it is drawn, and a measurement that disagreed with the drawing would put
    /// every elision and every wrap in the wrong place.
    #[must_use]
    pub fn width(&self, text: &str, size: f32, style: Style) -> f32 {
        let face = self.face(style);
        text.chars()
            .map(|character| Self::set(face, character).1 * size)
            .sum()
    }

    /// How many of a string's characters this face states no code for.
    ///
    /// [`Self::text`] draws a box for one, which says *that* something is missing and cannot say
    /// how much; this is what lets a caller say how many — which is what §12.5.6.14's popup does
    /// under a note (ADR 0191).
    ///
    /// Counted through [`Self::set`], so it counts exactly the boxes: a blank and a control
    /// character are not among them, and a count that disagreed with the picture beside it would
    /// be worse than no count.
    #[must_use]
    pub fn without_a_code(&self, text: &str, style: Style) -> usize {
        let face = self.face(style);
        text.chars()
            .filter(|character| matches!(Self::set(face, *character).0, Set::Missing))
            .count()
    }

    /// Draws a string with its left edge at `at` and its **baseline** at `at.1`.
    ///
    /// Returns where the next run would start. The transform is the flip this module's header
    /// names: glyph outlines are y-up in font units and the panel is y-down in pixels, so the
    /// `d` component is negative and `f` is the baseline itself.
    pub fn text(
        &self,
        list: &mut DisplayList,
        text: &str,
        at: (f32, f32),
        size: f32,
        style: Style,
        colour: Color,
    ) -> f32 {
        let face = self.face(style);
        let mut x = at.0;
        for character in text.chars() {
            let (set, advance) = Self::set(face, character);
            match set {
                Set::Glyph(code) => {
                    if let Some(path) = face.outline(code) {
                        list.push(Command::Fill {
                            path,
                            transform: Transform {
                                a: size,
                                b: 0.0,
                                c: 0.0,
                                d: -size,
                                e: x,
                                f: at.1,
                            },
                            fill_rule: FillRule::NonZero,
                            paint: Paint::Solid(colour),
                            clip: None,
                            mask: None,
                            blend: pdf_render::BlendMode::Normal,
                        });
                    }
                }
                Set::Blank => {}
                Set::Missing => missing_box(list, (x, at.1), size, colour),
            }
            x += advance * size;
        }
        x
    }
}

/// What stands for one character on a line of chrome.
///
/// Three cases rather than two, because a space and a character with no glyph at all are
/// different silences: one of them is what the document meant.
#[derive(Debug, Clone, Copy)]
enum Set {
    /// The face states a code for it, and this is it.
    Glyph(pdf_font::Code),
    /// Nothing is drawn and the line still moves.
    Blank,
    /// [`missing_box`], because §9.6.2.2's fourteen have no code for it.
    Missing,
}

/// The box drawn for a character the interface's own font cannot set.
///
/// A hollow rectangle: one path with two rings and [`FillRule::EvenOdd`], which is the cheapest
/// way to draw an outline in a display list whose only primitive here is a fill. `at.1` is the
/// baseline and this module's y runs downwards, so the box grows *up* from it.
fn missing_box(list: &mut DisplayList, at: (f32, f32), size: f32, colour: Color) {
    let edge = MISSING_EDGE * size;
    let left = at.0 + edge;
    let right = at.0 + (MISSING_WIDTH - MISSING_EDGE) * size;
    let bottom = at.1;
    let top = at.1 - MISSING_HEIGHT * size;
    let mut path = Path::new();
    for ring in [
        [(left, top), (right, top), (right, bottom), (left, bottom)],
        [
            (left + edge, top + edge),
            (right - edge, top + edge),
            (right - edge, bottom - edge),
            (left + edge, bottom - edge),
        ],
    ] {
        for (index, (x, y)) in ring.into_iter().enumerate() {
            let point = pdf_render::Point { x, y };
            if index == 0 {
                path.push(PathCommand::MoveTo(point));
            } else {
                path.push(PathCommand::LineTo(point));
            }
        }
        path.push(PathCommand::Close);
    }
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::EvenOdd,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: pdf_render::BlendMode::Normal,
    });
}

/// A filled axis-aligned rectangle, in the panel's pixels.
fn rectangle(list: &mut DisplayList, (x, y, w, h): (f32, f32, f32, f32), colour: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    outline_of(
        list,
        &[(x, y), (x + w, y), (x + w, y + h), (x, y + h)],
        colour,
    );
}

/// A filled polygon, which is every shape this module draws that is not a glyph.
fn outline_of(list: &mut DisplayList, corners: &[(f32, f32)], colour: Color) {
    let mut path = Path::new();
    for (index, (x, y)) in corners.iter().enumerate() {
        let point = pdf_render::Point { x: *x, y: *y };
        if index == 0 {
            path.push(PathCommand::MoveTo(point));
        } else {
            path.push(PathCommand::LineTo(point));
        }
    }
    path.push(PathCommand::Close);
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: pdf_render::BlendMode::Normal,
    });
}

/// What a row's left edge carries, and what a click there does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marker {
    /// Nothing: a heading, or a row with no state of its own.
    None,
    /// §12.3.3's disclosure triangle, keyed by the item's pre-order number.
    Disclosure {
        /// Which outline item, in the numbering [`Sidebar::toggled`] uses.
        id: usize,
        /// Whether its children are showing.
        open: bool,
    },
    /// §8.11.2.1's on/off state of an optional content group.
    Switch {
        /// Whether the group is on.
        on: bool,
        /// Table 99's `/Locked`, which forbids offering the switch at all.
        locked: bool,
    },
}

/// What the rest of the row does when it is clicked.
#[derive(Debug, Clone, PartialEq)]
enum Act {
    /// Nothing: a heading, an attachment, a group whose row acts only through its switch.
    None,
    /// §12.3.3: activate the item, whatever `/Dest` or `/A` says that means.
    Activate(ObjectId),
    /// §7.11.4: take this embedded file's bytes out, by its `/EmbeddedFiles` key.
    Extract(String),
    /// §12.3.4: show this page, by its zero-based index.
    GoTo(usize),
}

/// One drawn line of a panel.
#[derive(Debug, Clone)]
struct Row {
    /// How deep, for the indent.
    depth: usize,
    /// What it says.
    label: String,
    /// A second, dimmed run after the label — a media type, a size.
    detail: Option<String>,
    /// Which face.
    style: Style,
    /// The label's colour, which for an outline item is Table 151's `/C`.
    colour: Color,
    /// The left edge.
    marker: Marker,
    /// What the rest of the row does.
    act: Act,
    /// §8.11's group, where the marker is a switch.
    group: Option<ObjectId>,
    /// How tall the row is, in row heights.
    ///
    /// One for every row of text, which is every row this panel drew before §12.3.4's thumbnails
    /// arrived. A picture needs more than a line of text, and expressing that as a *multiple*
    /// rather than as a pixel height keeps the scroll arithmetic integral: a list's extent is the
    /// sum of its units, and the row under a point is found by walking them.
    units: usize,
    /// §12.3.4's thumbnail, drawn inside the row's box above the label.
    image: Option<pdf_render::Image>,
}

impl Row {
    /// A plain row of text at a depth.
    fn plain(depth: usize, label: String) -> Self {
        Self {
            depth,
            label,
            detail: None,
            style: Style::default(),
            colour: Color::BLACK,
            marker: Marker::None,
            act: Act::None,
            group: None,
            units: 1,
            image: None,
        }
    }
}

/// Which of the sidebar's lists it is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// §12.3.3's outline.
    #[default]
    Contents,
    /// §12.3.4's thumbnail images, one row per page.
    Pages,
    /// §8.11.4.3's `/Order`.
    Layers,
    /// §7.11.4's embedded files.
    Files,
    /// §12.4.3's article threads.
    Articles,
    /// §14.3.3's document information dictionary.
    Document,
}

impl Tab {
    /// The six, in the order they are drawn.
    const ALL: [Self; 6] = [
        Self::Contents,
        Self::Pages,
        Self::Layers,
        Self::Files,
        Self::Articles,
        Self::Document,
    ];

    /// What the tab says.
    const fn label(self) -> &'static str {
        match self {
            Self::Contents => "Contents",
            Self::Pages => "Pages",
            Self::Layers => "Layers",
            Self::Files => "Files",
            // Short enough for a sixth tab in 300 logical pixels, and the clause's own noun:
            // §12.4.3 calls the thing "an article thread" and its `/I` title is the article's.
            Self::Articles => "Read",
            Self::Document => "About",
        }
    }

    /// Which of [`Self::ALL`] this is, for the per-tab scroll positions.
    const fn index(self) -> usize {
        match self {
            Self::Contents => 0,
            Self::Pages => 1,
            Self::Layers => 2,
            Self::Files => 3,
            Self::Articles => 4,
            Self::Document => 5,
        }
    }
}

/// What the sidebar shows, gathered by the host from three queries.
///
/// Borrowed rather than owned, and passed on every call rather than kept: the sidebar holds only
/// what the *view* owns — which tab, which subtrees are open, how far each list is scrolled — so
/// nothing here can go stale against the document.
#[derive(Debug, Clone, Copy)]
pub struct Content<'a> {
    /// [`viewer_core::Query::Outline`].
    pub outline: &'a pdf_model::outline::Outline,
    /// [`viewer_core::Query::Layers`].
    pub layers: &'a [Layer],
    /// [`viewer_core::Query::Attachments`].
    pub attachments: &'a [pdf_model::attachment::Attachment],
    /// [`viewer_core::Query::Articles`]: §12.4.3's threads, in the `/Threads` array's order.
    pub articles: &'a [pdf_model::article::Thread],
    /// [`viewer_core::Query::Collection`]: §12.3.5's collection, where the document states one.
    ///
    /// `None` for every document anyone has opened. Where it is `Some`, the files tab draws
    /// §12.3.5.2's folder tree and the schema's visible columns instead of a flat list — the same
    /// files, presented as the clause says a collection shall be presented.
    pub collection: Option<&'a pdf_model::collection::Collection>,
    /// §14.3.3's Table 349, from [`viewer_core::Query::Properties`].
    pub information: &'a pdf_model::metadata::Information,
    /// §14.3.2's metadata stream, read — `None` where the catalog names none.
    pub metadata: Option<&'a Result<pdf_model::xmp::Xmp, pdf_model::xmp::XmpError>>,
    /// One entry per page, for §12.3.4's tab: its label and its thumbnail where it has one.
    ///
    /// Built by the host rather than queried here, and **only while that tab is open**: a
    /// thumbnail is a decoded image, `viewer_core::Query::Thumbnail` answers one page at a time,
    /// and a thousand-page document would otherwise decode a thousand miniatures to draw eight.
    pub pages: &'a [Page],
}

/// One page, as §12.3.4's tab shows it.
#[derive(Debug, Clone)]
pub struct Page {
    /// What the row says: §12.4.2's label where the document states one, else the number.
    pub label: String,
    /// The decoded thumbnail, where the page states a `/Thumb` this program could read.
    pub thumbnail: Option<pdf_render::Image>,
}

/// What a click on the sidebar asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    /// §12.3.3: activate this outline item — jump, or trigger whatever `/A` states.
    Activate(ObjectId),
    /// §7.11.4: take this embedded file's bytes out, by its `/EmbeddedFiles` key.
    Extract(String),
    /// §12.3.4: show this page, by its zero-based index.
    ///
    /// A page index rather than an object, unlike the outline's [`Self::Activate`]: §12.3.4's
    /// thumbnail "represent[s] the contents of its page", and the page is already numbered — so
    /// there is no destination to resolve and nothing for the document to decide.
    GoTo(usize),
    /// §8.11: switch an optional content group on or off.
    SetGroup {
        /// Which group.
        group: ObjectId,
        /// Whether it is now on.
        on: bool,
    },
    /// The sidebar changed shape — a tab, a disclosure — and wants drawing again.
    Redraw,
    /// Inside the sidebar, on nothing that acts. Swallowed, so it does not reach the page.
    Nothing,
}

/// The three lists a document keeps about itself, as this program draws them.
#[derive(Debug, Default)]
pub struct Sidebar {
    /// Whether the sidebar is shown at all.
    pub shown: bool,
    /// Which list.
    tab: Tab,
    /// Outline items whose open state differs from the document's own `/Count`.
    ///
    /// A *difference* rather than a set of open items, so that §12.3.3's own answer is the
    /// default: "if the item is open, the total number of its open descendants … if the item is
    /// closed, a negative integer". A document that opens its first two levels opens them here,
    /// and this records only what a person changed.
    toggled: std::collections::BTreeSet<usize>,
    /// How far each list is scrolled, in logical pixels, never negative.
    scroll: [f32; 6],
    /// Which row the pointer is over, for the hover highlight.
    hovered: Option<usize>,
}

impl Sidebar {
    /// The sidebar's width in device pixels, or zero when it is hidden.
    ///
    /// This is what insets the page: the viewport `viewer-core` is told about is the window less
    /// this, so the page centres itself in what is left rather than behind the sidebar.
    #[must_use]
    pub fn inset(&self, scale: f32) -> u32 {
        if self.shown {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a panel width in pixels: positive, and a few hundred"
            )]
            let width = (PANEL_WIDTH * scale).round() as u32;
            width
        } else {
            0
        }
    }

    /// Whether §12.3.4's tab is the one showing.
    ///
    /// Asked by the host before it builds the page list, because that list means decoding every
    /// thumbnail the document carries and a document opens at a page rather than at a contact
    /// sheet.
    #[must_use]
    pub const fn shows_pages(&self) -> bool {
        self.shown && matches!(self.tab, Tab::Pages)
    }

    /// Shows or hides the sidebar.
    pub fn toggle(&mut self) {
        self.shown = !self.shown;
    }

    /// Opens the sidebar on a tab, which is what Table 29's `/PageMode` asks for.
    pub fn show(&mut self, tab: Tab) {
        self.shown = true;
        self.tab = tab;
    }

    /// Scrolls the current list by a number of **logical** pixels, clamped to its own ends.
    ///
    /// Clamped rather than wrapped or unbounded, for the same reason `Command::Scroll` clamps a
    /// page: a list scrolled past its last row shows nothing and says nothing about why.
    pub fn scroll(&mut self, by: f32, content: Content<'_>, height: u32, scale: f32) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a row count and a window height, both thousands at most"
        )]
        let (rows, tall) = (
            self.rows(content)
                .iter()
                .map(|row| row.units)
                .sum::<usize>() as f32,
            height as f32 / scale.max(0.01),
        );
        let furthest = (rows * (TEXT_SIZE * ROW_HEIGHT) - (tall - TABS)).max(0.0);
        let at = self.tab.index();
        if let Some(scroll) = self.scroll.get_mut(at) {
            *scroll = (*scroll + by).clamp(0.0, furthest);
        }
    }

    /// How far the current list is scrolled.
    fn scrolled(&self) -> f32 {
        self.scroll.get(self.tab.index()).copied().unwrap_or(0.0)
    }

    /// Remembers where the pointer is, and answers whether the highlight moved.
    pub fn hover(&mut self, at: (f32, f32), content: Content<'_>, scale: f32) -> bool {
        let was = self.hovered;
        self.hovered = self
            .row_at(at, content, scale)
            .map(|(index, _)| index)
            .filter(|_| self.shown);
        was != self.hovered
    }

    /// What a click at a point in the window does.
    ///
    /// `None` for a point outside the sidebar, which is the caller's signal to hand it to the
    /// page. Everything inside is answered, including a click on empty space: a panel that let
    /// clicks fall through to the document underneath it would start a text selection on a page
    /// the person cannot see.
    #[must_use]
    pub fn click(&mut self, at: (f32, f32), content: Content<'_>, scale: f32) -> Option<Hit> {
        if !self.shown || at.0 >= PANEL_WIDTH * scale {
            return None;
        }
        if at.1 < TABS * scale {
            let chosen = tab_at(at.0, scale);
            if chosen != self.tab {
                self.tab = chosen;
                self.hovered = None;
            }
            return Some(Hit::Redraw);
        }
        let Some((_, row)) = self.row_at(at, content, scale) else {
            return Some(Hit::Nothing);
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a nesting depth, which pdf-model bounds"
        )]
        let indent = (MARKER + INDENT * row.depth as f32) * scale;
        if at.0 < indent {
            return Some(match row.marker {
                Marker::Disclosure { id, .. } => {
                    if !self.toggled.insert(id) {
                        self.toggled.remove(&id);
                    }
                    Hit::Redraw
                }
                // §8.11.4.3 on `/Locked`: "[t]he state of a locked group cannot be changed
                // through the user interface of an interactive PDF processor." So the switch is
                // drawn — a person is entitled to see the state — and clicking it does nothing.
                Marker::Switch { on, locked: false } => match row.group {
                    Some(group) => Hit::SetGroup { group, on: !on },
                    None => Hit::Nothing,
                },
                Marker::Switch { locked: true, .. } | Marker::None => Hit::Nothing,
            });
        }
        Some(match row.act {
            Act::Activate(object) => Hit::Activate(object),
            Act::Extract(name) => Hit::Extract(name),
            Act::GoTo(page) => Hit::GoTo(page),
            Act::None => Hit::Nothing,
        })
    }

    /// The row under a point, and which of the visible rows it is.
    fn row_at(&self, at: (f32, f32), content: Content<'_>, scale: f32) -> Option<(usize, Row)> {
        if !self.shown || at.0 >= PANEL_WIDTH * scale || at.1 < TABS * scale {
            return None;
        }
        let row_height = TEXT_SIZE * ROW_HEIGHT * scale;
        let offset = at.1 - TABS * scale + self.scrolled() * scale;
        if offset < 0.0 {
            return None;
        }
        // Walked rather than divided, because a row is as many row heights tall as its
        // `units` says and §12.3.4's thumbnails are the reason there is more than one kind.
        let mut top = 0.0_f32;
        for (index, row) in self.rows(content).into_iter().enumerate() {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a row's height in row units, which is one or a handful"
            )]
            let bottom = top + row.units as f32 * row_height;
            if offset < bottom {
                return Some((index, row));
            }
            top = bottom;
        }
        None
    }

    /// Whether an outline item's children are shown.
    ///
    /// §12.3.3's `/Count` states the document's own answer and [`pdf_model::outline::Item::open`]
    /// carries it; the set records only where a person disagreed.
    fn is_open(&self, id: usize, stated: bool) -> bool {
        stated != self.toggled.contains(&id)
    }

    /// The visible rows of the current tab, in the order they are drawn.
    fn rows(&self, content: Content<'_>) -> Vec<Row> {
        let mut out = Vec::new();
        match self.tab {
            Tab::Contents => {
                let mut next = 0;
                self.flatten(&content.outline.items, 0, &mut next, &mut out);
                if out.is_empty() {
                    out.push(nothing("This document states no outline."));
                }
            }
            Tab::Layers => {
                layer_rows(content.layers, 0, &mut out);
                if out.is_empty() {
                    out.push(nothing("This document states no optional content."));
                }
            }
            Tab::Files => {
                match content.collection {
                    // §12.3.5: "[i]f this dictionary is present in a PDF document, the interactive
                    // PDF processor shall present the document as a portable collection." The same
                    // files, in §12.3.5.2's folders and with the schema's columns beside them.
                    Some(collection) => collection_rows(collection, content.attachments, &mut out),
                    None => {
                        for file in content.attachments {
                            let mut row = Row::plain(
                                0,
                                file.file_name.clone().unwrap_or_else(|| file.name.clone()),
                            );
                            row.detail = describe(file);
                            // §7.11.4's whole point from a person's side: the bytes are inside the
                            // document and a click takes them out. The key is the tree's, which is
                            // what `Command::Extract` names a file by.
                            row.act = Act::Extract(file.name.clone());
                            out.push(row);
                        }
                    }
                }
                if out.is_empty() {
                    out.push(nothing("This document embeds no files."));
                }
            }
            Tab::Pages => {
                for (index, page) in content.pages.iter().enumerate() {
                    let mut row = Row::plain(0, page.label.clone());
                    row.act = Act::GoTo(index);
                    // A page with no thumbnail is still a row: §12.3.4's NOTE says they "are not
                    // required, and can be included for some pages and not for others", so a
                    // panel that listed only the pages that have one would be a list of the
                    // document's *thumbnails* rather than of its pages.
                    if let Some(image) = page.thumbnail.clone() {
                        row.units = THUMBNAIL_UNITS;
                        row.image = Some(image);
                    }
                    out.push(row);
                }
                if out.is_empty() {
                    out.push(nothing("This document has no pages."));
                }
            }
            Tab::Articles => {
                for thread in content.articles {
                    // §12.4.3 puts the title in the thread's *information* dictionary — Table 159
                    // is §14.3.3's Table 349 by another name — and a thread that states none is
                    // still a thread, so it gets the clause's own noun and its place in the array.
                    let label = thread
                        .title
                        .clone()
                        .unwrap_or_else(|| format!("Article {}", out.len().saturating_add(1)));
                    let mut row = Row::plain(0, label);
                    row.detail = Some(match thread.beads.len() {
                        1 => "1 bead".to_owned(),
                        count => format!("{count} beads"),
                    });
                    // The same message an outline row sends, and for the same reason: the
                    // *document* decides what activating an object means. `interact` composes
                    // §12.6.4.7's own thread action out of it, so a click lands on Table 163's
                    // `/R` rather than on the page the first bead happens to sit on.
                    row.act = Act::Activate(thread.id);
                    out.push(row);
                }
                if out.is_empty() {
                    out.push(nothing("This document states no article threads."));
                }
            }
            Tab::Document => property_rows(content, &mut out),
        }
        out
    }

    /// [`Self::rows`] for §12.3.3, one level at a time.
    ///
    /// `next` numbers **every** item in pre-order, open or not, so that an identity does not
    /// change when a subtree above it closes — a set of open items keyed by a *visible* row index
    /// would reassign every identity on the first click.
    fn flatten(
        &self,
        items: &[pdf_model::outline::Item],
        depth: usize,
        next: &mut usize,
        out: &mut Vec<Row>,
    ) {
        for item in items {
            let id = *next;
            *next = next.saturating_add(1);
            let open = self.is_open(id, item.open);
            out.push(Row {
                depth,
                label: item.title.clone(),
                detail: None,
                style: Style {
                    bold: item.bold,
                    italic: item.italic,
                },
                colour: Color {
                    r: item.colour[0],
                    g: item.colour[1],
                    b: item.colour[2],
                    a: 1.0,
                },
                marker: if item.children.is_empty() {
                    Marker::None
                } else {
                    Marker::Disclosure { id, open }
                },
                // **Activation rather than navigation**, because §12.3.3 asks for both halves
                // of one sentence: "jump to a destination or trigger an action associated with
                // the item". A `Follow` here would perform the first half of an item whose `/A`
                // is a URI and silently drop the second.
                act: Act::Activate(item.id),
                group: None,
                units: 1,
                image: None,
            });
            if item.children.is_empty() {
                continue;
            }
            if open {
                self.flatten(&item.children, depth.saturating_add(1), next, out);
            } else {
                // The subtree is not drawn, but its identities are still spent, which is what
                // keeps them stable across a close and a reopen.
                let mut skipped = 0;
                count(&item.children, &mut skipped);
                *next = next.saturating_add(skipped);
            }
        }
    }

    /// The display list for the sidebar, in device pixels of the window.
    #[must_use]
    pub fn draw(
        &self,
        chrome: &Chrome,
        content: Content<'_>,
        height: u32,
        scale: f32,
    ) -> DisplayList {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window height in pixels, which is thousands and not billions"
        )]
        let tall = height as f32;
        let width = PANEL_WIDTH * scale;
        // The list's "page size" is the sidebar itself: nothing here is a page, and what the
        // backends use it for is the extent of the thing being drawn.
        let mut list = DisplayList::new(pdf_render::Size {
            width,
            height: tall,
        });
        if !self.shown {
            return list;
        }
        rectangle(&mut list, (0.0, 0.0, width, tall), BACKGROUND);

        let size = TEXT_SIZE * scale;
        let strip = TABS * scale;
        let row_height = TEXT_SIZE * ROW_HEIGHT * scale;
        let mut next = strip - self.scrolled() * scale;
        for (index, row) in self.rows(content).into_iter().enumerate() {
            let top = next;
            #[expect(
                clippy::cast_precision_loss,
                reason = "a row's height in row units, which is one or a handful"
            )]
            let tall_row = row.units as f32 * row_height;
            next = top + tall_row;
            if top + tall_row < strip || top > tall {
                continue;
            }
            if self.hovered == Some(index) {
                rectangle(&mut list, (0.0, top, width - scale, tall_row), HOVER);
            }
            // §12.3.4's miniature, above its own label and inside the row's box. Scaled to fit
            // the width the panel has, never magnified: the entry is "a small image … of the
            // page's appearance", and a thumbnail blown up past its own grid is a blur claiming
            // to be a page.
            if let Some(image) = row.image {
                draw_thumbnail(
                    &mut list,
                    &image,
                    (0.0, top, width, tall_row - row_height),
                    scale,
                );
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "a nesting depth, which pdf-model bounds"
            )]
            let indent = (MARKER + INDENT * row.depth as f32) * scale;
            draw_marker(
                &mut list,
                row.marker,
                (
                    indent - MARKER * 0.5 * scale,
                    top + tall_row - row_height * 0.5,
                ),
                scale,
            );
            // Clipped by measurement rather than by a clip path: the label is truncated to what
            // fits and an ellipsis says so, which is what a list of sentences needs. A clip would
            // cut a letter in half and say nothing.
            let baseline = top + tall_row - row_height * 0.3;
            let room = width - indent - 8.0 * scale;
            let label = elide(chrome, &row.label, size, row.style, room);
            let after = chrome.text(
                &mut list,
                &label,
                (indent, baseline),
                size,
                row.style,
                row.colour,
            );
            if let Some(detail) = row.detail {
                let left = after + 6.0 * scale;
                let detail = elide(
                    chrome,
                    &detail,
                    size,
                    Style::default(),
                    width - left - 8.0 * scale,
                );
                chrome.text(
                    &mut list,
                    &detail,
                    (left, baseline),
                    size,
                    Style::default(),
                    DIMMED,
                );
            }
        }

        // The tab strip is drawn *last*, over its own background: a scrolled row's top half would
        // otherwise appear above the separator, which is what the first run of this panel did.
        // A clip would do the same job and would cut the letters in half rather than hide them.
        self.draw_tabs(&mut list, chrome, width, size, scale);
        rectangle(&mut list, (0.0, strip - scale, width, scale), EDGE);
        rectangle(&mut list, (width - scale, 0.0, scale, tall), EDGE);
        list
    }

    /// The strip of tab labels across the top, with the current one lit.
    fn draw_tabs(
        &self,
        list: &mut DisplayList,
        chrome: &Chrome,
        width: f32,
        size: f32,
        scale: f32,
    ) {
        let strip = TABS * scale;
        rectangle(list, (0.0, 0.0, width, strip - scale), BACKGROUND);
        #[expect(
            clippy::cast_precision_loss,
            reason = "the number of tabs, which is six"
        )]
        let each = width / Tab::ALL.len() as f32;
        for (index, tab) in Tab::ALL.into_iter().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "one of five tabs")]
            let left = index as f32 * each;
            if tab == self.tab {
                rectangle(list, (left, 0.0, each, strip - scale), HOVER);
            }
            let label = tab.label();
            let centred = left + (each - chrome.width(label, size, Style::default())) * 0.5;
            chrome.text(
                list,
                label,
                (centred, strip - 10.0 * scale),
                size,
                Style {
                    bold: tab == self.tab,
                    italic: false,
                },
                Color::BLACK,
            );
        }
    }
}

/// Which tab a horizontal position is in.
fn tab_at(x: f32, scale: f32) -> Tab {
    #[expect(
        clippy::cast_precision_loss,
        reason = "the number of tabs, which is five"
    )]
    let each = (PANEL_WIDTH * scale) / Tab::ALL.len() as f32;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a tab index from a position already known to be inside the strip"
    )]
    let index = (x / each.max(1.0)) as usize;
    Tab::ALL.get(index).copied().unwrap_or(Tab::Document)
}

/// §12.3.5's collection, as rows: the folder tree, with each file under the folder it names.
///
/// **The container's pages stay on the screen**, and that is the one decision this panel makes
/// that the clause leaves open. §12.3.5 says a processor "shall present the document as a portable
/// collection" and does not say *instead of what*; §7.6.7's unencrypted wrapper is the case that
/// settles it — a wrapper's whole purpose is a page saying the payload is encrypted, and Table
/// 153's `/View H` is how such a document asks for the file list to start hidden. A viewer that
/// replaced the page with a file browser would hide the sentence the wrapper exists to show. So
/// the collection is a panel over a page, like every other tab.
///
/// §12.3.5.2's key format is what files a folder: a name-tree key `<3>report.pdf` is *report.pdf*
/// in folder 3, and `collection::folder_of` reads it. A key that does not conform names no folder,
/// and the clause says such files "shall be treated as associated with the root folder" — so they
/// are drawn at depth zero, above the folders, which is where the root's own files belong.
fn collection_rows(
    collection: &pdf_model::collection::Collection,
    files: &[pdf_model::attachment::Attachment],
    out: &mut Vec<Row>,
) {
    // The schema's visible columns in Table 155's `/O` order, which is "[t]he relative order of
    // the field name in the user interface". A field with no `/O` sorts after the ones that state
    // one, by key, which is the only order left when the file states none.
    let mut columns: Vec<(&String, &pdf_model::collection::Field)> = collection
        .schema
        .iter()
        .filter(|(_, field)| field.visible)
        .collect();
    columns.sort_by_key(|(key, field)| (field.order.unwrap_or(i64::MAX), (*key).clone()));

    let mut under = |folder: Option<u32>, out: &mut Vec<Row>, depth: usize| {
        for file in files {
            let (id, name) = match pdf_model::collection::folder_of(&file.name) {
                Some((id, name)) => (Some(id), name.to_owned()),
                None => (None, file.name.clone()),
            };
            if id != folder {
                continue;
            }
            let mut row = Row::plain(depth, file.file_name.clone().unwrap_or(name));
            row.detail = columns_of(&columns, file).or_else(|| describe(file));
            row.act = Act::Extract(file.name.clone());
            out.push(row);
        }
    };

    under(None, out, 0);
    if let Some(root) = collection.folders.as_ref() {
        folder_rows(root, 0, &mut under, out);
    }
}

/// One folder and everything under it.
fn folder_rows(
    folder: &pdf_model::collection::Folder,
    depth: usize,
    under: &mut impl FnMut(Option<u32>, &mut Vec<Row>, usize),
    out: &mut Vec<Row>,
) {
    let mut row = Row::plain(depth, folder.name.clone());
    row.detail.clone_from(&folder.description);
    // A folder is not a file: it has no bytes to extract, so its row acts through its children.
    row.act = Act::None;
    out.push(row);
    under(Some(folder.id), out, depth.saturating_add(1));
    for child in &folder.children {
        folder_rows(child, depth.saturating_add(1), under, out);
    }
}

/// The schema's columns for one file, as `name: value` joined — the detail line of its row.
///
/// Table 47's `/P` prefix is concatenated with the value and not with the name, which is what the
/// table says it is for: "[a] prefix string that shall be concatenated with the text string
/// presented to the user".
fn columns_of(
    columns: &[(&String, &pdf_model::collection::Field)],
    file: &pdf_model::attachment::Attachment,
) -> Option<String> {
    let shown: Vec<String> = columns
        .iter()
        .filter_map(|(key, field)| {
            let value = collection_value(key, field, file)?;
            Some(format!("{}: {value}", field.name))
        })
        .collect();
    (!shown.is_empty()).then(|| shown.join("  ·  "))
}

/// One column's value for one file.
///
/// Table 155's `/Subtype` decides *where the value lives*, which is the distinction
/// `collection::FieldKind` exists for: the first three kinds read §7.11.6's collection item, and
/// the file-related ones read the file specification this host already has. Only the second group
/// is answered here — the item is on the file specification's `/CI` and `Attachment` does not
/// carry it, which is a gap this row records rather than papers over.
fn collection_value(
    _key: &str,
    field: &pdf_model::collection::Field,
    file: &pdf_model::attachment::Attachment,
) -> Option<String> {
    use pdf_model::collection::FieldKind;
    match field.kind {
        FieldKind::FileName => file.file_name.clone(),
        FieldKind::Description => file.description.clone(),
        FieldKind::Size => file.size.map(|size| format!("{size}")),
        FieldKind::ModificationDate => file.modified.clone(),
        FieldKind::CreationDate => file.created.clone(),
        _ => None,
    }
}

/// A row that says why a list is empty.
///
/// A list is empty because the *document* said nothing, and saying so is not decoration: an
/// empty panel and a panel this program failed to fill look identical, and only one of them is
/// a fact about the file.
fn nothing(sentence: &str) -> Row {
    let mut row = Row::plain(0, sentence.to_owned());
    row.colour = DIMMED;
    row.style = Style {
        bold: false,
        italic: true,
    };
    row
}

/// §7.11.4's second line for an attachment: what it is and how big the file says it is.
fn describe(file: &pdf_model::attachment::Attachment) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(media) = &file.media_type {
        parts.push(media.clone());
    }
    // Table 45's `/Size` is "the size of the uncompressed embedded file, in bytes" — the
    // document's claim rather than a measurement, which is why it is shown beside the media type
    // rather than presented as a fact about the stream.
    if let Some(size) = file.size {
        parts.push(match size {
            0..=1023 => format!("{size} bytes"),
            1024..=1_048_575 => format!("{} kB", size / 1024),
            _ => format!("{} MB", size / 1_048_576),
        });
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// §14.3.3's Table 349, as a list of what the document says about itself.
///
/// A label and a value on one row, with the label bold: nothing here is clickable, and the tab
/// exists because §14.3.3's ledger row said `inapplicable` on the reason "a viewer with a
/// document-properties panel would read it; this one has no panel" — which stopped being true in
/// the hundred-and-sixty-sixth session.
///
/// **§14.3.2's own answers follow, under their own heading**, since the
/// two-hundred-and-ninety-fourth session (ADR 0186). Table 349's every text entry carries a NOTE
/// pointing at an XMP counterpart, so a document with a metadata stream may be saying something
/// else about itself there — and §14.3.4 leaves the disagreement "at the discretion of the PDF
/// processor", which for a panel means showing both rather than picking one.
fn property_rows(content: Content<'_>, out: &mut Vec<Row>) {
    let information = content.information;
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
                pdf_model::metadata::Trapped::Unknown => None,
                other => Some(format!("{other:?}")),
            },
        ),
    ];
    for (label, value) in stated {
        let Some(value) = value else { continue };
        let mut row = Row::plain(0, format!("{label}:"));
        row.style = Style {
            bold: true,
            italic: false,
        };
        row.detail = Some(value);
        out.push(row);
    }
    if out.is_empty() {
        out.push(nothing("This document states no §14.3.3 information."));
    }
    metadata_rows(&content, out);
}

/// §14.3.2's stream, under §14.3.3's dictionary and marked as the other place.
///
/// The two tables state the same seven facts and Table 349's NOTEs pair them up, so a panel that
/// merged them would be hiding a disagreement rather than resolving one — §12.2 ranks `dc:title`
/// above `/Title` and nothing ranks the rest. What is shown is therefore the *stream's* answer,
/// labelled, for the four properties a person recognises, and a count for everything else.
fn metadata_rows(content: &Content<'_>, out: &mut Vec<Row>) {
    let Some(metadata) = content.metadata else {
        return;
    };
    let xmp = match metadata {
        Ok(xmp) => xmp,
        Err(error) => {
            out.push(nothing(&format!(
                "§14.3.2's metadata stream could not be read: {error}"
            )));
            return;
        }
    };

    let stated: Vec<(&str, Option<String>)> = vec![
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
    out.push(nothing("§14.3.2 (XMP):"));
    for (label, value) in stated {
        let Some(value) = value else { continue };
        let mut row = Row::plain(0, format!("{label}:"));
        row.style = Style {
            bold: true,
            italic: false,
        };
        row.detail = Some(value);
        out.push(row);
    }
    let rest = xmp.properties().len().saturating_sub(shown);
    if rest > 0 {
        out.push(nothing(&format!("and {rest} other propert(ies).")));
    }
}

/// A §7.9.4 date as a person reads it, or the file's own string where it does not conform.
///
/// **The string is never dropped.** A producer that wrote a malformed date still wrote
/// something, and showing nothing would hide it — which is the same choice
/// [`pdf_model::metadata::Information`] makes by keeping the bytes beside the parse.
fn stamp(parsed: Option<pdf_syntax::Date>, written: Option<&String>) -> Option<String> {
    match parsed {
        Some(date) => Some(format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            date.year, date.month, date.day, date.hour, date.minute
        )),
        None => written.cloned(),
    }
}

/// §8.11.4.3's `/Order`, flattened into rows.
///
/// A collection nests and is **not** disclosable: the clause makes the nesting a statement about
/// the content — "a nested array … specifying the order of the optional content groups" — rather
/// than a place to hide things, and a heading with its groups folded away would be a panel
/// deciding what a person may see. An outline's `/Count` says the opposite in as many words,
/// which is why the two differ here.
fn layer_rows(layers: &[Layer], depth: usize, out: &mut Vec<Row>) {
    for layer in layers {
        match layer {
            Layer::Group {
                group,
                name,
                on,
                locked,
            } => {
                let mut row = Row::plain(
                    depth,
                    name.clone().unwrap_or_else(|| "(unnamed layer)".to_owned()),
                );
                row.marker = Marker::Switch {
                    on: *on,
                    locked: *locked,
                };
                row.group = Some(*group);
                if *locked {
                    row.colour = DIMMED;
                }
                out.push(row);
            }
            Layer::Collection { label, children } => {
                if let Some(label) = label {
                    let mut row = Row::plain(depth, label.clone());
                    row.style = Style {
                        bold: true,
                        italic: false,
                    };
                    out.push(row);
                }
                layer_rows(children, depth.saturating_add(1), out);
            }
        }
    }
}

/// A row's left-edge marker.
/// Draws §12.3.4's thumbnail inside a row's picture box, fitted and centred.
///
/// > A PDF document may contain thumbnail images representing the contents of its pages in
/// > miniature form.
///
/// The clause states no size for one and no rule for placing it, so both are this program's: the
/// miniature is fitted to the box, kept to its own aspect ratio, and centred.
///
/// **Fitted rather than drawn at its own sample size**, which is a choice and was made the other
/// way first. A thumbnail is typically a few score samples on a side and this box is a hundred
/// and twenty logical pixels tall: at 1:1 the panel shows a stamp in the middle of a lot of
/// background, which tells a person less about the page than the row's label already does. What
/// it costs is a magnification of about two, and §8.9.5.3 is where the file gets a say in how
/// that looks — `/Interpolate` is "an attempt to produce a smooth transition between adjacent
/// sample values when rendering an image whose resolution is significantly lower than that of
/// the output device", which is exactly this, and the backends already honour it.
fn draw_thumbnail(
    list: &mut DisplayList,
    image: &pdf_render::Image,
    box_of: (f32, f32, f32, f32),
    scale: f32,
) {
    let (left, top, width, height) = box_of;
    if image.width == 0 || image.height == 0 || width <= 0.0 || height <= 0.0 {
        return;
    }
    let margin = 6.0 * scale;
    let (room_x, room_y) = (width - margin * 2.0, height - margin);
    if room_x <= 0.0 || room_y <= 0.0 {
        return;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a thumbnail's sample count, which §12.3.4 makes miniature"
    )]
    let (w, h) = (image.width as f32, image.height as f32);
    let factor = (room_x / w).min(room_y / h);
    let (drawn_w, drawn_h) = (w * factor, h * factor);
    let x = left + (width - drawn_w) * 0.5;
    let y = top + margin + (room_y - drawn_h) * 0.5;
    // The unit square carries an image with its top row at unit y = 1 (§8.9.5), so placing the
    // top row at the box's own top takes a flip — the same composition `render-quorra`'s
    // presenter makes for a CPU raster.
    list.push(Command::Image {
        image: image.clone().into(),
        transform: Transform {
            a: drawn_w,
            b: 0.0,
            c: 0.0,
            d: -drawn_h,
            e: x,
            f: y + drawn_h,
        },
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: pdf_render::BlendMode::Normal,
    });
}

fn draw_marker(list: &mut DisplayList, marker: Marker, centre: (f32, f32), scale: f32) {
    let (cx, cy) = centre;
    match marker {
        Marker::None => {}
        Marker::Disclosure { open, .. } => {
            let size = 7.0 * scale;
            let (half, reach) = (size * 0.5, size * 0.45);
            let corners = if open {
                [
                    (cx - half, cy - reach),
                    (cx + half, cy - reach),
                    (cx, cy + reach),
                ]
            } else {
                [
                    (cx - reach, cy - half),
                    (cx + reach, cy),
                    (cx - reach, cy + half),
                ]
            };
            outline_of(list, &corners, EDGE);
        }
        Marker::Switch { on, locked } => {
            let half = 4.5 * scale;
            rectangle(
                list,
                (cx - half, cy - half, half * 2.0, half * 2.0),
                if locked { BACKGROUND } else { Color::WHITE },
            );
            // The box's edge, drawn as four thin rectangles rather than as a stroke: this module
            // draws fills only, so that the two backends' stroke rules (ADR 0028) never enter an
            // interface's appearance.
            let edge = scale;
            for side in [
                (cx - half, cy - half, half * 2.0, edge),
                (cx - half, cy + half - edge, half * 2.0, edge),
                (cx - half, cy - half, edge, half * 2.0),
                (cx + half - edge, cy - half, edge, half * 2.0),
            ] {
                rectangle(list, side, EDGE);
            }
            if on {
                rectangle(
                    list,
                    (
                        cx - half + 2.0 * scale,
                        cy - half + 2.0 * scale,
                        (half - 2.0 * scale) * 2.0,
                        (half - 2.0 * scale) * 2.0,
                    ),
                    if locked { DIMMED } else { Color::BLACK },
                );
            }
        }
    }
}

/// Every item in a subtree, including the ones a closed parent hides.
fn count(items: &[pdf_model::outline::Item], out: &mut usize) {
    for item in items {
        *out = out.saturating_add(1);
        count(&item.children, out);
    }
}

/// A label cut to the width available, with `…` where anything was dropped.
fn elide(chrome: &Chrome, label: &str, size: f32, style: Style, room: f32) -> String {
    // A row is one line: §12.3.3 states no wrapping, and a newline inside a `/Title` would
    // otherwise leave the rest of the row's text drawn on top of itself.
    let label = label.replace(['\n', '\r', '\t'], " ");
    if chrome.width(&label, size, style) <= room {
        return label;
    }
    let ellipsis = chrome.width("…", size, style);
    let mut kept = String::new();
    let mut used = 0.0;
    for character in label.chars() {
        let mut one = [0_u8; 4];
        let advance = chrome.width(character.encode_utf8(&mut one), size, style);
        if used + advance + ellipsis > room {
            break;
        }
        used += advance;
        kept.push(character);
    }
    kept.push('…');
    kept
}

/// The size the notice is set at, in logical pixels.
///
/// Chosen from the text rather than from taste: `/NOTICE` is wrapped to 98 columns, and 98 cells
/// of Courier's fixed 600-thousandths pitch at this size is 588 logical pixels — narrow enough
/// to fit a window a person has not maximised, and large enough to read.
const NOTICE_SIZE: f32 = 10.0;

/// The line spacing of the notice, as a multiple of its size.
const NOTICE_LEADING: f32 = 1.35;

/// How far the card sits inside the window, in logical pixels.
const NOTICE_MARGIN: f32 = 24.0;

/// The `/NOTICE` this binary carries, shown over the page.
///
/// **This is a licence obligation with a surface.** Both licences covering the compiled-in
/// standard 14 fonts require a *binary* distribution to reproduce their notices "in the
/// documentation and/or other materials provided with the distribution"; `--licences` has
/// printed them since the hundred-and-forty-eighth session, and a command-line flag is a poor
/// answer for a person who is looking at a window. This is the other half, and it is the About
/// panel the project owner asked for.
///
/// Set in Courier and **not re-wrapped**: the file's own line breaks are what keep a BSD
/// licence's paragraphs and a font list's columns readable, and re-flowing text a licence
/// obliges this program to reproduce would be editing it.
#[derive(Debug, Default)]
pub struct About {
    /// Whether the card is over the page.
    pub shown: bool,
    /// How far down the text is scrolled, in logical pixels, never negative.
    scroll: f32,
}

impl About {
    /// Shows or hides the card.
    pub fn toggle(&mut self) {
        self.shown = !self.shown;
        self.scroll = 0.0;
    }

    /// Scrolls the notice by a number of **logical** pixels, clamped to its own ends.
    pub fn scroll(&mut self, by: f32, notice: &str, height: u32, scale: f32) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a line count and a window height, both thousands at most"
        )]
        let (lines, tall) = (
            notice.lines().count() as f32,
            height as f32 / scale.max(0.01),
        );
        let content = lines * NOTICE_SIZE * NOTICE_LEADING;
        let furthest = (content - (tall - NOTICE_MARGIN * 4.0)).max(0.0);
        self.scroll = (self.scroll + by).clamp(0.0, furthest);
    }

    /// The card, in device pixels of the window.
    #[must_use]
    pub fn draw(
        &self,
        chrome: &Chrome,
        notice: &str,
        width: u32,
        height: u32,
        scale: f32,
    ) -> DisplayList {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window's extent in pixels, which is thousands"
        )]
        let (wide, tall) = (width as f32, height as f32);
        let mut list = DisplayList::new(pdf_render::Size {
            width: wide,
            height: tall,
        });
        if !self.shown {
            return list;
        }
        // The page is still there and is still the document; dimming it says the card is over it
        // rather than instead of it, which is what a modal panel with no window manager behind
        // it has to say for itself.
        rectangle(
            &mut list,
            (0.0, 0.0, wide, tall),
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.45,
            },
        );
        let margin = NOTICE_MARGIN * scale;
        let card = (
            margin,
            margin,
            (wide - margin * 2.0).max(0.0),
            (tall - margin * 2.0).max(0.0),
        );
        rectangle(&mut list, card, Color::WHITE);

        let size = NOTICE_SIZE * scale;
        let leading = size * NOTICE_LEADING;
        let left = margin + 16.0 * scale;
        let first = margin + 16.0 * scale + size - self.scroll * scale;
        for (index, line) in notice.lines().enumerate() {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a line index, bounded by the notice's own length"
            )]
            let baseline = first + index as f32 * leading;
            // Bounded by the card rather than clipped: a line above or below it is not drawn at
            // all, which is cheaper than a clip and, for a list of whole lines, identical.
            if baseline < margin + size || baseline > tall - margin {
                continue;
            }
            chrome.mono_text(&mut list, line, (left, baseline), size);
        }
        list
    }
}

/// How tall a popup window's title bar is, as a multiple of the text size.
///
/// §12.5.6.14 says a popup "displays text in a popup window" and describes no furniture at all,
/// so every number here is this host's. A native one draws a real window.
const POPUP_TITLE_HEIGHT: f32 = 1.6;

/// How far a popup window's text is inset from its `/Rect`, in logical pixels.
const POPUP_PADDING: f32 = 5.0;

/// The colour a popup's title bar takes where the annotation states no `/C`.
const POPUP_TITLE: Color = Color {
    r: 0.98,
    g: 0.92,
    b: 0.60,
    a: 1.0,
};

/// A popup window's paper.
const POPUP_PAPER: Color = Color {
    r: 1.0,
    g: 0.99,
    b: 0.90,
    a: 1.0,
};

/// §12.5.6.14's windows, drawn over the page.
///
/// **A window is not page content**, which is the whole reason this is here rather than in
/// `pdf-model`: the clause says a popup "shall have no appearance stream", so there is nothing
/// for `crate::appearance` to construct and nothing for either backend to be compared on. What
/// crosses from the core is [`viewer_core::PopupWindow`] — a rectangle in the window's own pixels
/// and the strings the document states — and everything below decides what that *looks* like,
/// which is a host's business and is written down as a choice:
///
/// - a title bar in Table 166's `/C`, which is the one thing the standard does say about a
///   popup's appearance: "[t]he title bar of the annotation's popup window";
/// - Table 172's `/T` in it, "[t]he text label that shall be displayed in the title bar of the
///   annotation's popup window", with Table 166's `/M` after it where there is room;
/// - the `/Contents` under that, wrapped, with Table 172's own clause's paragraph rule applied
///   (ISO 32000-2 section 12.5.6.2): "[w]hen
///   separating text into paragraphs, a CARRIAGE RETURN (0Dh) shall be used and not, for example,
///   a LINE FEED character (0Ah)" — so a carriage return starts a paragraph here, and a line feed
///   is accepted as one too, because a reader that obeyed the writer's rule as if it were its own
///   would show a paragraph break as a space.
///
/// **A character this interface's own font states no code for is counted and said out loud.**
/// `Chrome::text` skips one silently, which is right for a glyph in a title it is eliding and
/// wrong for the text of a note: six of the corpus's seven open popups are in Chinese, and a
/// blank window would be this program telling a person the note is empty. Trap 5, in an
/// interface.
pub fn popup_windows(
    chrome: &Chrome,
    windows: &[viewer_core::PopupWindow],
    width: u32,
    height: u32,
    scale: f32,
) -> Option<DisplayList> {
    if windows.is_empty() {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    let mut list = DisplayList::new(pdf_render::Size::new(width as f32, height as f32));
    for window in windows {
        draw_popup(chrome, &mut list, window, scale);
    }
    Some(list)
}

/// One window: its paper, its title bar, and as much of its text as fits.
fn draw_popup(
    chrome: &Chrome,
    list: &mut DisplayList,
    window: &viewer_core::PopupWindow,
    scale: f32,
) {
    let (x, y) = (window.quad[0], window.quad[1]);
    let (w, h) = (window.quad[2] - x, window.quad[5] - y);
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let size = TEXT_SIZE * scale;
    let padding = POPUP_PADDING * scale;
    let bar = size * POPUP_TITLE_HEIGHT;
    rectangle(list, (x, y, w, h), EDGE);
    rectangle(list, (x + 1.0, y + 1.0, w - 2.0, h - 2.0), POPUP_PAPER);
    rectangle(
        list,
        (x + 1.0, y + 1.0, w - 2.0, bar.min(h - 2.0)),
        window.colour.unwrap_or(POPUP_TITLE),
    );

    let room = w - 2.0 * padding;
    if room <= 0.0 {
        return;
    }
    let baseline = y + bar - size * 0.4;
    let title = window.title.as_deref().unwrap_or_default();
    let used = chrome.text(
        list,
        &elide(
            chrome,
            title,
            size,
            Style {
                bold: true,
                italic: false,
            },
            room,
        ),
        (x + padding, baseline),
        size,
        Style {
            bold: true,
            italic: false,
        },
        Color::BLACK,
    );
    // Table 166's `/M`, in whatever format the file spells it — the table makes displaying it a
    // `shall` and puts no format on the string. Only where the title has left room for it.
    if let Some(modified) = window.modified.as_deref() {
        let stamp = stamp(
            pdf_syntax::Date::parse(modified),
            Some(&modified.to_owned()),
        )
        .unwrap_or_else(|| modified.to_owned());
        let stamp_width = chrome.width(&stamp, size * 0.85, Style::default());
        let at = x + w - padding - stamp_width;
        if at > used + padding {
            chrome.text(
                list,
                &stamp,
                (at, baseline),
                size * 0.85,
                Style::default(),
                DIMMED,
            );
        }
    }

    let mut line = y + bar + size;
    let bottom = y + h - padding;
    let text = window.text.as_deref().unwrap_or_default();
    for paragraph in text.split(['\r', '\n']) {
        for run in wrap(chrome, paragraph, size, room) {
            if line > bottom {
                return;
            }
            chrome.text(
                list,
                &run,
                (x + padding, line),
                size,
                Style::default(),
                Color::BLACK,
            );
            line += size * 1.25;
        }
    }
    // What the face could not set. Counted over the whole value rather than per line, because
    // the sentence is about the note and not about a row of it — and it is still worth saying
    // beside the boxes `Chrome::text` now draws, because a count is what a person needs to know
    // whether a word or a paragraph is missing.
    let missing = chrome.without_a_code(text, Style::default());
    if missing > 0 && line <= bottom {
        let note =
            format!("[{missing} characters this interface's font cannot set, shown as boxes]");
        chrome.text(
            list,
            &elide(chrome, &note, size * 0.85, Style::default(), room),
            (x + padding, line),
            size * 0.85,
            Style::default(),
            DIMMED,
        );
    }
}

/// Breaks a paragraph into lines that fit `room`, at word boundaries where it can.
///
/// A word longer than the line is broken by character, because the alternative is a line that
/// runs out of the window — and the window is the document's rectangle rather than this host's,
/// so there is nowhere for it to go.
fn wrap(chrome: &Chrome, paragraph: &str, size: f32, room: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in paragraph.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if chrome.width(&candidate, size, Style::default()) <= room {
            line = candidate;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        // The word alone, broken where it stops fitting.
        for character in word.chars() {
            let wider = format!("{line}{character}");
            if !line.is_empty() && chrome.width(&wider, size, Style::default()) > room {
                lines.push(std::mem::take(&mut line));
            }
            line.push(character);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
