//! ISO 32000-2 §12.4.4's transition, one frame at a time.
//!
//! §12.4.4.1 says what a transition is and Table 164 says which ones exist:
//!
//! > The Trans entry shall contain a transition dictionary describing the style and duration of
//! > the visual transition to use when moving from another page to the given page during a
//! > presentation.
//!
//! `pdf_model::navigation` reads that dictionary; [`crate::Event::Transition`] names it; this is
//! what a frame of one *looks like*. The division is rule 3's: **the shape of a frame is this
//! crate's and the clock is the host's.** A host asks for the frame at a fraction of the way
//! through, so nothing here knows what a second is, and the same fraction produces the same
//! frame in a test with no display as in a window.
//!
//! # What the standard states, and what it does not
//!
//! Table 164 states which styles exist, what each one's `/Dm`, `/M`, `/Di`, `/SS` and `/B` mean,
//! and that `/D` is "[t]he duration of the transition effect, in seconds". It states **not one
//! word about what a frame in the middle looks like**: no timing curve, no line count for
//! `Blinds`, no band width for `Glitter`, no dissolve pattern. So every geometric choice below is
//! a *choice*, recorded as one, in the manner ADR 0211 recorded a caret's colour and ADR 0225 its
//! placement — and the choices are confined to the two questions the clause leaves open:
//!
//! - **Progress is linear in time.** A host divides elapsed time by `/D`; nothing here reads a
//!   curve into the clause that the clause does not state.
//! - **A sweep reveals what it has passed over.** Table 164's verbs are "sweep across the
//!   screen, revealing the new page" and "slides on to the screen … covering the old page", so
//!   the swept or covered area shows the page being moved to, and the rest shows the page being
//!   left. That is a reading of the table's own sentences rather than an invention, and it is the
//!   only part of this module that claims to be.
//!
//! # Which styles are shaped here, and why those
//!
//! **The seven whose frame is determined by Table 164's own words**: `Wipe`, `Split`, `Box`,
//! `Cover`, `Uncover`, `Push` and `Fade`. Each one is a placement of two pages plus a rectangular
//! region — nothing in any of them needs a number the standard does not give.
//!
//! The other five are **named and reported rather than silently cut**, which is trap 5's rule
//! everywhere else in this tree, and each is left for the same kind of reason: it needs a
//! quantity Table 164 does not state.
//!
//! | style | what the clause does not say |
//! |---|---|
//! | `Blinds` | how many "[m]ultiple lines, evenly spaced across the screen" there are |
//! | `Glitter` | how wide "a wide band" is, and what a dissolve looks like inside it |
//! | `Dissolve` | what "dissolves gradually" does to a pixel between the two pages |
//! | `Fly` | what "[c]hanges" are — the flown object is the *difference* between two pages |
//! | `R` | nothing: it is a cut by the table's own definition, "no special transition effect" |
//!
//! `R` is therefore not reported. The other four are, by name, so that a person watching a slide
//! show that cuts knows the file asked for something this reader does not draw.
//!
//! **A style is not the whole of what decides a frame, and for four of the seven the direction
//! decides it too.** `Wipe`, `Cover`, `Uncover` and `Push` travel along `/Di`, and every value
//! Table 164 gives those four is a quarter turn — 0 and 270 for the three that slide, and 90 and
//! 180 as well for `Wipe`, with 315 belonging to `Glitter` and the name `None` to `Fly`. A file
//! stating any other direction has asked for an effect no rectangle here sweeps, so [`frame`]
//! shapes none — and [`note`] therefore asks the *whole* transition rather than its style, since
//! a report keyed on less than what decides the drawing is a report that fires on the wrong
//! condition (trap 11).
//!
//! # What a host does with this
//!
//! Two pages' pixels and a fraction in, one [`pdf_render::DisplayList`] out. The list holds at
//! most two image commands, so **both backends draw it** — which is what keeps the CPU
//! rasteriser's job as the frame a graphics device refuses (`CLAUDE.md`'s startup rules), and is
//! the same reason the sidebar and the caret cross as display lists rather than as pixels.

use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
use pdf_render::{
    BlendMode, Clip, Command, DisplayList, DisplayListError, FillRule, Image, ImageSource, Path,
    PathCommand, Point, Raster, RasterFormat, Rect, Size, Transform,
};

/// Which of a transition's two pages a [`Layer`] draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// The page being left — what the screen showed before the presentation advanced.
    Outgoing,
    /// The page being moved to, which §12.4.4.1 makes the one whose `/Trans` this is.
    Incoming,
}

/// One page's placement in one frame of a transition.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// Which page this draws.
    pub face: Face,
    /// Where the page goes, in device pixels, relative to the viewport it was rasterised for.
    ///
    /// `(0.0, 0.0)` for every style but the three that slide. A page offset by the viewport's
    /// full width is entirely off the screen, which is what the end of a `Push` is.
    pub offset: (f32, f32),
    /// Constant alpha in `0.0..=1.0`, which only `Fade` moves off 1.
    pub alpha: f32,
    /// The parts of the viewport this layer marks, in device pixels.
    ///
    /// One rectangle for a whole page, two for a `Split` sweeping inward, four for a `Box`
    /// doing the same — a `Box`'s inward sweep reveals the *complement* of a shrinking
    /// rectangle, and a complement of a rectangle inside a rectangle is four bands. Empty means
    /// the layer marks nothing at all, which is what the first frame of an outward `Box` is.
    pub reveal: Vec<Rect>,
}

