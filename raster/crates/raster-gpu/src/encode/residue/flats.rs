//! A residue link's flattening, made once per frame for every chain and every mark that
//! names the same outline under the same transform (ADR 1479).
//!
//! A link's device-space polylines are a function of three things: the outline's segments,
//! the clip's transform, and the frame's viewport. Within one frame the viewport is fixed,
//! so `(outline, transform bits)` names the flattening exactly — the same small key
//! [`super::super::hull::HullMemo`] answers a hull by, and the identity rather than the
//! content of the polylines, so a lookup hashes seven words and never a point. The rule is
//! not in the key: it decides what a winding means, not where an edge runs.
//!
//! A flattening asked for again also gets its edges listed by row ([`RowIndex`]), so that the
//! many small tiles a large clip is met over each read the few edges that reach them.
//!
//! **A memo changes what a frame spends, never what it draws.** A hit hands back the very
//! polylines a miss would have made — `raster::flatten` is deterministic (ADR 0008) — so
//! every rasterisation and every meet downstream reads the same edges in the same order.
//! What the memo may hold is a budget of its own, beside and equal to the regions' (so
//! keeping a flattening never changes which regions are admitted); a link past it is
//! flattened again at each use, which is what every use did before.

use std::sync::Arc;

use raster_scene::{Affine, OutlineId};

use crate::keyhash::FastMap;
use crate::raster::{Polyline, RowIndex};

/// One link's flattening as the frame keeps it: the polylines, and where the budget allowed,
/// their edges listed by row ([`RowIndex`]) so a tile reads the edges that reach it.
#[derive(Debug, Clone)]
pub(in crate::encode) struct Flat {
    pub(in crate::encode) polylines: Arc<[Polyline]>,
    pub(in crate::encode) index: Option<Arc<RowIndex>>,
}

/// What one flattening is filed under: the outline and its transform's bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::encode) struct LinkKey {
    outline: u32,
    transform: [u32; 6],
}

impl LinkKey {
    /// The key of `outline` placed by `transform` in this frame.
    pub(in crate::encode) fn of(outline: OutlineId, transform: Affine) -> Self {
        let Affine { a, b, c, d, e, f } = transform;
        Self {
            outline: outline.0,
            transform: [a, b, c, d, e, f].map(f32::to_bits),
        }
    }
}

/// One kept flattening, and whether its row index has been asked for.
#[derive(Debug)]
struct Held {
    flat: Flat,
    indexed: bool,
}

/// The frame's residue-link flattenings, under their own budget.
#[derive(Debug)]
pub(in crate::encode) struct LinkFlats {
    held: FastMap<LinkKey, Held>,
    budget: u64,
    spent: u64,
    /// Distinct flattenings kept: keys, not lookups (CLAUDE.md).
    pub(in crate::encode) kept: u32,
}

impl LinkFlats {
    /// An empty memo that may hold `budget` bytes of points.
    pub(in crate::encode) fn new(budget: u64) -> Self {
        Self {
            held: FastMap::default(),
            budget,
            spent: 0,
            kept: 0,
        }
    }

    /// The flattening filed under `key`, if this frame kept one.
    ///
    /// **Its row index is built the first time it is asked for again**, not when it is
    /// kept: a link used once — the common clip — never pays for listing its edges, and one
    /// used a second time is the case the index is for (trap 73: a cache is priced by how
    /// often it is read). Where what is left of the budget will not hold the index, the
    /// flattening is handed back without one.
    pub(in crate::encode) fn get(&mut self, key: LinkKey) -> Option<Flat> {
        let held = self.held.get_mut(&key)?;
        if !held.indexed {
            held.indexed = true;
            // An entry is eight bytes; the index is charged what it holds once built.
            let room =
                usize::try_from(self.budget.saturating_sub(self.spent) / 8).unwrap_or(usize::MAX);
            let spent = &mut self.spent;
            let budget = self.budget;
            held.flat.index = RowIndex::of(&held.flat.polylines, room).and_then(|index| {
                let bytes = index.bytes();
                (spent.saturating_add(bytes) <= budget).then(|| {
                    *spent = spent.saturating_add(bytes);
                    Arc::new(index)
                })
            });
        }
        Some(held.flat.clone())
    }

    /// Keep `polylines` under `key` where they fit what is left of the budget, and hand them
    /// back either way.
    pub(in crate::encode) fn keep(&mut self, key: LinkKey, polylines: Vec<Polyline>) -> Flat {
        let flat = Flat {
            polylines: Arc::from(polylines),
            index: None,
        };
        let bytes = held_bytes(&flat.polylines);
        if self.spent.saturating_add(bytes) <= self.budget {
            self.spent = self.spent.saturating_add(bytes);
            self.kept = self.kept.saturating_add(1);
            self.held.insert(
                key,
                Held {
                    flat: flat.clone(),
                    indexed: false,
                },
            );
        }
        flat
    }
}

/// The bytes a flattening holds: its points, at two `f32`s each.
fn held_bytes(polylines: &[Polyline]) -> u64 {
    let points: usize = polylines.iter().map(|p| p.points.len()).sum();
    u64::try_from(points).unwrap_or(u64::MAX).saturating_mul(8)
}

#[cfg(test)]
mod tests {
    use super::{LinkFlats, LinkKey};
    use crate::raster::Polyline;
    use raster_scene::{Affine, OutlineId, Point};

    fn square() -> Vec<Polyline> {
        vec![Polyline::polygon(vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
        ])]
    }

    /// The key is the outline and the transform's bits: a different translation is a
    /// different flattening, the same one is the same.
    #[test]
    fn a_flattening_is_filed_by_outline_and_transform() {
        let mut flats = LinkFlats::new(u64::MAX);
        let key = LinkKey::of(OutlineId(4), Affine::IDENTITY);
        assert!(flats.get(key).is_none());
        let kept = flats.keep(key, square());
        assert!(kept.index.is_none(), "a first use lists no rows");
        let again = flats.get(key).expect("kept under its key");
        assert!(std::sync::Arc::ptr_eq(&kept.polylines, &again.polylines));
        assert!(
            again.index.is_some(),
            "a second use lists its rows, which fit an unbounded budget"
        );
        assert!(
            flats
                .get(LinkKey::of(OutlineId(4), Affine::translate(1.0, 0.0)))
                .is_none()
        );
        assert!(
            flats
                .get(LinkKey::of(OutlineId(5), Affine::IDENTITY))
                .is_none()
        );
        assert_eq!(flats.kept, 1);
    }

    /// Past the budget the flattening is handed back and not kept: the next use makes it
    /// again, which costs time and changes no edge.
    #[test]
    fn a_flattening_past_the_budget_is_not_kept() {
        let mut flats = LinkFlats::new(8);
        let key = LinkKey::of(OutlineId(4), Affine::IDENTITY);
        let handed = flats.keep(key, square());
        assert_eq!(handed.polylines.len(), 1);
        assert!(handed.index.is_none());
        assert!(flats.get(key).is_none(), "24 bytes of points is over 8");
        assert_eq!(flats.kept, 0);
    }
}
