//! The line lane: one straight segment's exact area, computed in the shader (ADR 0097).
//!
//! A [`Command::Line`] is a stroke with nothing general left in
//! it: one open segment, so no join (ISO 32000-2 §8.4.3.4 applies where "consecutive segments
//! of a path connect"), no dash, a solid colour and `Normal`. §8.4.3.2 makes its set the points
//! within half the width of the segment, and §8.4.3.3's Table 53 adds the ends. Under a
//! transform that keeps the axes the disc of half the width maps to an ellipse whose axes are
//! the device's, so dividing x and y by its two semi-axes makes the pen the unit disc and the
//! set an ordinary one again: a band of half-width one around the segment, squared off at
//! the endpoints, carried half a width past them, or finished by half discs — in any
//! direction. A pixel stays an axis-aligned box under that division.
//!
//! **That shape has a closed-form area in any pixel**, which is what this lane is: the band is
//! a convex quadrilateral whose part in the box is a clipped polygon's shoelace area, and a half
//! disc's part is the unit disc inside the box cut by the endpoint's square, summed edge by
//! edge as a triangle or a circular sector. `line.wgsl` evaluates it per fragment; the host
//! computes the pen and writes one 60-byte instance. Nothing is uploaded, flattened, expanded
//! or rasterised for the mark, and no outline is resident for it, which is the whole of what
//! the page of millions of such marks needed (ADR 1817, ADR 1823).
//!
//! **The instance is the page's and the view is the shader's** (ADR 2022): the endpoints, the pen
//! and the clip's rectangle are written after the line's own transform and before the view, and
//! `line.wgsl`'s vertex stage maps them, resolves the width for its view and collapses a line the
//! view admits nothing of. A walk at a view no encode of the scene came before culls a line its
//! view admits nothing of, as the walk always has, so a page's first frame in a window sends what
//! the window shows; a walk that culled none wrote the page's whole stream, and the encode of the
//! next view takes it in runs and sends none of it again (`kept_lines`, ADR 2056).
//!
//! **A line of no length under a round cap is §8.5.3.2's dot**, "a filled circle centred at the
//! single point", and this set at length zero: the band vanishes and the two half discs about
//! one point are the disc. The closed form needs no direction for it — the shader takes any unit
//! vector where the endpoints coincide — so the dot is one instance like any line (viewer ADR
//! 1857). Under the other two caps a line of no length is refused by the scene.
//!
//! **Everything else about a line takes the general road, exactly**: a view or a transform that
//! does not keep the axes, a clip with a non-rectangular link, or a segment its own transform
//! collapses to a point. There the segment is a two-point outline expanded and filled as the
//! stroke arm would expand and fill it, so the line command draws what a stroke of the same
//! path draws wherever the closed form does not apply — except the dot, which the stroke arm
//! expands to nothing and which is drawn there as the stroker's own two round caps instead
//! ([`raster::dot_pieces`]).

use raster_scene::{
    Affine, ClipId, Color, Command, LineCap, LineJoin, MaskId, Point, Rect, Segment,
};

use super::Encoder;
use super::device_space::{apply, compose, transform_preserves_axes, unviewed};
use super::expansion::{self, Expansion};
use super::instance::{LINE_HEAD_BYTES, LineInstance, line_head};
use crate::error::RenderError;
use crate::raster::{self, DeviceTransform, Rule};

/// One line command's fields, as [`Encoder::encode_line`] takes them.
#[derive(Debug, Clone, Copy)]
pub(super) struct LineMark {
    pub from: Point,
    pub to: Point,
    pub transform: Affine,
    pub width: f32,
    pub adjust: bool,
    pub cap: LineCap,
    pub color: Color,
    pub clip: Option<ClipId>,
    pub mask: Option<MaskId>,
}

impl LineMark {
    /// A line command's fields, or `None` for every other command.
    pub(super) fn of(command: &Command) -> Option<Self> {
        match *command {
            Command::Line {
                from,
                to,
                transform,
                width,
                adjust,
                cap,
                color,
                clip,
                mask,
            } => Some(Self {
                from,
                to,
                transform,
                width,
                adjust,
                cap,
                color,
                clip,
                mask,
            }),
            _ => None,
        }
    }
}