/// One frame of a transition: what to draw, and in what order.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The layers, back to front. At most two, and both faces appear in every frame of every
    /// style shaped here — a transition that showed only one page would be a cut.
    pub layers: Vec<Layer>,
}

impl Frame {
    /// Turns this frame into commands, over the two pages' pixels.
    ///
    /// Both images are the page rasterised for the whole viewport — the same
    /// [`pdf_render::TargetSpec`] a frame outside a transition uses — so this places each of
    /// them by mapping the unit square onto the viewport, offset by the layer's own offset. A
    /// host converts each raster **once per transition** with [`drawable`] and draws every frame
    /// from the result: [`pdf_render::Image`] holds its samples behind an `Arc`, and the GPU
    /// backend's caches are keyed by that pointer, so two pages cross to the device once each
    /// however many frames the transition lasts.
    ///
    /// # Errors
    ///
    /// [`DisplayListError::TooManyClips`] cannot arise from a frame — the largest one here adds
    /// four clips — but the list's own limit is stated in its type and swallowing it would be a
    /// silence this project does not permit anywhere else.
    pub fn draw(
        &self,
        viewport: Rect,
        outgoing: &Image,
        incoming: &Image,
    ) -> Result<DisplayList, DisplayListError> {
        let mut list = DisplayList::new(Size::new(viewport.max.x, viewport.max.y));
        for layer in &self.layers {
            if layer.reveal.is_empty() || layer.alpha <= 0.0 {
                continue;
            }
            let mut path = Path::new();
            for rect in &layer.reveal {
                push_rect(&mut path, *rect);
            }
            let clip = list.add_clip(Clip {
                path,
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })?;
            let image = match layer.face {
                Face::Outgoing => outgoing,
                Face::Incoming => incoming,
            };
            list.push(Command::Image {
                image: ImageSource::Decoded(image.clone()),
                // The unit square onto the viewport, then wherever the layer slid it.
                //
                // **The y scale is negative and that is the whole of the flip.** A
                // `Command::Image` draws "the unit square in user space, with the image's
                // *top* row at y = 1", because PDF's user space has y growing upward; these
                // pixels are a *device* raster whose first row is the top one. A positive scale
                // would stand every frame of every transition on its head, which is invisible in
                // a page of flat colour and was found by looking at a real window.
                transform: Transform::scale(viewport.width(), -viewport.height()).then(
                    Transform::translate(
                        viewport.min.x + layer.offset.0,
                        viewport.min.y + layer.offset.1 + viewport.height(),
                    ),
                ),
                alpha: layer.alpha,
                clip: Some(clip),
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        Ok(list)
    }
}

/// A rasterised page as something a display list can draw, or `None` for a layout this cannot.
///
/// The copy is one page of pixels and happens once per transition rather than once per frame; see
/// [`Frame::draw`]. `None` for a raster that is not [`RasterFormat::Rgba8`] — the format is
/// `#[non_exhaustive]`, so a second layout can arrive without this file changing, and drawing
/// bytes under the wrong interpretation would put a plausible-looking wrong page on the screen.
#[must_use]
pub fn drawable(raster: &Raster) -> Option<Image> {
    if raster.format != RasterFormat::Rgba8 || raster.width == 0 || raster.height == 0 {
        return None;
    }
    Some(Image {
        width: raster.width,
        height: raster.height,
        data: raster.data.as_slice().into(),
        // §8.9.5.3's `/Interpolate` is about a low-resolution image blown up; this one is the
        // viewport's own pixels drawn at the viewport's own size, so there is nothing to smooth.
        interpolate: false,
    })
}

/// The frame of `transition` at `progress` of the way through it, or `None` for a style this
/// does not shape.
///
/// `progress` is a fraction: 0 is the moment the transition begins, showing the outgoing page,
/// and 1 is its end, showing the incoming one. A host divides elapsed time by Table 164's `/D`
/// and passes the result, which is the whole of what this crate knows about time (rule 3).
///
/// `viewport` is where the page sits in the host's own device pixels. Values outside `0..=1` are
/// clamped, and a fraction that is not a finite number is taken as 1 — a clock that has produced
/// nonsense ends the transition rather than freezing it, since the end state is the page the
/// document asked to arrive at.
///
/// `None` is the answer for the four styles Table 164 describes with a quantity it does not
/// state, for `R`, which the table defines as a cut, and for one of the four styles that travel
/// along `/Di` asked for in a direction the table does not give it. See this module's own
/// documentation for the list and [`note`] for what a person is told — every `None` here but
/// `R`'s has a sentence there, and that is a property this module tests rather than asserts.
#[must_use]
pub fn frame(transition: &Transition, viewport: Rect, progress: f32) -> Option<Frame> {
    let done = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let layers = match transition.style {
        // "A single line sweeps across the screen from one edge to the other in the direction
        // specified by the Di entry, revealing the new page."
        Style::Wipe => revealing(
            viewport,
            vec![swept_from(viewport, quarter(transition.direction)?, done)],
        ),
        // "Two lines sweep across the screen, revealing the new page. The lines may be either
        // horizontal or vertical and may move inward from the edges of the page or outward from
        // the centre, as specified by the Dm and M entries, respectively."
        //
        // Horizontal lines run across the screen and therefore travel *vertically*, which is
        // what makes `/Dm H` divide the height.
        Style::Split => {
            let across = transition.dimension == Dimension::Horizontal;
            revealing(
                viewport,
                match transition.motion {
                    Motion::Outward => vec![middle_band(viewport, done, across)],
                    Motion::Inward => edge_bands(viewport, done, across),
                },
            )
        }
        // "A rectangular box sweeps inward from the edges of the page or outward from the centre,
        // as specified by the M entry, revealing the new page."
        Style::Box => revealing(
            viewport,
            match transition.motion {
                Motion::Outward => vec![centred(viewport, done)],
                // Inward from the edges: what the box has passed over is everything outside the
                // rectangle it has shrunk to, which is four bands round a rectangle.
                Motion::Inward => outside(viewport, centred(viewport, 1.0 - done)),
            },
        ),
        // "The new page slides on to the screen in the direction specified by Di, covering the
        // old page." So the old page does not move and the new one arrives from the edge the
        // motion points away from.
        Style::Cover => vec![
            Layer::whole(Face::Outgoing, viewport),
            Layer::slid(
                Face::Incoming,
                viewport,
                travelled(transition, viewport, done - 1.0)?,
            ),
        ],
        // "The old page slides off the screen in the direction specified by Di, uncovering the
        // new page" — so the new page is underneath from the first frame and the old one leaves.
        Style::Uncover => vec![
            Layer::whole(Face::Incoming, viewport),
            Layer::slid(
                Face::Outgoing,
                viewport,
                travelled(transition, viewport, done)?,
            ),
        ],
        // "The old page slides off the screen while the new page slides in, pushing the old page
        // out in the direction specified by Di." Both move, one page apart, so nothing of either
        // is ever hidden by the other.
        Style::Push => vec![
            Layer::slid(
                Face::Outgoing,
                viewport,
                travelled(transition, viewport, done)?,
            ),
            Layer::slid(
                Face::Incoming,
                viewport,
                travelled(transition, viewport, done - 1.0)?,
            ),
        ],
        // "The new page gradually becomes visible through the old one" — the one style whose
        // frame is an opacity rather than a region.
        Style::Fade => vec![
            Layer::whole(Face::Outgoing, viewport),
            Layer {
                face: Face::Incoming,
                offset: (0.0, 0.0),
                alpha: done,
                reveal: vec![viewport],
            },
        ],
        Style::Blinds
        | Style::Glitter
        | Style::Dissolve
        | Style::Fly
        | Style::Replace
        | Style::Unrecognised(_) => return None,
    };
    Some(Frame { layers })
}

/// The whole outgoing page with the incoming one showing through `reveal`, which is the shape of
/// every style that sweeps rather than slides.
fn revealing(viewport: Rect, reveal: Vec<Rect>) -> Vec<Layer> {
    vec![
        Layer::whole(Face::Outgoing, viewport),
        Layer {
            face: Face::Incoming,
            offset: (0.0, 0.0),
            alpha: 1.0,
            reveal,
        },
    ]
}

/// How far a page has moved when it is `fraction` of a viewport along `/Di`.
///
/// A negative fraction is a page that has not arrived yet, which is what the incoming page of a
/// `Cover` or a `Push` is doing: one whole viewport behind where it will come to rest.
fn travelled(transition: &Transition, viewport: Rect, fraction: f32) -> Option<(f32, f32)> {
    let (dx, dy) = travel(
        quarter(transition.direction)?,
        viewport.width(),
        viewport.height(),
    );
    Some((dx * fraction, dy * fraction))
}

/// What to tell a person about a transition no frame is shaped for, or `None` where there is
/// nothing to say.
///
/// Trap 5's channel and this tree's rule everywhere else: a named effect that is silently drawn
/// as a cut is indistinguishable from a file that asked for a cut. `R` is the one style with
/// nothing to report, because Table 164 defines it as the cut — "[t]he new page simply replaces
/// the old one with no special transition effect".
///
/// **This asks the whole transition rather than its style**, because [`frame`] does: four of the
/// seven styles it shapes need a direction it can sweep along, and until the
/// seven-hundred-and-twentieth session a `/Di` outside the four quarter turns produced no frame
/// and no sentence at all. `note` and `frame` are one decision made in two expressions, which is
/// why [`the_report_fires_on_exactly_what_is_not_drawn`] holds them against each other over every
/// style rather than against a list written out by hand.
///
/// [`the_report_fires_on_exactly_what_is_not_drawn`]: tests::the_report_fires_on_exactly_what_is_not_drawn
pub(crate) fn note(transition: &Transition) -> Option<String> {
    let missing = match &transition.style {
        Style::Blinds => "how many evenly spaced lines it has",
        Style::Glitter => "how wide its band is",
        Style::Dissolve => "what dissolving does to a pixel",
        Style::Fly => "what the changes are that fly",
        // A name the table does not define, in the file's own spelling — except where the file
        // wrote the empty name, which is a legal PDF name and reads as a bare `/` in a sentence
        // a person is meant to understand. It is not a hypothetical: the empty `/S` is the
        // *only* unrecognised style `examples/presentation_census` finds anywhere in the
        // `CC-MAIN-2021-31` crawl, on 106 pages of seven documents.
        Style::Unrecognised(name) if name.as_bytes().is_empty() => {
            return Some(
                "transition: an empty /S is not one of Table 164's styles, so the page is shown \
                 at once"
                    .to_owned(),
            );
        }
        Style::Unrecognised(name) => {
            return Some(format!(
                "transition: /{} is not one of Table 164's styles, so the page is shown at once",
                String::from_utf8_lossy(name.as_bytes())
            ));
        }
        // The four that travel along `/Di`: shaped where the direction is one Table 164 gives
        // them, and reported where it is not.
        Style::Wipe | Style::Cover | Style::Uncover | Style::Push => return askew(transition),
        // The three whose frame needs no direction, and `R`, which is the cut the table defines.
        Style::Split | Style::Box | Style::Fade | Style::Replace => return None,
    };
    Some(format!(
        "transition: /{} is named but not drawn — ISO 32000-2 Table 164 does not state {missing} \
         — so the page is shown at once",
        spelling(&transition.style)
    ))
}

/// What to tell a person about an effect this module shapes, asked for in a direction it cannot.
///
/// Every `/Di` Table 164 gives `Wipe`, `Cover`, `Uncover` and `Push` is a quarter turn: 0 and 270
/// for all four, 90 and 180 for `Wipe` alone, with 315 reserved to `Glitter` and the name `None`
/// to `Fly`. [`quarter`] is the expression that says so and is the one [`frame`] refuses on, so
/// this asks it rather than restating the list — the duplicate that has to agree is the defect
/// this function was added to close, not one to introduce a second time.
///
/// `None` where the direction is one of the four, which is every conforming file: a document has
/// to state a direction the table does not give the style it stated for this to say anything.
fn askew(transition: &Transition) -> Option<String> {
    if quarter(transition.direction).is_some() {
        return None;
    }
    let stated = match transition.direction {
        Direction::None => "the name /None".to_owned(),
        Direction::Degrees(degrees) => format!("{degrees} degrees"),
    };
    Some(format!(
        "transition: /{} is named but not drawn — its /Di is {stated}, which ISO 32000-2 \
         Table 164 does not give that style — so the page is shown at once",
        spelling(&transition.style)
    ))
}

/// Table 164's own spelling of a style, for a sentence a person reads beside the file.
fn spelling(style: &Style) -> &'static str {
    match style {
        Style::Split => "Split",
        Style::Blinds => "Blinds",
        Style::Box => "Box",
        Style::Wipe => "Wipe",
        Style::Dissolve => "Dissolve",
        Style::Glitter => "Glitter",
        Style::Replace => "R",
        Style::Fly => "Fly",
        Style::Push => "Push",
        Style::Cover => "Cover",
        Style::Uncover => "Uncover",
        Style::Fade => "Fade",
        Style::Unrecognised(_) => "",
    }
}

impl Layer {
    /// A face drawn where it was rasterised, whole and opaque.
    fn whole(face: Face, viewport: Rect) -> Self {
        Self {
            face,
            offset: (0.0, 0.0),
            alpha: 1.0,
            reveal: vec![viewport],
        }
    }

    /// A whole face moved off where it was rasterised, which is what the three sliding styles do.
    ///
    /// The reveal stays the viewport: what is drawn is the whole page, and what a person sees of
    /// it is however much of it the viewport still contains.
    fn slid(face: Face, viewport: Rect, offset: (f32, f32)) -> Self {
        Self {
            face,
            offset,
            alpha: 1.0,
            reveal: vec![viewport],
        }
    }
}

/// One of the four quarter turns Table 164's `/Di` enumerates for a sweep or a slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quarter {
    /// 0: "Left to right".
    Rightward,
    /// 90: "Bottom to top".
    Upward,
    /// 180: "Right to left".
    Leftward,
    /// 270: "Top to bottom".
    Downward,
}

/// How far a `/Di` of these degrees is from one of the four, in degrees.
///
/// The table enumerates five numbers — 0, 90, 180, 270 and 315 — and a file may state a sixth,
/// which `pdf_model::navigation` keeps as written because "a file stating a sixth angle has
/// stated a direction and not a style". A tolerance rather than an equality because the value
/// arrives as a PDF number narrowed to `f32`, and a producer writing `90.0000001` has said 90.
const TOLERANCE: f32 = 0.5;

/// Which quarter turn `/Di` names, or `None` for a direction no rectangular sweep expresses.
///
/// 315 is Table 164's fifth value and is `Glitter`'s alone, which is not shaped here; the name
/// `None` "is relevant only for the Fly transition", which is not either. Both are therefore a
/// direction this module cannot draw with rather than a direction it draws wrongly, and the
/// caller reports the style by name.
fn quarter(direction: Direction) -> Option<Quarter> {
    let Direction::Degrees(degrees) = direction else {
        return None;
    };
    if !degrees.is_finite() {
        return None;
    }
    // A file may state 360 or −90 for what the table calls 0 and 270; the angle is what it is
    // and reading it modulo a turn is arithmetic rather than interpretation.
    let turned = degrees.rem_euclid(360.0);
    for (angle, quarter) in [
        (0.0, Quarter::Rightward),
        (90.0, Quarter::Upward),
        (180.0, Quarter::Leftward),
        (270.0, Quarter::Downward),
        (360.0, Quarter::Rightward),
    ] {
        if (turned - angle).abs() <= TOLERANCE {
            return Some(quarter);
        }
    }
    None
}

/// How far a page travels, in device pixels, when it slides a whole viewport in this direction.
///
/// Device pixels grow downward and `/Di` is measured counterclockwise from a left-to-right
/// direction, so "bottom to top" is a *negative* y — the table's own warning that its angle
/// "differs from the page object's Rotate entry" is the same trap one axis over.
fn travel(quarter: Quarter, width: f32, height: f32) -> (f32, f32) {
    match quarter {
        Quarter::Rightward => (width, 0.0),
        Quarter::Upward => (0.0, -height),
        Quarter::Leftward => (-width, 0.0),
        Quarter::Downward => (0.0, height),
    }
}

/// The part of `viewport` a single line moving this way has swept after `done` of its journey.
///
/// A line moving left to right starts at the left edge, so what it has passed over is the strip
/// against that edge — which is the reading of "sweeps across the screen … revealing the new
/// page" this module's documentation records as a choice.
fn swept_from(viewport: Rect, quarter: Quarter, done: f32) -> Rect {
    let (width, height) = (viewport.width(), viewport.height());
    match quarter {
        Quarter::Rightward => Rect::from_corners(
            viewport.min,
            Point::new(viewport.min.x + width * done, viewport.max.y),
        ),
        Quarter::Leftward => Rect::from_corners(
            Point::new(viewport.max.x - width * done, viewport.min.y),
            viewport.max,
        ),
        Quarter::Upward => Rect::from_corners(
            Point::new(viewport.min.x, viewport.max.y - height * done),
            viewport.max,
        ),
        Quarter::Downward => Rect::from_corners(
            viewport.min,
            Point::new(viewport.max.x, viewport.min.y + height * done),
        ),
    }
}

/// The band `done` of the way from the centre out to both edges, across or down the viewport.
fn middle_band(viewport: Rect, done: f32, horizontal_lines: bool) -> Rect {
    if horizontal_lines {
        let half = viewport.height() * done / 2.0;
        let centre = f32::midpoint(viewport.min.y, viewport.max.y);
        Rect::from_corners(
            Point::new(viewport.min.x, centre - half),
            Point::new(viewport.max.x, centre + half),
        )
    } else {
        let half = viewport.width() * done / 2.0;
        let centre = f32::midpoint(viewport.min.x, viewport.max.x);
        Rect::from_corners(
            Point::new(centre - half, viewport.min.y),
            Point::new(centre + half, viewport.max.y),
        )
    }
}

/// The two bands `done` of the way in from opposite edges, which together are the same area
/// [`middle_band`] covers and at the other end of the sweep.
fn edge_bands(viewport: Rect, done: f32, horizontal_lines: bool) -> Vec<Rect> {
    if horizontal_lines {
        let reach = viewport.height() * done / 2.0;
        vec![
            Rect::from_corners(
                viewport.min,
                Point::new(viewport.max.x, viewport.min.y + reach),
            ),
            Rect::from_corners(
                Point::new(viewport.min.x, viewport.max.y - reach),
                viewport.max,
            ),
        ]
    } else {
        let reach = viewport.width() * done / 2.0;
        vec![
            Rect::from_corners(
                viewport.min,
                Point::new(viewport.min.x + reach, viewport.max.y),
            ),
            Rect::from_corners(
                Point::new(viewport.max.x - reach, viewport.min.y),
                viewport.max,
            ),
        ]
    }
}

/// The viewport scaled about its own centre by `fraction`.
fn centred(viewport: Rect, fraction: f32) -> Rect {
    let (half_width, half_height) = (
        viewport.width() * fraction / 2.0,
        viewport.height() * fraction / 2.0,
    );
    let (x, y) = (
        f32::midpoint(viewport.min.x, viewport.max.x),
        f32::midpoint(viewport.min.y, viewport.max.y),
    );
    Rect::from_corners(
        Point::new(x - half_width, y - half_height),
        Point::new(x + half_width, y + half_height),
    )
}

/// The four bands of `viewport` that `inner` leaves uncovered.
///
/// Bands rather than one region with a hole, because a [`Layer`]'s reveal is a list of
/// rectangles and because a hole would need a fill rule to mean anything. They meet at the
/// corners and do not overlap: the top and bottom run the full width, the left and right fill
/// what is left beside `inner`.
fn outside(viewport: Rect, inner: Rect) -> Vec<Rect> {
    let mut bands = Vec::with_capacity(4);
    let mut keep = |rect: Rect| {
        if rect.width() > 0.0 && rect.height() > 0.0 {
            bands.push(rect);
        }
    };
    keep(Rect::from_corners(
        viewport.min,
        Point::new(viewport.max.x, inner.min.y),
    ));
    keep(Rect::from_corners(
        Point::new(viewport.min.x, inner.max.y),
        viewport.max,
    ));
    keep(Rect::from_corners(
        Point::new(viewport.min.x, inner.min.y),
        Point::new(inner.min.x, inner.max.y),
    ));
    keep(Rect::from_corners(
        Point::new(inner.max.x, inner.min.y),
        Point::new(viewport.max.x, inner.max.y),
    ));
    bands
}

/// Appends `rect` to `path` as a closed subpath.
fn push_rect(path: &mut Path, rect: Rect) {
    path.push(PathCommand::MoveTo(rect.min));
    path.push(PathCommand::LineTo(Point::new(rect.max.x, rect.min.y)));
    path.push(PathCommand::LineTo(rect.max));
    path.push(PathCommand::LineTo(Point::new(rect.min.x, rect.max.y)));
    path.push(PathCommand::Close);
}

#[cfg(test)]
mod tests {
    use super::{Face, Quarter, frame, note, quarter};
    use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
    use pdf_render::{Point, Rect};

