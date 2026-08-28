//! Reduced image samples, produced once and shared by everything that would produce them again.
//!
//! # What is being shared, and with whom
//!
//! [`pdf_render::Image::area_averaged`] averages the blocks of source samples that would share
//! one device pixel before `tiny-skia` filters between them (ADR 0025), and its cost is per
//! **source** sample: a 2480×3506 scan placed on a 596×842 page reads 8.7 million samples
//! whatever the page's own extent is. That work is repeated by two mechanisms this crate has,
//! and neither of them changes a single byte of the answer:
//!
//! - **A strip is a replay of the display list.** [`crate::CpuRasterizer`] cuts a target into
//!   horizontal strips and draws the whole list into each one on its own thread (ADR 0139), and
//!   a full-page image reaches every strip. The strip planner bounds that replay with
//!   [`pdf_render::replay_ratio`], which counts the *rows* a command covers — the right measure
//!   for a fill, and blind to a reduction whose cost is in the source grid rather than on the
//!   page. `issue12963.pdf` page 1 is one image and 1201 other commands: the planner reads its
//!   replay as 1.00 and grants thirteen strips, and the page then reduces the same 8.7 million
//!   samples thirteen times.
//! - **A redraw is another replay.** A host that draws one page repeatedly — `viewer-confined`'s
//!   worker holds a rasteriser across every frame it is asked for — reduces the same samples
//!   again for each. That is the half of `doc/todo/45` §2a `render-quorra` closed for the window
//!   in ADR 0297, on the same key this module uses.
//!
//! # Why the key is an address, and what makes that sound
//!
//! `(the source samples' address, the reduction's two factors)`, which is ADR 0297's key exactly:
//! the reduced samples are a pure function of those three, so an entry answers for any placement
//! that asks for the same factors, and for no other. [`pdf_render::Image::reduction`] exists to
//! answer the factor half *before* the reduction is paid for.
//!
//! An address is only an identity while the allocation lives, so an entry **pins** the
//! `Arc<[u8]>` it was keyed on. A pinned address cannot be recycled under a live entry, which is
//! the whole of the ABA argument; `render-quorra`'s `cache` module states the same one for the
//! same reason.
//!
//! # What is not memoised, and why that is not a gap
//!
//! [`pdf_render::ImageSource::AtDeviceScale`] — §11.6.5.2's soft-mask image, whose samples do not
//! exist until the device scale does — produces a fresh buffer on every call, so it has no
//! address that outlives one draw and nothing here could key on it. Such a source is drawn
//! exactly as it was before this module existed.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use pdf_render::{Image, Reduction, Transform};

/// Largest total the reduced images may reach before the oldest are dropped, in bytes.
///
/// **Derived rather than measured, because the derivation is the stronger statement.** A
/// reduction's factor is the *floor* of source samples per device pixel, so the grid it produces
/// is between one and two samples per device pixel in each axis — under four samples, and so
/// under sixteen bytes, per device pixel the image covers. A full-page image on a 612×792 page
/// therefore holds under 7.8 MB, and this budget holds four such images at once; a page at eight
/// times magnification is a target sixty-four times the area, and there this holds one image
/// covering a sixth of it.
///
/// Chosen equal to this crate's `MASK_BUDGET` because the two are the same kind of number — what one
/// rasteriser may hold in derived rasters — and because nothing has been measured to want them
/// apart. **What eviction costs is exactly what this module buys**, and no more: an evicted
/// entry is reduced again, which is what every draw did before, so the budget bounds the memory
/// without bounding the correctness of anything.
const IMAGE_BUDGET: usize = 32 << 20;

/// The samples' address and the reduction between them and what was produced.
type Key = (usize, u32, u32);

/// A reduction in progress or finished, shared by every strip that asked for it.
///
/// A [`OnceLock`] rather than the reduced image itself, and that is the whole reason the strips
/// cooperate rather than race: the map's lock is held only long enough to hand out this `Arc`, so
/// the first strip to ask runs the reduction while the others **block on the value** instead of
/// each starting their own. Filling the map with the finished image under the map's own lock
/// would serialise strips that want different images; taking the map's lock twice around an
/// unsynchronised reduction would let all thirteen strips of a one-image page start together and
/// buy nothing at all.
type Slot = Arc<OnceLock<Option<Image>>>;

/// One entry: the pinned identity and the slot its value arrives in.
struct Entry {
    /// Held for what holding it *means*: the `Arc`'s address cannot be recycled while this clone
    /// lives, which is what makes an address an identity. Its bytes are not this cache's — the
    /// display list already holds them — and are not counted against [`IMAGE_BUDGET`].
    #[expect(
        dead_code,
        reason = "held for its lifetime rather than its value: an address is an identity only \
                  while its allocation lives, and this clone is what keeps the key's honest"
    )]
    pin: Arc<[u8]>,
    slot: Slot,
}

