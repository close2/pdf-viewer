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
//! word about what a frame in the middle looks like**, so the choices below are recorded as
//! choices, in the manner ADR 0211 recorded a caret's colour and ADR 0225 its placement. Two are
//! shared by every style:
//!
//! - **Progress is linear in time.** A host divides elapsed time by `/D`; nothing here reads a
//!   curve into the clause that the clause does not state.
//! - **A sweep reveals what it has passed over.** Table 164's verbs are "sweep across the
//!   screen, revealing the new page" and "slides on to the screen … covering the old page", so
//!   the swept or covered area shows the page being moved to, and the rest shows the page being
//!   left. That is a reading of the table's own sentences rather than an invention.
//!
//! # Eleven styles are drawn, and four of them at a quantity this program chose
//!
//! **Seven are determined by Table 164's own words**: `Wipe`, `Split`, `Box`, `Cover`,
//! `Uncover`, `Push` and `Fade`. Each is a placement of two pages plus a rectangular region, and
//! nothing in any of them needs a number the standard does not give.
//!
//! **Four name the kind of mark and withhold only a quantity**, and the owner's ruling for that
//! shape (`doc/questions/A72`) is to choose the quantity, write it down as this program's, and
//! draw (ADR 1299):
//!
//! | style | what the clause names | the quantity it withholds | this program's choice |
//! |---|---|---|---|
//! | `Blinds` | "[m]ultiple lines, evenly spaced across the screen" | how many | [`BLINDS`] lines |
//! | `Dissolve` | "[t]he old page dissolves gradually" | the grain and the order | square cells, [`DISSOLVE_CELLS`] along the longer side, in one fixed pseudo-random order |
//! | `Glitter` | a dissolve that "sweeps across the page in a wide band" | how wide | [`GLITTER_BAND`] of the distance swept, over `Dissolve`'s cells and order |
//! | `Fly` | "[c]hanges are flown out or in" | what the changes are | the pixels in which the two pages differ |
//!
//! [`note`] says so to a person on every page that asks for one of the four, because a quantity
//! the reader chose is not one the producer specified. `R` is the cut by the table's own
//! definition, "no special transition effect", and is neither drawn nor reported.
//!
//! **The dissolve's order is a fixed sequence, not a random one**, for two reasons that are both
//! requirements. A frame is a pure function of the transition, the view and the fraction, which
//! is what lets a test with no display see the frame a window shows and what lets the two
//! backends be compared; and a cell once replaced has to stay replaced for the page to dissolve
//! *gradually* rather than flicker, which a draw per frame achieves only if every frame of one
//! transition agrees on the order. A generator seeded by the clock would satisfy neither.
//!
//! **A style is not the whole of what decides a frame, and for six of the eleven the direction
//! decides it too.** `Wipe`, `Cover`, `Uncover`, `Push` and `Fly` travel along `/Di` in quarter
//! turns — every value Table 164 gives them is one, with 315 belonging to `Glitter` and the name
//! `None` to `Fly` where `/SS` is not 1 — and `Glitter`'s band can sweep along any angle. A file
//! stating a direction none of those expresses has asked for an effect nothing here draws, so
//! [`frame`] shapes none — and [`note`] therefore asks the *whole* transition rather than its
//! style, since a report keyed on less than what decides the drawing is a report that fires on
//! the wrong condition (trap 11).
//!
//! # What a host does with this
//!
//! Two pages' pixels and a fraction in, one [`pdf_render::DisplayList`] out. The list holds
//! two image commands, so **both backends draw it** — which is what keeps the CPU rasteriser's
//! job as the frame a graphics device refuses (`CLAUDE.md`'s startup rules), and is the same
//! reason the sidebar and the caret cross as display lists rather than as pixels.

use std::sync::OnceLock;

use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
use pdf_render::{
    BlendMode, Clip, Command, DisplayList, DisplayListError, FillRule, Image, ImageSource, Path,
    PathCommand, Point, Raster, RasterFormat, Rect, Size, Transform,
};

/// How many lines a `Blinds` sweeps — this program's number, not the standard's (ADR 1299).
///
/// Table 164: "Multiple lines, evenly spaced across the screen, synchronously sweep in the same
/// direction to reveal the new page." *Multiple* is the whole of what it says about the count.
/// Eight is enough that the effect reads as blinds rather than as a `Wipe`, and few enough that
/// each band is still an eighth of the view on a small screen.
pub const BLINDS: u16 = 8;

/// How many square cells a `Dissolve` or a `Glitter` divides the view's longer side into — this
/// program's number, not the standard's (ADR 1299).
///
/// Table 164 says the old page "dissolves gradually to reveal the new one" and states no grain.
/// A count along the view rather than a size in pixels, so the pattern is the same at every
/// window size and the cells never become too many to clip by; the shorter side takes as many
/// cells of the same size as it needs, the last row or column cut by the view's edge.
pub const DISSOLVE_CELLS: u16 = 40;

/// How wide a `Glitter`'s band is, as a fraction of the distance the band travels — this
/// program's number, not the standard's (ADR 1299).
///
/// Table 164: "Similar to Dissolve , except that the effect sweeps across the page in a wide band
/// moving from one side of the screen to the other". *Wide* is the whole of what it says.
pub const GLITTER_BAND: f32 = 0.25;

/// The seed of the fixed sequence [`DISSOLVE_CELLS`]'s cells are replaced in.
///
/// Any constant would do; what matters is that it is one (this module's documentation has the
/// argument). The ADR's number, so that a reader who meets it knows where it was decided.
const DISSOLVE_SEED: u64 = 1299;

/// Which of a transition's pictures a [`Layer`] draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// The page being left — what the screen showed before the presentation advanced.
    Outgoing,
    /// The page being moved to, which §12.4.4.1 makes the one whose `/Trans` this is.
    Incoming,
    /// What a `Fly` carries: the part of one page that differs from the other, and nothing else.
    ///
    /// The page flown in for `/M /I` and the page flown out for `/M /O`; [`Faces`] builds it.
    Flown,
}