    /// The viewport every test here shapes a frame in: 200 wide, 100 tall, at the origin.
    fn viewport() -> Rect {
        Rect::from_corners(Point::new(0.0, 0.0), Point::new(200.0, 100.0))
    }

    /// A transition of one style, with Table 164's defaults everywhere else.
    fn of(style: Style) -> Transition {
        Transition {
            style,
            duration: 1.0,
            dimension: Dimension::Horizontal,
            motion: Motion::Inward,
            direction: Direction::Degrees(0.0),
            scale: 1.0,
            opaque: false,
        }
    }

    /// A `Wipe` in each of the four directions reveals the strip against the edge it starts at.
    ///
    /// Table 164: "[a] single line sweeps across the screen from one edge to the other in the
    /// direction specified by the Di entry, revealing the new page", with 0 "[l]eft to right",
    /// 90 "[b]ottom to top", 180 "[r]ight to left" and 270 "[t]op to bottom". Device pixels grow
    /// downward, which is why bottom-to-top reveals the strip at the *larger* y.
    #[test]
    fn a_wipe_reveals_the_strip_its_line_has_passed_over() {
        for (degrees, expected) in [
            (
                0.0,
                Rect::from_corners(Point::new(0.0, 0.0), Point::new(50.0, 100.0)),
            ),
            (
                90.0,
                Rect::from_corners(Point::new(0.0, 75.0), Point::new(200.0, 100.0)),
            ),
            (
                180.0,
                Rect::from_corners(Point::new(150.0, 0.0), Point::new(200.0, 100.0)),
            ),
            (
                270.0,
                Rect::from_corners(Point::new(0.0, 0.0), Point::new(200.0, 25.0)),
            ),
        ] {
            let mut transition = of(Style::Wipe);
            transition.direction = Direction::Degrees(degrees);
            let shaped = frame(&transition, viewport(), 0.25).expect("a wipe is shaped");
            assert_eq!(shaped.layers.len(), 2, "both pages are in every frame");
            assert_eq!(shaped.layers[0].face, Face::Outgoing);
            assert_eq!(
                shaped.layers[0].reveal,
                vec![viewport()],
                "whole, underneath"
            );
            assert_eq!(shaped.layers[1].face, Face::Incoming);
            assert_eq!(
                shaped.layers[1].reveal,
                vec![expected],
                "at {degrees} degrees"
            );
        }
    }

