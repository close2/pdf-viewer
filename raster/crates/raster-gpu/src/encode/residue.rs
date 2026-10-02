//! What one frame keeps of a clip chain's residue: the region it occupies, rasterised
//! once, and the rule that decides whether keeping it is worth anything (ADR 0049).
//!
//! ISO 32000-2 §8.5.4 makes a chain **one region** (ADR 0030), and a region does not
//! depend on which mark is asking about it. Until ADR 0049 the encoder rasterised the
//! chain again for every clipped command, over that command's own tile: the artwork
//! archetype's 185 chains were rasterised 600 times, 3.6 million pixels of coverage for
//! 1.25 million distinct ones.
//!
//! # The rule: a region must not cost more than the tiles it replaces
//!
//! A cache is worth what it is *used* for, which is ADR 0029's finding about the glyph
//! atlas arriving here with a different unit. A chain's region is its links' device
//! bounds intersected — which can be the whole page — while the tile a command asks for
//! is that command's own mark. So a page-sized clip over forty small marks would pay two
//! million pixels to save nine thousand, and that is not a cache, it is a bomb with a
//! cache's name on it.
//!
//! **What a fill costs is its bytes and the edges in its rows** (ADR 1491). A region pays
//! every byte it holds and every row piece of every edge; a tile pays its own bytes and the
//! row pieces of the edges in its rows — about the chain's pieces in proportion to the rows,
//! since a tile reads every edge its rows hold whatever their columns. Counting bytes alone
//! priced a declined tile at four bytes when it filled sixteen hundred row pieces, and so
//! declined the one region `bug1721218_reduced.pdf` asks for three thousand times. A piece is
//! [`PIECE_BYTES`] bytes' worth of work, measured.
//!
//! [`ResidueRegions::admit`] is the whole of the decision, and both halves of it are
//! checked **before anything is allocated**:
//!
//! - **the region must not cost more than the tiles** — `region + k·pieces ≤ uses × (tile +
//!   k·pieces·tile_rows / region_rows)`, where `uses` is counted from the scene
//!   ([`ResidueRegions::of`]), `tile` is the tile the first command asked for and `k` is
//!   [`PIECE_BYTES`]. Conservative in the direction that matters: a tile also pays a
//!   flattening and its rows' topology, which the right side leaves out, so an admitted
//!   region is cheaper than the comparison says. **Which side of the rule a chain falls on
//!   moves no byte**: a clip's residue is the same coverage over a region and over a tile
//!   (ADR 1491), so this is a question of cost alone.
//! - **the frame's regions must fit their budget** — a quarter of the caller's stated
//!   `Options::max_frame_bytes` ([`budget`]).
//!
//! # Why exceeding the budget is not a refusal
//!
//! Principle 6 says a frame is drawn or refused. A frame that could be drawn and is
//! refused because a *cache* filled up is the worse half of that trade: there is a
//! fallback here that costs only time — rasterising the chain per tile, which is what
//! every frame did before this module existed — and the atlas already sets the
//! precedent, since a tile it will not admit goes down another lane rather than failing
//! the frame (ADR 0029). What the budget does is refuse the **allocation**, before it
//! happens, which is what principle 3 asks of it.
//!
//! The counters say which happened, and they count keys rather than lookups
//! ([`crate::frame::Counters::clip_residue_regions`]).

use std::sync::Arc;

use raster_scene::{Command, Scene};

use crate::keyhash::FastMap;
use crate::raster::{CoverageMask, RowEdges};

mod flats;

pub(super) use flats::{LinkFlats, LinkKey};

/// The share of the frame budget one frame's residue regions may hold: a quarter.
///
/// Derived from the caller's own `Options::max_frame_bytes` rather than stated as a
/// constant of its own, because a caller who lowers that number is describing a machine
/// rather than a lane, and a budget nobody can reach from the API is a budget nobody can
/// plan against (brief section 5). At the 268 MiB default this is 67 MB, which holds two full-page
/// regions at 4× magnification.
const RESIDUE_BUDGET_SHARE: u64 = 4;

/// The cost of filling one row piece of an edge, in bytes of coverage filled at the same cost.
///
/// Measured on the fill this module chooses between: a 1000 × 1000 rectangle over its own
/// region costs 6.4–6.6 ns a byte, and a 100 000-gon over a 210 × 210 region 103–143 ns a row
/// piece once its bytes are taken away — sixteen to twenty-two bytes. The lower figure, so
/// that the rule leans towards declining, which is what every frame did before ADR 1491.
const PIECE_BYTES: u64 = 16;

/// What one frame's residue regions may hold, given the frame's own budget.
pub(super) fn budget(frame_budget_bytes: u64) -> u64 {
    frame_budget_bytes.saturating_div(RESIDUE_BUDGET_SHARE)
}

