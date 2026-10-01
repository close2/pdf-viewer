//! A stroke's expansion, made once for every placement of it that differs only by where it
//! sits (ADR 1445).
//!
//! ISO 32000-2 §8.4.3.2 makes a stroke a set of points near its path:
//!
//! > stroking a path shall entail painting all points whose perpendicular distance from the
//! > path in user space is less than or equal to half the line width
//!
//! A translation carries that set onto the translated path's set, point for point, and the
//! device width is the linear part's alone ([`raster::resolve_width`]). So the pieces the
//! stroker builds depend on the outline, the stroke's own numbers and the device transform's
//! linear part — and the translation moves them afterwards. **Every stroke is therefore
//! expanded under its linear part with no translation, and then placed**, whether or not a
//! second placement asks: an expansion that a lookup hands out and an expansion made on the
//! spot are then one computation, so a frame's bytes do not depend on which placement made
//! it, on the thread count, or on whether the cache had room.
//!
//! The page this is for places one Type 3 glyph cell at several positions of a tiling
//! pattern, each glyph stroked wider than itself, so every bend is tight and each subpath of
//! about a hundred and fifty pieces is re-cut into a tiling (ADR 1375) — the same tiling at
//! each position: 144 expansions of 24 shapes on a page turn. Made once per shape, its
//! process falls from 2 127 to 989 M instructions and its turn from 11.5 to 6.0 ms (pinned,
//! encode 10.0 to 4.6); a page of strokes each placed once pays 0.26% for the translation
//! added afterwards (ADR 1445). The cost to a reader is this module and one field on a job.
//!
//! # What is kept, and for how long
//!
//! One frame. The key is the outline, the linear part's bits and the stroke's bits, the same
//! identity the glyph atlas keys on (ADR 0009) with the stroke added; a zoom step is a new
//! linear part and a new key, so nothing kept past the frame would be asked for by the
//! frame after it except a repaint, which does not encode. Only a shape the scene places
//! more than once gets a slot ([`Expansions::of`] counts before the walk, as the census
//! does for fills, ADR 0029), so a page of strokes each drawn once holds nothing it did not
//! hold before. And what the slots hold together is bounded by a share of the caller's
//! frame budget ([`HELD_BUDGET_SHARE`]): an expansion past it is made, used and dropped by
//! the placement that made it, and every later placement makes its own — the same pieces,
//! at the price of a placement that found no slot (CLAUDE.md principle 3).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use raster_scene::{Command, LineCap, LineJoin, MaskDef, Scene, Segment, Stroke};

use crate::keyhash::FastMap;
use crate::raster::{self, DeviceTransform, Polyline};

/// The share of the caller's `max_frame_bytes` the frame's kept expansions may hold
/// together: a sixteenth of the 268 MiB default is 16 MiB, about two hundred of the Type 3
/// page's glyph expansions (ADR 1445 has their size), and a caller who lowers the frame
/// budget lowers this with it.
const HELD_BUDGET_SHARE: u64 = 16;

/// A stroke's pieces under its linear part, before any translation, and whether they tile
/// its set by construction (ADR 1397, ADR 1421, ADR 1431).
pub(super) struct Expansion {
    pieces: Vec<Polyline>,
    tiles: bool,
}

impl Expansion {
    /// Expand `segments` under `transform`'s linear part.
    fn of(segments: &[Segment], transform: DeviceTransform, stroke: Stroke) -> Self {
        let linear = linear(transform);
        let flattened = raster::flatten_stroke(segments, linear);
        let stroked =
            raster::stroke_pieces(&flattened, stroke, raster::resolve_width(stroke, linear));
        Self {
            pieces: stroked.pieces,
            tiles: stroked.tiles,
        }
    }

    /// The pieces moved by `transform`'s translation, and whether they tile the set: moved
    /// where they lie when no other placement holds them, copied when one does.
    ///
    /// The flattening applies `a·x + c·y + e`; under the linear part it computes `a·x + c·y`
    /// and this adds `e`, so a flattened point is the placed flattening's to the bit and only
    /// the stroker's arithmetic on the points is done at the origin rather than at the
    /// placement.
    pub(super) fn placed(this: Arc<Self>, transform: DeviceTransform) -> (Vec<Polyline>, bool) {
        let (e, f) = (transform.e, transform.f);
        let place = |p: &raster_scene::Point| raster_scene::Point::new(p.x + e, p.y + f);
        match Arc::try_unwrap(this) {
            Ok(mut own) => {
                for piece in &mut own.pieces {
                    for p in &mut piece.points {
                        *p = place(p);
                    }
                }
                (own.pieces, own.tiles)
            }
            Err(shared) => {
                let pieces = shared
                    .pieces
                    .iter()
                    .map(|piece| Polyline {
                        points: piece.points.iter().map(place).collect(),
                        closed: piece.closed,
                        tangents: piece.tangents.clone(),
                    })
                    .collect();
                (pieces, shared.tiles)
            }
        }
    }

    /// The host memory the pieces hold, for [`HELD_BUDGET_SHARE`].
    fn bytes(&self) -> u64 {
        let points: usize = self.pieces.iter().map(|piece| piece.points.len()).sum();
        let records = self.pieces.len().saturating_mul(size_of::<Polyline>());
        (points.saturating_mul(size_of::<raster_scene::Point>())).saturating_add(records) as u64
    }
}