    /// The seven styles a frame is shaped for.
    const SHAPED: [Style; 7] = [
        Style::Wipe,
        Style::Split,
        Style::Box,
        Style::Cover,
        Style::Uncover,
        Style::Push,
        Style::Fade,
    ];

    /// Which face the frame shows over the whole viewport, where one of them does.
    ///
    /// The last layer that is opaque, unmoved and reveals the whole viewport is what a screen
    /// shows, because everything under it is covered and everything over it marks less than all
    /// of it. That is the only reading of a frame that does not need a rasteriser, which is what
    /// makes it the assertion for the two ends of a transition.
    fn shows_everywhere(shaped: &super::Frame) -> Option<Face> {
        shaped
            .layers
            .iter()
            .rev()
            .find(|layer| {
                let area: f32 = layer
                    .reveal
                    .iter()
                    .map(|rect| rect.width() * rect.height())
                    .sum();
                layer.offset == (0.0, 0.0)
                    && layer.alpha >= 1.0
                    && (area - 200.0 * 100.0).abs() < 0.01
            })
            .map(|layer| layer.face)
    }

    /// Every shaped style begins on the page it is leaving and ends on the page it moved to.
    ///
    /// The property that makes a transition a transition rather than an effect: at `/D` seconds
    /// the screen is the page the document moved to, whatever route it took to get there — and
    /// before the first frame it is still the one being left. Failing either end is a cut with
    /// extra steps.
    #[test]
    fn every_shaped_style_runs_from_the_old_page_to_the_new_one() {
        for style in SHAPED {
            let transition = of(style.clone());
            let began = frame(&transition, viewport(), 0.0).expect("shaped at its start");
            assert_eq!(
                shows_everywhere(&began),
                Some(Face::Outgoing),
                "{style:?} at 0 shows the page being left"
            );
            let ended = frame(&transition, viewport(), 1.0).expect("shaped at its end");
            assert_eq!(
                shows_everywhere(&ended),
                Some(Face::Incoming),
                "{style:?} at 1 shows the page moved to"
            );
            // And both pages are in every frame between: a frame naming one face is a cut.
            let middle = frame(&transition, viewport(), 0.5).expect("shaped in flight");
            assert_eq!(middle.layers.len(), 2, "{style:?} in flight");
        }
    }

