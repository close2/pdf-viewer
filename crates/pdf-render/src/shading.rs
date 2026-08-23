//! Smooth colour transitions, resolved to a device-independent form.
//!
//! PDF defines seven shading types. They arrive here with their colour spaces already
//! resolved to RGB and their functions already evaluated, for the same reason [`Color`]
//! does: colour management belongs in one place, upstream, so that the backends cannot
//! disagree about it.
//!
//! What survives that resolution is *geometry* — where a colour transition starts and
//! ends — plus the colours it passes through. That is what a backend needs and all it
//! needs.
//!
//! # Why the types collapse into four
//!
//! The specification's seven types describe three different things. Axial (2) and radial
//! (3) are the two the underlying rasterisers implement natively, so they stay distinct.
//! Function-based shadings (1) are an arbitrary function of two variables and reduce to a
//! grid of samples — on a grid the *device* chooses, through [`DeferredColours`], because
//! the function machinery lives above this crate and the resolution question has no answer
//! until something says how large the domain will be drawn. The four mesh types (4, 5, 6, 7)
//! all describe the same thing — patches of smoothly varying colour — and differ only in how
//! the file writes them down; Coons and tensor patches are subdivided into the triangles
//! that types 4 and 5 give directly, so all four arrive here as triangles.
//!
//! Nothing is lost by that grouping except the name of the type, which no backend needs.

use std::sync::Arc;

use crate::geom::{Point, Transform};
use crate::paint::{Color, Grid};

/// A colour transition, together with the space it is defined in.
#[derive(Debug, Clone, PartialEq)]
pub struct Shading {
    /// The geometry of the transition, in the shading's own coordinates.
    ///
    /// Shared rather than owned because one shading object is commonly painted many times —
    /// a pattern filling every cell of a chart, or an `sh` inside a form invoked once per
    /// data point — and each of those is the same colours under a different transform.
    /// `bug1721218_reduced.pdf` paints 3576 of them from three function objects, which is
    /// why this is an `Arc` and `pdf_model::shading::Cache` exists: building the kind again
    /// per use was 14% of that page (ADR 0069).
    pub kind: Arc<ShadingKind>,
    /// Maps the shading's own coordinates into the space the command is drawn in.
    ///
    /// Separate from the drawn path's transform because they are genuinely different: a
    /// shading used as a pattern is positioned by the *pattern* matrix relative to the
    /// page, not by the transform in force when the path was filled.
    pub transform: Transform,
    /// ISO 32000-2 §8.7.4.3 Table 77's `/Background`, resolved through the shading's own
    /// colour space — the colour this shading answers where its geometry answers nothing.
    ///
    /// > ( Optional ) An array of colour components appropriate to the colour space,
    /// > specifying a single background colour value. If present, this colour shall be used,
    /// > before any painting operation involving the shading, to fill those portions of the
    /// > area to be painted that lie outside the bounds of the shading object.
    ///
    /// # Why it is a colour on the paint rather than a second fill under it
    ///
    /// Table 77's NOTE 1 describes the two-operation construction — "the effect is as if the
    /// painting operation were performed twice" — and says which model it holds in: *the
    /// opaque imaging model*. §11.6.7 states the construction for the transparent one, and
    /// this field is that sentence:
    ///
    /// > If the shading dictionary has a Background entry, the pattern's imp licit
    /// > transparency group shall be filled with the specified background colour before the
    /// > sh operator is invoked.
    ///
    /// The wash goes *inside* the pattern's implicit group, so the object's shape, its
    /// §11.6.4.4 constant alpha and its blend mode apply once to the two of them together.
    /// Filling the path twice on this device would instead put the background into every
    /// boundary pixel's anti-aliased coverage a second time — `(1 − c)²` where the clause
    /// leaves `1 − c` — and apply `ca` twice inside the bounds.
    ///
    /// # Why only a pattern carries one
    ///
    /// Table 77 confines it: "applied only when the shading is used as part of a shading
    /// pattern, not when painted directly with the sh operator". §8.7.4.2 says the same of the
    /// operator, and §11.6.4.2 a third time of the shape a `sh` contributes. So the
    /// interpreter sets this where a `/PatternType 2` pattern is resolved and nowhere else,
    /// and [`Shading::painting_bounds`] is unaffected by it.
    pub background: Option<Color>,
}

impl Shading {
    /// Whether every colour this shading can paint is fully opaque.
    ///
    /// Asked for the same reason as [`crate::Image::is_opaque`]: §11.4.6's knockout differs
    /// from ordinary compositing only where the upper object is not opaque, and a shading's
    /// alpha lives in its colours rather than in a single field a caller can read.
    ///
    /// A shading that does not extend leaves part of its region unpainted, which is a shape
    /// of zero rather than an opacity, so it is not what this answers. A `/Background` is the
    /// one thing that turns such a region into a colour, so it is one of the colours asked
    /// about here.
    #[must_use]
    pub fn is_opaque(&self) -> bool {
        let opaque = |colour: &Color| colour.a >= 1.0;
        if self.background.is_some_and(|colour| !opaque(&colour)) {
            return false;
        }
        match self.kind.as_ref() {
            ShadingKind::Axial { ramp, .. } | ShadingKind::Radial { ramp, .. } => {
                ramp.stops.iter().all(|stop| opaque(&stop.colour))
            }
            ShadingKind::Sampled { source, .. } => source.is_opaque(),
            // A parametric mesh's colours are all in its ramp, and its corners hold none.
            ShadingKind::Mesh { triangles, ramp } => match ramp {
                Some(ramp) => ramp.stops.iter().all(|stop| opaque(&stop.colour)),
                None => triangles.iter().all(|triangle| match &triangle.corners {
                    Corners::Colours(colours) => colours.iter().all(opaque),
                    Corners::Parameters(_) => false,
                }),
            },
        }
    }

    /// The rectangle, in the shading's own coordinates, outside which it paints nothing.
    ///
    /// ISO 32000-2 §11.6.4.2 states what a `sh` marks, and it is a property of the shading
    /// rather than of the path that stands in for one:
    ///
    /// > For objects painted with the sh operator (8.7.4.2, "Shading operator"), the shape
    /// > shall be 1.0 inside and 0.0 outside the bounds of the shading's painti ng geometry,
    /// > disregarding the Background entry in the shading dictionary (see 8.7.4.3, "Shading
    /// > dictionaries").
    ///
    /// `None` where those bounds are not a rectangle in this space. An axial shading paints
    /// an infinite strip either side of its axis and a radial one an expanding cone, both
    /// unbounded whatever `/Extend` says, so neither has one; a type 1 shading's is its
    /// `/Domain`, which the interpreter already applies as §8.7.4.5.2's clip. What is left is
    /// the mesh types, whose geometry is exactly the triangles the file states.
    ///
    /// # Why a caller wants it rather than the page
    ///
    /// A `sh` has no path — §8.7.4.2: "This operator does not require the creation of a
    /// pattern dictionary or a path and works without reference to the current colour in the
    /// graphics state" — so a display list has to fill *something*, and a page-sized rectangle
    /// is the obvious stand-in. It is the wrong one inside a tiling pattern's cell, because
    /// [`crate::Cell::repeat`] displaces every command's geometry and the page is not part of
    /// the figure being moved: the site whose shading lands on the page is the site whose
    /// stand-in rectangle has just left it. A rectangle taken from the shading's own
    /// coordinates travels with the shading and is therefore over it at every site.
    #[must_use]
    pub fn painting_bounds(&self) -> Option<[f32; 4]> {
        let ShadingKind::Mesh { triangles, .. } = self.kind.as_ref() else {
            return None;
        };
        let mut bounds: Option<[f32; 4]> = None;
        for point in triangles.iter().flat_map(|triangle| triangle.points) {
            if !point.x.is_finite() || !point.y.is_finite() {
                return None;
            }
            bounds = Some(match bounds {
                None => [point.x, point.y, point.x, point.y],
                Some([x0, y0, x1, y1]) => [
                    x0.min(point.x),
                    y0.min(point.y),
                    x1.max(point.x),
                    y1.max(point.y),
                ],
            });
        }
        bounds
    }

    /// Returns this shading with every colour's alpha scaled by `alpha`.
    ///
    /// A shading is the one paint whose colours are not a single [`Color`] the caller can
    /// modify, so a constant alpha has to reach *every* colour it carries or reach none of
    /// them. ISO 32000-2 §11.6.4.4 makes it every: the alpha constants are a property of the
    /// graphics state applied to "all other painting operations" — a path filled with a
    /// shading pattern and an `sh` alike — rather than of the colour being painted.
    ///
    /// The clone is deliberate and paid for only where `alpha` is below 1: shadings are
    /// shared behind an `Arc` because one pattern commonly paints many paths, and a
    /// half-transparent fill of that pattern is a different paint from an opaque one.
    #[must_use]
    pub fn with_alpha(&self, alpha: f32) -> Self {
        let scale = |colour: &Color| Color {
            a: colour.a * alpha,
            ..*colour
        };
        let ramp = |ramp: &Ramp| Ramp {
            stops: ramp
                .stops
                .iter()
                .map(|stop| Stop {
                    at: stop.at,
                    colour: scale(&stop.colour),
                })
                .collect(),
        };
        let kind = match self.kind.as_ref() {
            ShadingKind::Axial {
                start,
                end,
                ramp: colours,
                extend,
            } => ShadingKind::Axial {
                start: *start,
                end: *end,
                ramp: ramp(colours),
                extend: *extend,
            },
            ShadingKind::Radial {
                start,
                start_radius,
                end,
                end_radius,
                ramp: colours,
                extend,
            } => ShadingKind::Radial {
                start: *start,
                start_radius: *start_radius,
                end: *end,
                end_radius: *end_radius,
                ramp: ramp(colours),
                extend: *extend,
            },
            // A sampled shading's colours do not exist yet, so the alpha travels with the
            // producer and reaches each colour as it is produced.
            // The program goes, and it has to: a device evaluating it produces the colour and
            // nothing else, so §11.6.4.4's constant alpha has nowhere to be applied on that
            // path. The producer carries it (`faded`) and the producer is what draws.
            ShadingKind::Sampled {
                domain,
                source,
                program: _,
            } => ShadingKind::Sampled {
                domain: *domain,
                source: source.faded(alpha),
                program: None,
            },
            // A parametric mesh carries its colours in the ramp, so that is where the alpha
            // goes; a corner holding a parameter has none to scale.
            ShadingKind::Mesh {
                triangles,
                ramp: colours,
            } => ShadingKind::Mesh {
                triangles: triangles
                    .iter()
                    .map(|triangle| Triangle {
                        points: triangle.points,
                        corners: match triangle.corners {
                            Corners::Colours(corners) => {
                                Corners::Colours(corners.map(|colour| scale(&colour)))
                            }
                            parameters @ Corners::Parameters(_) => parameters,
                        },
                    })
                    .collect(),
                ramp: colours.as_ref().map(ramp),
            },
        };
        Self {
            kind: Arc::new(kind),
            transform: self.transform,
            // §11.6.4.4's constant applies to the painting operation, and §11.6.7 puts the
            // wash inside the group that operation paints — so it is scaled with everything
            // else the shading answers rather than exempted from it.
            background: self.background.as_ref().map(scale),
        }
    }