/// One picture's placement in one frame of a transition.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// Which picture this draws.
    pub face: Face,
    /// Where the page goes, in device pixels, relative to the viewport it was rasterised for.
    ///
    /// `(0.0, 0.0)` for every style but the ones that slide or fly. A page offset by the
    /// viewport's full width is entirely off the screen, which is what the end of a `Push` is.
    pub offset: (f32, f32),
    /// The size the picture is drawn at, about the viewport's centre, where 1 is its own.
    ///
    /// Only a `Fly` moves it off 1: Table 164's `/SS` is "[t]he starting or ending scale at which
    /// the changes shall be drawn". The table names no point to scale about, and the viewport's
    /// centre is this program's choice (ADR 1299).
    pub scale: f32,
    /// Constant alpha in `0.0..=1.0`, which only `Fade` moves off 1.
    pub alpha: f32,
    /// The parts of the viewport this layer marks, in device pixels.
    ///
    /// One rectangle for a whole page, two for a `Split` sweeping inward, four for a `Box`
    /// doing the same — a `Box`'s inward sweep reveals the *complement* of a shrinking
    /// rectangle, and a complement of a rectangle inside a rectangle is four bands — and one per
    /// run of replaced cells for a `Dissolve`. Empty means the layer marks nothing at all, which
    /// is what the first frame of an outward `Box` is.
    pub reveal: Vec<Rect>,
}

/// One frame of a transition: what to draw, and in what order.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The layers, back to front. Always two: a transition that showed only one picture would be
    /// a cut.
    pub layers: Vec<Layer>,
}

/// The pictures one transition's frames are drawn from, prepared once per transition.
///
/// The two pages, rasterised for the whole viewport, and — only for a `Fly`, and only when a
/// frame first asks — the picture of what differs between them. A host builds this **once per
/// transition**: [`pdf_render::Image`] holds its samples behind an `Arc`, and the GPU backend's
/// caches are keyed by that pointer, so each picture crosses to the device once however many
/// frames the transition lasts.
#[derive(Debug, Clone)]
pub struct Faces {
    /// The page being left.
    outgoing: Image,
    /// The page being moved to.
    incoming: Image,
    /// Table 164's `/M`, which decides which page's changes fly.
    motion: Motion,
    /// Table 164's `/B`, which decides whether they fly as their shape or as its rectangle.
    opaque: bool,
    /// [`Face::Flown`]'s picture, built the first time a frame draws one.
    flown: OnceLock<Image>,
}

impl Faces {
    /// The pictures `transition`'s frames are drawn from.
    #[must_use]
    pub fn new(transition: &Transition, outgoing: Image, incoming: Image) -> Self {
        Self {
            outgoing,
            incoming,
            motion: transition.motion,
            opaque: transition.opaque,
            flown: OnceLock::new(),
        }
    }

    /// The picture a layer of this face draws.
    fn image(&self, face: Face) -> &Image {
        match face {
            Face::Outgoing => &self.outgoing,
            Face::Incoming => &self.incoming,
            Face::Flown => self.flown.get_or_init(|| match self.motion {
                // "Changes are flown out or in (as specified by M)": flown *in* is the arriving
                // page's changes landing on the page being left, and flown *out* is the leaving
                // page's changes departing from the page arrived at.
                Motion::Inward => changes(&self.incoming, &self.outgoing, self.opaque),
                Motion::Outward => changes(&self.outgoing, &self.incoming, self.opaque),
            }),
        }
    }
}