    /// A `Split` divides the axis its lines do not run along, and its two motions are each
    /// other's mirror.
    ///
    /// "Two lines sweep across the screen … may be either horizontal or vertical and may move
    /// inward from the edges of the page or outward from the centre, as specified by the Dm and
    /// M entries" — horizontal lines run across the screen and so travel down it, which is what
    /// makes `/Dm H` divide the *height*.
    #[test]
    fn a_split_sweeps_two_lines_along_the_dimension_it_is_given() {
        let mut outward = of(Style::Split);
        outward.motion = Motion::Outward;
        let shaped = frame(&outward, viewport(), 0.5).expect("shaped");
        assert_eq!(
            shaped.layers[1].reveal,
            vec![Rect::from_corners(
                Point::new(0.0, 25.0),
                Point::new(200.0, 75.0)
            )],
            "horizontal lines, outward: a band about the middle of the height"
        );

        let mut vertical = outward.clone();
        vertical.dimension = Dimension::Vertical;
        let shaped = frame(&vertical, viewport(), 0.5).expect("shaped");
        assert_eq!(
            shaped.layers[1].reveal,
            vec![Rect::from_corners(
                Point::new(50.0, 0.0),
                Point::new(150.0, 100.0)
            )],
            "vertical lines, outward: a band about the middle of the width"
        );

        let inward = of(Style::Split);
        let shaped = frame(&inward, viewport(), 0.5).expect("shaped");
        assert_eq!(
            shaped.layers[1].reveal,
            vec![
                Rect::from_corners(Point::new(0.0, 0.0), Point::new(200.0, 25.0)),
                Rect::from_corners(Point::new(0.0, 75.0), Point::new(200.0, 100.0)),
            ],
            "inward: the same area, against the two edges instead of the middle"
        );
    }