/// Reduced images, bounded and shared.
pub(crate) struct ReducedImages {
    held: Mutex<Held>,
    /// Largest total the reduced samples may reach before the oldest are dropped.
    ///
    /// A field rather than [`IMAGE_BUDGET`] itself for the reason `MaskCache`'s budget is one:
    /// a bound nothing can vary is a bound no test can drive, and eviction is the half of this
    /// module whose defects are silent.
    budget: usize,
}

impl Default for ReducedImages {
    fn default() -> Self {
        Self {
            held: Mutex::default(),
            budget: IMAGE_BUDGET,
        }
    }
}

/// What the lock protects.
#[derive(Default)]
struct Held {
    entries: HashMap<Key, Entry>,
    /// Keys in insertion order, for eviction.
    order: VecDeque<Key>,
    /// Bytes of reduced samples the entries hold, excluding the pins.
    bytes: usize,
}

impl std::fmt::Debug for ReducedImages {
    /// The count and the bytes, never the samples: a rasteriser's `Debug` is printed by hosts
    /// and by test failures, and a megabyte of image data in either is noise.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (entries, bytes) = self.held.lock().map_or((0, 0), |held| {
            let bytes = held.bytes;
            (held.entries.len(), bytes)
        });
        f.debug_struct("ReducedImages")
            .field("entries", &entries)
            .field("bytes", &bytes)
            .field("budget", &self.budget)
            .finish()
    }
}

impl ReducedImages {
    /// The samples to draw `image` with under `placement`, reduced once however many callers ask.
    ///
    /// `None` where no reduction applies — the placement gathers no two samples into a pixel, or
    /// the image's dimensions and buffer disagree — which is [`pdf_render::Image::reduction`]'s
    /// own answer and is the caller's signal to draw the source samples as they stand.
    ///
    /// The reduction itself is [`pdf_render::Image::area_averaged`] and nothing here changes it,
    /// so a memoised draw and a fresh one produce the same bytes by construction.
    pub(crate) fn reduced(&self, image: &Image, placement: Transform) -> Option<Image> {
        let Reduction { factors, .. } = image.reduction(placement)?;
        let key = (address(&image.data), factors.0, factors.1);

        // Two locks with no work between them, so a strip that wants a different image is never
        // waiting on this one's reduction.
        let (slot, fresh) = {
            let mut held = self.held.lock().ok()?;
            if let Some(entry) = held.entries.get(&key) {
                (Arc::clone(&entry.slot), false)
            } else {
                let slot: Slot = Arc::new(OnceLock::new());
                held.entries.insert(
                    key,
                    Entry {
                        pin: Arc::clone(&image.data),
                        slot: Arc::clone(&slot),
                    },
                );
                held.order.push_back(key);
                (slot, true)
            }
        };

        let produced = slot.get_or_init(|| image.area_averaged(placement)).clone();

        // Only the caller that created the slot accounts for it: every other one is looking at
        // bytes already counted, and counting them twice would evict on a page that fits.
        if fresh {
            let bytes = produced.as_ref().map_or(0, |reduced| reduced.data.len());
            if let Ok(mut held) = self.held.lock() {
                held.bytes = held.bytes.saturating_add(bytes);
                held.evict_to(self.budget);
            }
        }
        produced
    }
}

impl Held {
    /// Drops the oldest entries until the reduced samples fit in `budget`.
    ///
    /// The newest entry is never dropped, even where it alone exceeds the budget: it is the one
    /// the caller is about to draw with and the strips beside it are about to ask for, so
    /// dropping it would spend the reduction and keep none of it.
    fn evict_to(&mut self, budget: usize) {
        while self.bytes > budget && self.order.len() > 1 {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                let bytes = entry
                    .slot
                    .get()
                    .and_then(Option::as_ref)
                    .map_or(0, |image| image.data.len());
                self.bytes = self.bytes.saturating_sub(bytes);
            }
        }
    }
}

/// An allocation's address as an integer, which is all a key needs of it.
fn address(data: &Arc<[u8]>) -> usize {
    data.as_ptr() as usize
}

#[cfg(test)]
mod memo {
    use std::sync::Arc;

    use pdf_render::{Image, Transform};

    use super::ReducedImages;

    /// An `n`×`n` image whose samples vary, so that a wrong reduction is visible in the bytes.
    fn image(n: u32) -> Image {
        let samples = (n as usize).saturating_mul(n as usize);
        let mut data = Vec::with_capacity(samples.saturating_mul(4));
        for i in 0..samples {
            let v = u8::try_from(i % 251).unwrap_or(0);
            data.extend_from_slice(&[v, v.wrapping_add(7), v.wrapping_add(53), 255]);
        }
        Image {
            width: n,
            height: n,
            data: data.into(),
            interpolate: false,
        }
    }

