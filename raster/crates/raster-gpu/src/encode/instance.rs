//! One mark's instance bytes, and the run of consecutive marks it joins.
//!
//! Two streams, one per lane the brief's section 0 says a page is mostly made of: a rectangle
//! instance is eight floats that `rect.wgsl` evaluates analytically, a coverage quad is
//! sixteen that `coverage.wgsl` weights by a tile of the frame's sheet. Both strides are
//! declared here beside the writer that fills them, because the two numbers and the
//! order of the fields are the same fact stated twice — once in Rust and once in WGSL —
//! and a writer that drifted from its stride draws a picture rather than failing.
//!
//! A [`Batch`] is what the device is actually asked to draw: a run of consecutive
//! instances in one lane, under one style and one soft mask. Runs are formed by
//! *extending* — a switch of any of the three breaks the batch and starts another — so
//! the painter's algorithm survives as the order the instances were written in, and
//! nothing here ever reorders a mark.
//!
//! Three of [`LaneCounts`](crate::LaneCounts)' four fields are counted here, and here is
//! the only place they could honestly be: which lane a mark *asked* for is decided across
//! five arms and can still change afterwards — a glyph tile the atlas refuses falls
//! through to the sheet at commit — while the instance is written exactly once, by the
//! lane that actually drew it.

use raster_scene::{Point, Rect};

use super::parallel::{Draw, Job};
use super::{Encoder, Op};
use crate::error::RenderError;

/// Bytes per rectangle instance: device rect (4 × f32), premultiplied colour
/// (4 × f32). Must match `rect.wgsl`.
pub(crate) const RECT_INSTANCE_STRIDE: u64 = 32;

/// Bytes per coverage-quad instance: dest min (2), size (2), texel origin + source
/// selector (4), premultiplied colour (4), clip rect (4) — 16 × f32. Must match
/// `coverage.wgsl`.
pub(crate) const QUAD_INSTANCE_STRIDE: u64 = 64;

/// Bytes per line instance: the two endpoints (4 × f32), the pen's semi-axes and the end with
/// the width's resolution (3 × f32), premultiplied colour (4 × f32) and the rectangle the clip
/// admits (4 × f32) — fifteen floats, every one of them the page's and none the view's, which
/// `line.wgsl` applies (ADR 2022). Must match `line.wgsl` (ADR 0097).
pub(crate) const LINE_INSTANCE_STRIDE: u64 = 60;

/// [`LINE_INSTANCE_STRIDE`] as the length of the array one instance is written from, held to it
/// at compile time.
pub(super) const LINE_INSTANCE_BYTES: usize = 60;
const _: () = assert!(LINE_INSTANCE_BYTES as u64 == LINE_INSTANCE_STRIDE);

/// The bytes of a line instance that its clip does not decide: everything before the rectangle
/// the chain admits, which is the instance's last four floats.
pub(super) const LINE_HEAD_BYTES: usize = 44;
const _: () = assert!(LINE_HEAD_BYTES + 16 == LINE_INSTANCE_BYTES);

/// The view `line.wgsl` maps a line's page-space instance by (ADR 2022): the viewport's affine,
/// `a b c d` then `e f`, and the target's size, whose rectangle meets every line's clip as
/// [`target_rect`](super::device_space::target_rect) does on the host. The eight floats
/// `Globals` carries after the attachment's own four, written into every region's globals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineView(pub [f32; 8]);

/// The bytes of `line.wgsl`'s `Globals`: the attachment's four floats, which are all
/// `rect.wgsl`'s and `coverage.wgsl`'s declare, then the [`LineView`]'s eight.
pub(crate) const GLOBALS_BYTES: usize = 48;

impl LineView {
    /// The view of `viewport`.
    #[expect(clippy::cast_precision_loss)] // extents inside f32's exact integer range
    pub(super) fn of(viewport: &crate::viewport::Viewport<'_>) -> Self {
        let t = viewport.transform;
        Self([
            t.a,
            t.b,
            t.c,
            t.d,
            t.e,
            t.f,
            viewport.width as f32,
            viewport.height as f32,
        ])
    }
}

/// Which lane a batch draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BatchKind {
    Rect,
    Quad,
    /// One-segment strokes whose area `line.wgsl` computes (ADR 0097).
    Line,
}

/// Where a coverage quad's texels come from — the glyph lane's atlas, or the frame's
/// own sheet.
///
/// A named pair rather than the `0.0`/`1.0` this used to be passed as, because the two
/// are the same fact twice over: `coverage.wgsl` selects its texture by the number, and
/// [`LaneCounts`](crate::LaneCounts) counts the mark by the lane. A float parameter could
/// carry a third value that means nothing to either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoverageSource {
    /// The persistent R8 glyph atlas (ADR 0008).
    Atlas,
    /// The frame's coverage sheet, whichever lane filled it (ADR 0021, ADR 0016).
    Sheet,
}