    /// For a [`ShadingKind::Sampled`] shading: its colours, resolved for the device.
    ///
    /// `page_to_device` maps the space [`Self::transform`] targets onto the device, and
    /// `target` is that device's extent in pixels. The grid is [`Grid::for_placement`]'s
    /// answer for the placement carrying the unit square onto the transformed domain
    /// rectangle — derived here and nowhere else, because a resolution decision made per
    /// backend is a decision the backends can disagree about, which is the same reason
    /// [`Grid::for_placement`] itself lives in this crate — and the *block* of it worth
    /// producing is [`Patch::for_target`]'s, for the same reason and in the same place.
    ///
    /// `None` for every other kind, which is how a backend that draws sampled shadings its
    /// own way (as a pattern, as a clipped image) shares the one grid while keeping its own
    /// drawing.
    #[must_use]
    pub fn sampled_at(&self, page_to_device: Transform, target: (u32, u32)) -> Option<ColourGrid> {
        let ShadingKind::Sampled { domain, source, .. } = self.kind.as_ref() else {
            return None;
        };
        // The unit square onto the domain rectangle, then the shading's own matrix and the
        // caller's map carry it to the device — the same composition every backend draws
        // the resolved grid under, so the cells asked for are the pixels covered.
        let [x0, x1, y0, y1] = *domain;
        let onto_domain = Transform::new(x1 - x0, 0.0, 0.0, y1 - y0, x0, y0);
        let placement = onto_domain.then(self.transform).then(page_to_device);
        let grid = Grid::for_placement(placement);
        Some(source.colours(Patch::for_target(grid, placement, target)))
    }

    /// For a [`ShadingKind::Sampled`] shading: the program a device may evaluate instead of
    /// asking [`Self::sampled_at`] for a grid, where one was built.
    ///
    /// `None` for every other kind, and for a sampled shading `pdf_model::shading` found no
    /// device statement of — a function of another type, a colour space whose components are
    /// not the device's, or a `/Domain` the two paths would read differently. The backend that
    /// gets `None` draws the grid, which is what every backend did before ADR 0376 and what
    /// the correctness oracle still does.
    #[must_use]
    pub fn device_program(&self) -> Option<&crate::ShadingProgram> {
        match self.kind.as_ref() {
            ShadingKind::Sampled { program, .. } => program.as_ref(),
            _ => None,
        }
    }
}

/// Colours on a grid, standing in for §8.7.4.5.2's function of two variables.
///
/// Row-major with row zero at the `y_min` edge of [`Self::covers`], which is where the
/// shading's own coordinates put it: a backend stretches the grid over that rectangle —
/// [`Self::onto_shading`] is the transform — and the shading's transform carries any flip the
/// page states.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourGrid {
    /// Cells across [`Self::covers`], at least one.
    pub width: u32,
    /// Cells down [`Self::covers`], at least one.
    pub height: u32,
    /// Row-major colours, `width * height` of them.
    pub pixels: Arc<[Color]>,
    /// The part of the domain these cells cover, in the shading's own coordinates and in
    /// Table 78's `/Domain` order `[x_min, x_max, y_min, y_max]`.
    ///
    /// The whole domain wherever the target reaches all of it, which is every page drawn at
    /// a magnification that fits. Past that it is the block [`Patch::for_target`] asked for,
    /// and a backend that reads this instead of the shading's `/Domain` draws the same
    /// picture at a fraction of the evaluation. ADR 0408.
    pub covers: [f32; 4],
}

impl ColourGrid {
    /// The transform carrying this grid's own unit square onto the shading's coordinates.
    ///
    /// What a backend places the cells with, so that neither of them derives it from the
    /// shading's `/Domain` and a clipped block and a whole grid are drawn by one expression.
    #[must_use]
    pub fn onto_shading(&self) -> Transform {
        let [x0, x1, y0, y1] = self.covers;
        Transform::new(x1 - x0, 0.0, 0.0, y1 - y0, x0, y0)
    }
}

/// Which cells of a [`Grid`] a target can actually sample.
///
/// [`Grid`] says how *finely* a producer is asked for colours; this says which of them are
/// worth producing. A viewer past the magnification at which a page fits its window does not
/// rasterise the page — it rasterises the **window**, at a transform that scales the page and
/// translates the region of interest into view (`viewer-ui`'s own surface, and
/// `render-quorra/examples/zoom_ladder.rs`) — so a shading's domain grows with the zoom while
/// the target does not, and the share of the grid anybody can see falls as the square of the
/// magnification. Measured rather than reasoned: over the four corpus documents that state a
/// type 1 shading, `pdf-model/examples/shading_grid_census.rs` reads 55.7% of the resolved
/// cells inside a 900×1100 window at 1× and **5.5%** at 8×.
///
/// # Why this is exact
///
/// **The lattice is [`Self::grid`] whatever [`Self::within`] names.** A cell's colour is the
/// function at the cell's centre, and the centre is decided by the cell's index in the *full*
/// grid — so a cell inside the block carries the colour it would have carried had the block
/// been the whole grid, bit for bit rather than to a tolerance. Producing a smaller grid over
/// a smaller domain instead would move every sample by whatever the arithmetic rounded to,
/// and §8.7.4.5.2's function "need not be smooth or continuous": ADR 0406's own witness draws
/// digits through a `truncate`, where an ulp is a whole unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    /// The lattice the colours sit on — one cell per device pixel of the *whole* domain.
    pub grid: Grid,
    /// The part of the domain's unit square the target's own pixels reach, as
    /// `[u_min, u_max, v_min, v_max]`, each in `0.0..=1.0`.
    ///
    /// A producer answers for every cell this touches **and one more cell on each side**,
    /// because both backends read the grid with a bilinear filter: a device pixel just inside
    /// the target reads the cell either side of it, and a block cut exactly to the target
    /// would show the block's own edge colour there instead of the domain's. That margin is
    /// the whole of what the clip is conservative about.
    pub within: [f32; 4],
}

impl Patch {
    /// The whole grid: what a caller with no target to clip against asks for.
    #[must_use]
    pub fn whole(grid: Grid) -> Self {
        Self {
            grid,
            within: [0.0, 1.0, 0.0, 1.0],
        }
    }

    /// The part of `grid` a `target`-sized device can sample under `placement`.
    ///
    /// `placement` carries the domain's unit square onto the device, so the target's own
    /// rectangle mapped back through it is the part of the domain that can be seen. The four
    /// corners are mapped and their bounding box taken, which is exact under the scale and
    /// translation a viewer's zoom produces and conservative under a rotation or a skew — a
    /// clip that is too generous costs evaluations and a clip that is too tight costs
    /// pixels, so the box errs in the only direction that is safe.
    ///
    /// The whole grid where the placement cannot be inverted, which is a domain collapsed to
    /// a line: there is no back-mapping to take, and the producer's own answer for a
    /// degenerate placement is what it always was.
    #[must_use]
    pub fn for_target(grid: Grid, placement: Transform, target: (u32, u32)) -> Self {
        let Some(to_unit) = placement.invert() else {
            return Self::whole(grid);
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a target extent, bounded by MAX_EXTENT = 2^24, which f32 holds exactly"
        )]
        let (width, height) = (target.0 as f32, target.1 as f32);
        let mut box_ = [f32::MAX, f32::MIN, f32::MAX, f32::MIN];
        for (x, y) in [(0.0, 0.0), (width, 0.0), (0.0, height), (width, height)] {
            let unit = to_unit.apply(Point { x, y });
            if !unit.x.is_finite() || !unit.y.is_finite() {
                return Self::whole(grid);
            }
            box_[0] = box_[0].min(unit.x);
            box_[1] = box_[1].max(unit.x);
            box_[2] = box_[2].min(unit.y);
            box_[3] = box_[3].max(unit.y);
        }
        Self {
            grid,
            within: [
                box_[0].clamp(0.0, 1.0),
                box_[1].clamp(0.0, 1.0),
                box_[2].clamp(0.0, 1.0),
                box_[3].clamp(0.0, 1.0),
            ],
        }
    }
}

/// A sampled shading's colours, produced once the device grid is known.
///
/// The same contract as [`crate::ImageAtDeviceScale`], one paint over: [`Self::colours`] is
/// asked for a [`Patch`] — a lattice, and the block of it a target can sample — and answers
/// with colours **no finer than that lattice** in either axis. A producer bounding its own
/// work (§10.7.3 permits "internal limits") answers coarser, and the caller draws whatever
/// grid comes back stretched over [`ColourGrid::covers`]. It is infallible for the same reason
/// that trait is: everything checkable without evaluating was checked and reported by the
/// interpreter, and what remains is drawn as well as it can be.
///
/// Implementations are `Send + Sync` because a display list is drawn on every core.
pub trait ColoursAtDeviceScale: std::fmt::Debug + Send + Sync {
    /// The colours of `patch`'s block, on a lattice no finer than `patch.grid` in either axis.
    fn colours(&self, patch: Patch) -> ColourGrid;

    /// Whether every colour [`Self::colours`] can produce is fully opaque, answered without
    /// producing any.
    ///
    /// A method rather than the pessimistic constant [`crate::ImageSource::is_opaque`]
    /// returns for a deferred image, because the two facts differ: a deferred image exists
    /// *because* a second raster contributes per-sample alpha, while a sampled shading's
    /// colours are almost always opaque — and §11.4.6's knockout question, which
    /// [`Shading::is_opaque`] answers with this, would otherwise report a difference on
    /// every page a function-based shading touches.
    fn is_opaque(&self) -> bool;
}

/// A shared [`ColoursAtDeviceScale`], so that a shading carrying one stays cloneable.
#[derive(Clone)]
pub struct DeferredColours(Arc<dyn ColoursAtDeviceScale>);

impl DeferredColours {
    /// Wraps a producer of colours.
    #[must_use]
    pub fn new(source: Arc<dyn ColoursAtDeviceScale>) -> Self {
        Self(source)
    }

    /// The colours of `patch`'s block, on a lattice no finer than `patch.grid`.
    #[must_use]
    pub fn colours(&self, patch: Patch) -> ColourGrid {
        self.0.colours(patch)
    }

    /// Whether every colour this can produce is fully opaque, without producing any.
    #[must_use]
    pub fn is_opaque(&self) -> bool {
        self.0.is_opaque()
    }

