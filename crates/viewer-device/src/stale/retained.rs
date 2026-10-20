//! The layer under the base: whole pages drawn small, kept newest first, and placed one by one
//! under the view being asked for (ADR 0443).

use pdf_render::{DisplayList, TargetSpec, Transform};

use super::{Depicts, PROXY_EDGE, Picture, Placed};

/// How many pages this host retains a low-resolution picture of, when nobody says otherwise.
///
/// **A default is a decision, so this one is chosen from a measurement rather than picked**, and
/// the measurement separates two costs that look like one. Drawing a page's picture is paid
/// **once per page shown**, whatever this number is — the render thread draws it while idle, and
/// the frame after it pays what a rebuilt scene costs instead of what a replay does:
///
/// | page | the proxy | the frame after it | that frame as a replay |
/// |---|---|---|---|
/// | `doc/ISO_32000-2_sponsored_EC3.pdf` p60 | 21.9 ms | 2.1 ms | 1.1 ms |
/// | the owner's witness, 58 029 commands | 411.0 ms | 82.0 ms | 1.9 ms |
///
/// **What *this* number costs is memory alone**, at 741 KB a page, and what it buys is not
/// drawing a page's picture a second time after an eviction — so a larger extent is strictly
/// cheaper in renders and strictly dearer in bytes. Eight is where the whole retained set, 5.9 MB
/// for A4, is still smaller than one of the two window textures a frame holds anyway: a budget
/// stated against something this program already spends rather than a round number. It covers the
/// gestures that actually land on a held page — `Left` and `Right` about where a person is, a
/// continuous column's rows, a `GoTo` back where they came from — and a `GoTo` into a thousand
/// pages is beyond any extent, which is why the curve is here rather than further out.
///
/// The price to know about is neither of the two above: **a view change arriving while a page's
/// picture is being drawn waits for it to finish**, up to 411 ms on that witness. The window
/// still answers at once, from the layers `crate::stale` is about; what is late is the real frame.
/// One page per idle turn is what bounds it (`crate::renderer`). `--proxy-pages 0` turns the
/// whole of it off.
pub const PROXY_PAGES: usize = 8;

/// A whole page, drawn once at [`PROXY_EDGE`], and what it is a picture of.
///
/// Generic in its pixels so that one policy serves both threads and the tests: the render thread
/// holds `Arc<wgpu::Texture>` and so does the presenter, and neither of them is what decides which
/// pages are kept.
#[derive(Debug)]
pub struct Retained<P> {
    /// Which page's picture it is, and of which ink — [`Picture`], and **not** the `Arc` this was
    /// keyed by until ADR 0457.
    pub of: Picture,
    /// Where the whole page sits in this picture: [`proxy_target`]'s answer, kept beside the
    /// pixels rather than recomputed, so that a change to that function cannot silently re-place
    /// a raster already drawn.
    pub target: TargetSpec,
    /// Whatever holds the pixels on the thread this lives on.
    pub pixels: P,
}

/// Where a whole page goes in its own retained picture, or `None` for a page with no extent.
///
/// The same `TargetSpec::for_page` the core builds a render request with (`viewer_core::Viewer`),
/// at a smaller scale — which is what makes the composition below exact rather than approximate:
/// both are affine maps out of the same page's user space, so `proxy⁻¹ ∘ asked` is the map from
/// this raster's texels to the window's pixels and nothing has to agree about anything else.
pub fn proxy_target(list: &DisplayList) -> Option<TargetSpec> {
    let size = list.page_size;
    let longest = size.width.max(size.height);
    if !longest.is_finite() || longest <= 0.0 {
        return None;
    }
    let scale = f64::from(PROXY_EDGE) / f64::from(longest);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a ratio of two page dimensions, far inside f32's range"
    )]
    let scale = scale as f32;
    // The budget is the square of the edge and one pixel: `TargetSpec::for_page` rounds each
    // dimension **up**, so that the raster contains the page rather than clipping the fraction of
    // a row at its edge (ADR 0064), and a page whose longer side divides exactly still lands one
    // pixel over when the ratio is not exact in `f32`. Stated here so that a page of an absurd
    // aspect ratio is refused by that function rather than allocated for.
    let edge = u64::from(PROXY_EDGE).saturating_add(1);
    TargetSpec::for_page(list, scale, edge.saturating_mul(edge)).ok()
}

