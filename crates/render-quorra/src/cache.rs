//! The `Arc`-identity resource caches — and, since `QUORRA_FEEDBACK.md` section 2, the way
//! out of them.
//!
//! Each entry **holds a clone of the `Arc` it is keyed by**. That is not a
//! convenience: a pointer key alone is an ABA bug — drop a display list, let the
//! allocator hand the same address to a different path, and the cache serves the
//! old outline for the new geometry, sporadically, by allocator mood. Pinning the
//! allocation makes the address unique for as long as the entry lives, and is what
//! lets a cache span frames (a zoom re-uploads nothing, `RENDER_LIBRARY.md` section 2.2).
//!
//! What pinning alone lacked was an exit: nothing ever decided an entry should
//! stop living, so a host with documents open all afternoon marched the device to
//! its resource budget and every later upload was refused — correct behaviour,
//! with no way to make room. The corpus gate at 4× scale was the instrument that
//! could see it (533 of 952 pages refused, each passing alone). So the caches now
//! carry recency, and [`ResourceCaches::evict_settled`] runs after every frame:
//! while the device holds more than **half** its resource budget, the
//! least-recently-used entries that this frame did not touch are released, pin and
//! all. Half, so that eviction is not a cliff at the budget's edge: a frame always
//! finds at least half the budget free for what it newly needs, and an entry hot
//! enough to be touched every frame is never evicted at all.
//!
//! **And the pin answers a second question it was not taken for: when an entry has
//! become unreachable.** A key is an address the pin keeps unique, so an entry whose
//! pin is the *only* reference left to that allocation can never be looked up again
//! — no display list can hold the address, because the pin holds it. That is a proof
//! rather than a policy, so [`ResourceCaches::evict_settled`] releases those first
//! and unconditionally, before it asks the device what it is holding. It matters most
//! for what this cache started keeping in the four-hundred-and-sixty-second session:
//! a reduced raster is keyed by its *source* image's identity, so its pin is a whole
//! scanned page's samples — 37 MB where the device entry it guards is 9 — and a host
//! that turned the page would otherwise hold that until the device's own budget, which
//! counts none of it, happened to notice. ADR 0297.

use std::collections::HashMap;
use std::sync::Arc;

use pdf_render::{LineCap, LineJoin, Path, ProgramStep, ShadingKind, Stroke};
use quorra_scene::ResourceId;

use crate::QuorraRasterError;

/// What an uploaded image is keyed by: the source samples' address, and the reduction
/// that stands between them and what was uploaded.
///
/// `(1, 1)` is the image itself, uploaded whole. Anything else is the grid
/// [`pdf_render::Image::area_averaged`] produced from it, and the factors belong in the
/// key because they decide every byte of it: the same source drawn at two scales deep
/// enough to reduce differently is two rasters, and serving either for the other would
/// draw the page at the wrong resolution.
type ImageKey = (usize, u32, u32);

/// What an **expanded** stroke's outline is keyed by: the source path's address, and every
/// parameter that decided the geometry `kurbo::stroke` produced from it.
///
/// `crate::stroke` outlines a stroke in path space wherever the placement is anisotropic —
/// §8.4.3.2's own note, because a scalar device width is exactly wrong under a shear — and the
/// result is computed geometry rather than the display list's own path. That used to make it a
/// *transient*: uploaded and released every frame, so its identifier moved between two renders of
/// one unchanged page. quorra keys every glyph-lane tile on that identifier, so every key went
/// foreign every frame and its atlas repacked at period two, for ever
/// (`render-lib/doc/notes-atlas-budget.md` section 5; ADR 0402).
///
/// **The key is the source path plus the arguments, never the expansion.** Hashing the expanded
/// outline would be the obvious alternative and it is the wrong one twice over: it would cost a
/// walk of geometry that is often larger than the path it came from, and it would have to be
/// computed before it could be looked up — where this key is what lets a hit skip the expansion
/// altogether. The pin is the source path for the module's ABA reason, which does not care that
/// the bytes on the device are a different shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct StrokeKey {
    /// The source `Arc<Path>`'s address, kept unique by the entry's pin.
    path: usize,
    /// The stroke width in the path's own space, by bits: two widths that differ in the last
    /// place are two outlines, and `f32` has no `Eq`.
    width: u32,
    /// The flattening tolerance, by bits — it is a function of the placement's stretch, so a
    /// page drawn at two magnifications expands its curves twice, as it must.
    tolerance: u32,
    /// §8.4.3.5's ratio, by bits, already clamped to the smallest legal value by the caller.
    miter_limit: u32,
    cap: LineCap,
    join: LineJoin,
}