    /// This source with every produced colour's alpha scaled by `alpha`.
    ///
    /// How [`Shading::with_alpha`] reaches colours that do not exist yet: §11.6.4.4's
    /// constant alpha travels with the producer and is applied to each colour as it is
    /// produced.
    fn faded(&self, alpha: f32) -> Self {
        Self(Arc::new(Faded {
            source: self.clone(),
            alpha,
        }))
    }
}

impl std::fmt::Debug for DeferredColours {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DeferredColours").field(&self.0).finish()
    }
}

impl PartialEq for DeferredColours {
    /// Two deferred sources are the same one when they are the same object.
    ///
    /// [`crate::paint::DeferredImage`]'s argument, unchanged: comparing the colours would
    /// mean producing them, and identity is what the display list's own `PartialEq` — which
    /// exists so a test can say a list was rebuilt unchanged — needs.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// A deferred source with a constant alpha applied to whatever it produces.
#[derive(Debug)]
struct Faded {
    source: DeferredColours,
    alpha: f32,
}

impl ColoursAtDeviceScale for Faded {
    fn colours(&self, patch: Patch) -> ColourGrid {
        let inner = self.source.colours(patch);
        ColourGrid {
            width: inner.width,
            height: inner.height,
            pixels: inner
                .pixels
                .iter()
                .map(|colour| Color {
                    a: colour.a * self.alpha,
                    ..*colour
                })
                .collect(),
            covers: inner.covers,
        }
    }

    fn is_opaque(&self) -> bool {
        self.alpha >= 1.0 && self.source.is_opaque()
    }
}

/// The geometry of a colour transition.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ShadingKind {
    /// Colour varies along a line, perpendicular to it (PDF type 2).
    Axial {
        /// Where the ramp's first colour sits.
        start: Point,
        /// Where its last colour sits.
        end: Point,
        /// The colours passed through.
        ramp: Ramp,
        /// Whether the shading continues beyond `start` and beyond `end`.
        ///
        /// Where it does not, nothing is painted there at all — which is not the same as
        /// painting the end colour, and is the difference between a band and a wash.
        extend: (bool, bool),
    },
    /// Colour varies between two circles (PDF type 3).
    Radial {
        /// Centre of the circle carrying the ramp's first colour.
        start: Point,
        /// Its radius.
        start_radius: f32,
        /// Centre of the circle carrying the ramp's last colour.
        end: Point,
        /// Its radius.
        end_radius: f32,
        /// The colours passed through.
        ramp: Ramp,
        /// Whether the shading continues beyond each circle.
        extend: (bool, bool),
    },
    /// Colour is an arbitrary function of position, resolved on the device's grid (PDF type 1).
    ///
    /// ISO 32000-2 §8.7.4.5.2:
    ///
    /// > In Type 1 (function-based) shadings, the colour at every point in the domain is
    /// > defined by a specified mathematical function. The function need not be smooth or
    /// > continuous.
    ///
    /// *Every point* is finer than any grid, and the function itself cannot travel: the
    /// function machinery lives above this crate, and the interpreter deliberately does not
    /// know the device scale — a display list is re-rasterisable at any zoom without being
    /// interpreted again. So the display list carries a *producer* instead, the same shape
    /// [`crate::ImageAtDeviceScale`] gives a raster the file does not hold (ADR 0210), and a
    /// backend resolves it through [`Shading::sampled_at`] once it knows how many device
    /// pixels the domain covers. Until the display list carried this, the grid was fixed at
    /// 128 cells per axis when the list was built — the one place in the display list where
    /// resolution was baked in, and exactly the interpret-time decision the deferred image
    /// removed for a soft mask.
    Sampled {
        /// The rectangle the function is defined over, as Table 78's `/Domain` order
        /// `[x_min, x_max, y_min, y_max]`.
        domain: [f32; 4],
        /// The colours, produced once a device has said how many cells the domain covers.
        source: DeferredColours,
        /// The *same* colours stated as the program that computes them, where the shading is
        /// one a device can be handed (ADR 0376) — a §7.10.5 type 4 function over a colour
        /// space whose components are the device's own.
        ///
        /// **An alternative statement, never a replacement.** `source` answers for every
        /// shading and is what the correctness oracle draws; this is present only where a
        /// backend has somewhere to put it, and a backend with nowhere ignores it. Where both
        /// are drawn the two are the same picture within a difference *of colour*, which is
        /// the currency §10.7.3 already measures a shading's accuracy in — never a difference
        /// of *branch*, which is what `pdf_model::shading`'s conditions on this field and the
        /// device's own admission between them keep out.
        program: Option<crate::ShadingProgram>,
    },
    /// Colour varies smoothly across triangles (PDF types 4, 5, 6 and 7).
    Mesh {
        /// The triangles, each carrying a colour or a parametric value per corner.
        triangles: Arc<[Triangle]>,
        /// The shading's `/Function`, sampled — present exactly where the corners carry
        /// [`Corners::Parameters`], because that is the entry whose presence makes them
        /// parameters. `None` where they carry colours.
        ramp: Option<Ramp>,
    },
}

/// A colour ramp: positions along a shading's parameter, each with a colour.
///
/// PDF states a shading's colours as a function of one parameter. Sampling it is what lets a
/// backend hand the result to a gradient implementation, which is how both rasterisers draw
/// these natively and quickly.
///
/// # Why the positions are carried rather than implied
///
/// An evenly spaced array cannot express a *step*. §8.7.4.5.3's colour at a point is whatever
/// the function says, and a type 3 stitching function with two equal `/Bounds` says green up to
/// one point and blue after it — a discontinuity, exactly representable by two stops at the
/// same position and not representable at all by samples that are averaged between.
/// `issue10572.pdf` is the page that made the difference visible: 24 hard stripes drawn as
/// seven-pixel gradients, because 256 even samples over an 1800-unit axis put a sample every
/// seven pixels and every step landed inside one interval.
#[derive(Debug, Clone, PartialEq)]
pub struct Ramp {
    /// The stops, in ascending position order, spanning `0.0..=1.0`. Never empty.
    pub stops: Arc<[Stop]>,
}

/// How far a dropped stop may sit from the line its neighbours draw, per channel.
///
/// Both rasterisers interpolate a gradient linearly between consecutive stops and deliver eight
/// bits per channel, so a stop within **half a level** of the line through the stops either side
/// of it produces the same byte whether it is there or not. `1.0 / 512.0` is that half level in
/// the `0.0..=1.0` this crate's [`Color`] uses.
///
/// It is a *lossless* bound rather than a quality knob: raising it would start changing pixels,
/// which is why it is stated as a fraction of a level and not as a tolerance.
const COLLINEAR: f32 = 1.0 / 512.0;

/// Drops the stops a rasteriser would have computed anyway.
///
/// [`Ramp::sample_across`] samples a colour function at [`Ramp::RESOLUTION`] positions because
/// that is the resolution at which a *function* has to be believed. What a gradient needs is
/// something else: the positions where the colour stops being a straight line. A shading whose
/// function is an exponential with `/N 1` — which is most of them, and every `/FunctionType 2`
/// interpolation between two colours — is one straight line and needs **two** stops, not 256.
///
/// The cost of the difference is not in building the ramp. `tiny-skia` walks a gradient's stop
/// list per pixel batch, so 256 stops is 128 times the search 2 stops is, and on
/// `bug1721218_reduced.pdf` `tiny_skia::pipeline::lowp::gradient` was **68% of a 144 G
/// instruction page**. Vello's shader does the same walk on the GPU.
///
/// The rule is exact rather than approximate: a stop is dropped only where every dropped stop
/// lies within [`COLLINEAR`] of the line the surviving neighbours draw, and both backends
/// interpolate linearly between those neighbours — so the colour a rasteriser computes at every
/// position is the same to eight bits. Two stops at one position, which is how
/// [`Ramp::sample_across`] expresses a discontinuity, are never collapsed into one: a vertical
/// segment fails the test at once.
fn simplify(stops: &[Stop]) -> Vec<Stop> {
    let Some(first) = stops.first().copied() else {
        return Vec::new();
    };
    let mut out = vec![first];
    let mut anchor = 0usize;
    let mut index = 1usize;
    while index < stops.len() {
        // Extend the run while every stop between the anchor and the candidate lies on the
        // line the two of them draw. Checking *all* of them, rather than only the one being
        // dropped, is what stops the error accumulating over a long run.
        let mut end = index;
        while end.saturating_add(1) < stops.len() {
            let next = end.saturating_add(1);
            let (Some(&start), Some(&finish)) = (stops.get(anchor), stops.get(next)) else {
                break;
            };
            let straight = stops
                .get(anchor.saturating_add(1)..next)
                .unwrap_or_default()
                .iter()
                .all(|middle| on_the_line(start, finish, *middle));
            if !straight {
                break;
            }
            end = next;
        }
        if let Some(&keep) = stops.get(end) {
            out.push(keep);
        }
        anchor = end;
        index = end.saturating_add(1);
    }
    out
}

/// Whether `middle` is what linear interpolation between `start` and `finish` would give.
fn on_the_line(start: Stop, finish: Stop, middle: Stop) -> bool {
    let span = finish.at - start.at;
    // A zero-width span is a discontinuity, and a stop inside one has no line to lie on. A
    // NaN position is neither, and `<=` answers false for it, which is the same refusal.
    if !span.is_finite() || span <= 0.0 {
        return false;
    }
    let fraction = ((middle.at - start.at) / span).clamp(0.0, 1.0);
    let between = |a: f32, b: f32| a + (b - a) * fraction;
    (middle.colour.r - between(start.colour.r, finish.colour.r)).abs() <= COLLINEAR
        && (middle.colour.g - between(start.colour.g, finish.colour.g)).abs() <= COLLINEAR
        && (middle.colour.b - between(start.colour.b, finish.colour.b)).abs() <= COLLINEAR
        && (middle.colour.a - between(start.colour.a, finish.colour.a)).abs() <= COLLINEAR
}

/// One entry of a [`Ramp`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    /// Where along the shading's parameter this colour applies, in `0.0..=1.0`.
    pub at: f32,
    /// The colour there.
    pub colour: Color,
}

impl Ramp {
    /// How many samples a ramp carries.
    ///
    /// Enough that a full-page gradient has more samples than it has pixels of gradient
    /// axis at ordinary magnifications, so the sampling is not what limits fidelity. The
    /// cost is one array of this length per shading, built once.
    pub const RESOLUTION: usize = 256;

    /// The most samples a ramp will carry, whatever a document asks for.
    ///
    /// ISO 32000-2 §10.7.3 lets a document state a smoothness tolerance and says in the same
    /// breath that "each output device may have internal limits". This is that limit, and it
    /// is a bound on *work* rather than a fidelity choice: a tolerance of 1/4096 already
    /// samples sixteen times more finely than the eight bits a channel is drawn with.
    pub const MAX_RESOLUTION: usize = 4096;