    /// A placement covering `pixels` device pixels each way.
    fn drawn_at(pixels: f32) -> Transform {
        Transform::new(pixels, 0.0, 0.0, pixels, 0.0, 0.0)
    }

    /// A hit is the same bytes the reduction produces, and it is *the same buffer*.
    ///
    /// Two assertions because two different defects are possible and each is invisible to the
    /// other. Byte equality catches a memo serving a valid reduction of the wrong request, which
    /// is what a key missing a field does. Buffer identity catches a memo that stores nothing and
    /// reduces again on every ask — which produces the right bytes for ever and would leave this
    /// whole module measurable only in a profiler.
    #[test]
    fn a_memoised_reduction_is_the_reduction_and_is_the_same_buffer() {
        let source = image(64);
        let placement = drawn_at(8.0);
        let expected = source
            .area_averaged(placement)
            .expect("eight source samples per device pixel is a reduction");

        let memo = ReducedImages::default();
        let first = memo.reduced(&source, placement).expect("a reduction");
        let second = memo.reduced(&source, placement).expect("the same one");

        assert_eq!(first, expected, "the first answer is the reduction");
        assert_eq!(second, expected, "and so is the memoised one");
        assert!(
            Arc::ptr_eq(&first.data, &second.data),
            "the second ask was answered from the memo rather than reduced again"
        );
    }

    /// Two placements deep enough to reduce differently are two entries, not one served twice.
    ///
    /// This is the failure a key missing its factors would have: a valid reduction of the wrong
    /// depth draws the page at the wrong resolution and nothing downstream can tell.
    #[test]
    fn two_reductions_of_one_image_are_two_answers() {
        let source = image(64);
        let memo = ReducedImages::default();

        let deep = memo.reduced(&source, drawn_at(8.0)).expect("8x reduction");
        let shallow = memo.reduced(&source, drawn_at(16.0)).expect("4x reduction");

        assert_ne!(deep.width, shallow.width, "different grids");
        assert_eq!(
            shallow,
            source
                .area_averaged(drawn_at(16.0))
                .expect("four source samples per device pixel is a reduction"),
            "the second placement gets its own reduction rather than the first's"
        );
    }

    /// A placement that gathers nothing is not memoised and says so.
    #[test]
    fn a_placement_that_reduces_nothing_answers_none() {
        let source = image(64);
        let memo = ReducedImages::default();
        assert!(memo.reduced(&source, drawn_at(64.0)).is_none());
        assert!(memo.reduced(&source, drawn_at(128.0)).is_none());
    }

    /// The budget bounds what is held, the newest entry survives it, and eviction costs only
    /// the work.
    ///
    /// Driven by a budget of one byte rather than by the shipped one, because what has to be
    /// exercised is the eviction and a document reaching 32 MB of reduced samples is not a unit
    /// test. Three claims, each with its own defect: the memo stays inside the bound; the entry
    /// the caller is about to draw with is never the one dropped; and an evicted entry is
    /// reduced again to the same bytes, which is what makes the bound free of consequences.
    #[test]
    fn eviction_bounds_the_memo_keeps_the_newest_and_costs_only_the_work() {
        let memo = ReducedImages {
            held: std::sync::Mutex::default(),
            budget: 1,
        };
        let sources: Vec<_> = (0..3).map(|_| image(64)).collect();
        let placement = drawn_at(8.0);
        let first = memo.reduced(&sources[0], placement).expect("a reduction");
        for source in &sources[1..] {
            let _ = memo.reduced(source, placement);
        }

        {
            let held = memo.held.lock().expect("no thread panicked holding it");
            assert_eq!(held.entries.len(), 1, "only the newest entry survives");
            assert_eq!(held.entries.len(), held.order.len(), "the two agree");
        }

        let again = memo
            .reduced(&sources[0], placement)
            .expect("an evicted entry is reduced again");
        assert_eq!(again, first, "and reduced to the same bytes");
        assert!(
            !Arc::ptr_eq(&again.data, &first.data),
            "which is a fresh reduction rather than the evicted buffer"
        );
    }

    /// Every strip of one page gets the same bytes, whichever of them ran the reduction.
    #[test]
    fn concurrent_askers_agree() {
        let source = Arc::new(image(256));
        let memo = Arc::new(ReducedImages::default());
        let placement = drawn_at(16.0);
        let expected = source.area_averaged(placement).expect("a reduction");

        let answers: Vec<_> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    let memo = Arc::clone(&memo);
                    let source = Arc::clone(&source);
                    scope.spawn(move || memo.reduced(&source, placement))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("no thread panicked"))
                .collect()
        });

        for answer in answers {
            assert_eq!(answer.as_ref(), Some(&expected));
        }
    }
}
