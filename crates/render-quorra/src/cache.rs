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

use pdf_render::{Path, ShadingKind};
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

/// The three caches, one frame clock, one eviction policy.
pub(crate) struct ResourceCaches {
    outlines: HashMap<usize, Entry<Arc<Path>, quorra_scene::OutlineId>>,
    images: HashMap<ImageKey, Entry<Arc<[u8]>, quorra_scene::ImageId>>,
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
    Image(ImageKey),
    Ramp(usize),
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

    use quorra_scene::{ImageId, ResourceId};

    use super::ResourceCaches;

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