    /// How many samples §10.7.3's smoothness tolerance asks for.
    ///
    /// > Smoothness is the allowable colour error between a shading approximated by
    /// > piecewise linear interpolation and the true value of a (possibly nonlinear) shading
    /// > function.
    ///
    /// The tolerance is "expressed as a fraction of the range of the colour component, from
    /// 0.0 to 1.0", so a tolerance of `t` is honoured by sampling the colour function at
    /// least `1/t` times. This device's
    /// own limit is [`Self::RESOLUTION`], and it applies in one direction only:
    ///
    /// - A **coarser** tolerance than that — 0.02 is what the corpus states most often, five
    ///   times coarser — asks for a *maximum* error and is met by a finer answer. Coarsening
    ///   would trade fidelity for work on a page nobody measured, so it is not taken, and
    ///   §10.7.3's own sentence about internal limits is what permits keeping ours.
    /// - A **finer** one is honoured up to [`Self::MAX_RESOLUTION`]. 5 corpus documents state
    ///   0.002, which is finer than 1/256, and before this they got 1/256 and nothing said so.
    ///
    /// What the finer sampling buys is *detail*, not smoothness: ADR 0068 drops every stop
    /// that lies within half an eight-bit level of the line its neighbours draw, so extra
    /// samples of a smooth function are discarded again immediately. It is a feature narrower
    /// than 1/256 of the domain — a type 0 function with thousands of samples, a stitching
    /// function with narrow sub-domains — that a coarser sampling misses altogether.
    ///
    /// A tolerance outside `0.0..=1.0` is not a tolerance: Table 57 makes it a fraction, and
    /// one corpus document writes `/SM 6`. It is ignored, which is the same answer as absent.
    #[must_use]
    pub fn resolution_for(tolerance: Option<f32>) -> usize {
        /// [`Ramp::MAX_RESOLUTION`] as the float the comparison needs, written out because
        /// `usize as f32` is a lossy cast on a 64-bit target and this number is neither.
        const MAX_AS_FLOAT: f32 = 4096.0;

        let Some(tolerance) = tolerance.filter(|t| t.is_finite() && *t > 0.0 && *t <= 1.0) else {
            return Self::RESOLUTION;
        };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the reciprocal of a positive fraction, clamped to MAX_RESOLUTION before \
                      it is narrowed"
        )]
        let wanted = (1.0 / tolerance).ceil().min(MAX_AS_FLOAT) as usize;
        wanted.max(Self::RESOLUTION)
    }

    /// Builds a ramp by sampling a colour function evenly over `0.0..=1.0`.
    ///
    /// The function is called [`Self::RESOLUTION`] times and never afterwards, which is
    /// what keeps PDF functions out of the display list.
    #[must_use]
    pub fn sample(colour_at: impl FnMut(f32) -> Color) -> Self {
        Self::sample_across(&[], colour_at)
    }

    /// The same, with positions at which the function is known to be discontinuous.
    ///
    /// `breaks` are positions in `0.0..=1.0` — for a PDF shading, a type 3 function's
    /// `/Bounds` mapped onto the shading's own domain. Each gets **two** stops at the same
    /// position, taken from just below and just above it, which is how a gradient expresses a
    /// step; the samples between two breaks are spread evenly over the interval, so a ramp
    /// with no breaks is exactly what [`Self::sample`] used to build.
    ///
    /// The total number of stops stays near [`Self::RESOLUTION`]: a function with many bounds
    /// gets fewer samples inside each interval rather than more stops overall, because the
    /// stops are what a rasteriser walks per pixel batch.
    #[must_use]
    pub fn sample_across(breaks: &[f32], colour_at: impl FnMut(f32) -> Color) -> Self {
        Self::sample_across_at(Self::RESOLUTION, breaks, colour_at)
    }

    /// The same, at a resolution §10.7.3's smoothness tolerance chose.
    ///
    /// `resolution` comes from [`Self::resolution_for`]; every other caller wants
    /// [`Self::sample_across`], which is this at the device's own.
    #[must_use]
    pub fn sample_across_at(
        resolution: usize,
        breaks: &[f32],
        mut colour_at: impl FnMut(f32) -> Color,
    ) -> Self {
        /// How far either side of a break the two stops are sampled.
        ///
        /// Small enough that a continuous function's two values are the same colour to eight
        /// bits, and large enough to be a distinct `f32` anywhere in the unit interval.
        const NUDGE: f32 = 1e-5;

        let mut edges: Vec<f32> = breaks
            .iter()
            .copied()
            .filter(|at| at.is_finite() && *at > 0.0 && *at < 1.0)
            .collect();
        edges.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        edges.dedup();

        // Samples per interval, so that the whole ramp is about RESOLUTION stops however many
        // intervals there are. Three is the floor: the ends of the interval and its middle.
        let intervals = edges.len().saturating_add(1);
        let per_interval = resolution.checked_div(intervals).unwrap_or(3).max(3);

        let mut stops: Vec<Stop> = Vec::with_capacity(resolution.saturating_add(8));
        let mut low = 0.0f32;
        for index in 0..intervals {
            let high = edges.get(index).copied().unwrap_or(1.0);
            let span = high - low;
            for step in 0..per_interval {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "step and per_interval are bounded by MAX_RESOLUTION"
                )]
                let fraction = step as f32 / (per_interval.saturating_sub(1).max(1)) as f32;
                let at = low + span * fraction;
                // The last sample of an interval belongs to the function *below* the break,
                // and the first of the next to the function above it: two stops at one
                // position, which is the step.
                let sampled = if step.saturating_add(1) == per_interval && high < 1.0 {
                    (high - NUDGE).max(low)
                } else if step == 0 && low > 0.0 {
                    (low + NUDGE).min(high)
                } else {
                    at
                };
                stops.push(Stop {
                    at: at.clamp(0.0, 1.0),
                    colour: colour_at(sampled),
                });
            }
            low = high;
        }

        Self {
            stops: simplify(&stops).into(),
        }
    }

    /// Returns the colour at a position in `0.0..=1.0`, interpolating between stops.
    #[must_use]
    pub fn colour_at(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let Some(first) = self.stops.first() else {
            return Color::BLACK;
        };
        if t <= first.at {
            return first.colour;
        }
        let mut previous = *first;
        for stop in self.stops.iter().skip(1) {
            if t <= stop.at {
                let span = stop.at - previous.at;
                let fraction = if span > 0.0 {
                    (t - previous.at) / span
                } else {
                    1.0
                };
                let (a, b) = (previous.colour, stop.colour);
                return Color {
                    r: a.r + (b.r - a.r) * fraction,
                    g: a.g + (b.g - a.g) * fraction,
                    b: a.b + (b.b - a.b) * fraction,
                    a: a.a + (b.a - a.a) * fraction,
                };
            }
            previous = *stop;
        }
        previous.colour
    }
}

/// A mesh shading rasterised into device pixels, ISO 32000-2 §8.7.4.5.5.
///
/// # Why a raster rather than a pile of flat triangles
///
/// §8.7.4.5.5 states it in one sentence, of which the load-bearing half is:
///
/// > The colour at each vertex of the triangles is specified, and a technique known as
/// > Gouraud interpolation is used to colour the interiors.
///
/// Neither `tiny-skia` nor Vello has a Gouraud primitive, so both backends used to subdivide
/// a triangle until its corner colours agreed to within 1/512 and then fill the piece flat.
/// That produced three defects at once, and `issue2948.pdf` showed all three: a visible
/// lattice where the flat pieces meet, a *bias* — a piece takes the mean of its corners,
/// which on a ramp is not the colour at any of its pixels — and, because two abutting
/// antialiased edges do not sum to full coverage, seams that had to be closed by growing
/// every piece by 0.8 pixels, which is itself a departure nobody could derive.
///
/// Rasterising the mesh once, at device resolution, removes all three. A pixel's colour is
/// the clause's own interpolation at that pixel's centre; adjacent triangles tile exactly
/// under point sampling, so there are no seams to repair; and the arithmetic is the same on
/// both backends because it is this function.
///
/// # What is given up, and to what
///
/// The mesh's own outer boundary is point-sampled and therefore *not* antialiased. In every
/// case that matters it does not show: a mesh is painted through the path being filled, and
/// that path's edge is antialiased by the backend as it always was — so the hard edge appears
/// only where a mesh ends *inside* its own shape, which is a mesh that does not cover the
/// region its document asked it to fill. That is a real, small departure and it buys the
/// removal of a larger one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshRaster {
    /// Device x of the raster's first column.
    pub left: i32,
    /// Device y of the raster's first row.
    pub top: i32,
    /// Straight-alpha RGBA8 samples, one per device pixel.
    pub image: crate::Image,
}

impl MeshRaster {
    /// Rasterises `triangles` into the part of a `width` by `height` target they cover.
    ///
    /// `to_device` carries the mesh's own coordinates onto the target. `ramp` is
    /// [`ShadingKind::Mesh`]'s, and is what turns a [`Corners::Parameters`] corner into a
    /// colour *after* the interpolation §8.7.4.5.5 requires be done on the parameter.
    ///
    /// Returns `None` when the mesh covers no pixel of it, which a clipped-away or degenerate
    /// mesh does — and when a triangle carries parameters with no ramp to resolve them, which
    /// [`ShadingKind::Mesh`]'s own invariant rules out and which would otherwise be painted as
    /// though a parameter were a colour.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "every cast below is between a device pixel index and its coordinate, both                   bounded by the target's extent, which `MAX_EXTENT` keeps under 2^24"
    )]
    pub fn build(
        triangles: &[Triangle],
        ramp: Option<&Ramp>,
        to_device: Transform,
        width: u32,
        height: u32,
    ) -> Option<Self> {
        if triangles.is_empty() || width == 0 || height == 0 {
            return None;
        }
        if ramp.is_none()
            && triangles
                .iter()
                .any(|triangle| matches!(triangle.corners, Corners::Parameters(_)))
        {
            return None;
        }
        let device: Vec<Triangle> = triangles
            .iter()
            .map(|triangle| Triangle {
                points: triangle.points.map(|point| to_device.apply(point)),
                corners: triangle.corners,
            })
            .collect();

        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for triangle in &device {
            for point in triangle.points {
                if !point.x.is_finite() || !point.y.is_finite() {
                    return None;
                }
                x0 = x0.min(point.x);
                y0 = y0.min(point.y);
                x1 = x1.max(point.x);
                y1 = y1.max(point.y);
            }
        }
        // Half a pixel of margin on each side, because a pixel is sampled at its centre and a
        // triangle ending at x = 10.0 still covers the sample at 9.5.
        let left = (x0 - 0.5).floor().max(0.0) as u32;
        let top = (y0 - 0.5).floor().max(0.0) as u32;
        let right = (x1 + 0.5).ceil().max(0.0).min(width as f32) as u32;
        let bottom = (y1 + 0.5).ceil().max(0.0).min(height as f32) as u32;
        let (span, rows) = (right.checked_sub(left)?, bottom.checked_sub(top)?);
        if span == 0 || rows == 0 {
            return None;
        }

        let mut data = vec![
            0u8;
            (span as usize)
                .saturating_mul(rows as usize)
                .saturating_mul(4)
        ];
        for triangle in &device {
            triangle.paint(&mut data, ramp, left, top, span, rows);
        }

        Some(Self {
            left: i32::try_from(left).ok()?,
            top: i32::try_from(top).ok()?,
            image: crate::Image {
                width: span,
                height: rows,
                data: data.into(),
                // Nearest sampling: the raster is already at device resolution and is drawn
                // at 1:1, so no filter can be reached — and asking for one would let a
                // backend blur the mesh against the transparent pixels outside it.
                interpolate: false,
            },
        })
    }
}