    /// A `Box` sweeping inward reveals the complement of a shrinking rectangle: four bands that
    /// tile what is outside it exactly once.
    #[test]
    fn an_inward_box_reveals_four_bands_that_tile_what_is_outside_it() {
        let shaped = frame(&of(Style::Box), viewport(), 0.5).expect("shaped");
        let bands = &shaped.layers[1].reveal;
        assert_eq!(bands.len(), 4, "{bands:?}");
        let area: f32 = bands.iter().map(|rect| rect.width() * rect.height()).sum();
        // The inner rectangle is half the viewport's extent on each axis, so a quarter of its
        // area, and what is outside it is the other three quarters.
        assert!(
            (area - 200.0 * 100.0 * 0.75).abs() < 0.01,
            "{area} of an expected 15000"
        );

        let mut outward = of(Style::Box);
        outward.motion = Motion::Outward;
        let shaped = frame(&outward, viewport(), 0.5).expect("shaped");
        assert_eq!(
            shaped.layers[1].reveal,
            vec![Rect::from_corners(
                Point::new(50.0, 25.0),
                Point::new(150.0, 75.0)
            )],
            "outward: one rectangle about the centre"
        );
    }

    /// The three sliding styles differ in *which* page moves, which is the whole of what Table
    /// 164 says separates them.
    #[test]
    fn cover_uncover_and_push_move_different_pages() {
        let half = 0.5;
        let cover = frame(&of(Style::Cover), viewport(), half).expect("shaped");
        assert_eq!(cover.layers[0].face, Face::Outgoing);
        assert_eq!(
            cover.layers[0].offset,
            (0.0, 0.0),
            "the old page holds still"
        );
        assert_eq!(
            cover.layers[1].offset,
            (-100.0, 0.0),
            "the new page is half on, arriving from the left"
        );

        let uncover = frame(&of(Style::Uncover), viewport(), half).expect("shaped");
        assert_eq!(uncover.layers[0].face, Face::Incoming, "underneath");
        assert_eq!(uncover.layers[0].offset, (0.0, 0.0));
        assert_eq!(
            uncover.layers[1].offset,
            (100.0, 0.0),
            "the old page is half off, leaving to the right"
        );

        let push = frame(&of(Style::Push), viewport(), half).expect("shaped");
        assert_eq!(push.layers[0].offset, (100.0, 0.0), "the old page leaving");
        assert_eq!(
            push.layers[1].offset,
            (-100.0, 0.0),
            "and the new one exactly one page behind it"
        );
    }