/// The part of `carried` that differs from `other`, transparent everywhere else.
///
/// Table 164's `Fly` flies "[c]hanges" and does not say what they are; **the pixels in which the
/// two pages differ** is this program's reading (ADR 1299), and it is the one under which the
/// last frame is the arriving page exactly: the old page with the new page's changes laid on it
/// is the new page. `/B` is the table's own refinement of it — "[i]f true , the area that shall
/// be flown in is rectangular and opaque" — so `rectangular` takes the changes' bounding
/// rectangle, whole and opaque, instead of their exact shape.
///
/// Two pages rasterised at different sizes cannot be compared, and every pixel is then a change.
fn changes(carried: &Image, other: &Image, rectangular: bool) -> Image {
    let comparable = carried.width == other.width
        && carried.height == other.height
        && carried.data.len() == other.data.len();
    let differs = |index: usize| {
        let at = index.saturating_mul(4);
        !comparable
            || carried.data.get(at..at.saturating_add(4))
                != other.data.get(at..at.saturating_add(4))
    };
    let width = (carried.width as usize).max(1);
    let mut data = vec![0_u8; carried.data.len()];
    let bounds = if rectangular {
        (0..carried.data.len() / 4)
            .filter(|index| differs(*index))
            .fold(
                None,
                |bounds: Option<(usize, usize, usize, usize)>, index| {
                    let (x, y) = column_and_row(index, width);
                    Some(bounds.map_or((x, y, x, y), |(x0, y0, x1, y1)| {
                        (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
                    }))
                },
            )
    } else {
        None
    };
    for (index, (pixel, out)) in carried
        .data
        .chunks_exact(4)
        .zip(data.chunks_exact_mut(4))
        .enumerate()
    {
        let kept = match bounds {
            Some((x0, y0, x1, y1)) => {
                let (x, y) = column_and_row(index, width);
                (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
            }
            None => !rectangular && differs(index),
        };
        if kept {
            out.copy_from_slice(pixel);
            if rectangular {
                // "rectangular and opaque": the rectangle hides what it lands on.
                out[3] = u8::MAX;
            }
        }
    }
    Image {
        width: carried.width,
        height: carried.height,
        data: data.into(),
        interpolate: carried.interpolate,
        sample_alpha: carried.sample_alpha,
    }
}

impl Frame {
    /// Turns this frame into commands, over the transition's pictures.
    ///
    /// Every picture is the viewport's size — the page rasterised for the whole viewport, the
    /// same [`pdf_render::TargetSpec`] a frame outside a transition uses — so this places each of
    /// them by mapping the unit square onto the viewport, scaled about its centre and offset by
    /// the layer's own numbers. A host converts each raster once per transition with
    /// [`drawable`], builds [`Faces`] from the two, and draws every frame from those.
    ///
    /// # Errors
    ///
    /// [`DisplayListError::TooManyClips`] cannot arise from a frame — a frame adds two clips —
    /// but the list's own limit is stated in its type and swallowing it would be a silence this
    /// project does not permit anywhere else.
    pub fn draw(&self, viewport: Rect, faces: &Faces) -> Result<DisplayList, DisplayListError> {
        let mut list = DisplayList::new(Size::new(viewport.max.x, viewport.max.y));
        let centre = (
            f32::midpoint(viewport.min.x, viewport.max.x),
            f32::midpoint(viewport.min.y, viewport.max.y),
        );
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
            list.push(Command::Image {
                image: ImageSource::Decoded(faces.image(layer.face).clone()),
                // The unit square onto the viewport, then scaled about its centre, then wherever
                // the layer slid it.
                //
                // **The y scale is negative and that is the whole of the flip.** A
                // `Command::Image` draws "the unit square in user space, with the image's
                // *top* row at y = 1", because PDF's user space has y growing upward; these
                // pixels are a *device* raster whose first row is the top one. A positive scale
                // would stand every frame of every transition on its head, which is invisible in
                // a page of flat colour and was found by looking at a real window.
                transform: Transform::scale(viewport.width(), -viewport.height())
                    .then(Transform::translate(
                        viewport.min.x - centre.0,
                        viewport.min.y + viewport.height() - centre.1,
                    ))
                    .then(Transform::scale(layer.scale, layer.scale))
                    .then(Transform::translate(
                        centre.0 + layer.offset.0,
                        centre.1 + layer.offset.1,
                    )),
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
        // A page's own pixels are not a document's image, and nothing in §11 reads their
        // alpha; the rectangle they cover is all they state, which is `Shape`.
        sample_alpha: pdf_render::SampleAlpha::Shape,
    })
}

/// The frame of `transition` at `progress` of the way through it, or `None` for one this does
/// not shape.
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
/// `None` is the answer for `R`, which the table defines as a cut, for a name the table does not
/// define, and for a style asked for in a direction nothing here draws it in — see this module's
/// own documentation. Every `None` but `R`'s has a sentence in [`note`], and that is a property
/// this module tests rather than asserts.
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
        // "Multiple lines, evenly spaced across the screen, synchronously sweep in the same
        // direction to reveal the new page. The lines may be either horizontal or vertical, as
        // specified by the Dm entry. Horizontal lines move downward; vertical lines move to the
        // right." So `/M` and `/Di` do not apply, which is also what the table's scoping of
        // those two entries says.
        Style::Blinds => revealing(
            viewport,
            blinds(
                viewport,
                done,
                transition.dimension == Dimension::Horizontal,
            ),
        ),
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
        // "The old page dissolves gradually to reveal the new one." The first `done` of the
        // cells in the fixed order are the new page's.
        Style::Dissolve => revealing(viewport, dissolved(viewport, done)),
        // "Similar to Dissolve , except that the effect sweeps across the page in a wide band
        // moving from one side of the screen to the other in the direction specified by the Di
        // entry." Ahead of the band nothing is replaced and behind it everything is; inside it,
        // a cell goes once the band's leading edge is its own share of the band past it.
        Style::Glitter => revealing(viewport, glittered(viewport, transition.direction, done)?),
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
        Style::Fly => fly(transition, viewport, done)?,
        // "The new page gradually becomes visible through the old one" — the one style whose
        // frame is an opacity rather than a region.
        Style::Fade => vec![
            Layer::whole(Face::Outgoing, viewport),
            Layer {
                alpha: done,
                ..Layer::whole(Face::Incoming, viewport)
            },
        ],
        Style::Replace | Style::Unrecognised(_) => return None,
    };
    Some(Frame { layers })
}

/// The cells of a `Dissolve` replaced after `done` of it: the first `done` of them in the fixed
/// order.
fn dissolved(viewport: Rect, done: f32) -> Vec<Rect> {
    let cells = Cells::of(viewport);
    let ranks = cells.ranks();
    let replaced = done * f32::from(cells.count());
    cells.runs(|index| {
        ranks
            .get(index)
            .is_some_and(|rank| f32::from(rank.saturating_add(1)) <= replaced)
    })
}

/// The cells of a `Glitter` replaced after `done` of it, or `None` for a `/Di` that names no angle.
///
/// Ahead of the band nothing is replaced and behind it everything is; inside it, a cell goes once
/// the band's leading edge is its own share of the band past it — the share being the cell's
/// place in the same fixed sequence `Dissolve` uses.
fn glittered(viewport: Rect, direction: Direction, done: f32) -> Option<Vec<Rect>> {
    let Direction::Degrees(degrees) = direction else {
        return None;
    };
    if !degrees.is_finite() {
        return None;
    }
    let cells = Cells::of(viewport);
    let along = Along::new(viewport, degrees);
    let front = done * (1.0 + GLITTER_BAND);
    Some(cells.runs(|index| {
        cells.rect(index).is_some_and(|rect| {
            let centre = Point::new(
                f32::midpoint(rect.min.x, rect.max.x),
                f32::midpoint(rect.min.y, rect.max.y),
            );
            along.position(centre) + GLITTER_BAND * unit(cell_key(index)) <= front
        })
    }))
}

/// Table 164's `Fly`, or `None` for a direction it does not fly in.
///
/// "Changes are flown out or in (as specified by M ), in the direction specified by Di , to or
/// from a location that is offscreen except when Di is None ." And `/SS`: "If M specifies an
/// inward transition, the scale of the changes drawn shall progress from SS to 1.0 over the
/// course of the transition. If M specifies an outward transition, the scale of the changes drawn
/// shall progress from 1.0 to SS over the course of the transition".
///
/// The offscreen end is the nearest place a picture at that end's scale is wholly off the view,
/// which follows from the sentence rather than being chosen: a page drawn at `/SS` about the
/// centre is off the screen once it has travelled half the view plus half of itself. `/Di /None`
/// flies nowhere and only scales, and the table makes it "relevant only for the Fly transition
/// when the value of SS is not 1.0" — with `/SS` 1 nothing would move, so that one is not drawn
/// and [`note`] says why.
fn fly(transition: &Transition, viewport: Rect, done: f32) -> Option<Vec<Layer>> {
    // A scale is a size, and a size below nothing is not one; a scale that is not a number at all
    // is taken as the table's default.
    let end_scale = if transition.scale.is_finite() {
        transition.scale.max(0.0)
    } else {
        1.0
    };
    let way = match transition.direction {
        Direction::None if (end_scale - 1.0).abs() < f32::EPSILON => return None,
        Direction::None => None,
        Direction::Degrees(_) => Some(quarter(transition.direction)?),
    };
    // How far from rest the changes are: all the way at the start of a flight in, and at the end
    // of a flight out. The base is the page the changes are *not* part of.
    let (base, from_rest, sign) = match transition.motion {
        Motion::Inward => (Face::Outgoing, 1.0 - done, -1.0),
        Motion::Outward => (Face::Incoming, done, 1.0),
    };
    let offset = way.map_or((0.0, 0.0), |way| {
        let reach = f32::midpoint(1.0, end_scale);
        let (dx, dy) = travel(way, viewport.width() * reach, viewport.height() * reach);
        (sign * dx * from_rest, sign * dy * from_rest)
    });
    Some(vec![
        Layer::whole(base, viewport),
        Layer {
            offset,
            scale: 1.0 + (end_scale - 1.0) * from_rest,
            ..Layer::whole(Face::Flown, viewport)
        },
    ])
}

/// The whole outgoing page with the incoming one showing through `reveal`, which is the shape of
/// every style that sweeps or dissolves rather than slides.
fn revealing(viewport: Rect, reveal: Vec<Rect>) -> Vec<Layer> {
    vec![
        Layer::whole(Face::Outgoing, viewport),
        Layer {
            reveal,
            ..Layer::whole(Face::Incoming, viewport)
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

/// What to tell a person about a transition, or `None` where there is nothing to say.
///
/// Two things are said, and they are the two ways a picture can differ from what the producer
/// specified. **A style asked for and not drawn** is trap 5's channel: a named effect silently
/// drawn as a cut is indistinguishable from a file that asked for a cut. **A style drawn at a
/// quantity this program chose** is the owner's ruling in `doc/questions/A72`: the number is
/// ours, and a person watching is told so rather than left to take it for the document's.
///
/// `R` is the one style with nothing to report, because Table 164 defines it as the cut — "[t]he
/// new page simply replaces the old one with no special transition effect".
///
/// **This asks [`frame`] rather than restating its conditions**, because a style is not the whole
/// of what decides a frame: `note` and `frame` are one decision in two expressions, which is why
/// [`the_report_fires_on_exactly_what_is_not_drawn`] holds them against each other over every
/// style and direction rather than against a list written out by hand.
///
/// [`the_report_fires_on_exactly_what_is_not_drawn`]: tests::the_report_fires_on_exactly_what_is_not_drawn
pub(crate) fn note(transition: &Transition) -> Option<String> {
    match &transition.style {
        Style::Replace => return None,
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
        _ => {}
    }
    // Any viewport answers whether a frame is shaped: no style's refusal depends on its size.
    let probe = Rect::from_corners(Point::new(0.0, 0.0), Point::new(1.0, 1.0));
    if frame(transition, probe, 0.5).is_none() {
        return Some(askew(transition));
    }
    let chosen = match transition.style {
        Style::Blinds => format!("with {BLINDS} lines, a number Table 164 does not state"),
        Style::Dissolve => format!(
            "in square cells, {DISSOLVE_CELLS} along the longer side, replaced in a fixed order \
             — Table 164 states neither"
        ),
        Style::Glitter => format!(
            "as Dissolve's cells in a band {GLITTER_BAND} of the distance it sweeps wide, a width \
             Table 164 does not state"
        ),
        Style::Fly => "by flying the pixels in which the two pages differ — Table 164 does not \
                       say what the changes are"
            .to_owned(),
        _ => return None,
    };
    Some(format!(
        "transition: /{} is drawn {chosen}, so that quantity is this program's own choice rather \
         than the document's",
        spelling(&transition.style)
    ))
}

/// What to tell a person about a style asked for in a direction nothing here draws it in.
///
/// Every `/Di` Table 164 gives `Wipe`, `Cover`, `Uncover`, `Push` and `Fly` is a quarter turn, with
/// 315 reserved to `Glitter` and the name `None` to `Fly` where `/SS` is not 1 — "which is
/// relevant only for the Fly transition when the value of SS is not 1.0". A document has to state
/// a direction the table does not give the style it stated for this to be reached.
fn askew(transition: &Transition) -> String {
    let stated = match transition.direction {
        Direction::None if transition.style == Style::Fly => {
            "the name /None with an /SS of 1, which ISO 32000-2 Table 164 makes relevant only \
             when /SS is not 1"
                .to_owned()
        }
        Direction::None => {
            "the name /None, which ISO 32000-2 Table 164 gives only to /Fly".to_owned()
        }
        Direction::Degrees(degrees) => {
            format!("{degrees} degrees, which ISO 32000-2 Table 164 does not give that style")
        }
    };
    format!(
        "transition: /{} is named but not drawn — its /Di is {stated} — so the page is shown at \
         once",
        spelling(&transition.style)
    )
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
            scale: 1.0,
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
            offset,
            ..Self::whole(face, viewport)
        }
    }
}

/// `Blinds`' [`BLINDS`] bands, each revealed `done` of the way along the way its line moves.
///
/// "Horizontal lines move downward; vertical lines move to the right" — so a horizontal blind
/// reveals from its top edge down, and a vertical one from its left edge rightward. Every band
/// is revealed by the same share at the same moment, which is the table's "synchronously".
fn blinds(viewport: Rect, done: f32, horizontal_lines: bool) -> Vec<Rect> {
    let count = f32::from(BLINDS);
    (0..BLINDS)
        .map(|band| {
            let band = f32::from(band);
            if horizontal_lines {
                let height = viewport.height() / count;
                let top = viewport.min.y + height * band;
                Rect::from_corners(
                    Point::new(viewport.min.x, top),
                    Point::new(viewport.max.x, top + height * done),
                )
            } else {
                let width = viewport.width() / count;
                let left = viewport.min.x + width * band;
                Rect::from_corners(
                    Point::new(left, viewport.min.y),
                    Point::new(left + width * done, viewport.max.y),
                )
            }
        })
        .collect()
}

/// The square cells `Dissolve` and `Glitter` replace, [`DISSOLVE_CELLS`] along the longer side.
struct Cells {
    /// The view the grid covers.
    viewport: Rect,
    /// A cell's side, in device pixels.
    side: f32,
    /// How many cells across.
    columns: u16,
    /// How many cells down.
    rows: u16,
}

impl Cells {
    /// The grid over `viewport`.
    fn of(viewport: Rect) -> Self {
        let longer = viewport.width().max(viewport.height());
        let side = longer / f32::from(DISSOLVE_CELLS);
        // As many cells as cover each side: the longer one by construction, the shorter one
        // counted, since a cell is square and the shorter side need not be a whole number of
        // them. The allowance of a ten-thousandth of a cell is for the division's rounding, so
        // that a side of exactly twenty cells is not counted as twenty-one. A view of no extent
        // has no cells.
        let along = |extent: f32| {
            if side.is_nan() || side <= 0.0 {
                return 0;
            }
            (1..=DISSOLVE_CELLS)
                .find(|count| f32::from(*count) * side >= extent - side * 1e-4)
                .unwrap_or(DISSOLVE_CELLS)
        };
        Self {
            viewport,
            side,
            columns: along(viewport.width()),
            rows: along(viewport.height()),
        }
    }

    /// How many cells there are. At most [`DISSOLVE_CELLS`] squared, which is why a `u16` holds
    /// it.
    fn count(&self) -> u16 {
        self.columns.saturating_mul(self.rows)
    }

    /// Cell `index`, row by row from the top left, cut by the view's edge.
    fn rect(&self, index: usize) -> Option<Rect> {
        let columns = usize::from(self.columns);
        if columns == 0 || index >= usize::from(self.count()) {
            return None;
        }
        let (column, row) = column_and_row(index, columns);
        let (column, row) = (u16::try_from(column).ok()?, u16::try_from(row).ok()?);
        let (x, y) = (
            self.viewport.min.x + f32::from(column) * self.side,
            self.viewport.min.y + f32::from(row) * self.side,
        );
        Some(Rect::from_corners(
            Point::new(x, y),
            Point::new(
                (x + self.side).min(self.viewport.max.x),
                (y + self.side).min(self.viewport.max.y),
            ),
        ))
    }

    /// Each cell's place in the fixed order, indexed by cell.
    ///
    /// The order sorts the cells by [`cell_key`], ties by index, so it is one permutation of the
    /// grid and the same one in every frame of every transition over a view of this shape.
    fn ranks(&self) -> Vec<u16> {
        let count = usize::from(self.count());
        let mut order: Vec<usize> = (0..count).collect();
        order.sort_by_key(|index| (cell_key(*index), *index));
        let mut ranks = vec![0_u16; count];
        for (rank, index) in order.into_iter().enumerate() {
            if let (Some(slot), Ok(rank)) = (ranks.get_mut(index), u16::try_from(rank)) {
                *slot = rank;
            }
        }
        ranks
    }

    /// The replaced cells, as one rectangle per unbroken run along a row.
    ///
    /// A run rather than a cell because the rectangles become one clip path, and a row of
    /// neighbours is one rectangle to a rasteriser whichever way it is written; it also keeps the
    /// last frames, where nearly everything is replaced, near one rectangle per row.
    fn runs(&self, replaced: impl Fn(usize) -> bool) -> Vec<Rect> {
        let columns = usize::from(self.columns);
        let mut runs = Vec::new();
        for row in 0..usize::from(self.rows) {
            let mut open: Option<Rect> = None;
            for column in 0..columns {
                let index = row.saturating_mul(columns).saturating_add(column);
                let Some(cell) = self.rect(index).filter(|_| replaced(index)) else {
                    runs.extend(open.take());
                    continue;
                };
                open = Some(open.map_or(cell, |run| Rect::from_corners(run.min, cell.max)));
            }
            runs.extend(open);
        }
        runs
    }
}

/// A cell's place in the fixed sequence: `SplitMix64`'s finaliser over its index and
/// [`DISSOLVE_SEED`].
///
/// A hash rather than a stateful generator so that a cell's key depends on nothing but the cell,
/// which is what makes the order a pure function. `SplitMix64` because its three lines are the
/// whole of it and every output bit depends on every input bit; nothing here needs more.
fn cell_key(index: usize) -> u64 {
    let mut z = (index as u64)
        .wrapping_add(DISSOLVE_SEED)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A key as a fraction in `0..1`, from its top sixteen bits.
fn unit(key: u64) -> f32 {
    // The shift leaves sixteen bits, so the conversion cannot fail and the fallback is unreached.
    f32::from(u16::try_from(key >> 48).unwrap_or(u16::MAX)) / 65_536.0
}

/// Where a point lies along a direction across the view, as a fraction from the side the
/// direction starts at (0) to the side it ends at (1).
struct Along {
    /// The direction as a unit vector in device space.
    direction: (f32, f32),
    /// The projection of the view's first corner the direction reaches.
    start: f32,
    /// How far the direction crosses the view.
    length: f32,
}

impl Along {
    /// `/Di`'s angle across `viewport`: counterclockwise from left-to-right, in a device space
    /// whose y grows downward — so 270, "[t]op to bottom", is a *positive* y.
    fn new(viewport: Rect, degrees: f32) -> Self {
        let radians = degrees.to_radians();
        let direction = (radians.cos(), -radians.sin());
        let project = |point: Point| point.x * direction.0 + point.y * direction.1;
        let corners = [
            viewport.min,
            viewport.max,
            Point::new(viewport.min.x, viewport.max.y),
            Point::new(viewport.max.x, viewport.min.y),
        ];
        let (start, end) = corners
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), corner| {
                let at = project(*corner);
                (lo.min(at), hi.max(at))
            });
        Self {
            direction,
            start,
            length: end - start,
        }
    }

    /// `point`'s position along the direction, 0 at the side the band starts from.
    fn position(&self, point: Point) -> f32 {
        if self.length <= 0.0 {
            return 0.0;
        }
        (point.x * self.direction.0 + point.y * self.direction.1 - self.start) / self.length
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
/// 315 is Table 164's fifth value and is `Glitter`'s alone, whose band is not a rectangle and is
/// placed by [`Along`] instead; the name `None` is `Fly`'s alone, and [`fly`] reads it before
/// asking this. Both are therefore a direction a sweep or a slide cannot draw with rather than a
/// direction it draws wrongly, and the caller reports the style by name.
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

/// Where item `index` of a row-major grid `width` wide sits: its column and its row.
///
/// A grid of no width has nothing in it, and every index is then the first cell's.
fn column_and_row(index: usize, width: usize) -> (usize, usize) {
    (
        index.checked_rem(width).unwrap_or(0),
        index.checked_div(width).unwrap_or(0),
    )
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
    use super::{
        BLINDS, Face, Faces, Frame, GLITTER_BAND, Quarter, cell_key, frame, note, quarter,
    };
    use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
    use pdf_render::{Image, Point, Rasterizer, Rect, TargetSpec, Transform};

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

    /// The area a list of rectangles covers, which none of this module's reveals overlap.
    fn area(rects: &[Rect]) -> f32 {
        rects.iter().map(|rect| rect.width() * rect.height()).sum()
    }

    /// Whether `point` is inside one of the incoming layer's rectangles.
    fn revealed(shaped: &Frame, point: Point) -> bool {
        shaped.layers[1].reveal.iter().any(|rect| {
            (rect.min.x..rect.max.x).contains(&point.x)
                && (rect.min.y..rect.max.y).contains(&point.y)
        })
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

    /// The ten styles whose frame is two whole pages and a region, alpha or slide.
    const SHAPED: [Style; 10] = [
        Style::Wipe,
        Style::Split,
        Style::Blinds,
        Style::Box,
        Style::Dissolve,
        Style::Glitter,
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
    fn shows_everywhere(shaped: &Frame) -> Option<Face> {
        shaped
            .layers
            .iter()
            .rev()
            .find(|layer| {
                layer.offset == (0.0, 0.0)
                    && (layer.scale - 1.0).abs() < f32::EPSILON
                    && layer.alpha >= 1.0
                    && (area(&layer.reveal) - 200.0 * 100.0).abs() < 0.01
            })
            .map(|layer| layer.face)
    }

    /// Every region-shaped style begins on the page it is leaving and ends on the page it moved
    /// to.
    ///
    /// The property that makes a transition a transition rather than an effect: at `/D` seconds
    /// the screen is the page the document moved to, whatever route it took to get there — and
    /// before the first frame it is still the one being left. Failing either end is a cut with
    /// extra steps. `Fly` is held to the same property in pixels, by
    /// [`a_fly_begins_on_the_old_page_and_lands_on_the_new_one`].
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

    /// A `Blinds` half way through has revealed the top half of every band, or the left half.
    ///
    /// Table 164: "Multiple lines, evenly spaced across the screen, synchronously sweep in the
    /// same direction to reveal the new page. The lines may be either horizontal or vertical, as
    /// specified by the Dm entry. Horizontal lines move downward; vertical lines move to the
    /// right." *Synchronously* is what makes the share the same in every band.
    #[test]
    fn blinds_reveal_the_same_share_of_every_band() {
        for done in [0.0_f32, 0.5, 1.0] {
            let shaped = frame(&of(Style::Blinds), viewport(), done).expect("shaped");
            let bands = &shaped.layers[1].reveal;
            assert_eq!(bands.len(), usize::from(BLINDS));
            let height = 100.0 / f32::from(BLINDS);
            for (index, band) in bands.iter().enumerate() {
                let top = height * f32::from(u16::try_from(index).expect("eight bands"));
                assert!(
                    (band.min.y - top).abs() < 1e-3,
                    "band {index} starts at its top"
                );
                assert!(
                    (band.height() - height * done).abs() < 1e-3,
                    "band {index} at {done}: {band:?}"
                );
                assert!(
                    (band.width() - 200.0).abs() < 1e-3,
                    "a horizontal line spans the width"
                );
            }
        }

        let mut vertical = of(Style::Blinds);
        vertical.dimension = Dimension::Vertical;
        let shaped = frame(&vertical, viewport(), 0.5).expect("shaped");
        let width = 200.0 / f32::from(BLINDS);
        for (index, band) in shaped.layers[1].reveal.iter().enumerate() {
            let left = width * f32::from(u16::try_from(index).expect("eight bands"));
            assert!(
                (band.min.x - left).abs() < 1e-3,
                "vertical lines move to the right"
            );
            assert!((band.width() - width / 2.0).abs() < 1e-3, "{band:?}");
            assert!((band.height() - 100.0).abs() < 1e-3);
        }
    }

    /// A `Dissolve` has replaced exactly the fraction of its cells the transition has run for,
    /// and a cell once replaced stays replaced.
    ///
    /// Table 164: "The old page dissolves gradually to reveal the new one." The viewport is 200 by
    /// 100, so the cells are 5 pixels square, 40 by 20 of them, and half of 800 cells is half the
    /// view.
    #[test]
    fn a_dissolve_replaces_the_share_of_its_cells_it_has_run_for() {
        let at = |done: f32| frame(&of(Style::Dissolve), viewport(), done).expect("shaped");
        assert!(
            area(&at(0.0).layers[1].reveal) < 0.01,
            "nothing at the start"
        );
        assert!(
            (area(&at(0.5).layers[1].reveal) - 10_000.0).abs() < 0.5,
            "half the view half way: {}",
            area(&at(0.5).layers[1].reveal)
        );
        assert!(
            (area(&at(1.0).layers[1].reveal) - 20_000.0).abs() < 0.5,
            "everything at the end"
        );
        // Gradually: every cell replaced at a third is still replaced at two thirds, and the same
        // fraction asked twice is the same frame.
        let (early, late) = (at(1.0 / 3.0), at(2.0 / 3.0));
        for row in 0..20_u16 {
            for column in 0..40_u16 {
                let centre = Point::new(2.5 + 5.0 * f32::from(column), 2.5 + 5.0 * f32::from(row));
                assert!(
                    !revealed(&early, centre) || revealed(&late, centre),
                    "cell {column},{row} came back"
                );
            }
        }
        assert_eq!(at(0.4), at(0.4), "a pure function of the fraction");
        assert_ne!(
            cell_key(0),
            cell_key(1),
            "and the order is not the grid's own"
        );
    }

    /// A `Glitter` has replaced everything its band has passed and nothing it has not reached.
    ///
    /// Table 164: "Similar to Dissolve , except that the effect sweeps across the page in a wide
    /// band moving from one side of the screen to the other in the direction specified by the Di
    /// entry." Left to right at a half: the band's leading edge is at `0.5 × (1 + band)` of the
    /// width and its trailing edge one band behind, and a cell is judged at its centre, so the
    /// margins below are one cell wide.
    #[test]
    fn a_glitter_is_a_dissolve_inside_a_band_that_sweeps_along_di() {
        let shaped = frame(&of(Style::Glitter), viewport(), 0.5).expect("shaped");
        let front = 0.5 * (1.0 + GLITTER_BAND) * 200.0;
        let behind = front - GLITTER_BAND * 200.0;
        for row in 0..20_u16 {
            let y = 2.5 + 5.0 * f32::from(row);
            for column in 0..40_u16 {
                let x = 2.5 + 5.0 * f32::from(column);
                if x < behind - 5.0 {
                    assert!(
                        revealed(&shaped, Point::new(x, y)),
                        "{x},{y} is behind the band"
                    );
                }
                if x > front + 5.0 {
                    assert!(
                        !revealed(&shaped, Point::new(x, y)),
                        "{x},{y} is ahead of it"
                    );
                }
            }
        }
        // And the band is a dissolve rather than a wipe: inside it some cells have gone and some
        // have not.
        let inside: Vec<bool> = (0..20_u16)
            .map(|row| revealed(&shaped, Point::new(97.5, 2.5 + 5.0 * f32::from(row))))
            .collect();
        assert!(
            inside.contains(&true) && inside.contains(&false),
            "{inside:?}"
        );

        // 315 is "[t]op-left to bottom-right", Glitter's own: the top-left corner goes first.
        let mut diagonal = of(Style::Glitter);
        diagonal.direction = Direction::Degrees(315.0);
        let shaped = frame(&diagonal, viewport(), 0.5).expect("shaped");
        assert!(
            revealed(&shaped, Point::new(2.5, 2.5)),
            "the corner it starts from"
        );
        assert!(
            !revealed(&shaped, Point::new(197.5, 97.5)),
            "and not the one it ends at"
        );
    }

    /// A `Fly` in the table's own directions: the changes travel from offscreen, at `/SS`.
    ///
    /// Table 164's `/SS`: "If M specifies an inward transition, the scale of the changes drawn
    /// shall progress from SS to 1.0 over the course of the transition." At the start the flown
    /// picture is `/SS` of its size and just off the left edge — its right edge, `100 + offset +
    /// 100 × SS`, at zero.
    #[test]
    fn a_fly_carries_the_changes_from_offscreen_at_its_starting_scale() {
        let mut inward = of(Style::Fly);
        inward.scale = 0.5;
        let began = frame(&inward, viewport(), 0.0).expect("shaped");
        assert_eq!(
            began.layers[0].face,
            Face::Outgoing,
            "the old page holds still"
        );
        let flown = &began.layers[1];
        assert_eq!(flown.face, Face::Flown);
        assert!((flown.scale - 0.5).abs() < 1e-6);
        assert!(
            (100.0 + flown.offset.0 + 100.0 * flown.scale).abs() < 1e-3,
            "{flown:?}"
        );
        let ended = frame(&inward, viewport(), 1.0).expect("shaped");
        assert_eq!(ended.layers[1].offset, (0.0, 0.0), "at rest");
        assert!(
            (ended.layers[1].scale - 1.0).abs() < 1e-6,
            "at its own size"
        );

        // Outward is the mirror: from rest to offscreen, from 1 to `/SS`, over the new page.
        let mut outward = inward.clone();
        outward.motion = Motion::Outward;
        outward.direction = Direction::Degrees(270.0);
        let ended = frame(&outward, viewport(), 1.0).expect("shaped");
        assert_eq!(ended.layers[0].face, Face::Incoming);
        assert!((ended.layers[1].scale - 0.5).abs() < 1e-6);
        assert!(
            (50.0 + ended.layers[1].offset.1 - 25.0 - 100.0).abs() < 1e-3,
            "downward, its top edge at the bottom of the view: {:?}",
            ended.layers[1]
        );

        // `/Di /None` scales in place, and is not drawn at all where `/SS` is 1.
        let mut still = inward.clone();
        still.direction = Direction::None;
        let half = frame(&still, viewport(), 0.5).expect("shaped");
        assert_eq!(half.layers[1].offset, (0.0, 0.0));
        assert!((half.layers[1].scale - 0.75).abs() < 1e-6);
        still.scale = 1.0;
        assert!(frame(&still, viewport(), 0.5).is_none());
    }

    /// A page of one colour with a rectangle of another, at the viewport's size.
    fn page(background: [u8; 4], mark: [u8; 4], marked: impl Fn(u32, u32) -> bool) -> Image {
        let mut data = Vec::with_capacity(200 * 100 * 4);
        for y in 0..100 {
            for x in 0..200 {
                data.extend_from_slice(if marked(x, y) { &mark } else { &background });
            }
        }
        Image {
            width: 200,
            height: 100,
            data: data.into(),
            interpolate: false,
            sample_alpha: pdf_render::SampleAlpha::Shape,
        }
    }

    /// Where pixel `(x, y)` of a 200-wide raster starts.
    fn pixel(x: usize, y: usize) -> usize {
        y.saturating_mul(200).saturating_add(x).saturating_mul(4)
    }

    /// The colour a frame puts at one device pixel.
    fn drawn(shaped: &Frame, faces: &Faces, x: usize, y: usize) -> Vec<u8> {
        let list = shaped.draw(viewport(), faces).expect("a frame builds");
        let raster = render_cpu::CpuRasterizer::new()
            .rasterize(
                &list,
                TargetSpec {
                    width: 200,
                    height: 100,
                    transform: Transform::IDENTITY,
                },
            )
            .expect("the CPU backend draws a frame");
        let at = pixel(x, y);
        raster.data[at..at.saturating_add(3)].to_vec()
    }

    /// A `Fly` begins on the old page and lands on the new one, flying only what changed.
    ///
    /// The old page is white with a red square on the left; the new one is white with a blue
    /// square on the right. What differs is both squares' pixels, so the flown picture is the new
    /// page there and nothing elsewhere: half way through a flight in from the left, the blue
    /// square is half a view short of its place and the old red square still shows.
    #[test]
    fn a_fly_begins_on_the_old_page_and_lands_on_the_new_one() {
        let white = [255, 255, 255, 255];
        let outgoing = page(white, [255, 0, 0, 255], |x, y| {
            (20..40).contains(&x) && (40..60).contains(&y)
        });
        let incoming = page(white, [0, 0, 255, 255], |x, y| {
            (160..180).contains(&x) && (40..60).contains(&y)
        });
        let transition = of(Style::Fly);
        let faces = Faces::new(&transition, outgoing, incoming);

        let began = frame(&transition, viewport(), 0.0).expect("shaped");
        assert_eq!(
            drawn(&began, &faces, 30, 50),
            vec![255, 0, 0],
            "the old page's square"
        );
        assert_eq!(drawn(&began, &faces, 170, 50), vec![255, 255, 255]);

        let half = frame(&transition, viewport(), 0.5).expect("shaped");
        assert_eq!(
            drawn(&half, &faces, 70, 50),
            vec![0, 0, 255],
            "the new square in flight"
        );
        assert_eq!(
            drawn(&half, &faces, 170, 50),
            vec![255, 255, 255],
            "not yet landed"
        );

        let ended = frame(&transition, viewport(), 1.0).expect("shaped");
        assert_eq!(drawn(&ended, &faces, 170, 50), vec![0, 0, 255], "landed");
        assert_eq!(
            drawn(&ended, &faces, 30, 50),
            vec![255, 255, 255],
            "and the old square is covered by the new page's white, which is a change too"
        );
    }

    /// `/B` makes the flown area the changes' bounding rectangle, opaque.
    ///
    /// Table 164: "If true , the area that shall be flown in is rectangular and opaque." Two
    /// changed pixels at opposite corners of a square span the square, so under `/B` the white
    /// between them flies too.
    #[test]
    fn b_flies_the_rectangle_round_the_changes() {
        let white = [255, 255, 255, 255];
        let outgoing = page(white, white, |_, _| false);
        let incoming = page(white, [0, 0, 0, 255], |x, y| {
            (x, y) == (50, 20) || (x, y) == (90, 60)
        });
        let mut transition = of(Style::Fly);
        transition.opaque = true;
        let faces = Faces::new(&transition, outgoing.clone(), incoming.clone());
        let flown = faces.image(Face::Flown);
        let alpha = |x: usize, y: usize| flown.data[pixel(x, y).saturating_add(3)];
        assert_eq!(alpha(70, 40), u8::MAX, "inside the rectangle, opaque");
        assert_eq!(alpha(10, 10), 0, "outside it, nothing");

        transition.opaque = false;
        let faces = Faces::new(&transition, outgoing, incoming);
        let flown = faces.image(Face::Flown);
        assert_eq!(
            flown.data[(40 * 200 + 70) * 4 + 3],
            0,
            "without /B, only what changed"
        );
        assert_eq!(flown.data[(20 * 200 + 50) * 4 + 3], u8::MAX);
    }

    /// A `Box` sweeping inward reveals the complement of a shrinking rectangle: four bands that
    /// tile what is outside it exactly once.
    #[test]
    fn an_inward_box_reveals_four_bands_that_tile_what_is_outside_it() {
        let shaped = frame(&of(Style::Box), viewport(), 0.5).expect("shaped");
        let bands = &shaped.layers[1].reveal;
        assert_eq!(bands.len(), 4, "{bands:?}");
        // The inner rectangle is half the viewport's extent on each axis, so a quarter of its
        // area, and what is outside it is the other three quarters.
        assert!(
            (area(bands) - 200.0 * 100.0 * 0.75).abs() < 0.01,
            "{} of an expected 15000",
            area(bands)
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

    /// The four styles drawn at a chosen quantity say so, and the one the table defines as a cut
    /// says nothing.
    ///
    /// `doc/questions/A72`: where a clause names the kind of mark and withholds only a quantity,
    /// the quantity is chosen, written down as this program's, and a report says it is ours.
    #[test]
    fn a_style_drawn_at_a_chosen_quantity_says_the_quantity_is_ours() {
        for style in [Style::Blinds, Style::Glitter, Style::Dissolve, Style::Fly] {
            assert!(
                frame(&of(style.clone()), viewport(), 0.5).is_some(),
                "{style:?}"
            );
            let said = note(&of(style)).expect("a sentence naming the choice");
            assert!(
                said.contains("Table 164") && said.contains("this program's own"),
                "{said}"
            );
        }
        assert!(note(&of(Style::Blinds)).is_some_and(|said| said.contains("8 lines")));
        // `R` is the cut, by the table's own definition, so there is nothing to report.
        assert!(frame(&of(Style::Replace), viewport(), 0.5).is_none());
        assert_eq!(note(&of(Style::Replace)), None);
        // And the seven the table determines have nothing to say either.
        assert_eq!(note(&of(Style::Wipe)), None);
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

    /// A report fires on exactly what is not drawn or drawn at a chosen quantity, over every style
    /// and every direction.
    ///
    /// Trap 11, and the defect that put this here: a report keyed on the *style* beside a frame
    /// decided by the style **and** `/Di` let a `Wipe` at an angle Table 164 does not give it
    /// arrive as a cut with nothing said, which is the one outcome trap 5 forbids. Holding the two
    /// expressions against each other over the whole cross-product is what a list written out by
    /// hand cannot do: a style added to either side without the other fails here.
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
        let chosen = [Style::Blinds, Style::Dissolve, Style::Glitter, Style::Fly];
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
            // transition when the value of SS is not 1.0."
            Direction::None,
        ];
        for style in styles {
            for direction in directions {
                for scale in [1.0, 0.5] {
                    let mut transition = of(style.clone());
                    transition.direction = direction;
                    transition.scale = scale;
                    let drawn = frame(&transition, viewport(), 0.5).is_some();
                    let said = note(&transition);
                    let owed = if drawn {
                        chosen.contains(&style)
                    } else {
                        style != Style::Replace
                    };
                    assert_eq!(
                        said.is_some(),
                        owed,
                        "{style:?} at {direction:?}, /SS {scale}: drawn {drawn}, said {said:?}"
                    );
                    if let (true, Some(said)) = (drawn, &said) {
                        assert!(said.contains("this program's own"), "{said}");
                    }
                }
            }
        }
    }

    /// `/Di` is read as an angle, and only the four quarter turns describe a rectangular sweep.
    ///
    /// 315 is Table 164's fifth value and belongs to `Glitter` alone; the name `None` "is
    /// relevant only for the Fly transition". Neither may quietly become one of the four.
    #[test]
    fn only_the_four_quarter_turns_name_a_sweep() {
        assert_eq!(quarter(Direction::Degrees(0.0)), Some(Quarter::Rightward));
        assert_eq!(quarter(Direction::Degrees(360.0)), Some(Quarter::Rightward));
        assert_eq!(quarter(Direction::Degrees(-90.0)), Some(Quarter::Downward));
        assert_eq!(quarter(Direction::Degrees(315.0)), None);
        assert_eq!(quarter(Direction::None), None);
        assert_eq!(quarter(Direction::Degrees(f32::NAN)), None);

        // And a `Wipe` at an angle no rectangle sweeps is reported rather than drawn at some
        // nearby angle the file did not ask for.
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