/// What the closed form makes of a line before its clip is known: the instance's head
/// ([`line_head`]), which is the page's, and the box the line's ink lies in on the device at
/// this view.
///
/// **A pure function of the command and the view** (ADR 1878 §3): nothing it reads is decided
/// by the frame's order, nor by the clip chain, whose resolution is the walk's. That is what lets
/// the walk have it made on other threads before it reaches the line (`lines_ahead`, ADR 1884),
/// and it is the one construction both the walk and those threads ask, so the two cannot come to
/// differ. **The head reads no view at all** (ADR 2022): it is the line in the page's space, and
/// `line.wgsl` maps it by the view uniform, so the stream a zoom step writes is the stream the
/// view before it wrote.
#[derive(Debug, Clone, Copy)]
pub(super) struct Unclipped {
    head: [u8; LINE_HEAD_BYTES],
    ink: Rect,
}

impl Unclipped {
    /// The closed form of `line` under the view `view`, or `None` where the line takes the
    /// stroke road: a view or a transform that does not keep the axes, or no pen
    /// ([`page_pen`]).
    pub(super) fn of(line: &LineMark, view: &DeviceTransform) -> Option<Self> {
        if !transform_preserves_axes(view) {
            return None;
        }
        let page = page_pen(line)?;
        Some(Self {
            head: line_head(
                (page.from, page.to),
                (page.half, page.cap, page.resolution.code()),
                line.color,
            ),
            ink: page.on_device(view).ink(),
        })
    }

    /// The instance under a chain of rectangles alone, whose rectangle is `page_admit` in the
    /// page's space and, met with the target's, `admit` on the device — or `None` where no pixel
    /// of the ink is admitted, which is a line the walk culls and a kept run's vertex stage
    /// collapses (ADR 2056).
    pub(super) fn under(&self, page_admit: Rect, admit: Rect) -> Option<LineInstance> {
        let ink = self.ink.intersection(admit);
        (!ink.is_empty()).then(|| LineInstance::under(&self.head, page_admit, ink))
    }

    /// The instance under the same chain whatever the view admits of it, its box empty where the
    /// view admits nothing: the idle build's, which culls no line (viewer ADR 2134).
    pub(super) fn whole_under(&self, page_admit: Rect, admit: Rect) -> LineInstance {
        LineInstance::under(&self.head, page_admit, self.ink.intersection(admit))
    }
}

/// The box a closed-form line's ink lies in on the device under the view `view`, or `None` where
/// the line takes the stroke road at every view ([`page_pen`]) — the box [`Unclipped::of`] carries,
/// by the same arithmetic, for a walk that keeps the line's instance from the encode before
/// (`kept_lines`, ADR 2056).
pub(super) fn device_ink(line: &LineMark, view: &DeviceTransform) -> Option<Rect> {
    Some(page_pen(line)?.on_device(view).ink())
}

/// How the vertex stage resolves a line's width at the view it draws in: ADR 0085's rule,
/// §8.4.3.2's zero and §10.7.5's adjustment, asked of the device's stretch per view rather than
/// decided on the host (ADR 2022). The number is `line.wgsl`'s, packed beside Table 53's end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Resolution {
    /// The stated width under the view's stretch.
    Scaled,
    /// The stated width under the view's stretch, or one device pixel where §10.7.5's
    /// adjustment finds it below half a pixel.
    Adjusted,
    /// A device pen no view changes: §8.4.3.2's zero width, which is one device pixel, or the
    /// width the resolution substitutes under a transform that stretches nothing.
    Fixed,
}

impl Resolution {
    /// The number `line.wgsl` reads the resolution by: 0, 1 or 2.
    fn code(self) -> u32 {
        match self {
            Self::Scaled => 0,
            Self::Adjusted => 1,
            Self::Fixed => 2,
        }
    }
}

/// A line in the page's space, as its instance states it (ADR 2022): the two endpoints after
/// the line's own transform, the pen's semi-axes there — half the width times each page axis's
/// stretch, or the device pen [`Resolution::Fixed`] names — the end, and how the view resolves
/// the width.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PagePen {
    from: Point,
    to: Point,
    half: (f32, f32),
    cap: LineCap,
    resolution: Resolution,
}