impl StrokeKey {
    /// The key for `path` stroked with `stroke` at `width` in the path's own space, flattened
    /// to `tolerance`.
    ///
    /// The two scalars are arguments rather than derived here because they are the caller's
    /// resolution of §8.4.3.2 and §10.7.2 — the same values it hands `kurbo::stroke` — and a key
    /// that recomputed either could disagree with the geometry it names.
    pub(crate) fn new(path: &Arc<Path>, stroke: &Stroke, width: f32, tolerance: f32) -> Self {
        Self {
            path: key(Arc::as_ptr(path).cast::<u8>()),
            width: width.to_bits(),
            tolerance: tolerance.to_bits(),
            miter_limit: stroke.miter_limit.max(1.0).to_bits(),
            cap: stroke.cap,
            join: stroke.join,
        }
    }
}

/// One cached upload: the pinned identity, the device id, and when it was last
/// part of a frame.
struct Entry<Pin, Id> {
    /// Held first for what holding it *means*: the `Arc`'s address cannot be recycled
    /// while this clone lives, which is the whole ABA argument above. Its *count* is
    /// then read for the second thing that follows from the same fact — a pin nobody
    /// else holds is an entry nobody can look up ([`ResourceCaches::drop_unreachable`]).
    pin: Pin,
    id: Id,
    last_used: u64,
}

/// The five caches, one frame clock, one eviction policy.
pub(crate) struct ResourceCaches {
    outlines: HashMap<usize, Entry<Arc<Path>, quorra_scene::OutlineId>>,
    /// Stroke outlines this crate expanded rather than uploaded as they stood ([`StrokeKey`]).
    strokes: HashMap<StrokeKey, Entry<Arc<Path>, quorra_scene::OutlineId>>,
    images: HashMap<ImageKey, Entry<Arc<[u8]>, quorra_scene::ImageId>>,
    ramps: HashMap<usize, Entry<Arc<ShadingKind>, quorra_scene::RampId>>,
    /// §7.10.5 programs the device has admitted and compiled a shader for.
    ///
    /// **A program is the one resource here whose upload buys something the upload does not
    /// hold**: the device keys its generated shader on the program's contents and drops it
    /// when the last id naming those contents is released (quorra's ADR 0053). So a program
    /// uploaded and released per frame — which is what a transient would be — would recompile
    /// a shader on every frame of a still page, which is the launch-path cost `CLAUDE.md`
    /// forbids arriving once per frame instead. Keyed like a ramp, by the `Arc`'s address,
    /// under the module's own ABA argument.
    programs: HashMap<usize, Entry<Arc<[ProgramStep]>, quorra_scene::FunctionId>>,
    /// Advances once per frame; entries touched this frame are never evicted.
    frame: u64,
    /// How many entries this frame *stored* — that is, how many lookups missed and
    /// became an upload.
    ///
    /// Counted here rather than at the nine `upload_*` call sites because this is
    /// where the miss is decided, and because a counter costs an increment where a
    /// timer at each site would cost a clock read. What it answers is the question a
    /// slow frame raises first: whether the caches are working at all. A page drawn
    /// twice with an unchanged display list should store nothing the second time —
    /// entries are keyed by `Arc` identity, so a list rebuilt from scratch every
    /// frame would re-upload every resource and this is what would say so.
    stored: u32,
    /// What those uploads cost — the wall clock spent *inside* quorra's own `upload_*` calls.
    ///
    /// **The duration [`Self::stored`] never had, and the reason the comment above says a timer
    /// would cost a clock read is now an argument that was answered rather than one that stands.**
    /// A cold frame of the project owner's own drawing hands 58 029 resources over and spends
    /// hundreds of milliseconds in `scene`; which side of the boundary those milliseconds are on
    /// decided nothing before this field, because `scene` was one number and `up` was a count
    /// beside it (ADR 0387's trap, one phase along). The clock reads are two per *miss* rather
    /// than two per lookup, and ADR 0423 measures what they cost on the frame that misses most.
    ///
    /// Accumulated here beside the count for one reason worth stating: transient outlines and the
    /// fallback raster never reach a cache at all, so a timer hung on the miss path would leave
    /// exactly the uploads no entry pins unmeasured, and a frame drawn entirely from transients
    /// would report nothing handed over.
    handed: std::time::Duration,
    /// How many path segments went into those uploads — [`Self::handed`]'s denominator.
    ///
    /// **A duration with no denominator beside it is read as though the count next to it were
    /// one**, which is the trap ADR 0387 found on the `transfer` row: the frame line printed a
    /// *resource* count beside it and the two were unrelated by three orders of magnitude. An
    /// outline upload's cost is not per resource — a glyph is a handful of segments and a
    /// draughtsman's line work is tens each over tens of thousands of paths — so the number that
    /// makes `handed` comparable between two documents is this one. Outlines only: they are the
    /// per-segment upload, and an image's or a ramp's own size has quorra's `bytes_uploaded`.
    segments: u64,
}

