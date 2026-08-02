//! Text and panels this program draws for itself.
//!
//! Everything here is **this host's own chrome**, and none of it belongs in `viewer-core`. A
//! native host draws §12.3.3's outline in a `QTreeView`, an `NSOutlineView` or a `GtkListView`,
//! with the platform's fonts, its focus ring and its scrollbars, and it gets the data from
//! [`viewer_core::Query::Outline`]. This program has no toolkit — winit is a window and an event
//! loop and nothing else — so it draws the panel itself, and that is the whole reason this module
//! exists.
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
use viewer_core::PageTarget;

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

/// How much of the row's left edge the disclosure triangle owns, in logical pixels.
///
/// A click inside it toggles the subtree instead of following the destination, which is what
/// every outline view does and what §12.3.3's `/Count` sign makes meaningful: a negative count
/// is a closed item, and closed items are the ones with something to disclose.
const DISCLOSURE: f32 = 14.0;

/// The panel's background.
const BACKGROUND: Color = Color {
    r: 0.94,
    g: 0.94,
    b: 0.95,
    a: 1.0,
};

/// The line separating the panel from the page.
const EDGE: Color = Color {
    r: 0.78,
    g: 0.78,
    b: 0.80,
    a: 1.0,
};

/// The row under the pointer.
const HOVER: Color = Color {
    r: 0.86,
    g: 0.88,
    b: 0.93,
    a: 1.0,
};

/// The fonts this program draws its own text with.
///
/// Four faces rather than one because §12.3.3's Table 155 gives an outline item a `/F` whose two
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
    /// Table 155's `/F` bit 2.
    pub bold: bool,
    /// Table 155's `/F` bit 1.
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
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(pdf_render::Point { x, y }));
    path.push(PathCommand::LineTo(pdf_render::Point { x: x + w, y }));
    path.push(PathCommand::LineTo(pdf_render::Point {
        x: x + w,
        y: y + h,
    }));
    path.push(PathCommand::LineTo(pdf_render::Point { x, y: y + h }));
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