/// The blend circle that paints a point, ISO 32000-2 §8.7.4.5.4.
///
/// # What the clause states, and what a gradient library states instead
///
/// §8.7.4.5.4 builds a *family* of circles indexed by a parameter `s`, with centre
/// `c(s) = c0 + s(c1 - c0)` and radius `r(s) = r0 + s(r1 - r0)`, and says how they compose:
///
/// > Conceptually, all of the blend circles shall be painted in order of increasing values of
/// > s , from smallest to largest. … The painting is opaque, with the colour of each circle
/// > completely overlaying those preceding it. Therefore, if a point lies on more than one
/// > blend circle, its final colour shall be that of the last of the enclosing circles to be
/// > painted, corresponding to the greatest value of s .
///
/// A point lies on `c(s)` exactly when `|p − c(s)| = r(s)`, which squares to a quadratic in
/// `s` with at most two roots. The clause therefore asks for **the greatest root that is
/// actually painted** — and *painted* is the second half of it, because `/Extend` decides
/// whether the family exists outside `[0, 1]` at all:
///
/// > If the first of the two elements is true , the shading shall be extended beyond the
/// > defined starting circle to values of s less than 0.0; if the second element is true , the
/// > shading shall be extended beyond the defined ending circle to s values greater than 1.0
/// > unless radii r 0 and r 1 in the Coords array are both zero.
///
/// So a root outside `[0, 1]` that `/Extend` does not admit is not "clamped" and is not
/// "transparent": it is **not a circle**, and the other root — if it is admissible — is the
/// one that paints the point. That fallback is what no two-point conical gradient expresses,
/// because a conical gradient solves for one root and clamps it with a spread mode. It is
/// why `radial_gradients.pdf` pages 4 and 5 draw a crescent under `tiny_skia::RadialGradient`
/// where four other renderers draw a filled disc with a cone on it.
///
/// NOTE 1's two limits need no separate handling. "[T]he family of blend circles continues as
/// far as that value of s for which the radius of the blend circle r(s) = 0" is
/// `r(s) >= 0` below, and "as far as that s value for which r(s) is large enough to encompass
/// the shading's entire bounding box" is a statement about *where painting stops being
/// visible*, not about which circle passes through a given point — solving for the root
/// answers that directly and needs no bounding box.
///
/// # Returns
///
/// The greatest admissible `s`, or `None` where no blend circle passes through the point —
/// which is a point the shading leaves unpainted, not a point painted with an end colour.
/// The caller clamps to `[0, 1]` before reading the ramp, because the clause paints an
/// extended circle "in the same colour defined […] for the starting circle" and likewise for
/// the ending one.
#[must_use]
pub fn blend_parameter(
    point: Point,
    start: Point,
    start_radius: f32,
    end: Point,
    end_radius: f32,
    extend: (bool, bool),
) -> Option<f32> {
    let (dx, dy, dr) = (end.x - start.x, end.y - start.y, end_radius - start_radius);
    let (px, py) = (point.x - start.x, point.y - start.y);

    // |p − c(s)|² = r(s)² expanded and collected in s.
    let a = dr.mul_add(-dr, dx.mul_add(dx, dy * dy));
    let b = -2.0 * start_radius.mul_add(dr, px.mul_add(dx, py * dy));
    let c = start_radius.mul_add(-start_radius, px.mul_add(px, py * py));

    // Both radii zero: r(s) is zero everywhere, so no circle has an interior and the
    // clause's own proviso withdraws the upper extension. Nothing is painted.
    if start_radius == 0.0 && end_radius == 0.0 {
        return None;
    }

    let admits = |s: f32| -> Option<f32> {
        if !s.is_finite() || start_radius.mul_add(1.0, s * dr) < 0.0 {
            return None;
        }
        if (s < 0.0 && !extend.0) || (s > 1.0 && !extend.1) {
            return None;
        }
        Some(s)
    };

    // The centres are exactly |dr| apart, so the quadratic degenerates to a line. This is
    // not a rounding case to be nudged: it is the geometry where the two circles are
    // internally tangent, and one root has gone to infinity.
    if a == 0.0 {
        if b == 0.0 {
            return None;
        }
        return admits(-c / b);
    }

    let discriminant = b.mul_add(b, -4.0 * a * c);
    if discriminant < 0.0 {
        return None;
    }
    // The stable form of the quadratic formula: adding the root of the discriminant to the
    // coefficient of like sign avoids the cancellation that costs the smaller root its
    // significant figures, and the smaller root is the one this function falls back to.
    let sign: f32 = if b < 0.0 { -1.0 } else { 1.0 };
    let q = -0.5 * sign.mul_add(discriminant.sqrt(), b);
    let first = q / a;
    let second = if q == 0.0 { first } else { c / q };
    let (lower, upper) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };

    admits(upper).or_else(|| admits(lower))
}

/// A radial shading rasterised into device pixels, ISO 32000-2 §8.7.4.5.4.
///
/// The same construction as [`MeshRaster`] and for the same reason: the clause states an
/// algorithm no rasteriser's native primitive implements, so it is evaluated once here, at
/// device resolution, and both backends draw the result as an image confined to the shape
/// being painted. The *colour* is this crate's and identical everywhere; the *edge* is the
/// shape's, antialiased by the backend as every other fill's is.
///
/// [`blend_parameter`] is why it cannot be a gradient. What is given up is the same thing
/// [`MeshRaster`] gives up — the shading's own boundary is point-sampled rather than
/// antialiased — and §10.7.4 asks for a hard edge there in any case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadialRaster {
    /// Device x of the raster's first column.
    pub left: i32,
    /// Device y of the raster's first row.
    pub top: i32,
    /// Straight-alpha RGBA8 samples, one per device pixel.
    pub image: crate::Image,
}

/// A radial shading's geometry and colours, as [`ShadingKind::Radial`] carries them.
///
/// A parameter list rather than six arguments, because [`RadialRaster::build`] would
/// otherwise take nine and the six travel together everywhere they appear.
#[derive(Debug, Clone, Copy)]
pub struct Radial<'a> {
    /// Centre of the circle carrying the ramp's first colour.
    pub start: Point,
    /// Its radius.
    pub start_radius: f32,
    /// Centre of the circle carrying the ramp's last colour.
    pub end: Point,
    /// Its radius.
    pub end_radius: f32,
    /// The colours passed through.
    pub ramp: &'a Ramp,
    /// Whether the shading continues beyond each circle.
    pub extend: (bool, bool),
}

impl RadialRaster {
    /// Rasterises a radial shading over the device pixels of `within`.
    ///
    /// `to_device` carries the shading's own coordinates onto the target. `within` is the
    /// region worth evaluating, in device pixels as `(left, top, right, bottom)` — the
    /// caller's shape bounds intersected with the target, because an extended shading covers
    /// everything and a raster the size of the page per shading is not a cost worth paying
    /// on a sheet of twenty-four of them.
    ///
    /// Returns `None` when the region is empty or the transform cannot be inverted.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "every cast is between a device pixel index and its coordinate, both \
                  bounded by the target's extent, which `MAX_EXTENT` keeps under 2^24 — \
                  except the channel quantisation, whose argument is clamped to [0, 1] \
                  one expression earlier and so cannot be negative"
    )]
    pub fn build(
        radial: Radial<'_>,
        to_device: Transform,
        within: (u32, u32, u32, u32),
    ) -> Option<Self> {
        let (left, top, right, bottom) = within;
        let (span, rows) = (right.checked_sub(left)?, bottom.checked_sub(top)?);
        if span == 0 || rows == 0 {
            return None;
        }
        let to_shading = to_device.invert()?;

        let mut data = vec![
            0u8;
            (span as usize)
                .saturating_mul(rows as usize)
                .saturating_mul(4)
        ];
        let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        for (row, line) in data
            .chunks_exact_mut((span as usize).saturating_mul(4))
            .enumerate()
        {
            let y = top.saturating_add(u32::try_from(row).unwrap_or(u32::MAX)) as f32 + 0.5;
            for (column, pixel) in line.chunks_exact_mut(4).enumerate() {
                // A pixel's colour is the shading's value at the pixel's centre, which is
                // the sampling `MeshRaster` uses and the one §10.7.4 describes for images.
                let device = Point {
                    x: left.saturating_add(u32::try_from(column).unwrap_or(u32::MAX)) as f32 + 0.5,
                    y,
                };
                let Some(s) = blend_parameter(
                    to_shading.apply(device),
                    radial.start,
                    radial.start_radius,
                    radial.end,
                    radial.end_radius,
                    radial.extend,
                ) else {
                    continue;
                };
                let colour = radial.ramp.colour_at(s.clamp(0.0, 1.0));
                pixel[0] = channel(colour.r);
                pixel[1] = channel(colour.g);
                pixel[2] = channel(colour.b);
                pixel[3] = channel(colour.a);
            }
        }

        Some(Self {
            left: i32::try_from(left).ok()?,
            top: i32::try_from(top).ok()?,
            image: crate::Image {
                width: span,
                height: rows,
                data: data.into(),
                // Nearest sampling, for `MeshRaster`'s reason: the raster is already at
                // device resolution and drawn at 1:1, so no filter can be reached.
                interpolate: false,
            },
        })
    }
}

/// A shading of any kind rasterised into device pixels, with ISO 32000-2 §8.7.4.3 Table 77's
/// `/Background` wherever its own geometry paints nothing.
///
/// # Why the wash is a raster rather than each backend's own gradient
///
/// §11.6.7 states the construction: the pattern's implicit transparency group "shall be filled
/// with the specified background colour before the sh operator is invoked", and the group's
/// colour, shape and opacity are then the object's source colour and shape. So the two are one
/// painting operation — one coverage at the path's edge, one §11.6.4.4 constant alpha, one
/// blend — and what the backends need is a single answer per device pixel: the shading's colour
/// inside its bounds and the background outside them.
///
/// All three backends already draw exactly that shape for a mesh and for §8.7.4.5.4's cone: a
/// straight-alpha raster at device resolution, placed at whole pixels and confined to the path
/// being filled ([`MeshRaster`], [`RadialRaster`], and `quorra_scene::Paint::Mesh`). Sending a
/// background-carrying shading of *any* kind down that same lane costs no new lane in any
/// backend, needs nothing of a gradient library that no gradient library has, and puts the
/// colour in one place — which is what trap 2 asks for and what the alternative, a stop or a
/// spread mode per rasteriser, would have spread across three.
///
/// The pricing this replaced went the other way round — a background-carrying stop for the two
/// gradient kinds, a clear colour for the two raster kinds, and an upstream ask for quorra's
/// gradient lane. Three of those four rows were wrong, and ADR 0529 has the derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadingRaster {
    /// Device x of the raster's first column.
    pub left: i32,
    /// Device y of the raster's first row.
    pub top: i32,
    /// Straight-alpha RGBA8 samples, one per device pixel.
    pub image: crate::Image,
}

