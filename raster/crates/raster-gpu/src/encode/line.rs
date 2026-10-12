//! The line lane: one straight segment's exact area, computed in the shader (ADR 0097).
//!
//! A [`Command::Line`](raster_scene::Command::Line) is a stroke with nothing general left in
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
//! **A line of no length under a round cap is §8.5.3.2's dot**, "a filled circle centred at the
//! single point", and this set at length zero: the band vanishes and the two half discs about
//! one point are the disc. The closed form needs no direction for it — the shader takes any unit
//! vector where the endpoints coincide — so the dot is one instance like any line (viewer ADR
//! 1857). Under the other two caps a line of no length is refused by the scene.
//!
//! **Everything else about a line takes the general road, exactly**: a transform that does
//! not keep the axes, a clip with a non-rectangular link, or a segment the device transform
//! collapses to a point. There the segment is a two-point outline expanded and filled as the
//! stroke arm would expand and fill it, so the line command draws what a stroke of the same
//! path draws wherever the closed form does not apply — except the dot, which the stroke arm
//! expands to nothing and which is drawn there as the stroker's own two round caps instead
//! ([`raster::dot_pieces`]).

use raster_scene::{
    Affine, ClipId, Color, Command, LineCap, LineJoin, MaskId, Point, Rect, Segment,
};

use super::Encoder;
use super::device_space::{apply, compose, transform_preserves_axes};
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

/// What the closed form makes of a line at one view before its clip is known: the instance's
/// head ([`line_head`]) and the box the line's ink lies in on the device.
///
/// **A pure function of the command and the view** (ADR 1878 §3): nothing it reads is decided
/// by the frame's order, nor by the clip chain, whose resolution is the walk's. That is what lets
/// the walk have it made on other threads before it reaches the line (`lines_ahead`, ADR 1884),
/// and it is the one construction both the walk and those threads ask, so the two cannot come to
/// differ.
#[derive(Debug, Clone, Copy)]
pub(super) struct Unclipped {
    head: [u8; LINE_HEAD_BYTES],
    ink: Rect,
}

impl Unclipped {
    /// The closed form of `line` under `to_device`, or `None` where the line takes the stroke
    /// road: a transform that does not keep the axes, or no pen ([`pen`]).
    pub(super) fn of(line: &LineMark, to_device: &DeviceTransform) -> Option<Self> {
        if !transform_preserves_axes(to_device) {
            return None;
        }
        let pen = pen(line, to_device)?;
        Some(Self {
            head: line_head((pen.from, pen.to), (pen.half, pen.cap), line.color),
            ink: pen.ink(),
        })
    }

    /// The instance under a chain of rectangles alone whose rectangle, met with the target's,
    /// is `admit` — or `None` where no pixel of the ink is admitted, which is a culled line.
    pub(super) fn under(&self, admit: Rect) -> Option<LineInstance> {
        let ink = self.ink.intersection(admit);
        (!ink.is_empty()).then(|| LineInstance::under(&self.head, admit, ink))
    }
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
        let mask_index = self.use_mask(line.mask)?;
        let resolved = self.resolve_clip(line.clip)?;
        let to_device = compose(line.transform, self.viewport);
        if resolved.residues.is_none()
            && let Some(unclipped) = Unclipped::of(line, &to_device)
        {
            return self.place_line(
                &unclipped,
                resolved.rect.intersection(self.visible),
                mask_index,
            );
        }
        self.encode_line_as_stroke(line, &to_device, &resolved, mask_index)
    }

    /// Place a line's closed form under the rectangle `admit` its chain and the target leave:
    /// counted as culled where no pixel of its ink is admitted, an instance otherwise.
    pub(super) fn place_line(
        &mut self,
        unclipped: &Unclipped,
        admit: Rect,
        mask_index: Option<u32>,
    ) -> Result<(), RenderError> {
        if let Some(instance) = unclipped.under(admit) {
            return self.push_line_instance(instance, mask_index);
        }
        self.note_culled();
        Ok(())
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
        let device_width = raster::resolve_width(stroke, *to_device);
        // A projecting cap's corner is half the width away along the segment and across it at
        // once, so the reach that bounds every end is the half-width's diagonal.
        let reach = device_width * 0.5 * std::f32::consts::SQRT_2;
        let (d0, d1) = (apply(to_device, line.from), apply(to_device, line.to));
        // §8.5.3.2's dot, which the expansion below would drop as a subpath with no length: the
        // pen where the transform keeps the axes, the resolved width's circle where it does not,
        // which is the width the expansion strokes everything else at.
        let dot = (d0 == d1 && line.cap == LineCap::Round).then(|| {
            pen(line, to_device).map_or((device_width * 0.5, device_width * 0.5), |pen| pen.half)
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
        let segments = [Segment::MoveTo(line.from), Segment::LineTo(line.to)];
        self.segments = self.segments.saturating_add(segments.len() as u64);
        let span = self.clock.start();
        let stroked = match dot {
            Some(half) => raster::dot_pieces(d0, half),
            None => {
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