impl PagePen {
    /// The device shape `line.wgsl`'s vertex stage makes of this pen under a view that keeps
    /// the axes, by the same steps: the endpoints through the view; the semi-axes each device
    /// axis receives — under a quarter turn the page's y stretch is the device's x — and one
    /// device pixel where the adjustment finds the device width below half a pixel. The host
    /// asks it for the box the plan is grown by; the pixels are the shader's, which WGSL
    /// section 15.7.5 lets a driver fuse, so the two may differ by an ulp (ADR 2022).
    fn on_device(self, view: &DeviceTransform) -> Pen {
        let (mut from, mut to) = (apply(view, self.from), apply(view, self.to));
        let (hx, hy) = self.half;
        // The pen the stated width makes under the view, before the half-pixel promotion: the
        // fit asks it whether the transform stretches both axes alike, as `raster::grid_fit`
        // asks the transform.
        let stated = (
            hx * view.a.abs() + hy * view.c.abs(),
            hx * view.b.abs() + hy * view.d.abs(),
        );
        let half = match self.resolution {
            Resolution::Fixed => self.half,
            Resolution::Scaled | Resolution::Adjusted => {
                let thin = 2.0 * stated.0.max(stated.1) < 0.5;
                if self.resolution == Resolution::Adjusted && thin {
                    (0.5, 0.5)
                } else {
                    stated
                }
            }
        };
        let half = if self.resolution == Resolution::Adjusted
            && let Some(fitted) = fit_on_device(stated, half, (&mut from, &mut to))
        {
            fitted
        } else {
            half
        };
        Pen {
            from,
            to,
            half,
            cap: self.cap,
        }
    }
}

/// §10.7.5's fit of a line on the device, as `raster::grid_fit` makes it of a stroke's outline
/// and `line.wgsl`'s `device_line` makes it per vertex: where the `stated` pen is round, the
/// resolved pen `half` is under [`raster::FIT_BELOW`] device pixels across, and the line lies
/// along a device axis, its ends move across it to the nearest pixel centre and the pen becomes
/// one device pixel — the new pen, with the ends moved in place, or `None` and nothing moved.
///
/// The stated pen is half the stated width times each device axis's stretch, so it is round
/// exactly where the transform stretches both axes alike, and [`raster::ALIKE`] is asked of it
/// rather than of the resolved pen, which the half-pixel promotion makes round under any
/// transform; a line of no length is the dot, whose ends the fit leaves where they are.
fn fit_on_device(
    stated: (f32, f32),
    half: (f32, f32),
    (from, to): (&mut Point, &mut Point),
) -> Option<(f32, f32)> {
    let round = stated.0.max(stated.1);
    if (stated.0 - stated.1).abs() > raster::ALIKE * round
        || 2.0 * half.0.max(half.1) >= raster::FIT_BELOW
    {
        return None;
    }
    let horizontal = (to.y - from.y).abs() <= raster::ALONG_AN_AXIS;
    let vertical = (to.x - from.x).abs() <= raster::ALONG_AN_AXIS;
    let centre = |v: f32| v.floor() + 0.5;
    match (horizontal, vertical) {
        (true, true) => {}
        (true, false) => (from.y, to.y) = (centre(from.y), centre(to.y)),
        (false, true) => (from.x, to.x) = (centre(from.x), centre(to.x)),
        (false, false) => return None,
    }
    Some((0.5, 0.5))
}

/// The line in the page's space where the closed form applies at every view that keeps the
/// axes: a transform of its own that keeps them, two distinct endpoints — or one, under a round
/// cap, which is the dot — and a pen of positive, finite size.
///
/// **Which road a line takes is the page's, not the view's** (ADR 2022), so that the line
/// stream is a function of the page: [`pen`]'s device rule asked of the page. Where a view
/// stretches nothing ADR 0085's resolution substitutes `max(width, 1)` device pixels, and
/// §8.4.3.2's zero is one device pixel, at every view — both [`Resolution::Fixed`]. Everything
/// else is half the width times each page axis's stretch, which the view scales; a page axis
/// the transform collapses has no pen at any view and takes the stroke road, whatever the
/// adjustment would substitute at one.
fn page_pen(line: &LineMark) -> Option<PagePen> {
    let page = unviewed(line.transform);
    if !transform_preserves_axes(&page) {
        return None;
    }
    let (from, to) = (apply(&page, line.from), apply(&page, line.to));
    if (from == to && line.cap != LineCap::Round) || !(from.is_finite() && to.is_finite()) {
        return None;
    }
    let usable = |v: f32| v > 0.0 && v.is_finite();
    let stretch = page.max_stretch();
    let (half, resolution) = if !stretch.is_finite() || stretch <= 0.0 {
        let side = line.width.max(1.0) / 2.0;
        ((side, side), Resolution::Fixed)
    } else if line.width <= 0.0 {
        ((0.5, 0.5), Resolution::Fixed)
    } else {
        let half = line.width / 2.0;
        let half = (
            half * (page.a.abs() + page.c.abs()),
            half * (page.b.abs() + page.d.abs()),
        );
        let resolution = if line.adjust {
            Resolution::Adjusted
        } else {
            Resolution::Scaled
        };
        (half, resolution)
    };
    (usable(half.0) && usable(half.1)).then_some(PagePen {
        from,
        to,
        half,
        cap: line.cap,
        resolution,
    })
}