impl ShadingRaster {
    /// Rasterises `shading` over the device pixels of `within`, washed with its `/Background`.
    ///
    /// `page_to_device` maps page space onto the target and `target` is that target's extent;
    /// `within` is the region worth evaluating, in device pixels as
    /// `(left, top, right, bottom)` — the caller's shape bounds intersected with the target,
    /// because Table 77's wash covers "the area to be painted" and that area is the path.
    ///
    /// Returns `None` where the shading states no background — a `sh`, or any shading not used
    /// as a pattern — where the region is empty, or where the placement cannot be inverted.
    #[must_use]
    pub fn build(
        shading: &Shading,
        page_to_device: Transform,
        within: (u32, u32, u32, u32),
        target: (u32, u32),
    ) -> Option<Self> {
        let background = shading.background?;
        let (left, top, right, bottom) = within;
        let (span, rows) = (right.checked_sub(left)?, bottom.checked_sub(top)?);
        if span == 0 || rows == 0 {
            return None;
        }
        let to_device = shading.transform.then(page_to_device);
        let to_shading = to_device.invert()?;

        // §11.6.7's first act: the group is filled with the background, and the shading is
        // painted into it afterwards.
        let mut data = vec![
            0u8;
            (span as usize)
                .saturating_mul(rows as usize)
                .saturating_mul(4)
        ];
        for pixel in data.chunks_exact_mut(4) {
            write_colour(pixel, background);
        }

        match shading.kind.as_ref() {
            ShadingKind::Axial {
                start,
                end,
                ramp,
                extend,
            } => {
                // §8.7.4.5.3 reads the colour from the point's projection onto the axis, and
                // an axis of zero length has no projection to take — so nothing is inside the
                // shading's bounds and the whole region stays the background.
                let (dx, dy) = (end.x - start.x, end.y - start.y);
                let length = dx.mul_add(dx, dy * dy);
                if length > 0.0 {
                    paint_pixels(&mut data, (left, top, span), to_shading, |point| {
                        let along = (point.x - start.x).mul_add(dx, (point.y - start.y) * dy);
                        admitted(along / length, *extend).map(|t| ramp.colour_at(t))
                    });
                }
            }
            ShadingKind::Radial {
                start,
                start_radius,
                end,
                end_radius,
                ramp,
                extend,
            } => {
                paint_pixels(&mut data, (left, top, span), to_shading, |point| {
                    // §8.7.4.5.4's greatest admissible blend circle, or no circle at all —
                    // which is precisely a point outside the shading's bounds.
                    blend_parameter(point, *start, *start_radius, *end, *end_radius, *extend)
                        .map(|s| ramp.colour_at(s.clamp(0.0, 1.0)))
                });
            }
            ShadingKind::Sampled { domain, .. } => {
                let grid = shading.sampled_at(page_to_device, target)?;
                let to_grid = grid.onto_shading().invert()?;
                // Table 78's order is [x min x max y min y max] and a file may write either
                // bound first, so the rectangle is taken by extent rather than by position.
                let [x0, x1, y0, y1] = *domain;
                let (xs, ys) = ((x0.min(x1), x0.max(x1)), (y0.min(y1), y0.max(y1)));
                paint_pixels(&mut data, (left, top, span), to_shading, |point| {
                    // §8.7.4.5.2: points that "fall outside this transformed domain rectangle
                    // shall be painted with the shading's background colour".
                    if point.x < xs.0 || point.x > xs.1 || point.y < ys.0 || point.y > ys.1 {
                        return None;
                    }
                    let cell = to_grid.apply(point);
                    Some(sample_grid(&grid, cell.x, cell.y))
                });
            }
            ShadingKind::Mesh { triangles, ramp } => {
                // The mesh's own rasterisation, over this region rather than over the mesh's
                // bounding box: every pixel no triangle covers keeps the background it was
                // filled with, which is the whole difference the entry makes.
                if ramp.is_none()
                    && triangles
                        .iter()
                        .any(|triangle| matches!(triangle.corners, Corners::Parameters(_)))
                {
                    return None;
                }
                for triangle in triangles.iter() {
                    Triangle {
                        points: triangle.points.map(|point| to_device.apply(point)),
                        corners: triangle.corners,
                    }
                    .paint(&mut data, ramp.as_ref(), left, top, span, rows);
                }
            }
        }

        Some(Self {
            left: i32::try_from(left).ok()?,
            top: i32::try_from(top).ok()?,
            image: crate::Image {
                width: span,
                height: rows,
                data: data.into(),
                // Nearest sampling, for [`MeshRaster`]'s reason: the raster is already at
                // device resolution and drawn at 1:1, so no filter can be reached.
                interpolate: false,
            },
        })
    }
}

/// Writes one straight-alpha RGBA8 pixel.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "each channel is clamped to [0, 1] before it is scaled, so the product is in \
              [0, 255] and cannot be negative"
)]
fn write_colour(pixel: &mut [u8], colour: Color) {
    for (slot, value) in pixel
        .iter_mut()
        .zip([colour.r, colour.g, colour.b, colour.a])
    {
        *slot = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

/// Whether a shading's parameter is one `/Extend` admits, clamped into `0.0..=1.0` where it is.
///
/// ISO 32000-2 §8.7.4.5.3 states both halves: outside `[0, 1]` the shading is extended only
/// where the corresponding element of `/Extend` is true, and where it is, the boundary colour
/// continues. `None` is a point outside the shading's bounds — the points Table 77's
/// `/Background` is about.
fn admitted(t: f32, extend: (bool, bool)) -> Option<f32> {
    if !t.is_finite() || (t < 0.0 && !extend.0) || (t > 1.0 && !extend.1) {
        return None;
    }
    Some(t.clamp(0.0, 1.0))
}

/// A [`ColourGrid`] read at a position in its own unit square, bilinearly.
///
/// The same filter the backends apply to a sampled shading's grid when they draw it as a
/// pattern, written here so that a background-carrying type 1 shading is the same picture
/// inside its domain as one without — and the same picture on all three backends, since this is
/// now the only place any of them samples one.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "grid extents are bounded by MAX_FUNCTION_CELLS, well inside f32's exact integer \
              range, and each index is clamped into the grid before it is narrowed"
)]
fn sample_grid(grid: &ColourGrid, u: f32, v: f32) -> Color {
    let at = |cells: u32, position: f32| -> (usize, usize, f32) {
        let last = cells.saturating_sub(1);
        // A cell's colour is the function at the cell's *centre*, so the sample position in
        // cell coordinates is half a cell behind the fraction of the domain.
        let scaled = position.mul_add(cells as f32, -0.5);
        let low = scaled.floor().clamp(0.0, f32::from(u16::MAX));
        let fraction = (scaled - low).clamp(0.0, 1.0);
        let low = (low as u32).min(last);
        let high = low.saturating_add(1).min(last);
        (low as usize, high as usize, fraction)
    };
    let (x0, x1, fx) = at(grid.width, u);
    let (y0, y1, fy) = at(grid.height, v);
    let cell = |x: usize, y: usize| -> Color {
        grid.pixels
            .get(y.saturating_mul(grid.width as usize).saturating_add(x))
            .copied()
            .unwrap_or(Color::BLACK)
    };
    let mix = |a: Color, b: Color, f: f32| Color {
        r: a.r + (b.r - a.r) * f,
        g: a.g + (b.g - a.g) * f,
        b: a.b + (b.b - a.b) * f,
        a: a.a + (b.a - a.a) * f,
    };
    let top = mix(cell(x0, y0), cell(x1, y0), fx);
    let bottom = mix(cell(x0, y1), cell(x1, y1), fx);
    mix(top, bottom, fy)
}

/// Writes `colour_at`'s answer into every pixel of a buffer it has one for.
///
/// The buffer's first pixel is device `(left, top)` and `to_shading` carries a device point
/// into the shading's own coordinates. A pixel's colour is the shading's value at the pixel's
/// *centre*, which is [`MeshRaster`]'s and [`RadialRaster`]'s sampling and the one §10.7.4
/// describes; a `None` leaves the background the caller filled the buffer with.
#[expect(
    clippy::cast_precision_loss,
    reason = "a device pixel index, bounded by the target's extent, which MAX_EXTENT keeps \
              under 2^24"
)]
fn paint_pixels(
    data: &mut [u8],
    (left, top, span): (u32, u32, u32),
    to_shading: Transform,
    colour_at: impl Fn(Point) -> Option<Color>,
) {
    for (row, line) in data
        .chunks_exact_mut((span as usize).saturating_mul(4))
        .enumerate()
    {
        let y = top.saturating_add(u32::try_from(row).unwrap_or(u32::MAX)) as f32 + 0.5;
        for (column, pixel) in line.chunks_exact_mut(4).enumerate() {
            let device = Point {
                x: left.saturating_add(u32::try_from(column).unwrap_or(u32::MAX)) as f32 + 0.5,
                y,
            };
            if let Some(colour) = colour_at(to_shading.apply(device)) {
                write_colour(pixel, colour);
            }
        }
    }
}

/// One triangle of a mesh shading, with what each corner carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    /// The corners, in the shading's own coordinates.
    pub points: [Point; 3],
    /// What each corner carries, in the same order.
    pub corners: Corners,
}

/// What a mesh triangle's corners carry, which is what gets interpolated across it.
///
/// ISO 32000-2 §8.7.4.5.5 gives a mesh shading two ways of stating colour, and the difference
/// between them is an *order of operations* rather than a format:
///
/// > If the shading dictionary contains a Function entry, the colour data for each vertex
/// > shall be specified by a single parametric value t rather than by n separate colour
/// > components. All linear interpolation within the triangle mesh shall be done using the t
/// > values. After interpolation, the results shall be passed to the function(s) specified in
/// > the Function entry to determine the colour at each point.
///
/// So a mesh with a `/Function` interpolates the parameter and colours afterwards; evaluating
/// the function at each corner and interpolating the *colours* is a different picture wherever
/// the function is not a straight line, and no report would name it. The distinction is in the
/// type because it cannot be recovered from a colour once one has been computed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Corners {
    /// A colour per corner: the mesh states its components and no `/Function`.
    Colours([Color; 3]),
    /// §8.7.4.5.5's parametric value per corner, as a fraction of the range `/Decode` gives
    /// it. The colour is [`ShadingKind::Mesh`]'s ramp at the interpolated value.
    Parameters([f32; 3]),
}