impl std::fmt::Debug for ResourceCaches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceCaches")
            .field("outlines", &self.outlines.len())
            .field("strokes", &self.strokes.len())
            .field("images", &self.images.len())
            .field("ramps", &self.ramps.len())
            .field("programs", &self.programs.len())
            .field("frame", &self.frame)
            .field("stored", &self.stored)
            .field("handed", &self.handed)
            .field("segments", &self.segments)
            .finish()
    }
}

impl ResourceCaches {
    pub(crate) fn new() -> Self {
        Self {
            outlines: HashMap::new(),
            strokes: HashMap::new(),
            images: HashMap::new(),
            ramps: HashMap::new(),
            programs: HashMap::new(),
            frame: 0,
            stored: 0,
            handed: std::time::Duration::ZERO,
            segments: 0,
        }
    }

    /// Starts a frame's clock tick; everything looked up from here on counts as
    /// in use by this frame.
    pub(crate) fn begin_frame(&mut self) {
        self.frame = self.frame.saturating_add(1);
        self.stored = 0;
        self.handed = std::time::Duration::ZERO;
        self.segments = 0;
    }

    /// How many lookups missed and became an upload since [`Self::begin_frame`].
    pub(crate) fn stored(&self) -> u32 {
        self.stored
    }

    /// Records that `spent` was passed inside one of quorra's `upload_*` calls.
    pub(crate) fn hand_over(&mut self, spent: std::time::Duration) {
        self.handed = self.handed.saturating_add(spent);
    }

    /// What this frame's uploads cost, since [`Self::begin_frame`]. See [`Self::handed`].
    pub(crate) fn handed(&self) -> std::time::Duration {
        self.handed
    }

    /// Records that an outline of `segments` segments is about to be handed over.
    pub(crate) fn count_segments(&mut self, segments: usize) {
        self.segments = self
            .segments
            .saturating_add(u64::try_from(segments).unwrap_or(u64::MAX));
    }

    /// How many outline segments this frame handed over. See [`Self::segments`].
    pub(crate) fn segments(&self) -> u64 {
        self.segments
    }

    pub(crate) fn outline(&mut self, path: &Arc<Path>) -> Option<quorra_scene::OutlineId> {
        let entry = self
            .outlines
            .get_mut(&key(Arc::as_ptr(path).cast::<u8>()))?;
        entry.last_used = self.frame;
        Some(entry.id)
    }

    pub(crate) fn store_outline(&mut self, path: &Arc<Path>, id: quorra_scene::OutlineId) {
        self.stored = self.stored.saturating_add(1);
        self.outlines.insert(
            key(Arc::as_ptr(path).cast::<u8>()),
            Entry {
                pin: Arc::clone(path),
                id,
                last_used: self.frame,
            },
        );
    }

