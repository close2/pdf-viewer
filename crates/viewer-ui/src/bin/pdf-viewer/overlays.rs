//! What is drawn over the page: geometry the core answered with, in this host's own colours.
//!
//! `doc/ui-boundary.md`'s rule that interactive chrome crosses as *geometry* is what this module
//! is: every shape here arrives from a query in the window's own device pixels, and the only
//! decisions taken are which colour it is drawn in and what order the lists go in. A native host
//! would ask its platform for the colours and draw the same shapes; this one has nobody to ask
//! and says so at each constant.

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command as DrawCommand, FillRule, Paint, Path, PathCommand, Point, Size,
    Transform,
};
use viewer_core::{Answer, Query};

use crate::app::App;
use crate::trace::Topic;

/// The colour §12.5.1's focus ring is drawn in.
///
/// A choice, and the only one available: the clause says nothing about showing a focus and this
/// host has no theme to ask. A native host uses its platform's ring and never sees this constant.
const FOCUS_RING: Color = Color {
    r: 0.10,
    g: 0.42,
    b: 0.85,
    a: 1.0,
};

/// How wide that ring is, in device pixels.
const FOCUS_RING_WIDTH: f32 = 2.0;

/// The colour the caret is drawn in.
///
/// A choice, and for the same reason the focus ring's is: no clause states a text cursor at all.
/// Black rather than the ring's blue, because a caret stands *in* the text and a person reads it
/// as part of the line — and this host has no theme to ask for the platform's insertion-point
/// colour, which a native one would use instead.
const CARET: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// How wide the caret is, in device pixels.
const CARET_WIDTH: f32 = 2.0;

/// The colour a selection is washed in.
///
/// A choice this host has to make and a native one does not: GTK draws the theme's foreground and
/// Qt draws `QPalette::Highlight`, and there is no platform here to ask.
const SELECTION: Color = Color {
    r: 140.0 / 255.0,
    g: 180.0 / 255.0,
    b: 1.0,
    a: 1.0,
};

/// The colour ISO 32000-2 Annex O's `highlight` rectangle is washed in.
///
/// Table Annex O.4 says "[t]he nature of the highlighting is implementation-dependent" outright,
/// so this is the one overlay whose colour the standard *hands* to a processor rather than leaving
/// unsaid. A third hue for the same reason the find bar's is a second one: "the rectangle the URI
/// asked for", "where else the word is" and "what you have selected" are three different
/// statements, and a person should not have to work out which wash is which. Green, because the
/// other two are blue and yellow.
const ANNEX_O_HIGHLIGHT: Color = Color {
    r: 0.60,
    g: 1.0,
    b: 0.62,
    a: 1.0,
};

impl App {
    /// Draws the outstanding request onto the surface and presents it.
    ///
    /// Returns what to tell the core, or `None` where there is nothing to tell it: a redraw the
    /// swapchain gave back never reached a screen, and saying it did would leave the window
    /// showing the last page until something else happened to change.
    ///
    /// **That last sentence is the whole reason this returns an outcome rather than a `bool`.**
    /// A draw that *fails* used to print a line to stdout and answer `Presented`, so the core
    /// recorded the page as shown, never asked again, and the window kept the previous page under
    /// a title bar naming the new one. A person looking at the window saw a page that would not
    /// change and no reason why. Trap 5, on a path a person reaches with an arrow key.
    /// The selection's shapes, in the window's own pixels, or `None` when nothing is selected.
    ///
    /// Interactive chrome crosses as geometry, not pixels: the core hands over the shapes and this
    /// host draws them in its own colour. A native one would use macOS's selection colour, KDE's
    /// accent or the Windows highlight brush; this one has no theme to ask, so it picks a blue and
    /// says so.
    pub(crate) fn selection_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let mut quads = match self.viewer.query(Query::Selection) {
            Answer::Selected(selection) => selection.quads,
            _ => Vec::new(),
        };
        // The quads are device pixels of the *page's* viewport, which begins where the panel
        // ends. One addition here rather than a second coordinate space in the core.
        for quad in &mut quads {
            for x in quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        // The number every part of `doc/todo/13` turned on: the frame the compositor refused was
        // 63 quads, and a present cost 1.9 ms a quad before it. Kept in the tree so that a
        // selection's cost stays visible rather than being rediscovered. On stdout with every
        // other trace line since the three-hundred-and-ninetieth: it was the one on stderr, and
        // PowerShell wrapped each of its lines in six of its own in the trace that raised
        // ADR 0227.
        self.trace.say(
            Topic::Selection,
            format_args!("SELECTION quads {}", quads.len()),
        );
        highlight_list(&quads, SELECTION, width, height)
    }

