//! A residue meet's bytes kept from one render for the next on the same device (ADR 1517).
//!
//! ISO 32000-2 §10.7.4 makes a clipped mark's coverage the intersection of two sets of
//! pixels, and [`super::Encoder::meet_residue`] computes it from both sets' edges. What it
//! computes is a function of what the scene states and nothing else: the mark's own tile and
//! polylines, its rule, each residue link's outline under its device transform and rule, and
//! the frame's visible rectangle. A key that holds all of those names the met tile exactly.
//!
//! **Why across renders and not within one.** A four-component group compositing in a space of
//! its own is drawn as two frames of the same elements, a chromatic one and a black one (ADR
//! 1471), and the second asks every pixel the first asked: on `bug1721218_reduced.pdf` 10 879
//! asked pixels in each of the two, against 1 558 asked twice inside one. So the meets one
//! render made are kept until the end of the next.
//!
//! **A key is the input itself, compared word for word**, so a hit is the same input and
//! never a hash's guess. The mark's part is its tile and polylines. The chain's part would be
//! every link's segments, which a meet cannot afford to compare per mark — a page's clip holds
//! a hundred thousand points — so a chain's content is named once a render by a number
//! ([`KeptMeets::chain`]): the same content, compared whole, keeps the number it had; new
//! content takes one never handed out before. The segments and not the outline's id, because
//! the two frames of one group upload their clips once each, under two ids.
//!
//! **Only a meet whose answer no budget decided is kept.** A chain's edges are kept for the
//! frame where its edge budget allows and remade over the meet's own rows where it does not
//! (ADR 1467), and only the remade build is bounded per meet: a chain whose rows pass that
//! bound meets exactly when its edges were kept and by `min` when they were not. Such a meet is
//! not kept, so a hit hands back what any render would have computed, whatever it had spent.
//!
//! **Two more answers are kept beside the meets, for the same pair of renders** (ADR 1529). A
//! mark's whole coverage tile — the fill of its polylines over its tile and the meet with its
//! chain — is a function of the tile, the rule, the polylines and the chain's number, so it is
//! kept under exactly those words and the second render neither fills nor meets it. And a
//! chain's region, which [`super::super::residue`] fills once a frame, is a function of the
//! chain's content alone (the content's words end with the frame's visible rectangle, which
//! bounds the region), so it is kept under the chain's number with the price its admission
//! was asked at; the second render asks the same admission and is handed the same bytes.
//!
//! What is held is bounded by a sixteenth of the frame budget per render; past it an answer
//! is not kept and the next render computes it again, which costs time and changes no byte.

use std::sync::Arc;

use crate::encode::residue::Fill;
use crate::keyhash::FastMap;
use crate::raster::CoverageMask;

/// The share of the frame budget one render's kept meets may hold: a sixteenth, so that the
/// two renders held at once stay under an eighth of what one frame may spend.
const KEPT_BUDGET_SHARE: u64 = 16;

/// What an entry costs beside its words and bytes: the map's slot and the two boxes' headers.
const ENTRY_OVERHEAD: u64 = 64;

/// A chain's region as one render priced and filled it (ADR 1529).
#[derive(Debug, Clone)]
pub(in crate::encode) struct KeptRegion {
    /// The fill [`super::super::residue::ResidueRegions::admit`] was asked to price.
    pub(in crate::encode) priced: Fill,
    /// The region itself; `None` is a chain whose links leave no region, which is a region.
    pub(in crate::encode) mask: Option<Arc<CoverageMask>>,
}

/// The met tiles, whole coverage tiles and chain regions of this render and of the one before
/// it, and the chains they were made under.
#[derive(Debug, Default)]
pub(crate) struct KeptMeets {
    current: FastMap<Box<[u32]>, Arc<[u8]>>,
    previous: FastMap<Box<[u32]>, Arc<[u8]>>,
    /// Whole coverage tiles, filled and met, by [`super::super::Encoder::coverage_tile`]'s words.
    tiles: FastMap<Box<[u32]>, Arc<[u8]>>,
    previous_tiles: FastMap<Box<[u32]>, Arc<[u8]>>,
    /// Chain regions by the number their chain's content was named by.
    regions: FastMap<u64, KeptRegion>,
    previous_regions: FastMap<u64, KeptRegion>,
    /// Each chain's content, as this render and the one before named it.
    chains: FastMap<Box<[u32]>, u64>,
    previous_chains: FastMap<Box<[u32]>, u64>,
    /// This render's chains by their leaf clip, which names a chain within one scene.
    by_leaf: FastMap<u32, u64>,
    /// The next number never handed out; `0` is the chain of no residue.
    next_chain: u64,
    budget: u64,
    spent: u64,
}