    /// The expanded stroke outline `key` names, if this device already holds one.
    ///
    /// A hit is worth more than the upload it saves: the caller has not run `kurbo::stroke` yet,
    /// so it also skips expanding caps, joins and miters over the whole path.
    pub(crate) fn stroke(&mut self, key: StrokeKey) -> Option<quorra_scene::OutlineId> {
        let entry = self.strokes.get_mut(&key)?;
        entry.last_used = self.frame;
        Some(entry.id)
    }

    /// Keeps `id` as the outline expanded from `path` under `key`.
    ///
    /// **The pin is the source path even though the upload is the expansion**, exactly as
    /// [`Self::store_image`] pins the samples behind a reduced raster: the key holds the source's
    /// address, and an entry that did not hold the source could have that address recycled under
    /// it — the module's ABA argument, which does not care which bytes went to the device.
    pub(crate) fn store_stroke(
        &mut self,
        path: &Arc<Path>,
        key: StrokeKey,
        id: quorra_scene::OutlineId,
    ) {
        self.stored = self.stored.saturating_add(1);
        self.strokes.insert(
            key,
            Entry {
                pin: Arc::clone(path),
                id,
                last_used: self.frame,
            },
        );
    }

    /// The upload of `data` reduced by `factors`, if this device already holds one.
    ///
    /// `(1, 1)` asks for the samples themselves.
    pub(crate) fn image(
        &mut self,
        data: &Arc<[u8]>,
        factors: (u32, u32),
    ) -> Option<quorra_scene::ImageId> {
        let entry = self.images.get_mut(&image_key(data, factors))?;
        entry.last_used = self.frame;
        Some(entry.id)
    }

    /// Keeps `id` as the upload of `data` reduced by `factors`.
    ///
    /// **The pin is the source's samples even where the upload is the reduced grid's**, which
    /// is what the key demands: the key is the source's address, and an entry that did not
    /// hold the source could have its address recycled under it — the module's ABA argument,
    /// which does not care which bytes went to the device.
    pub(crate) fn store_image(
        &mut self,
        data: &Arc<[u8]>,
        factors: (u32, u32),
        id: quorra_scene::ImageId,
    ) {
        self.stored = self.stored.saturating_add(1);
        self.images.insert(
            image_key(data, factors),
            Entry {
                pin: Arc::clone(data),
                id,
                last_used: self.frame,
            },
        );
    }

    pub(crate) fn ramp(&mut self, kind: &Arc<ShadingKind>) -> Option<quorra_scene::RampId> {
        let entry = self.ramps.get_mut(&key(Arc::as_ptr(kind).cast::<u8>()))?;
        entry.last_used = self.frame;
        Some(entry.id)
    }

    pub(crate) fn store_ramp(&mut self, kind: &Arc<ShadingKind>, id: quorra_scene::RampId) {
        self.stored = self.stored.saturating_add(1);
        self.ramps.insert(
            key(Arc::as_ptr(kind).cast::<u8>()),
            Entry {
                pin: Arc::clone(kind),
                id,
                last_used: self.frame,
            },
        );
    }

    pub(crate) fn program(
        &mut self,
        steps: &Arc<[ProgramStep]>,
    ) -> Option<quorra_scene::FunctionId> {
        let entry = self.programs.get_mut(&key(steps.as_ptr().cast::<u8>()))?;
        entry.last_used = self.frame;
        Some(entry.id)
    }

    pub(crate) fn store_program(
        &mut self,
        steps: &Arc<[ProgramStep]>,
        id: quorra_scene::FunctionId,
    ) {
        self.stored = self.stored.saturating_add(1);
        self.programs.insert(
            key(steps.as_ptr().cast::<u8>()),
            Entry {
                pin: Arc::clone(steps),
                id,
                last_used: self.frame,
            },
        );
    }