/// Every page this window is holding a low-resolution picture of, newest first.
///
/// **The extent is the host's setting and could not be anything else** (`doc/todo/37`, ADR 0443).
/// A stand-in is a wrong picture, so rule 2 keeps everything that makes one inside a private
/// module of a binary: the count may not become a `Command`, a field of a boundary type or
/// anything a gate, an oracle or a harness could link to. It is a flag beside `--cpu` and
/// `--backend`, and `PROXY_PAGES` is what it defaults to.
///
/// **Newest first, oldest dropped, and that is a choice with a reason.** A proxy is produced only
/// for a page of the most recent frame, so the pages on the screen are always the newest entries
/// and the one evicted is never one the window is showing — provided the extent is at least the
/// number of pages Table 29's arrangement puts up, which is what `PROXY_PAGES` is sized for.
///
/// **And what is dropped outright is every picture of a superseded ink** (ADR 0457). The store is
/// bounded twice over: by [`Self::extent`] in pages, which is what a memory budget can be stated
/// against, and by [`Picture::same_ink`], which is not a budget at all — a picture of ink nobody
/// may show is not worth a slot, and keeping one would let it occupy a place a showable page
/// wants. Nothing else evicts, and nothing else needs to: an edit, an undo, a redo, a layer
/// switch and §6.3.2.2's delegated appearances all arrive here as one changed number.
#[derive(Debug)]
pub struct Proxies<P> {
    held: Vec<Retained<P>>,
    extent: usize,
}

impl<P> Proxies<P> {
    /// A store that will hold `extent` pages. Zero is the feature turned off.
    #[must_use]
    pub fn new(extent: usize) -> Self {
        Self {
            held: Vec::new(),
            extent,
        }
    }

    /// Keeps a page's picture, dropping any picture of that page, any picture of another ink, and
    /// then the oldest of whatever is left.
    ///
    /// The ink is checked here rather than at every read, because this is the one moment the store
    /// learns that the document has moved on: a picture arrives from a frame of the current ink,
    /// and everything that disagrees with it was drawn against a state that no longer exists.
    pub fn keep(&mut self, one: Retained<P>) {
        self.held
            .retain(|held| held.of != one.of && held.of.same_ink(one.of));
        self.held.insert(0, one);
        self.held.truncate(self.extent);
    }

    /// The first page of `pages` with no picture yet, which is what the render thread draws next.
    ///
    /// `None` where every one of them is held, and `None` for an extent of zero — a store that
    /// keeps nothing must not ask for anything, or the render thread would draw a proxy per idle
    /// turn and throw each one away.
    #[must_use]
    pub fn wanted(&self, pages: &[Placed]) -> Option<Placed> {
        if self.extent == 0 {
            return None;
        }
        // A transition frame is not a picture of its page at any placement (`Depicts`), so it is
        // never the page this layer keeps.
        pages
            .iter()
            .filter(|placed| placed.depicts == Depicts::Page)
            .find(|placed| !self.held.iter().any(|held| held.of == placed.of))
            .cloned()
    }

    /// Where each retained page goes so that it depicts the view being asked for.
    ///
    /// One placement per page that is in both this store and the arrangement, as an index into
    /// [`Self::get`] — **not one affine for the whole picture**, which is the difference between
    /// this layer and the base. The base is one texture of the window and so needs one placement
    /// true of every page in it ([`Refusal::Rearranged`]); these are one texture *per page*, so a
    /// zoom in a column places each of them correctly and the arrangement's gap never comes into
    /// it.
    ///
    /// A page whose composition is not a finite affine is left out rather than refused: the rest
    /// of the picture is still true, and there is no single answer here to spoil.
    ///
    /// [`Refusal::Rearranged`]: super::Refusal::Rearranged
    #[must_use]
    pub fn placements(&self, asked: &[Placed]) -> Vec<(usize, Transform)> {
        let mut placed = Vec::new();
        for (index, held) in self.held.iter().enumerate() {
            let Some(target) = asked
                .iter()
                .find(|placed| placed.of == held.of)
                .map(|placed| placed.target)
            else {
                continue;
            };
            let Some(moved) = held
                .target
                .transform
                .invert()
                .map(|back| back.then(target.transform))
            else {
                continue;
            };
            if [moved.a, moved.b, moved.c, moved.d, moved.e, moved.f]
                .iter()
                .all(|coefficient| coefficient.is_finite())
            {
                placed.push((index, moved));
            }
        }
        placed
    }

    /// The page at an index [`Self::placements`] handed back.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Retained<P>> {
        self.held.get(index)
    }

    /// How many pages are held, for the trace.
    #[must_use]
    #[expect(
        clippy::len_without_is_empty,
        reason = "a count for a trace line; nothing asks whether the store is empty"
    )]
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// How many this store will hold — the host's setting, carried to the render thread.
    #[must_use]
    pub fn extent(&self) -> usize {
        self.extent
    }
}
