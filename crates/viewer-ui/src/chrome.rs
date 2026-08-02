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
//! [`Sidebar`] shows one of three things a document says about itself, chosen by a tab:
//! §12.3.3's outline, §8.11.4.3's `/Order` of optional content groups, and §7.11.4's embedded
//! files. They are one piece of code because they are one shape — indented rows with a label, a
//! marker at the left and something a click does — and because the differences between them are
//! the interesting part: an outline row *navigates*, a layer row is a **switch** the clause may
//! forbid, and an attachment row is, for now, only a statement.
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
use viewer_core::{Layer, PageTarget};

/// How wide the panel is, in logical pixels.
///
/// A choice, and the only rule behind it is that an outline's titles are sentences: §12.3.3's
/// `/Title` is "the text that shall be displayed on the screen for this item", and documents
/// write whole clause headings there. Narrower than this and the specification's own outline is
/// all ellipsis; wider and the page it is beside stops being the thing on the screen.
const PANEL_WIDTH: f32 = 260.0;

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
        })
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

    /// How wide a string is at a size, in the same pixels [`Self::text`] draws it in.
    ///
    /// A code the face does not map contributes nothing, which is the honest answer: nothing is
    /// drawn for it either, and inventing a width would put the rest of the line in the wrong
    /// place.
    #[must_use]
    pub fn width(&self, text: &str, size: f32, style: Style) -> f32 {
        let face = self.face(style);
        text.chars()
            .filter_map(|character| face.code_for(character))
            .map(|code| face.advance(code) * size)
            .sum()
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
            let Some(code) = face.code_for(character) else {
                continue;
            };
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
            x += face.advance(code) * size;
        }
        x
    }
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
    /// §12.3.2's destination.
    Follow(PageTarget),
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
        }
    }
}

/// Which of the three lists the sidebar is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// §12.3.3's outline.
    #[default]
    Contents,
    /// §8.11.4.3's `/Order`.
    Layers,
    /// §7.11.4's embedded files.
    Files,
}

impl Tab {
    /// The three, in the order they are drawn.
    const ALL: [Self; 3] = [Self::Contents, Self::Layers, Self::Files];

    /// What the tab says.
    const fn label(self) -> &'static str {
        match self {
            Self::Contents => "Contents",
            Self::Layers => "Layers",
            Self::Files => "Files",
        }
    }

    /// Which of [`Self::ALL`] this is, for the per-tab scroll positions.
    const fn index(self) -> usize {
        match self {
            Self::Contents => 0,
            Self::Layers => 1,
            Self::Files => 2,
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
}

/// What a click on the sidebar asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    /// Follow this outline item's §12.3.2 destination.
    Follow(PageTarget),
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
    scroll: [f32; 3],
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

    /// Shows or hides the sidebar.
    pub fn toggle(&mut self) {
        self.shown = !self.shown;
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
            self.rows(content).len() as f32,
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
            Act::Follow(target) => Hit::Follow(target),
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
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a row index derived from a non-negative pixel offset"
        )]
        let index = (offset / row_height) as usize;
        self.rows(content)
            .into_iter()
            .nth(index)
            .map(|row| (index, row))
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
                for file in content.attachments {
                    let mut row = Row::plain(
                        0,
                        file.file_name.clone().unwrap_or_else(|| file.name.clone()),
                    );
                    row.detail = describe(file);
                    out.push(row);
                }
                if out.is_empty() {
                    out.push(nothing("This document embeds no files."));
                }
            }
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
                act: match item.destination {
                    Some(destination) => Act::Follow(PageTarget::Destination(destination)),
                    None => Act::None,
                },
                group: None,
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
        for (index, row) in self.rows(content).into_iter().enumerate() {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a row index, bounded by the list's own length"
            )]
            let top = strip + index as f32 * row_height - self.scrolled() * scale;
            if top + row_height < strip || top > tall {
                continue;
            }
            if self.hovered == Some(index) {
                rectangle(&mut list, (0.0, top, width - scale, row_height), HOVER);
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "a nesting depth, which pdf-model bounds"
            )]
            let indent = (MARKER + INDENT * row.depth as f32) * scale;
            draw_marker(
                &mut list,
                row.marker,
                (indent - MARKER * 0.5 * scale, top + row_height * 0.5),
                scale,
            );
            // Clipped by measurement rather than by a clip path: the label is truncated to what
            // fits and an ellipsis says so, which is what a list of sentences needs. A clip would
            // cut a letter in half and say nothing.
            let baseline = top + row_height * 0.7;
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
        rectangle(&mut list, (0.0, 0.0, width, strip - scale), BACKGROUND);
        let each = width / 3.0;
        for (index, tab) in Tab::ALL.into_iter().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "one of three tabs")]
            let left = index as f32 * each;
            if tab == self.tab {
                rectangle(&mut list, (left, 0.0, each, strip - scale), HOVER);
            }
            let label = tab.label();
            let centred = left + (each - chrome.width(label, size, Style::default())) * 0.5;
            chrome.text(
                &mut list,
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
        rectangle(&mut list, (0.0, strip - scale, width, scale), EDGE);
        rectangle(&mut list, (width - scale, 0.0, scale, tall), EDGE);
        list
    }
}

/// Which tab a horizontal position is in.
fn tab_at(x: f32, scale: f32) -> Tab {
    let each = (PANEL_WIDTH * scale) / 3.0;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a tab index from a position already known to be inside the strip"
    )]
    let index = (x / each.max(1.0)) as usize;
    Tab::ALL.get(index).copied().unwrap_or(Tab::Files)
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