    /// A `Fade` is an opacity rather than a region: "[t]he new page gradually becomes visible
    /// through the old one".
    #[test]
    fn a_fade_is_the_incoming_page_at_the_fraction_of_the_way_through() {
        let shaped = frame(&of(Style::Fade), viewport(), 0.25).expect("shaped");
        assert_eq!(shaped.layers[1].face, Face::Incoming);
        assert!((shaped.layers[1].alpha - 0.25).abs() < f32::EPSILON);
        assert_eq!(shaped.layers[1].reveal, vec![viewport()], "the whole page");
    }

    /// The five styles no frame is shaped for, and the four of them a person is told about.
    #[test]
    fn a_style_with_a_quantity_the_table_does_not_state_is_named_rather_than_cut() {
        for style in [Style::Blinds, Style::Glitter, Style::Dissolve, Style::Fly] {
            assert!(
                frame(&of(style.clone()), viewport(), 0.5).is_none(),
                "{style:?}"
            );
            let said = note(&of(style)).expect("a sentence naming it");
            assert!(said.contains("Table 164"), "{said}");
        }
        // `R` is the cut, by the table's own definition, so there is nothing to report.
        assert!(frame(&of(Style::Replace), viewport(), 0.5).is_none());
        assert_eq!(note(&of(Style::Replace)), None);
        // And a name the table does not define is reported as the file wrote it.
        let unknown = Style::Unrecognised(pdf_syntax::Name::new(b"Swirl".to_vec()));
        assert!(frame(&of(unknown.clone()), viewport(), 0.5).is_none());
        let said = note(&of(unknown)).expect("a sentence");
        assert!(said.contains("/Swirl"), "{said}");
        // And the empty name, which is a legal PDF name and the only unrecognised style the
        // crawl carries, is described rather than printed as a bare slash.
        let empty = Style::Unrecognised(pdf_syntax::Name::new(Vec::new()));
        let said = note(&of(empty)).expect("a sentence");
        assert!(said.contains("an empty /S"), "{said}");
    }