impl KeptMeets {
    /// Begin a render whose frame may spend `frame_budget_bytes`: the meets and chains the
    /// render before kept stay readable, and the ones before that are let go.
    pub(crate) fn begin_render(&mut self, frame_budget_bytes: u64) {
        self.previous = std::mem::take(&mut self.current);
        self.previous_tiles = std::mem::take(&mut self.tiles);
        self.previous_regions = std::mem::take(&mut self.regions);
        self.previous_chains = std::mem::take(&mut self.chains);
        self.by_leaf.clear();
        self.budget = frame_budget_bytes / KEPT_BUDGET_SHARE;
        self.spent = 0;
    }

    /// The number this render named the chain whose leaf clip is `leaf` by, if it has.
    pub(super) fn chain_of(&self, leaf: u32) -> Option<u64> {
        self.by_leaf.get(&leaf).copied()
    }

    /// Name the chain whose leaf clip is `leaf` and whose residue is `content`
    /// (`Encoder::residue_content`): the number the same content had in this render or the
    /// one before, or one never handed out before.
    ///
    /// A number that was handed out names one content and no other, so a meet kept under it
    /// is a meet under that content. Content past the budget is named all the same and not
    /// kept, so the next render names it afresh and finds none of this render's meets.
    pub(super) fn chain(&mut self, leaf: u32, content: Vec<u32>) -> u64 {
        let found = self.chains.get(content.as_slice()).copied();
        let earlier = found.or_else(|| self.previous_chains.get(content.as_slice()).copied());
        let number = earlier.unwrap_or_else(|| {
            self.next_chain = self.next_chain.saturating_add(1);
            self.next_chain
        });
        if found.is_none() {
            let words = u64::try_from(content.len()).unwrap_or(u64::MAX);
            let cost = words.saturating_mul(4).saturating_add(ENTRY_OVERHEAD);
            if self.spent.saturating_add(cost) <= self.budget {
                self.spent = self.spent.saturating_add(cost);
                self.chains.insert(content.into_boxed_slice(), number);
            }
        }
        self.by_leaf.insert(leaf, number);
        number
    }

    /// The met tile kept under `key`, by this render or the one before.
    ///
    /// A hit from the render before is kept again for this one where the budget allows, so a
    /// run of renders of one page keeps meeting from the first.
    pub(super) fn find(&mut self, key: &[u32]) -> Option<Arc<[u8]>> {
        if let Some(met) = self.current.get(key) {
            return Some(Arc::clone(met));
        }
        let met = Arc::clone(self.previous.get(key)?);
        self.keep(key.into(), &met);
        Some(met)
    }

    /// Keep `met` under `key` where what is left of this render's budget holds it. A key this
    /// render already holds names the same bytes, and is not charged again: a meet asked twice
    /// before its first asking settled is kept twice (ADR 1541).
    pub(super) fn keep(&mut self, key: Box<[u32]>, met: &Arc<[u8]>) {
        if !self.current.contains_key(&key) && self.afford(key.len(), met.len()) {
            self.current.insert(key, Arc::clone(met));
        }
    }

    /// The whole coverage tile kept under `key`, by this render or the one before; a hit from
    /// the render before is kept again for this one, as [`KeptMeets::find`] keeps a meet.
    pub(in crate::encode) fn find_tile(&mut self, key: &[u32]) -> Option<Arc<[u8]>> {
        if let Some(tile) = self.tiles.get(key) {
            return Some(Arc::clone(tile));
        }
        let tile = Arc::clone(self.previous_tiles.get(key)?);
        self.keep_tile(key.into(), &tile);
        Some(tile)
    }

    /// Keep a whole coverage tile under `key` where what is left of this render's budget
    /// holds it; a key this render already holds is not charged again, as [`KeptMeets::keep`]
    /// says.
    pub(in crate::encode) fn keep_tile(&mut self, key: Box<[u32]>, tile: &Arc<[u8]>) {
        if !self.tiles.contains_key(&key) && self.afford(key.len(), tile.len()) {
            self.tiles.insert(key, Arc::clone(tile));
        }
    }

    /// The region kept for the chain named `chain`, by this render or the one before; a hit
    /// from the render before is kept again for this one.
    pub(in crate::encode) fn region(&mut self, chain: u64) -> Option<KeptRegion> {
        if let Some(kept) = self.regions.get(&chain) {
            return Some(kept.clone());
        }
        let kept = self.previous_regions.get(&chain)?.clone();
        self.keep_region(chain, kept.clone());
        Some(kept)
    }