/// What one fill of a chain's residue would cover, for [`ResidueRegions::admit`]'s price.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Fill {
    /// The coverage bytes it fills.
    pub(super) bytes: u64,
    /// The device rows it spans.
    pub(super) rows: u64,
    /// The row pieces of every edge of the chain inside those rows — the region's; zero on a
    /// tile, whose share the rule takes in proportion to its rows.
    pub(super) pieces: u64,
}

impl Fill {
    /// A fill of `bytes` over `rows` rows, holding `pieces` row pieces.
    pub(super) fn new(bytes: u64, rows: u64, pieces: u64) -> Self {
        Self {
            bytes,
            rows,
            pieces,
        }
    }
}

/// What the frame has decided about one chain, as a caller of
/// [`ResidueRegions::verdict`] sees it.
pub(super) enum Verdict<'a> {
    /// A region is held: `None` inside is the empty region, whose every window is
    /// transparent.
    Region(Option<&'a CoverageMask>),
    /// Decided against a region; this chain rasterises over the asking tile.
    PerTile,
    /// Not yet seen this frame.
    Undecided,
}

/// What the frame has decided about one chain's edges ([`ResidueRegions::edges`]).
pub(super) enum Edges {
    /// Built and kept: every link's edges, bucketed by row over the chain's region.
    Kept(Arc<[RowEdges]>),
    /// Past the frame's edge budget, or no region: the chain meets by `min`.
    Declined,
    /// No meet has asked yet.
    Unasked,
}

/// What this frame decided about one chain's region.
enum Held {
    /// Rasterised once over the chain's own region. `None` is a chain whose links do not
    /// overlap: every crop of it is transparent, and that is a legitimate region rather
    /// than an absent one.
    Region(Option<CoverageMask>),
    /// The chain pays per tile, as every chain did before ADR 0049 — because its region
    /// would cost more than the tiles it would replace, or because the frame's regions
    /// have reached their budget.
    PerTile,
}

/// The frame's residue regions, and the scene's count of what will ask for them.
pub(super) struct ResidueRegions {
    held: FastMap<u32, Held>,
    /// How many commands each clip id clips, propagated to its ancestors — see
    /// [`ResidueRegions::of`].
    uses: Vec<u32>,
    budget: u64,
    spent: u64,
    /// Distinct chains whose region was rasterised: the count of keys, which is the only
    /// honest thing to instrument a cache with (CLAUDE.md).
    pub(super) regions: u32,
    /// Residue rasterisations charged to one command's tile — the work this module did
    /// not remove.
    pub(super) tiles: u32,
    /// Each chain's links as the exact meet reads them (ADR 1467), built the first time a
    /// mark under the chain has a pixel both sets cut; `None` is a chain whose edges would
    /// pass what is left of [`ResidueRegions::edge_budget`], which meets by `min`.
    edges: FastMap<u32, Option<Arc<[RowEdges]>>>,
    /// What the frame's chain edges may hold, beside and equal to the regions' own budget:
    /// a separate account, so that keeping edges never changes which regions are admitted.
    edge_budget: u64,
    edges_spent: u64,
    /// Each residue link's flattening, filed by its outline and transform and shared by
    /// every chain and every use that names it (ADR 1479).
    pub(super) flats: LinkFlats,
}

impl ResidueRegions {
    /// Count what will ask for each chain, before the walk that asks.
    ///
    /// Two passes and neither touches a resource, so this can never refuse a scene the
    /// walk would have drawn: a clip whose outline is missing is still discovered where
    /// it always was, by the command that uses it.
    ///
    /// **The count is an upper bound on the uses of a region**, in the same deliberate
    /// direction as ADR 0029's census. A command's chain is keyed by its deepest
    /// non-rectangular link, so a command under a clip that adds a *second* residue link
    /// below this one is counted here and will ask for a different region. That
    /// over-counts only where residue clips nest, admits a region that may be used fewer
    /// times than the estimate, and is bounded by the budget above; under-counting would
    /// take the region away from the page this exists for.
    pub(super) fn of(scene: &Scene, budget: u64) -> Self {
        let mut uses = vec![0_u32; scene.clips().len()];
        count(scene.commands(), &mut uses);
        for mask in scene.masks() {
            count(&mask.commands, &mut uses);
        }
        // A parent's id is always smaller than its child's (`SceneBuilder::clip`), so one
        // descending pass carries every descendant's count into its ancestors.
        for id in (0..uses.len()).rev() {
            if let Some(parent) = scene.clips()[id].parent {
                let inherited = uses[id];
                let slot = &mut uses[parent.0 as usize];
                *slot = slot.saturating_add(inherited);
            }
        }
        Self {
            held: FastMap::default(),
            uses,
            budget,
            spent: 0,
            regions: 0,
            tiles: 0,
            edges: FastMap::default(),
            edge_budget: budget,
            edges_spent: 0,
            flats: LinkFlats::new(budget),
        }
    }