    /// ISO 32000-2 Annex O's `highlight`: the rectangle the URI's fragment asked to be shown.
    ///
    /// Table Annex O.4: "Open the document with the specified rectangle highlighted." The core
    /// answers with the shapes on the page being shown, in this window's own device pixels, and
    /// the annex leaves what they look like to a processor — so this host washes them in a colour
    /// of its own, exactly as it does a selection and a search's matches. Under everything else
    /// for the same reason the matches are: it belongs to the page rather than to what a person is
    /// doing now.
    ///
    /// `None` for every document opened without a fragment naming one, which is nearly all of
    /// them, and for a page other than the one the rectangle was measured on.
    pub(crate) fn annex_o_highlight_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let Answer::Highlighted(mut quads) = self.viewer.query(Query::Highlight) else {
            return None;
        };
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same one
        // addition `selection_list` and `matches_list` make.
        for quad in &mut quads {
            for x in quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        highlight_list(&quads, ANNEX_O_HIGHLIGHT, width, height)
    }

    /// §12.5.6.14's popup windows, over the page and under the sidebar.
    ///
    /// The core says which windows are open, where they are and what they say; this host decides
    /// what a window looks like, because the clause describes none of that — see
    /// `chrome::popup_windows`. Over the page and *under* the panel, which is the order the
    /// overlays already state: a window belongs to the document and the sidebar belongs to the
    /// program.
    pub(crate) fn popup_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        let Answer::Popups(mut windows) = self.viewer.query(Query::Popups) else {
            return None;
        };
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same
        // one addition `selection_list` makes, and for the same reason.
        for window in &mut windows {
            for x in window.quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        viewer_ui::chrome::popup_windows(chrome, &windows, width, height, scale)
    }