    /// Takes out every entry nothing can look up again, and says what to release.
    ///
    /// An entry's key is an address and its pin is what keeps that address unique, so a pin
    /// held by nobody else is a statement about the whole program: no display list anywhere
    /// carries that allocation, and no lookup can ever produce this key again. Releasing such
    /// an entry therefore costs no hit that could otherwise have happened — which is why it is
    /// done before, and independently of, the budget question below.
    ///
    /// An entry this frame *used* is kept whatever its count says, which the argument above
    /// makes unnecessary and which is here anyway: it is [`Self::evict_settled`]'s own rule —
    /// releasing what the scene just referenced would trade a full cache for a dangling id —
    /// and a rule that costs nothing is cheaper than a proof that it cannot be reached.
    ///
    /// Separate from [`Self::evict_settled`] so that it can be tested without a device: the
    /// rule is about `Arc` counts and needs no graphics adapter to be wrong.
    ///
    /// **The proof became a conservative one when [`Self::strokes`] arrived**, and saying so is
    /// cheaper than pretending otherwise: one source path can now be pinned by two entries — its
    /// own upload and an expansion of it, or two expansions at two widths — so a count above one
    /// no longer means a display list holds it, and such a group keeps itself alive here. Nothing
    /// leaks: those entries stop being *used*, so [`Self::evict_settled`]'s budget pass takes them
    /// oldest-first like any other settled entry. What is lost is only the promptness of the free,
    /// and what would buy it back is a reference count of this cache's own — a second bookkeeping
    /// of a thing `Arc` already counts, for a reclaim the budget pass already makes.
    fn drop_unreachable(&mut self) -> Vec<ResourceId> {
        let mut released: Vec<ResourceId> = Vec::new();
        let frame = self.frame;
        let mut keep = |last_used: u64, pin: usize, id: ResourceId| {
            let live = last_used >= frame || pin > 1;
            if !live {
                released.push(id);
            }
            live
        };
        self.outlines.retain(|_, entry| {
            keep(
                entry.last_used,
                Arc::strong_count(&entry.pin),
                entry.id.into(),
            )
        });
        self.strokes.retain(|_, entry| {
            keep(
                entry.last_used,
                Arc::strong_count(&entry.pin),
                entry.id.into(),
            )
        });
        self.images.retain(|_, entry| {
            keep(
                entry.last_used,
                Arc::strong_count(&entry.pin),
                entry.id.into(),
            )
        });
        self.ramps.retain(|_, entry| {
            keep(
                entry.last_used,
                Arc::strong_count(&entry.pin),
                entry.id.into(),
            )
        });
        self.programs.retain(|_, entry| {
            keep(
                entry.last_used,
                Arc::strong_count(&entry.pin),
                entry.id.into(),
            )
        });
        released
    }

    /// Releases what nothing can look up again, then the least-recently-used entries this
    /// frame did not touch, oldest first, until the device holds no more than half its
    /// resource budget.
    ///
    /// Runs after every frame, refused ones included — a long session must stay
    /// healthy through its refusals, not only its successes. An evicted entry is
    /// not an error waiting to happen: the map forgets the address with the pin,
    /// so the same `Arc` drawn again is a cache miss and a fresh upload.
    pub(crate) fn evict_settled(
        &mut self,
        device: &mut quorra_gpu::Device,
    ) -> Result<(), QuorraRasterError> {
        for id in self.drop_unreachable() {
            device.release(id)?;
        }
        let target = device.limits().max_resource_bytes / 2;
        if device.resource_bytes_in_use() <= target {
            return Ok(());
        }
        // Everything evictable, oldest first, addressed by its own map key so a
        // removal is a lookup. Entries touched this frame stay: releasing what
        // the scene just referenced would trade a full cache for a dangling id.
        let mut settled: Vec<(u64, Slot)> = Vec::new();
        settled.extend(
            self.outlines
                .iter()
                .filter(|(_, entry)| entry.last_used < self.frame)
                .map(|(map_key, entry)| (entry.last_used, Slot::Outline(*map_key))),
        );
        settled.extend(
            self.strokes
                .iter()
                .filter(|(_, entry)| entry.last_used < self.frame)
                .map(|(map_key, entry)| (entry.last_used, Slot::Stroke(*map_key))),
        );
        settled.extend(
            self.images
                .iter()
                .filter(|(_, entry)| entry.last_used < self.frame)
                .map(|(map_key, entry)| (entry.last_used, Slot::Image(*map_key))),
        );
        settled.extend(
            self.ramps
                .iter()
                .filter(|(_, entry)| entry.last_used < self.frame)
                .map(|(map_key, entry)| (entry.last_used, Slot::Ramp(*map_key))),
        );
        settled.extend(
            self.programs
                .iter()
                .filter(|(_, entry)| entry.last_used < self.frame)
                .map(|(map_key, entry)| (entry.last_used, Slot::Program(*map_key))),
        );
        settled.sort_unstable_by_key(|(last_used, _)| *last_used);

        for (_, slot) in settled {
            if device.resource_bytes_in_use() <= target {
                break;
            }
            let id: ResourceId = match slot {
                Slot::Outline(map_key) => match self.outlines.remove(&map_key) {
                    Some(entry) => entry.id.into(),
                    None => continue,
                },
                Slot::Stroke(map_key) => match self.strokes.remove(&map_key) {
                    Some(entry) => entry.id.into(),
                    None => continue,
                },
                Slot::Image(map_key) => match self.images.remove(&map_key) {
                    Some(entry) => entry.id.into(),
                    None => continue,
                },
                Slot::Ramp(map_key) => match self.ramps.remove(&map_key) {
                    Some(entry) => entry.id.into(),
                    None => continue,
                },
                Slot::Program(map_key) => match self.programs.remove(&map_key) {
                    Some(entry) => entry.id.into(),
                    None => continue,
                },
            };
            device.release(id)?;
        }
        Ok(())
    }
}