    /// The chain's edges, as decided the first time a meet asked.
    pub(super) fn edges(&self, key: u32) -> Edges {
        match self.edges.get(&key) {
            Some(Some(kept)) => Edges::Kept(Arc::clone(kept)),
            Some(None) => Edges::Declined,
            None => Edges::Unasked,
        }
    }

    /// What the chains' edges may still take, in bytes.
    pub(super) fn edge_room(&self) -> u64 {
        self.edge_budget.saturating_sub(self.edges_spent)
    }

    /// Keep a chain's edges, or remember that they were declined; charged once, in the
    /// order meets ask, which is the walk's order whatever the thread count.
    pub(super) fn keep_edges(&mut self, key: u32, edges: Option<Arc<[RowEdges]>>) {
        if let Some(kept) = &edges {
            let bytes = kept.iter().map(RowEdges::bytes).sum::<u64>();
            self.edges_spent = self.edges_spent.saturating_add(bytes);
        }
        self.edges.insert(key, edges);
    }

    /// What this frame has already decided about this chain — one lookup, because two
    /// would be two hashes of the same key on the path a page of clips takes per command
    /// (the correction ADR 0029 carries for the atlas, applied here at birth).
    pub(super) fn verdict(&self, key: u32) -> Verdict<'_> {
        match self.held.get(&key) {
            Some(Held::Region(region)) => Verdict::Region(region.as_ref()),
            Some(Held::PerTile) => Verdict::PerTile,
            None => Verdict::Undecided,
        }
    }

    /// Decide, before anything is allocated, whether this chain's region is worth
    /// rasterising — and remember the answer, so that every command under one chain is
    /// served the same way and the decline is counted once rather than once per command.
    pub(super) fn admit(&mut self, key: u32, region: Fill, tile: Fill) -> bool {
        let uses = u64::from(self.uses.get(key as usize).copied().unwrap_or(0)).max(1);
        let pieces = region.pieces.saturating_mul(PIECE_BYTES);
        let region_cost = region.bytes.saturating_add(pieces);
        let tile_pieces = pieces
            .saturating_mul(tile.rows)
            .checked_div(region.rows)
            .unwrap_or(pieces);
        let tile_cost = tile.bytes.saturating_add(tile_pieces);
        let worth_it = region_cost <= uses.saturating_mul(tile_cost);
        let region_bytes = region.bytes;
        let fits = self.spent.saturating_add(region_bytes) <= self.budget;
        if worth_it && fits {
            self.spent = self.spent.saturating_add(region_bytes);
            true
        } else {
            self.held.insert(key, Held::PerTile);
            false
        }
    }

    /// Keep a region [`ResidueRegions::admit`] said yes to.
    pub(super) fn insert(&mut self, key: u32, region: Option<CoverageMask>) {
        self.held.insert(key, Held::Region(region));
        self.regions = self.regions.saturating_add(1);
    }

    /// Count one residue rasterisation that a single command's tile paid for.
    pub(super) fn note_tile(&mut self) {
        self.tiles = self.tiles.saturating_add(1);
    }
}