    /// §12.5.1's focus ring: a stroked box round whatever the tab key last landed on.
    ///
    /// The clause lets a processor walk the annotations with the tab key and says nothing about
    /// showing which one a person is on — so the ring is entirely this host's, in this host's own
    /// colour, and a native one would use its platform's focus ring instead. What it is *not* is
    /// this host's arithmetic: `Query::Focus` answers with the quadrilateral in the viewport's
    /// own pixels, for the same reason `Query::Selection` does.
    pub(crate) fn focus_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let Answer::Focus { quad, .. } = self.viewer.query(Query::Focus) else {
            return None;
        };
        let mut quad = quad;
        for x in quad.iter_mut().step_by(2) {
            *x += edge;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
        let mut path = Path::new();
        for (index, corner) in quad.chunks_exact(2).enumerate() {
            let point = Point::new(corner[0], corner[1]);
            path.push(if index == 0 {
                PathCommand::MoveTo(point)
            } else {
                PathCommand::LineTo(point)
            });
        }
        path.push(PathCommand::Close);
        list.push(DrawCommand::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke: pdf_render::Stroke {
                width: FOCUS_RING_WIDTH,
                ..pdf_render::Stroke::default()
            },
            paint: Paint::Solid(FOCUS_RING),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        Some(list)
    }

    /// What is selected *inside* a field's value, in the window's own pixels.
    ///
    /// The same blue `Query::Selection`'s shapes are drawn in, deliberately: a person sweeping
    /// text on the page and a person sweeping text in a field are doing one thing, and two colours
    /// would say they were doing two. Where the shapes come from is the core — `Query::FieldSelection`
    /// answers one quadrilateral per line, because §12.7.5.3's Multiline flag lets the layout break
    /// a value where this host cannot see (ADR 0225).
    ///
    /// `None` while nothing is selected, which is a caret's ordinary state: the two offsets are
    /// equal and the core answers with no shapes at all.
    pub(crate) fn field_selection_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let typing = self.typing?;
        let (from, to) = typing.range();
        if from == to {
            return None;
        }
        let Answer::FieldSelection(mut quads) = self.viewer.query(Query::FieldSelection {
            at: typing.at,
            from,
            to,
        }) else {
            return None;
        };
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same one
        // addition `selection_list`, `focus_list` and `caret_list` make.
        for quad in &mut quads {
            for x in quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        highlight_list(&quads, SELECTION, width, height)
    }

    /// The caret: a line where the next character will be drawn, while a field has the keyboard.
    ///
    /// **The standard states no caret**, and §12.5.6.11's caret *annotation* is a different thing
    /// entirely — so its width, its colour and whether it blinks are this host's, exactly as
    /// §12.5.1's focus ring is. This one is a steady line two pixels wide: a blink needs a clock,
    /// and `viewer-core` has none by rule 3, so a host that wanted one would drive it from its own
    /// timer. What is *not* this host's is where it goes — `Query::Caret` answers that from
    /// §12.7.4.3's own layout, because a host laying the value out again to find the place would
    /// be a second opinion about the field's font, its auto-sizing and its wrapping. ADR 0211.
    pub(crate) fn caret_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let typing = self.typing?;
        let Answer::Caret { from, to } = self.viewer.query(Query::Caret {
            at: typing.at,
            offset: typing.caret,
        }) else {
            return None;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
        let mut path = Path::new();
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same
        // one addition `selection_list` and `focus_list` make.
        path.push(PathCommand::MoveTo(Point::new(from.0 + edge, from.1)));
        path.push(PathCommand::LineTo(Point::new(to.0 + edge, to.1)));
        list.push(DrawCommand::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke: pdf_render::Stroke {
                width: CARET_WIDTH,
                ..pdf_render::Stroke::default()
            },
            paint: Paint::Solid(CARET),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        Some(list)
    }
}

/// Lays a list of shapes over the page, washed in one colour.
///
/// Three overlays are the same drawing — what is selected, where else the find bar's string is,
/// and Annex O's highlighted rectangle — and they differ only in the colour, which is why that is
/// the argument and why there is one of these rather than three.
///
/// The quadrilaterals arrive from `viewer-core` in device pixels of this window, so nothing here
/// composes a transform: that is the whole point of chrome crossing as geometry rather than as
/// pixels. Drawn with `Multiply`, which darkens what is under it and leaves the glyphs readable —
/// §11.3.5.2 makes it the one mode whose "result colour is always at least as dark as either of
/// the two constituent colours", so the text under the wash survives it. A native host asks its
/// platform for the colour; this one has nobody to ask, and a hard-coded blue that says so is
/// better than one that pretends.
///
/// **One fill, one subpath per quad**, and the count matters rather than the shape: a compositor
/// gives every non-`Over` blend its own layer and prices its internal textures before allocating
/// them, so a fill per quad made a selection cost `(quads + 1) × 2 × width × height × 4` bytes of
/// frame budget — 6.4 MB a quad at 800 × 1000, spending a 256 MiB budget at 63 quads, which is one
/// short paragraph. Under one layer the cost stops depending on what is selected at all. The
/// per-quad blend it replaces was preserving something nobody wants: `Query::Selection` answers
/// one quad per *run*, runs tile rather than overlap, and the two overlapping pairs out of 171
/// measured on three lines of `tracemonkey.pdf` overlap by 0.28 and 0.17 of a device pixel. Under
/// the non-zero rule one path is one shape, so those slivers stop darkening twice as well.
pub(crate) fn highlight_list(
    quads: &[[f32; 8]],
    colour: Color,
    width: u32,
    height: u32,
) -> Option<pdf_render::DisplayList> {
    if quads.is_empty() {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
    let mut path = Path::new();
    for quad in quads {
        for (index, corner) in quad.chunks_exact(2).enumerate() {
            let point = Point::new(corner[0], corner[1]);
            path.push(if index == 0 {
                PathCommand::MoveTo(point)
            } else {
                PathCommand::LineTo(point)
            });
        }
        path.push(PathCommand::Close);
    }
    list.push(DrawCommand::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Multiply,
    });
    Some(list)
}

/// The chrome drawn over a page, as display lists in the window's own pixels.
///
/// Gathered once per frame and handed to the presenter beside the page, which is why they are
/// display lists and not backend calls: they draw through the same translation as the page
/// itself, and that is what a `--cpu` run and a page the graphics device refuses both need.
///
/// Held in one value so that the borrowed lists [`Self::lists`] hands the presenter outlive the
/// call: each of these is built for this frame and dropped after it.
#[derive(Default)]
pub(crate) struct Overlays {
    /// Annex O's highlighted rectangle, under everything: it says how the document was opened.
    annex_o_highlight: Option<pdf_render::DisplayList>,
    /// Every occurrence of the find bar's string on this page, under the selection.
    matches: Option<pdf_render::DisplayList>,
    /// What is selected on the page, which belongs to the page and so is under everything else.
    selection: Option<pdf_render::DisplayList>,
    /// What is selected inside a form field (ADR 0225).
    field_selection: Option<pdf_render::DisplayList>,
    /// §12.5.1's focus ring, round whatever the tab key last landed on.
    focus: Option<pdf_render::DisplayList>,
    /// §12.7.4.3's caret, where the next character goes (ADR 0211).
    caret: Option<pdf_render::DisplayList>,
    /// §12.5.6.14's popup windows, which belong to the document and so are under the sidebar.
    popups: Option<pdf_render::DisplayList>,
    /// The sidebar, where it is shown.
    panel: Option<pdf_render::DisplayList>,
    /// The find bar, over the sidebar and under the modal card.
    find: Option<pdf_render::DisplayList>,
    /// `/NOTICE`, where it is shown. **Last, so it is on top**: it is a modal card and the
    /// sidebar is behind it.
    about: Option<pdf_render::DisplayList>,
}

impl Overlays {
    /// Builds every one of them for this frame.
    pub(crate) fn of(app: &App, edge: f32, width: u32, height: u32) -> Self {
        Self {
            annex_o_highlight: app.annex_o_highlight_list(edge, width, height),
            matches: app.matches_list(edge, width, height),
            selection: app.selection_list(edge, width, height),
            field_selection: app.field_selection_list(edge, width, height),
            focus: app.focus_list(edge, width, height),
            caret: app.caret_list(edge, width, height),
            popups: app.popup_list(edge, width, height),
            panel: app.panel_list(height),
            find: app.find_list(width),
            about: app.about_list(width, height),
        }
    }

    /// The ones there are, in the order they are drawn: selection first (it belongs to the page),
    /// then the sidebar, then the modal card on top — the order the Vello host drew them in.
    pub(crate) fn lists(&self) -> Vec<&pdf_render::DisplayList> {
        [
            self.annex_o_highlight.as_ref(),
            self.matches.as_ref(),
            self.selection.as_ref(),
            self.field_selection.as_ref(),
            self.focus.as_ref(),
            self.caret.as_ref(),
            self.popups.as_ref(),
            self.panel.as_ref(),
            self.find.as_ref(),
            self.about.as_ref(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}