/// Which map an evictable entry lives in, by that map's own key.
enum Slot {
    Outline(usize),
    Stroke(StrokeKey),
    Image(ImageKey),
    Ramp(usize),
    Program(usize),
}

fn key(pointer: *const u8) -> usize {
    pointer as usize
}

/// The samples' address and the reduction between them and what was uploaded.
fn image_key(data: &Arc<[u8]>, factors: (u32, u32)) -> ImageKey {
    (key(data.as_ptr()), factors.0, factors.1)
}

#[cfg(test)]
mod entries {
    use std::sync::Arc;

    use pdf_render::{LineCap, LineJoin, Path, PathCommand, Point, Stroke};
    use quorra_scene::{ImageId, OutlineId, ResourceId};

    use super::{ResourceCaches, StrokeKey};

    /// A stroke with everything at a value a difference could be seen against.
    fn stroke() -> Stroke {
        Stroke {
            width: 2.0,
            adjust: false,
            cap: LineCap::Round,
            join: LineJoin::Bevel,
            miter_limit: 4.0,
            dash_array: Vec::new(),
            dash_phase: 0.0,
        }
    }

    fn line() -> Arc<Path> {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(10.0, 3.0)));
        Arc::new(path)
    }

    /// Every argument the expansion was made with is in the key, and each one alone moves it.
    ///
    /// The device test in `tests/stable_ids.rs` can only show this for the width, because two
    /// keys that collide draw the same pixels and only one difference at a time is visible that
    /// way. Here the key is the thing under test, so all four are asked at once — and the failure
    /// this discriminates is silent by nature: a key missing a field serves a valid outline
    /// expanded from the wrong arguments, which nothing downstream can tell from the right one.
    #[test]
    fn every_argument_the_expansion_was_made_with_moves_the_key() {
        let path = line();
        let base = StrokeKey::new(&path, &stroke(), 2.0, 0.25);

        assert_eq!(base, StrokeKey::new(&path, &stroke(), 2.0, 0.25), "stable");
        assert_ne!(base, StrokeKey::new(&path, &stroke(), 2.5, 0.25), "width");
        assert_ne!(
            base,
            StrokeKey::new(&path, &stroke(), 2.0, 0.5),
            "tolerance"
        );
        assert_ne!(
            base,
            StrokeKey::new(&path, &line_join(LineJoin::Miter), 2.0, 0.25),
            "join"
        );
        assert_ne!(
            base,
            StrokeKey::new(&path, &line_cap(LineCap::Butt), 2.0, 0.25),
            "cap"
        );
        assert_ne!(
            base,
            StrokeKey::new(&path, &mitred(9.0), 2.0, 0.25),
            "mitre limit"
        );
        assert_ne!(
            base,
            StrokeKey::new(&line(), &stroke(), 2.0, 0.25),
            "a different path, whose only difference is its address"
        );
    }

    fn line_join(join: LineJoin) -> Stroke {
        Stroke { join, ..stroke() }
    }

    fn line_cap(cap: LineCap) -> Stroke {
        Stroke { cap, ..stroke() }
    }

    fn mitred(miter_limit: f32) -> Stroke {
        Stroke {
            miter_limit,
            ..stroke()
        }
    }

    /// An expanded stroke settles and is released like any other entry once nothing holds it.
    ///
    /// The half that could have been got wrong: the entry pins the **source** path rather than
    /// the expansion, so what decides its reachability is whether a display list still holds the
    /// path it was expanded from.
    #[test]
    fn an_expanded_stroke_nothing_else_holds_is_released() {
        let mut caches = ResourceCaches::new();
        let held = line();
        let dropped = line();
        caches.begin_frame();
        caches.store_stroke(
            &held,
            StrokeKey::new(&held, &stroke(), 2.0, 0.25),
            OutlineId(1),
        );
        let gone = StrokeKey::new(&dropped, &stroke(), 2.0, 0.25);
        caches.store_stroke(&dropped, gone, OutlineId(2));

        drop(dropped);
        caches.begin_frame();

        assert_eq!(
            caches.drop_unreachable(),
            vec![ResourceId::Outline(OutlineId(2))]
        );
        assert_eq!(
            caches.stroke(StrokeKey::new(&held, &stroke(), 2.0, 0.25)),
            Some(OutlineId(1))
        );
    }

    /// Two reductions of one image are two entries, and neither answers for the other.
    ///
    /// The defect this discriminates is the one the key was widened to prevent: a scanned page
    /// scrolled at one magnification and then at another asks for two rasters of different
    /// sizes from the same samples, and a cache keyed by the samples alone would draw the
    /// second at the first's resolution — with no report, because both are valid uploads.
    #[test]
    fn a_reduced_upload_is_keyed_by_the_factors_that_produced_it() {
        let mut caches = ResourceCaches::new();
        let samples: Arc<[u8]> = Arc::from(vec![0_u8; 64]);
        caches.begin_frame();
        caches.store_image(&samples, (2, 2), ImageId(7));
        caches.store_image(&samples, (3, 3), ImageId(9));

        assert_eq!(caches.image(&samples, (2, 2)), Some(ImageId(7)));
        assert_eq!(caches.image(&samples, (3, 3)), Some(ImageId(9)));
        assert_eq!(caches.image(&samples, (1, 1)), None, "the whole image");
        assert_eq!(
            caches.image(&samples, (2, 3)),
            None,
            "a factor pair unasked"
        );
    }

    /// An entry whose source has been dropped is released, and one still held is not.
    ///
    /// Both halves matter and only together: releasing a live entry costs a re-upload of a
    /// page that has not changed, and keeping a dead one holds the source samples — a scanned
    /// page's whole 37 MB — against a budget that counts only the device's side of it.
    #[test]
    fn an_entry_nothing_else_holds_is_the_one_that_goes() {
        let mut caches = ResourceCaches::new();
        let held: Arc<[u8]> = Arc::from(vec![1_u8; 64]);
        let dropped: Arc<[u8]> = Arc::from(vec![2_u8; 64]);
        caches.begin_frame();
        caches.store_image(&held, (2, 2), ImageId(1));
        caches.store_image(&dropped, (2, 2), ImageId(2));

        // The display list's own reference goes, which is what a page turn does — and the
        // question is asked on the *next* frame, because an entry this frame used is kept
        // whatever its count says.
        drop(dropped);
        caches.begin_frame();

        assert_eq!(
            caches.drop_unreachable(),
            vec![ResourceId::Image(ImageId(2))]
        );
        assert_eq!(caches.image(&held, (2, 2)), Some(ImageId(1)));
    }
}