/// The device shape of a line under a transform that keeps the axes: the two endpoints and
/// the pen, which is the disc of half the width mapped to the device — an ellipse whose
/// semi-axes lie along the device axes.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pen {
    from: Point,
    to: Point,
    /// The pen's semi-axes along device x and y.
    half: (f32, f32),
    cap: LineCap,
}

impl Pen {
    /// The box the ink lies in: the segment's box grown by what each end adds. A round end
    /// and a butt end reach no further than the pen from an endpoint; a projecting end's
    /// corners lie at `±d ± n` from it in the frame where the pen is the unit disc, so it is
    /// grown by `|d| + |n|` along each axis, which is at most √2 of the pen (Table 53).
    fn ink(self) -> Rect {
        let (hx, hy) = self.half;
        let (gx, gy) = match self.cap {
            LineCap::Butt | LineCap::Round => (hx, hy),
            LineCap::Square => {
                // The direction in the unit-pen frame, and its normal: |d.x| + |n.x| is
                // |d.x| + |d.y|, and likewise for y.
                let (ux, uy) = (
                    (self.to.x - self.from.x) / hx,
                    (self.to.y - self.from.y) / hy,
                );
                let length = ux.hypot(uy);
                let reach = (ux.abs() + uy.abs()) / length;
                (hx * reach, hy * reach)
            }
        };
        Rect::new(
            Point::new(
                self.from.x.min(self.to.x) - gx,
                self.from.y.min(self.to.y) - gy,
            ),
            Point::new(
                self.from.x.max(self.to.x) + gx,
                self.from.y.max(self.to.y) + gy,
            ),
        )
    }
}

/// The line's device shape where the closed form applies: two distinct device endpoints — or
/// one, under a round cap, which is the dot — and a pen of positive, finite size under a
/// transform that keeps the axes.
///
/// A transform that keeps the axes has one of `a`, `c` and one of `b`, `d` zero, so
/// `|a| + |c|` and `|b| + |d|` are the stretches the two device axes receive, and the pen's
/// semi-axes are half the width times each. The width is resolved first by the rule every
/// stroke here is resolved by (ADR 0085): §8.4.3.2's zero, and §10.7.5's adjustment below half
/// a pixel, are a line one device pixel wide, whose pen is a disc of half a pixel.
fn pen(line: &LineMark, to_device: &DeviceTransform) -> Option<Pen> {
    let (from, to) = (apply(to_device, line.from), apply(to_device, line.to));
    if (from == to && line.cap != LineCap::Round) || !(from.is_finite() && to.is_finite()) {
        return None;
    }
    let stroke = raster_scene::Stroke {
        width: line.width,
        adjust: line.adjust,
        cap: line.cap,
        join: LineJoin::Miter,
        miter_limit: 1.0,
    };
    let resolved = raster::resolve_width(stroke, *to_device);
    // The resolution substituted one device pixel exactly where it did not return the
    // stated width's own stretch.
    #[expect(clippy::float_cmp)]
    let substituted = resolved != line.width * to_device.max_stretch();
    let half = if substituted {
        (resolved / 2.0, resolved / 2.0)
    } else {
        let half = line.width / 2.0;
        (
            half * (to_device.a.abs() + to_device.c.abs()),
            half * (to_device.b.abs() + to_device.d.abs()),
        )
    };
    let usable = |v: f32| v > 0.0 && v.is_finite();
    (usable(half.0) && usable(half.1)).then_some(Pen {
        from,
        to,
        half,
        cap: line.cap,
    })
}

