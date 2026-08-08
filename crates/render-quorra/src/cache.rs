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

use std::collections::HashMap;
use std::sync::Arc;

use pdf_render::{Path, ShadingKind};
use quorra_scene::ResourceId;

use crate::QuorraRasterError;

/// One cached upload: the pinned identity, the device id, and when it was last
/// part of a frame.
struct Entry<Pin, Id> {
    /// Never read — held for what holding it *means*: the `Arc`'s address cannot
    /// be recycled while this clone lives, which is the whole ABA argument above.
    #[expect(dead_code, reason = "a pin is held for its lifetime, not its value")]
    pin: Pin,
    id: Id,
    last_used: u64,
}

/// The three caches, one frame clock, one eviction policy.
pub(crate) struct ResourceCaches {
    outlines: HashMap<usize, Entry<Arc<Path>, quorra_scene::OutlineId>>,
    images: HashMap<usize, Entry<Arc<[u8]>, quorra_scene::ImageId>>,
    ramps: HashMap<usize, Entry<Arc<ShadingKind>, quorra_scene::RampId>>,
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
}

impl std::fmt::Debug for ResourceCaches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceCaches")
            .field("outlines", &self.outlines.len())
            .field("images", &self.images.len())
            .field("ramps", &self.ramps.len())
            .field("frame", &self.frame)
            .field("stored", &self.stored)
            .finish()
    }
}

impl ResourceCaches {
    pub(crate) fn new() -> Self {
        Self {
            outlines: HashMap::new(),
            images: HashMap::new(),
            ramps: HashMap::new(),
            frame: 0,
            stored: 0,
        }
    }

    /// Starts a frame's clock tick; everything looked up from here on counts as
    /// in use by this frame.
    pub(crate) fn begin_frame(&mut self) {
        self.frame = self.frame.saturating_add(1);
        self.stored = 0;
    }

    /// How many lookups missed and became an upload since [`Self::begin_frame`].
    pub(crate) fn stored(&self) -> u32 {
        self.stored
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

    pub(crate) fn image(&mut self, data: &Arc<[u8]>) -> Option<quorra_scene::ImageId> {
        let entry = self.images.get_mut(&key(data.as_ptr()))?;
        entry.last_used = self.frame;
        Some(entry.id)
    }

    pub(crate) fn store_image(&mut self, data: &Arc<[u8]>, id: quorra_scene::ImageId) {
        self.stored = self.stored.saturating_add(1);
        self.images.insert(
            key(data.as_ptr()),
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

    /// Releases least-recently-used entries this frame did not touch, oldest
    /// first, until the device holds no more than half its resource budget.
    ///
    /// Runs after every frame, refused ones included — a long session must stay
    /// healthy through its refusals, not only its successes. An evicted entry is
    /// not an error waiting to happen: the map forgets the address with the pin,
    /// so the same `Arc` drawn again is a cache miss and a fresh upload.
    pub(crate) fn evict_settled(
        &mut self,
        device: &mut quorra_gpu::Device,
    ) -> Result<(), QuorraRasterError> {
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
                Slot::Image(map_key) => match self.images.remove(&map_key) {
                    Some(entry) => entry.id.into(),
                    None => continue,
                },
                Slot::Ramp(map_key) => match self.ramps.remove(&map_key) {
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
    Image(usize),
    Ramp(usize),
}

fn key(pointer: *const u8) -> usize {
    pointer as usize
}