impl CoverageSource {
    /// The selector `coverage.wgsl` reads out of the instance's fifth float.
    fn selector(self) -> f32 {
        match self {
            Self::Atlas => 0.0,
            Self::Sheet => 1.0,
        }
    }
}

/// How a batch composites: ordinary premultiplied over, or the knockout two-pass
/// (per-element erase by shape, then additive deposit — ADR 0010, §11.4.6/brief section 4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrawStyle {
    Over,
    Knockout,
    /// §11.4.6's first stage alone: scale the backdrop by `1 − shape` and deposit
    /// nothing ([`Compose::DestOut`](raster_scene::Compose::DestOut), ADR 0025). The same erase pass the knockout pair
    /// opens with, asked for by name.
    DestOut,
    /// §11.4.6's second stage alone: add the mark, premultiplied ([`Compose::Plus`](raster_scene::Compose::Plus)).
    Plus,
    /// §11.7.4.3's special overprinting blend mode where it keeps every channel of the
    /// backdrop ([`Compose::DestOver`](raster_scene::Compose::DestOver)): one
    /// fixed-function pass, instanced like over, because a single blend state composes
    /// overlapping marks in order.
    DestOver,
}

/// A run of consecutive instances in one lane with one style and one soft mask, in
/// scene order — the painter's algorithm survives switching by batch breaks, not by
/// reordering.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Batch {
    pub kind: BatchKind,
    pub first: u32,
    pub count: u32,
    pub style: DrawStyle,
    /// The soft mask sampled by these draws, as a mask index.
    pub mask: Option<u32>,
}

/// One line instance as the line lane's stream holds it, and the box its ink lies in, as
/// [`Encoder::push_line_instance`] writes it and a queued job carries it to its commit.
#[derive(Debug, Clone, Copy)]
pub(super) struct LineInstance {
    /// The instance's bytes, in `line.wgsl`'s order.
    pub bytes: [u8; LINE_INSTANCE_BYTES],
    /// The mark's box on the device inside the rectangle the clip and the target admit, for
    /// the plan.
    pub ink: Rect,
}

impl LineInstance {
    /// The instance whose head is `head` — the line's own fields, [`line_head`] — under the
    /// clip chain's rectangle `admit` in the page's space (ADR 2022).
    pub(super) fn under(head: &[u8; LINE_HEAD_BYTES], admit: Rect, ink: Rect) -> Self {
        let mut bytes = [0_u8; LINE_INSTANCE_BYTES];
        let (front, back) = bytes.split_at_mut(LINE_HEAD_BYTES);
        front.copy_from_slice(head);
        let corners = [admit.min.x, admit.min.y, admit.max.x, admit.max.y];
        for (slot, value) in back.chunks_exact_mut(4).zip(corners) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
        Self { bytes, ink }
    }
}

/// The head of a line instance (`line.wgsl`'s first eleven floats): the two endpoints and the
/// pen's semi-axes in the page's space, Table 53's end packed with how the view resolves the
/// width (ADR 2022), and the premultiplied colour.
///
/// Assembled once into an array rather than written four bytes at a time: the stream grows by
/// millions of these on a page of lines (ADR 1878), and the array is what a line made ahead of
/// the walk carries to it (ADR 1884).
pub(super) fn line_head(
    (from, to): (Point, Point),
    ((half_x, half_y), cap, resolution): ((f32, f32), raster_scene::LineCap, u32),
    color: raster_scene::Color,
) -> [u8; LINE_HEAD_BYTES] {
    let values = [
        from.x,
        from.y,
        to.x,
        to.y,
        half_x,
        half_y,
        end_code(cap, resolution),
        color.r * color.a,
        color.g * color.a,
        color.b * color.a,
        color.a,
    ];
    let mut head = [0_u8; LINE_HEAD_BYTES];
    for (slot, value) in head.chunks_exact_mut(4).zip(values) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
    head
}

/// The number `line.wgsl` reads Table 53's end and the width's resolution by: the end — 0 a
/// butt, 1 a round, 2 a projecting end — plus three times the resolution's own number, so that
/// the instance keeps its fifteen floats (ADR 2022). Every value is a small integer, exact in an
/// `f32`.
fn end_code(cap: raster_scene::LineCap, resolution: u32) -> f32 {
    let end = match cap {
        raster_scene::LineCap::Butt => 0_u8,
        raster_scene::LineCap::Round => 1,
        raster_scene::LineCap::Square => 2,
    };
    let resolution = match resolution {
        0 => 0_u8,
        1 => 3,
        _ => 6,
    };
    // At most eight, so the sum is exact and never saturates.
    f32::from(end.saturating_add(resolution))
}