/// Every clip a command names, groups included — the mask walk is the caller's.
fn count(commands: &[Command], uses: &mut [u32]) {
    let note = |clip: Option<raster_scene::ClipId>, uses: &mut [u32]| {
        if let Some(id) = clip
            && let Some(slot) = uses.get_mut(id.0 as usize)
        {
            *slot = slot.saturating_add(1);
        }
    };
    for command in commands {
        match command {
            Command::Rect { clip, .. }
            | Command::Fill { clip, .. }
            | Command::Stroke { clip, .. }
            | Command::Image { clip, .. } => note(*clip, uses),
            Command::Group { spec, commands } => {
                note(spec.clip, uses);
                count(commands, uses);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Fill, Held, ResidueRegions, Verdict};
    use crate::raster::CoverageMask;
    use raster_scene::{
        Affine, BlendMode, Color, Compose, FillRule, OutlineId, Paint, Point, Scene, SceneBuilder,
        Segment,
    };

    /// A scene of `clipped` fills under one non-rectangular clip.
    fn scene_with(clipped: u32) -> Scene {
        let mut builder = SceneBuilder::new();
        let outline = OutlineId(3);
        let clip = builder
            .clip(outline, Affine::IDENTITY, FillRule::NonZero, None)
            .expect("a clip of an outline");
        for i in 0..u16::try_from(clipped).unwrap_or(u16::MAX) {
            builder
                .fill(
                    outline,
                    Affine::translate(f32::from(i) * 20.0, 0.0),
                    FillRule::NonZero,
                    Paint::Solid(Color::new(0.0, 0.0, 0.0, 1.0)),
                    Some(clip),
                    BlendMode::Normal,
                    Compose::SrcOver,
                    None,
                )
                .expect("a clipped fill");
        }
        let _ = Segment::MoveTo(Point::new(0.0, 0.0));
        builder.finish()
    }

    /// The count is of the commands that will ask, and it is what the rule divides by.
    #[test]
    fn a_chain_is_counted_once_per_command_that_clips_by_it() {
        let regions = ResidueRegions::of(&scene_with(7), u64::MAX);
        assert_eq!(regions.uses, vec![7]);
    }

    /// **A region that would cost more than the tiles it replaces is refused**, and the
    /// refusal is remembered rather than re-taken per command.
    #[test]
    fn a_region_costlier_than_its_tiles_is_not_kept() {
        let mut regions = ResidueRegions::of(&scene_with(40), u64::MAX);
        // A page-sized region against forty tiles of 224 pixels: the shape of a real
        // page's `q W n` around a paragraph, and the case this rule exists for.
        // Its curve crosses each of the page's rows twice.
        let page = Fill::new(1191 * 1684, 1684, 2 * 1684);
        assert!(!regions.admit(0, page, Fill::new(224, 14, 0)));
        assert!(matches!(regions.verdict(0), Verdict::PerTile));
        // The same region against forty tiles that are each a tenth of the page.
        let mut generous = ResidueRegions::of(&scene_with(40), u64::MAX);
        assert!(generous.admit(0, page, Fill::new(1191 * 168, 168, 0)));
    }

    /// **A small tile of a long chain pays the chain's edges in its rows** (ADR 1491): the
    /// shape of `bug1721218_reduced.pdf`'s clip — a 207 × 120 region of 111 677 edges, asked
    /// for by 3 025 tiles of two pixels — is declined by bytes alone and admitted once the
    /// tile's edges are priced.
    #[test]
    fn a_region_whose_tiles_pay_its_edges_is_kept() {
        let mut regions = ResidueRegions::of(&scene_with(3_025), u64::MAX);
        let region = Fill::new(207 * 120, 120, 111_677);
        assert!(regions.admit(0, region, Fill::new(4, 2, 0)));
        // With no edges, the rule is the bytes' rule it always was.
        let mut bare = ResidueRegions::of(&scene_with(3_025), u64::MAX);
        assert!(!bare.admit(0, Fill::new(207 * 120, 120, 0), Fill::new(4, 2, 0)));
    }

    /// **The budget is checked before the region exists**, and a frame whose regions
    /// reach it keeps drawing — per tile, as it did before this module.
    #[test]
    fn the_budget_declines_rather_than_refuses() {
        let mut regions = ResidueRegions::of(&scene_with(40), 4_000);
        let fill = Fill::new(3_000, 1, 0);
        assert!(regions.admit(0, fill, fill));
        assert!(!regions.admit(0, fill, fill), "3 000 more is over 4 000");
        assert!(matches!(regions.verdict(0), Verdict::PerTile));
        assert_eq!(
            regions.spent, 3_000,
            "nothing is charged for what was refused"
        );
    }

    /// A chain whose links do not overlap holds an empty region — which is a region, and
    /// not the absence of one: every crop of it is transparent.
    #[test]
    fn an_empty_region_is_held_as_a_region() {
        let mut regions = ResidueRegions::of(&scene_with(2), u64::MAX);
        regions.insert(0, None);
        assert!(matches!(regions.held.get(&0), Some(Held::Region(None))));
        assert!(matches!(regions.verdict(0), Verdict::Region(None)));
        assert_eq!(regions.regions, 1);
    }

    /// The keys are counted, not the lookups: two commands under one chain are one
    /// region, and the counter says one.
    #[test]
    fn the_counter_counts_regions_and_not_uses() {
        let mut regions = ResidueRegions::of(&scene_with(2), u64::MAX);
        let mask = CoverageMask {
            left: 0,
            top: 0,
            width: 1,
            height: 1,
            coverage: vec![255],
        };
        assert!(regions.admit(0, Fill::new(1, 1, 0), Fill::new(1, 1, 0)));
        regions.insert(0, Some(mask));
        assert!(matches!(regions.verdict(0), Verdict::Region(Some(_))));
        assert!(matches!(regions.verdict(0), Verdict::Region(Some(_))));
        assert_eq!(regions.regions, 1);
        assert_eq!(regions.tiles, 0);
    }
}