    /// A report fires on exactly what is not drawn, over every style and every direction.
    ///
    /// Trap 11, and the defect that put this here: [`note`] asked the *style* while [`frame`]
    /// asked the style **and** `/Di`, so a `Wipe` at an angle Table 164 does not give it produced
    /// no frame and no sentence — a cut with nothing said, which is the one outcome trap 5
    /// forbids. Holding the two expressions against each other over the whole cross-product is
    /// what a list written out by hand cannot do: a style added to either side without the other
    /// fails here.
    ///
    /// `R` is the single exception and it is the table's own: "[t]he new page simply replaces the
    /// old one with no special transition effect", so a file asking for `R` and getting a cut got
    /// what it asked for and there is nothing to say.
    #[test]
    fn the_report_fires_on_exactly_what_is_not_drawn() {
        let styles = [
            Style::Split,
            Style::Blinds,
            Style::Box,
            Style::Wipe,
            Style::Dissolve,
            Style::Glitter,
            Style::Replace,
            Style::Fly,
            Style::Push,
            Style::Cover,
            Style::Uncover,
            Style::Fade,
            Style::Unrecognised(pdf_syntax::Name::new(b"Swirl".to_vec())),
        ];
        let directions = [
            Direction::Degrees(0.0),
            Direction::Degrees(90.0),
            Direction::Degrees(180.0),
            Direction::Degrees(270.0),
            // Table 164's fifth value, which it gives to `Glitter` alone.
            Direction::Degrees(315.0),
            // An angle the table gives to nothing at all.
            Direction::Degrees(45.0),
            // "If the value is a name, it shall be None, which is relevant only for the Fly
            // transition."
            Direction::None,
        ];
        for style in styles {
            for direction in directions {
                let mut transition = of(style.clone());
                transition.direction = direction;
                let drawn = frame(&transition, viewport(), 0.5).is_some();
                let said = note(&transition).is_some();
                let owed = !drawn && style != Style::Replace;
                assert_eq!(said, owed, "{style:?} at {direction:?}: drawn {drawn}");
            }
        }
    }

    /// `/Di` is read as an angle, and only the four quarter turns describe a rectangular sweep.
    ///
    /// 315 is Table 164's fifth value and belongs to `Glitter` alone; the name `None` "is
    /// relevant only for the Fly transition". Neither is shaped, so neither may quietly become
    /// one of the four.
    #[test]
    fn only_the_four_quarter_turns_name_a_sweep() {
        assert_eq!(quarter(Direction::Degrees(0.0)), Some(Quarter::Rightward));
        assert_eq!(quarter(Direction::Degrees(360.0)), Some(Quarter::Rightward));
        assert_eq!(quarter(Direction::Degrees(-90.0)), Some(Quarter::Downward));
        assert_eq!(quarter(Direction::Degrees(315.0)), None);
        assert_eq!(quarter(Direction::None), None);
        assert_eq!(quarter(Direction::Degrees(f32::NAN)), None);

        // And a `Wipe` at an angle no rectangle sweeps is reported rather than drawn at some
        // nearby angle the file did not ask for. **The second half of that sentence was written
        // in the three-hundred-and-ninety-third session and was false until the
        // seven-hundred-and-twentieth**: the frame was refused and nothing was reported.
        let mut askew = of(Style::Wipe);
        askew.direction = Direction::Degrees(315.0);
        assert!(frame(&askew, viewport(), 0.5).is_none());
        let said = note(&askew).expect("a sentence naming the direction");
        assert!(said.contains("/Wipe") && said.contains("315"), "{said}");
    }

    /// A fraction outside the transition, and one that is not a number at all.
    #[test]
    fn a_fraction_past_the_end_is_the_end() {
        let past = frame(&of(Style::Wipe), viewport(), 4.0).expect("shaped");
        let ended = frame(&of(Style::Wipe), viewport(), 1.0).expect("shaped");
        assert_eq!(past, ended);
        assert_eq!(frame(&of(Style::Wipe), viewport(), f32::NAN), Some(ended));
        let before = frame(&of(Style::Wipe), viewport(), -1.0).expect("shaped");
        assert_eq!(
            before,
            frame(&of(Style::Wipe), viewport(), 0.0).expect("shaped")
        );
    }
}