/// Bytes to reserve for one instance stream, from the command count the frame budget
/// was checked against.
///
/// Called only after the budget check, so the value is bounded by the caller's stated
/// `frame_budget_bytes`: this reserves what the frame has already been *charged*, never
/// more. The arithmetic is `u64` because the check is, and a product that does not fit
/// a `usize` reserves **nothing** rather than saturating — a `Vec` that grows is
/// correct and slower, while `with_capacity(usize::MAX)` is an abort.
pub(super) fn instance_reserve(commands: usize, stride: u64) -> usize {
    usize::try_from((commands as u64).saturating_mul(stride)).unwrap_or(0)
}

impl Encoder<'_> {
    /// One instance of the analytic rectangle lane.
    ///
    /// `style` is a parameter rather than `self.style` because the lane now takes fills
    /// as well as [`Command::Rect`](raster_scene::Command::Rect)s (ADR 0047), and a fill names its own pass through
    /// [`Compose`](raster_scene::Compose): `Src` is §11.4.6's knockout, `DestOut` and `Plus` are its two stages
    /// asked for by name (ADR 0025). A `Rect` carries no compose mode and so passes the
    /// enclosing group's style, which is what this method used to read for itself.
    pub(super) fn push_rect_instance(
        &mut self,
        rect: Rect,
        color: raster_scene::Color,
        style: DrawStyle,
        mask: Option<u32>,
    ) -> Result<(), RenderError> {
        // Draw order is the scene's order, so behind anything queued the rectangle queues
        // too, and is written at its commit (ADR 1409): a drain here would cut a page of
        // text and rules into runs too short for the fan-out's floor.
        if !self.queue.is_empty() {
            return self.enqueue(Job::rect(rect, Draw::new(color, rect, style, mask)));
        }
        self.write_rect_instance(rect, color, style, mask);
        Ok(())
    }

    /// Write one rectangle instance, with nothing queued ahead of it.
    pub(super) fn write_rect_instance(
        &mut self,
        rect: Rect,
        color: raster_scene::Color,
        style: DrawStyle,
        mask: Option<u32>,
    ) {
        self.plan_mut()
            .mark([rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
        let premultiplied = [
            color.r * color.a,
            color.g * color.a,
            color.b * color.a,
            color.a,
        ];
        for value in [rect.min.x, rect.min.y, rect.max.x, rect.max.y] {
            self.rect_instances.extend_from_slice(&value.to_le_bytes());
        }
        for value in premultiplied {
            self.rect_instances.extend_from_slice(&value.to_le_bytes());
        }
        self.lanes.rectangle = self.lanes.rectangle.saturating_add(1);
        self.note_batch(BatchKind::Rect, style, mask);
    }

    /// One instance of the line lane (ADR 0097), in the scene's order: its draw queued behind
    /// anything queued, as a rectangle's is (ADR 1409, ADR 1884).
    ///
    /// The instance is a function of the command, the view and the clip chain alone
    /// ([`super::line::Unclipped`]), which is why most of it can be made ahead of the walk
    /// (`lines_ahead`, ADR 1884); what is placed here, in order, is everything that is not.
    pub(super) fn push_line_instance(
        &mut self,
        line: LineInstance,
        mask: Option<u32>,
    ) -> Result<(), RenderError> {
        let style = self.style;
        // The instance is written where the walk meets it whatever is queued, because nothing but
        // a line writes to this stream; behind a queued mark only its draw waits (ADR 1884). The
        // plan's box is a union, so growing it now rather than at the draw's commit grows it to
        // the same box: the commit is in this plan, since a plan is never left with marks queued.
        // Empty only in the idle build, for a line its view admits nothing of (viewer ADR 2134):
        // the vertex stage draws no pixel of it here, so it grows no box.
        let ink = line.ink;
        if !ink.is_empty() {
            self.plan_mut()
                .mark([ink.min.x, ink.min.y, ink.max.x, ink.max.y]);
        }
        // A walk keeping the encode before's stream writes no line of its own: a line outside its
        // runs that takes the closed form here did not take it there, so the runs are not this
        // view's (`kept_lines`, ADR 2056).
        self.before.diverge();
        // One write of the whole instance: each `extend_from_slice` asks for room (ADR 1878).
        self.line_instances.extend_from_slice(&line.bytes);
        self.lanes.line = self.lanes.line.saturating_add(1);
        if self.queue.is_empty() {
            self.note_batch(BatchKind::Line, style, mask);
            return Ok(());
        }
        let index = self.last_instance(BatchKind::Line);
        self.enqueue_line(index, style, mask)
    }

    #[expect(clippy::too_many_arguments)] // one instance layout, one writer
    pub(super) fn push_quad_instance(
        &mut self,
        dest: Point,
        width: f32,
        height: f32,
        tex_x: f32,
        tex_y: f32,
        source: CoverageSource,
        color: raster_scene::Color,
        clip: Rect,
        style: DrawStyle,
        mask: Option<u32>,
    ) -> Result<(), RenderError> {
        // Draw order is the scene's order, so anything queued draws first (`parallel`).
        self.drain_queue()?;
        self.plan_mut()
            .mark([dest.x, dest.y, dest.x + width, dest.y + height]);
        let premultiplied = [
            color.r * color.a,
            color.g * color.a,
            color.b * color.a,
            color.a,
        ];
        let values = [
            dest.x,
            dest.y,
            width,
            height,
            tex_x,
            tex_y,
            source.selector(),
            0.0,
            premultiplied[0],
            premultiplied[1],
            premultiplied[2],
            premultiplied[3],
            clip.min.x,
            clip.min.y,
            clip.max.x,
            clip.max.y,
        ];
        for value in values {
            self.quad_instances.extend_from_slice(&value.to_le_bytes());
        }
        let lane = match source {
            CoverageSource::Atlas => &mut self.lanes.glyph,
            CoverageSource::Sheet => &mut self.lanes.path,
        };
        *lane = lane.saturating_add(1);
        self.note_batch(BatchKind::Quad, style, mask);
        Ok(())
    }

    /// Extend the current batch, or start a new one on any switch of lane, style or
    /// mask — scene order is preserved by breaking batches, never by reordering.
    fn note_batch(&mut self, kind: BatchKind, style: DrawStyle, mask: Option<u32>) {
        let index = self.last_instance(kind);
        self.note_run(kind, (index, 1), style, mask);
    }

    /// The index of the instance just written to `kind`'s stream.
    ///
    /// # Why the `- 1` cannot wrap, and the `as u32` cannot truncate
    ///
    /// This is private to `encode`, and every caller appends **one whole instance** immediately
    /// before calling it — `kept_lines`'s `keep_line` included, which asks just after the line's
    /// instance is written — so the stream holds at least one stride and the index of its last
    /// instance is at least zero. There is no path that asks before writing one; if one were added, the subtraction would wrap to `u32::MAX` in
    /// release and the batch would name an instance the draw call does not have. The
    /// `no_batch_is_noted_before_its_instance_is_written` test states the invariant from
    /// the outside — a first mark's batch begins at index 0 — so the argument is gated
    /// rather than only written down.
    ///
    /// `as u32` is a truncation only above four billion instances in one stream, which is
    /// 137 GB of rectangle instances; `encode` refuses the frame at
    /// `Options::max_frame_bytes` (2 GiB by default, and never above the largest buffer the
    /// device makes) long before, having charged one rect and one quad per command up front.
    #[expect(clippy::cast_possible_truncation, clippy::arithmetic_side_effects)]
    pub(super) fn last_instance(&self, kind: BatchKind) -> u32 {
        (match kind {
            BatchKind::Rect => (self.rect_instances.len() as u64 / RECT_INSTANCE_STRIDE) - 1,
            BatchKind::Quad => (self.quad_instances.len() as u64 / QUAD_INSTANCE_STRIDE) - 1,
            BatchKind::Line => (self.line_instances.len() as u64 / LINE_INSTANCE_STRIDE) - 1,
        }) as u32
    }

    /// Draw `count` consecutive instances of one lane from `first`: the batch before extended
    /// where it is the same lane, style and mask and ends at `first`, a new one otherwise — so a
    /// run noted at once is the batch its instances noted one by one would have made.
    pub(super) fn note_run(
        &mut self,
        kind: BatchKind,
        (first, count): (u32, u32),
        style: DrawStyle,
        mask: Option<u32>,
    ) {
        if let Some(Op::Draw(last)) = self.plan_mut().ops.last_mut()
            && last.kind == kind
            && last.style == style
            && last.mask == mask
            && last.first.saturating_add(last.count) == first
        {
            last.count = last.count.saturating_add(count);
            return;
        }
        self.append_op(Op::Draw(Batch {
            kind,
            first,
            count,
            style,
            mask,
        }));
    }
}