/// `transform` with its translation taken off.
fn linear(transform: DeviceTransform) -> DeviceTransform {
    DeviceTransform {
        e: 0.0,
        f: 0.0,
        ..transform
    }
}

/// One shape's expansion, made by whichever placement asks first and read by the rest.
pub(super) struct Slot {
    /// `None` once made where the frame's share was spent: the maker used its own.
    made: OnceLock<Option<Arc<Expansion>>>,
    held: Arc<AtomicU64>,
    limit: u64,
}

/// A stroke's expansion under `transform`'s linear part: out of `slot` where the scene
/// places the shape more than once and the frame's share had room, made here otherwise.
///
/// A pure function of the outline, the linear part and the stroke whoever makes it, which
/// is what lets a placement on another thread read what this one made (ADR 1419's argument
/// for the two-values answer, here for the pieces).
pub(super) fn expansion(
    slot: Option<&Slot>,
    segments: &[Segment],
    transform: DeviceTransform,
    stroke: Stroke,
) -> Arc<Expansion> {
    let Some(slot) = slot else {
        return Arc::new(Expansion::of(segments, transform, stroke));
    };
    let mut own = None;
    let kept = slot.made.get_or_init(|| {
        let made = Arc::new(Expansion::of(segments, transform, stroke));
        let bytes = made.bytes();
        let before = slot.held.fetch_add(bytes, Ordering::Relaxed);
        if before.saturating_add(bytes) <= slot.limit {
            Some(made)
        } else {
            slot.held.fetch_sub(bytes, Ordering::Relaxed);
            own = Some(made);
            None
        }
    });
    match (kept, own) {
        (Some(kept), _) => Arc::clone(kept),
        (None, Some(own)) => own,
        (None, None) => Arc::new(Expansion::of(segments, transform, stroke)),
    }
}

/// What a stroke's expansion is keyed by: the outline, the linear part and the stroke, all
/// as bits — the same bits are the same shape, as the atlas keys a glyph (ADR 0009).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Shape {
    outline: u32,
    linear: [u32; 4],
    stroke: [u32; 3],
    cap_join: (u8, u8),
}

impl Shape {
    fn new(outline: u32, linear: [u32; 4], stroke: Stroke) -> Self {
        Self {
            outline,
            linear,
            stroke: [
                stroke.width.to_bits(),
                stroke.miter_limit.to_bits(),
                u32::from(stroke.adjust),
            ],
            cap_join: (
                match stroke.cap {
                    LineCap::Butt => 0,
                    LineCap::Round => 1,
                    LineCap::Square => 2,
                },
                match stroke.join {
                    LineJoin::Miter => 0,
                    LineJoin::Round => 1,
                    LineJoin::Bevel => 2,
                },
            ),
        }
    }
}

/// The frame's slots: one per stroked shape the scene places more than once.
#[derive(Default)]
pub(super) struct Expansions {
    /// How often the scene places each shape, by the command's own linear part; a count of
    /// one gets no slot.
    counts: FastMap<Shape, u32>,
    /// The slots handed out so far, by the device linear part.
    slots: FastMap<Shape, Arc<Slot>>,
    held: Arc<AtomicU64>,
    limit: u64,
}

impl Expansions {
    /// Counts the scene's strokes, including those inside groups and soft masks, before the
    /// walk; `frame_budget_bytes` is the caller's, of which the slots may hold
    /// [`HELD_BUDGET_SHARE`]th.
    pub(super) fn of(scene: &Scene, frame_budget_bytes: u64) -> Self {
        let mut expansions = Self {
            limit: frame_budget_bytes / HELD_BUDGET_SHARE,
            ..Self::default()
        };
        expansions.count(scene.commands());
        for mask in scene.masks() {
            let MaskDef { commands, .. } = mask;
            expansions.count(commands);
        }
        expansions
    }

    fn count(&mut self, commands: &[Command]) {
        for command in commands {
            match command {
                Command::Stroke {
                    outline,
                    transform,
                    stroke,
                    ..
                } => {
                    let linear = [transform.a, transform.b, transform.c, transform.d];
                    let shape = Shape::new(outline.0, linear.map(f32::to_bits), *stroke);
                    let count = self.counts.entry(shape).or_insert(0);
                    *count = count.saturating_add(1);
                }
                Command::Group { commands, .. } => self.count(commands),
                _ => {}
            }
        }
    }

    /// The slot for a stroke of `outline` whose command states `command_linear` and whose
    /// device transform is `to_device`, or `None` where the scene places it once.
    pub(super) fn slot(
        &mut self,
        outline: u32,
        command_linear: [f32; 4],
        to_device: DeviceTransform,
        stroke: Stroke,
    ) -> Option<Arc<Slot>> {
        let counted = Shape::new(outline, command_linear.map(f32::to_bits), stroke);
        if self.counts.get(&counted).is_none_or(|&count| count < 2) {
            return None;
        }
        let device = [to_device.a, to_device.b, to_device.c, to_device.d];
        let shape = Shape::new(outline, device.map(f32::to_bits), stroke);
        let (held, limit) = (&self.held, self.limit);
        Some(Arc::clone(self.slots.entry(shape).or_insert_with(|| {
            Arc::new(Slot {
                made: OnceLock::new(),
                held: Arc::clone(held),
                limit,
            })
        })))
    }
}