impl Triangle {
    /// Paints this triangle into a device-resolution buffer by §8.7.4.5.5's interpolation.
    ///
    /// The buffer's first pixel is device `(left, top)`. A pixel belongs to the triangle when
    /// its *centre* does — no antialiasing and no partial coverage — which is what makes two
    /// triangles sharing an edge tile exactly: every sample falls on one side or the other,
    /// and a sample exactly on the edge is claimed by both, the later one winning. Gaps are
    /// what a mesh cannot have; a one-pixel overlap between two nearly equal colours is
    /// invisible, which is the same property the old subdivision relied on.
    ///
    /// The interpolation itself is barycentric, which is the linear interpolation the clause
    /// asks for written in the coordinates that make it one expression per channel — and what
    /// it is applied to is [`Corners`]: a colour per channel where the mesh states colours, and
    /// the *parameter* where it states a `/Function`, whose ramp is then read at the
    /// interpolated value. Doing it the other way round makes a nonlinear function's interior
    /// a straight line between two of its values.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "indices and coordinates of device pixels, bounded by the caller's extent"
    )]
    #[expect(
        clippy::many_single_char_names,
        reason = "the clause's own notation for a triangle and its barycentric weights"
    )]
    fn paint(
        &self,
        data: &mut [u8],
        ramp: Option<&Ramp>,
        left: u32,
        top: u32,
        span: u32,
        rows: u32,
    ) {
        let [a, b, c] = self.points;
        // Twice the signed area. Zero means the three corners are collinear, so the triangle
        // covers nothing and has no interior to interpolate over.
        let area = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
        if area == 0.0 || !area.is_finite() {
            return;
        }

        let extent = |get: fn(&Point) -> f32| {
            let (p, q, r) = (get(&a), get(&b), get(&c));
            (p.min(q).min(r), p.max(q).max(r))
        };
        let (x0, x1) = extent(|point| point.x);
        let (y0, y1) = extent(|point| point.y);
        let first_x = ((x0 - 0.5).floor().max(0.0) as u32).saturating_sub(left);
        let first_y = ((y0 - 0.5).floor().max(0.0) as u32).saturating_sub(top);
        let last_x = (((x1 + 0.5).ceil().max(0.0) as u32).saturating_sub(left)).min(span);
        let last_y = (((y1 + 0.5).ceil().max(0.0) as u32).saturating_sub(top)).min(rows);

        for row in first_y..last_y {
            let y = top.saturating_add(row) as f32 + 0.5;
            for column in first_x..last_x {
                let x = left.saturating_add(column) as f32 + 0.5;
                // Barycentric coordinates, scaled by `area` so no division is needed to test
                // the sign — and divided once each only for a pixel that is inside.
                let w0 = (b.x - x) * (c.y - y) - (c.x - x) * (b.y - y);
                let w1 = (c.x - x) * (a.y - y) - (a.x - x) * (c.y - y);
                let w2 = (a.x - x) * (b.y - y) - (b.x - x) * (a.y - y);
                let inside = if area > 0.0 {
                    w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
                } else {
                    w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
                };
                if !inside {
                    continue;
                }
                let (u, v, w) = (w0 / area, w1 / area, w2 / area);
                let colour = match self.corners {
                    Corners::Colours([ca, cb, cc]) => {
                        let mix = |get: fn(&Color) -> f32| {
                            (u * get(&ca) + v * get(&cb) + w * get(&cc)).clamp(0.0, 1.0)
                        };
                        Color {
                            r: mix(|colour| colour.r),
                            g: mix(|colour| colour.g),
                            b: mix(|colour| colour.b),
                            a: mix(|colour| colour.a),
                        }
                    }
                    // §8.7.4.5.5: the interpolation is on the parameter, and the function —
                    // sampled into the ramp — is what turns the *interpolated* value into a
                    // colour. `build` has refused a parametric mesh with no ramp.
                    Corners::Parameters([ta, tb, tc]) => match ramp {
                        Some(ramp) => ramp.colour_at(u * ta + v * tb + w * tc),
                        None => continue,
                    },
                };
                let at = ((row as usize)
                    .saturating_mul(span as usize)
                    .saturating_add(column as usize))
                .saturating_mul(4);
                let Some(pixel) = data.get_mut(at..at.saturating_add(4)) else {
                    continue;
                };
                for (slot, value) in pixel
                    .iter_mut()
                    .zip([colour.r, colour.g, colour.b, colour.a])
                {
                    // Rounded rather than truncated, so a corner's own colour round-trips.
                    *slot = (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Corners, MeshRaster, Ramp, Triangle, blend_parameter};
    use crate::{Color, Point, Transform};

    /// A break makes a step, and a ramp without one averages across it.
    ///
    /// The function here is green below 0.5 and blue above it, which is what a type 3
    /// stitching function with two equal `/Bounds` states. Sampled evenly, the midpoint of the
    /// ramp is a blend of the two; sampled across the break, every position is one colour or
    /// the other and the two stops that share position 0.5 are what carry the jump.
    #[test]
    fn a_break_is_a_step_and_not_a_gradient() {
        let step = |t: f32| {
            if t < 0.5 {
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                }
            } else {
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                }
            }
        };

        let even = Ramp::sample(step);
        let across = Ramp::sample_across(&[0.5], step);

        // Just to one side of the break, an evenly sampled ramp is already part-way to the
        // other colour; a ramp with the break in it is not.
        let near = 0.5 - 0.001;
        assert!(
            even.colour_at(near).b > 0.05,
            "an even ramp bleeds blue below the step: {:?}",
            even.colour_at(near)
        );
        assert!(
            across.colour_at(near).b < 0.001,
            "a ramp sampled across the break does not: {:?}",
            across.colour_at(near)
        );
        assert!(across.colour_at(0.5 + 0.001).g < 0.001);

        // Two stops share the break's position, which is what a gradient needs to draw a step.
        let at_break = across
            .stops
            .iter()
            .filter(|stop| (stop.at - 0.5).abs() < 1e-6)
            .count();
        assert_eq!(at_break, 2, "the step is two stops at one position");
    }

    /// A ramp with no breaks spans the whole interval, and a straight one is two stops.
    ///
    /// The length assertion used to be `RESOLUTION`, which was a statement about how the ramp
    /// is *built* rather than about what it says. `simplify` drops every stop a rasteriser
    /// would have computed anyway, so a linear function — every `/FunctionType 2` with `/N 1`,
    /// which is most shadings in most documents — comes out as its two endpoints. That is the
    /// whole of ADR 0068, seen from the smallest possible case.
    #[test]
    fn an_unbroken_ramp_spans_the_whole_interval() {
        let ramp = Ramp::sample(|t| Color {
            r: t,
            g: t,
            b: t,
            a: 1.0,
        });
        assert_eq!(ramp.stops.len(), 2, "a straight line needs its two ends");
        assert!(
            ramp.stops
                .first()
                .is_some_and(|stop| stop.at.abs() < f32::EPSILON)
        );
        assert!(
            ramp.stops
                .last()
                .is_some_and(|stop| (stop.at - 1.0).abs() < f32::EPSILON)
        );
        assert!((ramp.colour_at(0.5).r - 0.5).abs() < 0.01);
    }

    /// A right triangle covering the top-left twenty pixels, with the given corners.
    fn triangle(corners: Corners) -> Triangle {
        Triangle {
            points: [
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
                Point::new(0.0, 20.0),
            ],
            corners,
        }
    }

    /// The colour of one device pixel of a rasterised mesh, as `(r, g, b, a)`.
    ///
    /// The raster's own origin is subtracted, so the coordinates are the target's.
    fn pixel(raster: &MeshRaster, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let column = x.saturating_sub(u32::try_from(raster.left).unwrap_or(0));
        let row = y.saturating_sub(u32::try_from(raster.top).unwrap_or(0));
        let at = (row
            .saturating_mul(raster.image.width)
            .saturating_add(column) as usize)
            .saturating_mul(4);
        let bytes = raster
            .image
            .data
            .get(at..at.saturating_add(4))
            .expect("in the raster");
        (bytes[0], bytes[1], bytes[2], bytes[3])
    }

    /// The pixel both mesh tests read, and the parameter the clause's interpolation gives it.
    ///
    /// Device pixel (10, 0) is sampled at its centre, (10.5, 0.5). In the triangle
    /// `(0,0), (20,0), (0,20)` that is barycentric `(0.45, 0.525, 0.025)`, so a corner set of
    /// `t = 0, 1, 0` interpolates to **0.525** there. Written down rather than assumed,
    /// because a test that takes its expected value from the code under test measures nothing.
    const PARAMETER: f32 = 0.525;

    /// A fraction of full scale as the eight-bit level [`Triangle::paint`] writes for it.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "test code: the argument is a fraction of full scale, so the rounded product \
                  is in 0..=255"
    )]
    fn level(fraction: f32) -> u8 {
        (fraction * 255.0).round() as u8
    }

    /// ISO 32000-2 §8.7.4.5.5's order: interpolate the parameter, *then* call the function.
    ///
    /// The ramp here is the square law `t²` in red, which is the simplest function that tells
    /// the two orders apart: it is the clause's answer squared where the other order's is
    /// linear. At [`PARAMETER`] the clause gives `0.525² = 0.2756` of full red — 70 — and
    /// evaluating the function at the corners first and interpolating the *colours* gives
    /// `0.525 × 1 = 0.525` — 134, which is what the next test measures on the same geometry.
    /// Both are plausible pictures and only one is the clause's.
    #[test]
    fn a_parametric_mesh_interpolates_the_parameter_and_not_the_colour() {
        let ramp = Ramp::sample(|t| Color::rgb(t * t, 0.0, 0.0));
        let mesh = [triangle(Corners::Parameters([0.0, 1.0, 0.0]))];
        let raster = MeshRaster::build(&mesh, Some(&ramp), Transform::IDENTITY, 32, 32)
            .expect("the mesh covers pixels");

        let (red, _, _, alpha) = pixel(&raster, 10, 0);
        assert_eq!(alpha, 255, "the mesh paints this pixel");
        let expected = level(PARAMETER * PARAMETER);
        assert!(
            red.abs_diff(expected) <= 1,
            "§8.7.4.5.5 puts the ramp at the interpolated parameter here, which is {expected}; \
             interpolating the corner colours would give 134. Got {red}"
        );
    }

    /// The same geometry with the function already applied at its corners is the picture the
    /// clause does *not* ask for, and this pins the difference rather than assuming it.
    #[test]
    fn interpolating_the_corner_colours_is_a_different_picture() {
        let mesh = [triangle(Corners::Colours([
            Color::rgb(0.0, 0.0, 0.0),
            Color::rgb(1.0, 0.0, 0.0),
            Color::rgb(0.0, 0.0, 0.0),
        ]))];
        let raster =
            MeshRaster::build(&mesh, None, Transform::IDENTITY, 32, 32).expect("it covers pixels");
        let (red, _, _, _) = pixel(&raster, 10, 0);
        let expected = level(PARAMETER);
        assert!(
            red.abs_diff(expected) <= 1,
            "a colour mesh interpolates the colours, which is {expected} of full red: {red}"
        );
    }

    /// A parametric mesh with no ramp is not a mesh whose colours can be known, and painting
    /// the parameters as though they were colours is the plausible answer trap 5 forbids.
    #[test]
    fn a_parametric_mesh_without_its_ramp_is_refused() {
        let mesh = [triangle(Corners::Parameters([0.0, 1.0, 0.0]))];
        assert!(MeshRaster::build(&mesh, None, Transform::IDENTITY, 32, 32).is_none());
    }

    /// §10.7.3's smoothness tolerance moves the sampling in one direction only.
    ///
    /// The clause states a *maximum* error and says "each output device may have internal
    /// limits", so a request coarser than this device's own is met by keeping ours — a
    /// tolerance is not an instruction to be less accurate. A finer one is honoured, which
    /// is what 5 corpus documents asking for 0.002 were not getting.
    #[test]
    fn a_smoothness_tolerance_only_ever_asks_for_more_samples() {
        assert_eq!(Ramp::resolution_for(None), Ramp::RESOLUTION);
        assert_eq!(
            Ramp::resolution_for(Some(0.02)),
            Ramp::RESOLUTION,
            "coarser than this device draws is met by drawing finer"
        );
        assert_eq!(
            Ramp::resolution_for(Some(0.002)),
            500,
            "finer is honoured, at the reciprocal the clause defines"
        );
        assert_eq!(
            Ramp::resolution_for(Some(1e-9)),
            Ramp::MAX_RESOLUTION,
            "and bounded, because the clause permits an internal limit"
        );
        for absurd in [Some(0.0), Some(-1.0), Some(6.0), Some(f32::NAN)] {
            assert_eq!(
                Ramp::resolution_for(absurd),
                Ramp::RESOLUTION,
                "Table 57 makes it a fraction; {absurd:?} is not one"
            );
        }
    }

    /// A finer tolerance reaches the stops only where the function has detail to find.
    ///
    /// This is the interaction between §10.7.3 and ADR 0068, and it is worth pinning because
    /// it is not obvious: `simplify` drops every stop that lies within half an eight-bit
    /// level of the line its neighbours draw, so sampling a *smooth* function a thousand
    /// times yields the same stops as sampling it 256 times — the extra samples land on the
    /// curve the survivors already describe. What a finer tolerance buys is a feature
    /// narrower than the default sampling, which is exactly what it is for: a type 0
    /// function with thousands of samples, or a stitching function with narrow sub-domains.
    #[test]
    fn a_finer_tolerance_reaches_the_stops_only_where_there_is_detail_to_find() {
        // A ramp that changes direction 400 times: features far narrower than 1/256.
        let detailed = |t: f32| Color::rgb((t * 400.0).sin().mul_add(0.5, 0.5), 0.0, 0.0);
        let coarse = Ramp::sample_across_at(Ramp::RESOLUTION, &[], detailed);
        let fine = Ramp::sample_across_at(2000, &[], detailed);
        assert!(
            fine.stops.len() > coarse.stops.len(),
            "detail below the default sampling is what a finer tolerance recovers: {} \
             against {}",
            fine.stops.len(),
            coarse.stops.len()
        );

        let smooth = |t: f32| Color::rgb(t * t * t, 0.0, 0.0);
        assert_eq!(
            Ramp::sample_across_at(2000, &[], smooth).stops.len(),
            Ramp::sample_across_at(Ramp::RESOLUTION, &[], smooth)
                .stops
                .len(),
            "and a smooth curve is already described to within half a level"
        );
    }
    /// §8.7.4.5.4's worked case, at the point the clause's own rule decides.
    ///
    /// `radial_gradients.pdf`'s cell states `/Coords [511 489 25 431 489 60]` and
    /// `/Extend [true false]`, so `c(s) = (511 − 80s, 489)` and `r(s) = 25 + 35s`: the
    /// centres are 80 apart and the radii differ by 35, which is NOTE 3's cone rather than a
    /// sphere. Take **P = (431, 489)**, the centre of the ending circle. `|P − c(s)| = 80|s − 1|`
    /// and `r(s) = 25 + 35s`, so the blend circles through P are at s = 0.478 and s = 2.333.
    ///
    /// The greater root is out of range and `/Extend[1]` is false, so it is not a circle at
    /// all — and the clause's "greatest value of s" is then the other one. A two-point conical
    /// gradient has no way to say that, which is the whole of this defect: for two hundred
    /// sessions the point was painted with nothing.
    #[test]
    fn the_greatest_root_a_cone_admits_is_not_always_the_greater_root() {
        let (start, end) = (Point { x: 511.0, y: 489.0 }, Point { x: 431.0, y: 489.0 });
        let s = blend_parameter(end, start, 25.0, end, 60.0, (true, false))
            .expect("the lower root is admissible and paints the point");
        assert!(
            (s - 0.478_260_9).abs() < 1e-4,
            "the clause's arithmetic gives 0.478, not {s}"
        );

        // And with the upper end extended, the greater root wins — same geometry, same point.
        let extended = blend_parameter(end, start, 25.0, end, 60.0, (true, true))
            .expect("both roots are admissible now");
        assert!(
            (extended - 2.333_333).abs() < 1e-4,
            "the greater root is 2.333, not {extended}"
        );
    }

    /// A point no blend circle passes through is unpainted, which is not the end colour.
    ///
    /// Two concentric circles of radius 10 and 20 with neither end extended paint the annulus
    /// and nothing else: the centre lies on no circle of the family, and §8.7.4.5.4 gives it
    /// no colour rather than the starting one.
    #[test]
    fn a_point_outside_every_admissible_circle_is_left_unpainted() {
        let centre = Point { x: 0.0, y: 0.0 };
        assert_eq!(
            blend_parameter(centre, centre, 10.0, centre, 20.0, (false, false)),
            None,
            "the centre is inside the starting circle and on none of them"
        );
        assert_eq!(
            blend_parameter(
                Point { x: 30.0, y: 0.0 },
                centre,
                10.0,
                centre,
                20.0,
                (false, false)
            ),
            None,
            "and so is a point beyond the ending circle"
        );
        let inside = blend_parameter(centre, centre, 10.0, centre, 20.0, (true, false))
            .expect("extending the smaller end reaches r(s) = 0 at s = −1");
        assert!(
            (inside - (-1.0)).abs() < 1e-5,
            "r(s) = 10 + 10s is zero at s = −1, not at {inside}"
        );
    }

    /// The clause's own proviso: both radii zero means the upper end does not extend.
    ///
    /// "[T]he shading shall be extended beyond the defined ending circle to s values greater
    /// than 1.0 **unless radii r0 and r1 in the Coords array are both zero**". A family of
    /// circles of radius zero has no interior at any s, so nothing is painted anywhere — which
    /// is what the proviso exists to prevent a processor from extending forever.
    #[test]
    fn a_family_of_zero_radius_circles_paints_nothing() {
        let start = Point { x: 0.0, y: 0.0 };
        let end = Point { x: 100.0, y: 0.0 };
        for point in [start, end, Point { x: 50.0, y: 0.0 }] {
            assert_eq!(
                blend_parameter(point, start, 0.0, end, 0.0, (true, true)),
                None,
                "no circle of the family has an interior"
            );
        }
    }

    /// A radius the family never reaches is not admissible, however the extension is set.
    ///
    /// NOTE 1: "[i]f the shading is extended at the smaller end, the family of blend circles
    /// continues as far as that value of s for which the radius of the blend circle r(s) = 0."
    /// So an extension downwards stops there rather than running on into negative radii, and a
    /// root beyond it is not a circle.
    #[test]
    fn a_root_with_a_negative_radius_is_not_a_circle() {
        // c(s) = (0, 0) for all s, r(s) = 20 + 20s: zero at s = −1, negative below.
        let centre = Point { x: 0.0, y: 0.0 };
        let far = Point { x: 200.0, y: 0.0 };
        // |P − c| = 200 needs r(s) = 200, so s = 9 — admissible only when the upper end does.
        assert_eq!(
            blend_parameter(far, centre, 20.0, centre, 40.0, (true, false)),
            None
        );
        let s = blend_parameter(far, centre, 20.0, centre, 40.0, (true, true))
            .expect("extended, the family reaches it");
        assert!(
            (s - 9.0).abs() < 1e-4,
            "r(s) = 20 + 20s is 200 at s = 9, not {s}"
        );
    }

    /// The degenerate case where the quadratic is a line, and it is geometry rather than noise.
    ///
    /// When the centres are exactly `|r1 − r0|` apart the two circles are internally tangent,
    /// the `s²` coefficient vanishes and one root has gone to infinity. The remaining root is
    /// the answer and must not be discarded: this is the shape of a cone whose apex is on the
    /// axis, and `radial_gradients.pdf` has cells of it.
    #[test]
    fn internally_tangent_circles_leave_one_root_and_it_is_used() {
        let start = Point { x: 0.0, y: 0.0 };
        let end = Point { x: 10.0, y: 0.0 };
        // |c1 − c0| = 10 and r1 − r0 = 10, so a = 0 exactly.
        let s = blend_parameter(
            Point { x: 10.0, y: 0.0 },
            start,
            10.0,
            end,
            20.0,
            (true, true),
        )
        .expect("the linear root is the answer");
        // |P − c(s)| = 10|s − 1| = r(s) = 10 + 10s gives s = 0.
        assert!(s.abs() < 1e-5, "the one root is 0, not {s}");
    }

    /// Ramp positions come from the parameter clamped, because extension paints an end colour.
    ///
    /// "Blend circles extending beyond the starting circle shall be painted in the same colour
    /// defined by the shading dictionary's Function entry for the starting circle (t = t0,
    /// s = 0.0)." So a negative parameter is a real circle painted with the first colour, not a
    /// circle painted with an extrapolated one — which is what the clamp in
    /// [`super::RadialRaster::build`] states and what this checks the ramp agrees with.
    #[test]
    fn an_extended_circle_carries_the_end_colour_rather_than_an_extrapolated_one() {
        let ramp = Ramp::sample(|t| Color::rgb(t, 0.0, 0.0));
        assert_eq!(
            ramp.colour_at((-3.0f32).clamp(0.0, 1.0)),
            ramp.colour_at(0.0)
        );
        assert_eq!(ramp.colour_at(7.0f32.clamp(0.0, 1.0)), ramp.colour_at(1.0));
    }
}