    /// Keep a chain's region under its number where what is left of this render's budget
    /// holds it. The chain of no residue, `0`, has no region and is never kept.
    pub(in crate::encode) fn keep_region(&mut self, chain: u64, kept: KeptRegion) {
        let bytes = kept.mask.as_ref().map_or(0, |mask| mask.coverage.len());
        if chain != 0 && self.afford(2, bytes) {
            self.regions.insert(chain, kept);
        }
    }

    /// Charge an entry of `words` key words and `bytes` held bytes to this render's budget,
    /// answering whether it fits; nothing is charged for an entry that does not.
    fn afford(&mut self, words: usize, bytes: usize) -> bool {
        let words = u64::try_from(words).unwrap_or(u64::MAX);
        let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        let cost = words
            .saturating_mul(4)
            .saturating_add(bytes)
            .saturating_add(ENTRY_OVERHEAD);
        let fits = self.spent.saturating_add(cost) <= self.budget;
        if fits {
            self.spent = self.spent.saturating_add(cost);
        }
        fits
    }
}

#[cfg(test)]
mod tests {
    use super::KeptMeets;
    use std::sync::Arc;

    fn met(bytes: &[u8]) -> Arc<[u8]> {
        Arc::from(bytes)
    }

    /// A meet kept by one render is found by the next and by no render after that unless
    /// the next found it again.
    #[test]
    fn a_meet_is_kept_for_the_next_render() {
        let mut kept = KeptMeets::default();
        kept.begin_render(1 << 20);
        let key: Box<[u32]> = vec![1, 2, 3].into();
        kept.keep(key.clone(), &met(&[7, 8]));
        assert_eq!(kept.find(&key).as_deref(), Some(&[7_u8, 8][..]));
        kept.begin_render(1 << 20);
        assert_eq!(kept.find(&key).as_deref(), Some(&[7_u8, 8][..]));
        kept.begin_render(1 << 20);
        assert!(kept.find(&key).is_some(), "found again, so carried on");
        kept.begin_render(1 << 20);
        kept.begin_render(1 << 20);
        assert!(kept.find(&key).is_none(), "two renders that never asked");
        assert!(
            kept.find(&[1, 2]).is_none(),
            "a key is its words, all of them"
        );
    }

    /// The same content keeps its number from one render to the next whatever leaf names it;
    /// other content takes a number never handed out.
    #[test]
    fn a_chain_is_named_by_its_content() {
        let mut kept = KeptMeets::default();
        kept.begin_render(1 << 20);
        let first = kept.chain(7, vec![1, 2, 3]);
        assert_eq!(kept.chain_of(7), Some(first));
        let other = kept.chain(8, vec![1, 2, 4]);
        assert_ne!(first, other);
        kept.begin_render(1 << 20);
        assert_eq!(
            kept.chain_of(7),
            None,
            "a leaf names a chain within one scene"
        );
        assert_eq!(kept.chain(40, vec![1, 2, 3]), first);
        kept.begin_render(1 << 20);
        kept.begin_render(1 << 20);
        let again = kept.chain(7, vec![1, 2, 3]);
        assert!(
            again != first && again != other,
            "a number is never handed out twice"
        );
    }

    /// A key kept twice in one render — a meet asked again before its first asking settled
    /// (ADR 1541) — is charged once: the budget that holds one entry still holds another.
    #[test]
    fn a_key_kept_twice_is_charged_once() {
        let mut kept = KeptMeets::default();
        // One entry of 8 key words and 8 bytes costs 32 + 8 + 64 = 104; the budget holds two.
        kept.begin_render(16 * 208);
        let key: Box<[u32]> = vec![1; 8].into();
        kept.keep(key.clone(), &met(&[1; 8]));
        kept.keep(key, &met(&[1; 8]));
        let other: Box<[u32]> = vec![2; 8].into();
        kept.keep(other.clone(), &met(&[2; 8]));
        assert!(
            kept.find(&other).is_some(),
            "the second key fits beside the first"
        );
        let tile: Box<[u32]> = vec![3; 8].into();
        kept.begin_render(16 * 208);
        kept.keep_tile(tile.clone(), &met(&[3; 8]));
        kept.keep_tile(tile, &met(&[3; 8]));
        kept.keep_tile(vec![4; 8].into(), &met(&[4; 8]));
        assert!(
            kept.find_tile(&[4; 8]).is_some(),
            "and so does a second tile"
        );
    }

    /// Past the budget a meet is not kept, and the next render computes it again.
    #[test]
    fn a_meet_past_the_budget_is_not_kept() {
        let mut kept = KeptMeets::default();
        kept.begin_render(16 * 80);
        let key: Box<[u32]> = vec![1; 8].into();
        kept.keep(key.clone(), &met(&[1; 8]));
        assert!(kept.find(&key).is_none(), "32 + 8 + 64 bytes is over 80");
    }
}