/// The disclosure triangle: pointing right when closed, down when open.
fn triangle(list: &mut DisplayList, centre: (f32, f32), size: f32, open: bool, colour: Color) {
    let (cx, cy) = centre;
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
    let mut path = Path::new();
    for (index, (x, y)) in corners.into_iter().enumerate() {
        let point = pdf_render::Point { x, y };
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

/// One row of the panel: an outline item at a depth, with what a click on it does.
#[derive(Debug, Clone)]
struct Row {
    /// Which item, in the pre-order numbering [`Panel`] uses for open and closed state.
    id: usize,
    /// How deep, for the indent.
    depth: usize,
    /// Table 155's `/Title`.
    title: String,
    /// Table 155's `/C`, which §12.3.3 calls "the colour to be used for the outline entry's
    /// text".
    colour: Color,
    /// Table 155's `/F`.
    style: Style,
    /// Whether the item has children, and so a disclosure triangle.
    parent: bool,
    /// §12.3.3's `/Count` sign: what the *document* says about this item being open.
    stated_open: bool,
    /// Where a click goes, where the item states a destination.
    destination: Option<pdf_model::destination::Destination>,
}

/// What a click on the panel asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    /// Follow this item's §12.3.2 destination.
    Follow(PageTarget),
    /// Open or close this item's children, and redraw.
    Toggle,
    /// Inside the panel, on nothing that acts. Swallowed, so it does not reach the page.
    Nothing,
}

/// §12.3.3's outline, as this program draws it.
///
/// Holds only what the *view* owns: which subtrees are open, and how far the list is scrolled.
/// Everything shown comes from [`viewer_core::Query::Outline`] on every frame, so nothing here
/// can go stale against the document.
#[derive(Debug, Default)]
pub struct Panel {
    /// Whether the panel is shown at all.
    pub shown: bool,
    /// Items whose open state differs from the document's own `/Count`.
    ///
    /// A *difference* rather than a set of open items, so that §12.3.3's own answer is the
    /// default: "if the item is open, the total number of its open descendants … if the item is
    /// closed, a negative integer". A document that opens its first two levels opens them here,
    /// and this records only what a person changed.
    toggled: std::collections::BTreeSet<usize>,
    /// How far the list is scrolled, in logical pixels, never negative.
    scroll: f32,
    /// Which row the pointer is over, for the hover highlight.
    hovered: Option<usize>,
}

impl Panel {
    /// The panel's width in device pixels, or zero when it is hidden.
    ///
    /// This is what insets the page: the viewport `viewer-core` is told about is the window less
    /// this, so the page centres itself in what is left rather than behind the panel.
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

    /// Shows or hides the panel, and answers whether anything changed.
    pub fn toggle(&mut self) {
        self.shown = !self.shown;
    }

    /// Scrolls the list by a number of **logical** pixels, clamped to its own ends.
    ///
    /// Clamped rather than wrapped or unbounded, for the same reason `Command::Scroll` clamps a
    /// page: a list scrolled past its last row is a panel showing nothing and saying nothing
    /// about why.
    pub fn scroll(
        &mut self,
        by: f32,
        outline: &pdf_model::outline::Outline,
        height: u32,
        scale: f32,
    ) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a row count and a window height, both thousands at most"
        )]
        let (rows, tall) = (
            self.rows(outline).len() as f32,
            height as f32 / scale.max(0.01),
        );
        let content = rows * (TEXT_SIZE * ROW_HEIGHT);
        let furthest = (content - (tall - HEADER)).max(0.0);
        self.scroll = (self.scroll + by).clamp(0.0, furthest);
    }

    /// Remembers where the pointer is, and answers whether the highlight moved.
    pub fn hover(
        &mut self,
        at: (f32, f32),
        outline: &pdf_model::outline::Outline,
        scale: f32,
    ) -> bool {
        let was = self.hovered;
        self.hovered = self
            .row_at(at, outline, scale)
            .map(|(index, _)| index)
            .filter(|_| self.shown);
        was != self.hovered
    }

    /// What a click at a point in the window does.
    ///
    /// `None` for a point outside the panel, which is the caller's signal to hand it to the
    /// page. Everything inside is answered, including a click on empty space: a panel that let
    /// clicks fall through to the document underneath it would start a text selection on a page
    /// the person cannot see.
    #[must_use]
    pub fn click(
        &mut self,
        at: (f32, f32),
        outline: &pdf_model::outline::Outline,
        scale: f32,
    ) -> Option<Hit> {
        if !self.shown || at.0 >= PANEL_WIDTH * scale {
            return None;
        }
        let Some((_, row)) = self.row_at(at, outline, scale) else {
            return Some(Hit::Nothing);
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a nesting depth, which pdf-model bounds"
        )]
        let indent = (DISCLOSURE + INDENT * row.depth as f32) * scale;
        if row.parent && at.0 < indent {
            if !self.toggled.insert(row.id) {
                self.toggled.remove(&row.id);
            }
            return Some(Hit::Toggle);
        }
        Some(match row.destination {
            Some(destination) => Hit::Follow(PageTarget::Destination(destination)),
            None => Hit::Nothing,
        })
    }

    /// The row under a point, and which of the visible rows it is.
    fn row_at(
        &self,
        at: (f32, f32),
        outline: &pdf_model::outline::Outline,
        scale: f32,
    ) -> Option<(usize, Row)> {
        if !self.shown || at.0 >= PANEL_WIDTH * scale || at.1 < HEADER * scale {
            return None;
        }
        let row_height = TEXT_SIZE * ROW_HEIGHT * scale;
        let offset = at.1 - HEADER * scale + self.scroll * scale;
        if offset < 0.0 {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a row index derived from a non-negative pixel offset"
        )]
        let index = (offset / row_height) as usize;
        self.rows(outline)
            .into_iter()
            .nth(index)
            .map(|row| (index, row))
    }

    /// Whether an item's children are shown.
    ///
    /// §12.3.3's `/Count` states the document's own answer and [`pdf_model::outline::Item::open`]
    /// carries it; the set records only where a person disagreed.
    fn is_open(&self, id: usize, stated: bool) -> bool {
        stated != self.toggled.contains(&id)
    }

    /// The visible rows, in the order they are drawn.
    fn rows(&self, outline: &pdf_model::outline::Outline) -> Vec<Row> {
        let mut out = Vec::new();
        let mut next = 0;
        self.flatten(&outline.items, 0, &mut next, &mut out);
        out
    }

    /// [`Self::rows`], one level at a time.
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
            out.push(Row {
                id,
                depth,
                title: item.title.clone(),
                colour: Color {
                    r: item.colour[0],
                    g: item.colour[1],
                    b: item.colour[2],
                    a: 1.0,
                },
                style: Style {
                    bold: item.bold,
                    italic: item.italic,
                },
                parent: !item.children.is_empty(),
                stated_open: item.open,
                destination: item.destination,
            });
            if item.children.is_empty() {
                continue;
            }
            if self.is_open(id, item.open) {
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

    /// The display list for the panel, in device pixels of the window.
    #[must_use]
    pub fn draw(
        &self,
        chrome: &Chrome,
        outline: &pdf_model::outline::Outline,
        height: u32,
        scale: f32,
    ) -> DisplayList {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window height in pixels, which is thousands and not billions"
        )]
        let tall = height as f32;
        let width = PANEL_WIDTH * scale;
        // The list's "page size" is the panel itself: nothing here is a page, and what the
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
        let header = HEADER * scale;
        let row_height = TEXT_SIZE * ROW_HEIGHT * scale;
        for (index, row) in self.rows(outline).into_iter().enumerate() {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a row index, bounded by the outline's item count"
            )]
            let top = header + index as f32 * row_height - self.scroll * scale;
            if top + row_height < header || top > tall {
                continue;
            }
            if self.hovered == Some(index) {
                rectangle(&mut list, (0.0, top, width - scale, row_height), HOVER);
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "a nesting depth, which pdf-model bounds"
            )]
            let indent = (DISCLOSURE + INDENT * row.depth as f32) * scale;
            if row.parent {
                triangle(
                    &mut list,
                    (indent - DISCLOSURE * 0.5 * scale, top + row_height * 0.5),
                    7.0 * scale,
                    self.is_open(row.id, row.stated_open),
                    EDGE,
                );
            }
            // Clipped by measurement rather than by a clip path: the title is truncated to what
            // fits and an ellipsis says so, which is what a list of sentences needs. A clip would
            // cut a letter in half and say nothing.
            let room = width - indent - 8.0 * scale;
            let title = elide(chrome, &row.title, size, row.style, room);
            chrome.text(
                &mut list,
                &title,
                (indent, top + row_height * 0.7),
                size,
                row.style,
                row.colour,
            );
        }

        // The heading is drawn *last*, over its own background: a scrolled row's top half would
        // otherwise appear above the separator, which is what the first run of this panel did.
        // A clip would do the same job and would cut the letters in half rather than hide them.
        rectangle(&mut list, (0.0, 0.0, width, header - scale), BACKGROUND);
        chrome.text(
            &mut list,
            "Contents",
            (12.0 * scale, header - 10.0 * scale),
            size,
            Style {
                bold: true,
                italic: false,
            },
            Color::BLACK,
        );
        rectangle(&mut list, (0.0, header - scale, width, scale), EDGE);
        rectangle(&mut list, (width - scale, 0.0, scale, tall), EDGE);
        list
    }
}

/// How tall the panel's heading is, in logical pixels.
const HEADER: f32 = TEXT_SIZE * ROW_HEIGHT + 6.0;

/// Every item in a subtree, including the ones a closed parent hides.
fn count(items: &[pdf_model::outline::Item], out: &mut usize) {
    for item in items {
        *out = out.saturating_add(1);
        count(&item.children, out);
    }
}

/// A title cut to the width available, with `…` where anything was dropped.
fn elide(chrome: &Chrome, title: &str, size: f32, style: Style, room: f32) -> String {
    // A title is one line: §12.3.3 states no wrapping, and a newline inside a `/Title` would
    // otherwise leave the rest of the row's text drawn on top of itself.
    let title = title.replace(['\n', '\r', '\t'], " ");
    if chrome.width(&title, size, style) <= room {
        return title;
    }
    let ellipsis = chrome.width("…", size, style);
    let mut kept = String::new();
    let mut used = 0.0;
    for character in title.chars() {
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