impl Encoder<'_> {
    /// The line arm: the closed-form lane under a transform that keeps the axes inside a
    /// rectangular clip, the stroke arm's own expansion otherwise.
    pub(super) fn encode_line(&mut self, line: &LineMark) -> Result<(), RenderError> {
        // Per viewport, like a stroke: which road a line takes is decided by where the
        // transform puts it, so the replay re-dispatches the command (`replay.rs`).
        self.record_slow();
        // Taken before the mask: a soft mask's first use walks the mask's own commands, which
        // moves the walk's current index (`layer.rs`).
        let index = self.current_command;
        let mask_index = self.use_mask(line.mask)?;
        let resolved = self.resolve_clip(line.clip)?;
        if resolved.residues.is_none()
            && let Some(unclipped) = Unclipped::of(line, &unviewed(self.viewport.transform))
        {
            if self.place_line(
                &unclipped,
                (resolved.page_rect, resolved.rect.intersection(self.visible)),
                mask_index,
            )? {
                self.keep_line(index, (line.clip, line.mask), resolved.page_rect);
            }
            return Ok(());
        }
        let to_device = compose(line.transform, self.viewport);
        self.encode_line_as_stroke(line, &to_device, &resolved, mask_index)
    }

    /// Place a line's closed form under its chain's rectangle, `page_admit` in the page's space
    /// and `admit` met with the target's on the device: counted as culled where no pixel of its
    /// ink is admitted, an instance otherwise — and answers which.
    ///
    /// **A culled line is a stream that is this view's**, so the encode keeps no runs for the next
    /// view, which walks the page again (`kept_lines`, ADR 2056). That is the price of a page's
    /// first frame in a window sending what the window shows rather than the page's whole stream:
    /// on the owner's witness 26 MB against 264, which the frame would otherwise wait for. The idle
    /// build culls nothing (`kept_lines`'s `Before::Whole`, viewer ADR 2134): it is the walk nobody
    /// waits for, and the stream it writes is the one every later view keeps.
    pub(super) fn place_line(
        &mut self,
        unclipped: &Unclipped,
        (page_admit, admit): (Rect, Rect),
        mask_index: Option<u32>,
    ) -> Result<bool, RenderError> {
        let instance = if self.before.culls() {
            let Some(instance) = unclipped.under(page_admit, admit) else {
                self.note_culled();
                self.kept_build.whole = false;
                return Ok(false);
            };
            instance
        } else {
            unclipped.whole_under(page_admit, admit)
        };
        self.push_line_instance(instance, mask_index)?;
        Ok(true)
    }

    /// A line the closed form does not reach, drawn as the stroke of a two-point outline.
    ///
    /// The same flattening, expansion and fill the stroke arm takes when nothing is queued,
    /// so a line drawn here is the stroke of the same path to the bit; the miter limit is
    /// immaterial, since one segment has no join.
    pub(super) fn encode_line_as_stroke(
        &mut self,
        line: &LineMark,
        to_device: &DeviceTransform,
        resolved: &super::ResolvedClip,
        mask_index: Option<u32>,
    ) -> Result<(), RenderError> {
        let stroke = raster_scene::Stroke {
            width: line.width,
            adjust: line.adjust,
            cap: line.cap,
            join: LineJoin::Miter,
            miter_limit: 1.0,
        };
        let segments = [Segment::MoveTo(line.from), Segment::LineTo(line.to)];
        // §10.7.5's fit at this placement, as the stroke arm takes it (`raster::grid_fit`): one
        // device pixel wide through pixel centres, an end moved at most half a pixel across.
        let fitted = raster::grid_fit(&segments, stroke, *to_device);
        let (device_width, moved) = if fitted.is_some() {
            (1.0, 0.5)
        } else {
            (raster::resolve_width(stroke, *to_device), 0.0)
        };
        // A projecting cap's corner is half the width away along the segment and across it at
        // once, so the reach that bounds every end is the half-width's diagonal.
        let reach = device_width * 0.5 * std::f32::consts::SQRT_2 + moved;
        let (d0, d1) = (apply(to_device, line.from), apply(to_device, line.to));
        // §8.5.3.2's dot, which the expansion below would drop as a subpath with no length: the
        // pen where the transform keeps the axes, the resolved width's circle where it does not,
        // which is the width the expansion strokes everything else at.
        // A fitted dot is one device pixel across: the fit moves no point of a segment with no
        // direction, and its width is the fit's.
        let dot = (d0 == d1 && line.cap == LineCap::Round).then(|| match &fitted {
            Some(_) => (0.5, 0.5),
            None => pen(line, to_device)
                .map_or((device_width * 0.5, device_width * 0.5), |pen| pen.half),
        });
        let bounds = (
            d0.x.min(d1.x) - reach,
            d0.y.min(d1.y) - reach,
            d0.x.max(d1.x) + reach,
            d0.y.max(d1.y) + reach,
        );
        if self.culled(bounds, resolved) {
            return Ok(());
        }
        self.segments = self.segments.saturating_add(segments.len() as u64);
        let span = self.clock.start();
        let stroked = match (dot, fitted) {
            (Some(half), _) => raster::dot_pieces(d0, half),
            (None, Some(fitted)) => raster::stroke_pieces(&fitted, stroke, device_width).pieces,
            (None, None) => {
                Expansion::placed(
                    expansion::expansion(None, &segments, *to_device, stroke),
                    *to_device,
                )
                .0
            }
        };
        self.clock.geometry(span);
        self.push_coverage(
            &stroked,
            Rule::NonZero,
            line.color,
            resolved,
            Some(device_width),
            mask_index,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{LineMark, Pen, pen};
    use crate::raster::DeviceTransform;
    use raster_scene::{Affine, Color, LineCap, Point, Rect};

    fn mark(from: (f32, f32), to: (f32, f32), width: f32, cap: LineCap) -> LineMark {
        LineMark {
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            transform: Affine::IDENTITY,
            width,
            adjust: false,
            cap,
            color: Color::new(0.0, 0.0, 0.0, 1.0),
            clip: None,
            mask: None,
        }
    }

    fn scale(sx: f32, sy: f32) -> DeviceTransform {
        DeviceTransform {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 10.0,
            f: 20.0,
        }
    }

    /// Under unequal scales the pen is the ellipse: half the width times each axis's stretch.
    #[test]
    fn the_pen_is_the_width_under_each_axis_stretch() {
        let found = pen(
            &mark((1.0, 1.0), (1.0, 3.0), 0.5, LineCap::Round),
            &scale(4.0, 1.0),
        );
        assert_eq!(
            found,
            Some(Pen {
                from: Point::new(14.0, 21.0),
                to: Point::new(14.0, 23.0),
                half: (1.0, 0.25),
                cap: LineCap::Round,
            })
        );
    }

    /// A round or butt end reaches no further than the pen; a projecting end on a diagonal
    /// reaches √2 of it, which is where its corner lies.
    #[test]
    fn the_ink_box_holds_every_end() {
        let at = scale(1.0, 1.0);
        let diagonal = |cap| {
            pen(&mark((0.0, 0.0), (1.0, 1.0), 2.0, cap), &at)
                .unwrap()
                .ink()
        };
        let rect = |x0, y0, x1, y1| Rect::new(Point::new(x0, y0), Point::new(x1, y1));
        assert_eq!(diagonal(LineCap::Round), rect(9.0, 19.0, 12.0, 22.0));
        assert_eq!(diagonal(LineCap::Butt), rect(9.0, 19.0, 12.0, 22.0));
        let square = diagonal(LineCap::Square);
        let reach = std::f32::consts::SQRT_2;
        assert!((square.min.x - (10.0 - reach)).abs() < 1e-5, "{square:?}");
        assert!((square.max.y - (21.0 + reach)).abs() < 1e-5, "{square:?}");
    }

    /// §8.4.3.2's zero width is the thinnest line, one device pixel wide: a pen of half a
    /// pixel each way, whatever the scales.
    #[test]
    fn a_zero_width_is_one_device_pixel() {
        let thin = pen(
            &mark((1.0, 1.0), (3.0, 1.0), 0.0, LineCap::Butt),
            &scale(3.0, 7.0),
        );
        assert_eq!(thin.map(|p| p.half), Some((0.5, 0.5)));
    }

    /// Two endpoints the transform puts on one device point have no direction, and take the
    /// stroke road under a butt or projecting cap.
    #[test]
    fn a_collapsed_segment_has_no_pen() {
        for cap in [LineCap::Butt, LineCap::Square] {
            let collapsed = pen(&mark((1.0, 1.0), (1.0, 3.0), 0.5, cap), &scale(1.0, 0.0));
            assert_eq!(collapsed, None, "{cap:?}");
        }
    }

    /// §8.5.3.2's dot keeps its pen: the half discs about one point need no direction, and its
    /// ink is the pen around the point.
    #[test]
    fn a_round_dot_has_the_pen_of_its_width() {
        let dot = pen(
            &mark((1.0, 1.0), (1.0, 1.0), 0.5, LineCap::Round),
            &scale(4.0, 1.0),
        )
        .expect("a round dot is the closed form's");
        assert_eq!(dot.half, (1.0, 0.25));
        let rect = |x0, y0, x1, y1| Rect::new(Point::new(x0, y0), Point::new(x1, y1));
        assert_eq!(dot.ink(), rect(13.0, 20.75, 15.0, 21.25));
    }
}
