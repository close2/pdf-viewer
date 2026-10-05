//! The exact areas of a frame's meets, made beside the walk rather than on its thread, and
//! written into the sheet where the walk packed each tile (ADR 1541).
//!
//! ISO 32000-2 §10.7.4 makes a clipped mark's coverage the intersection of two sets of pixels,
//! and where both sets cut a pixel [`super::exact_areas`] computes that intersection's area
//! from both sets' edges. On `bug1721218_reduced.pdf` that is 11 988 pixels a render, each cut
//! by about eighteen edges of two dots a pixel wide, and it was the largest serial cost of the
//! page's zoom step: 0.47 G instructions of the walk's thread, about 39 ms of its 136.
//!
//! **What a meet's exact pixels read is fixed when the walk reaches the mark**: the tile as
//! `min` left it, which pixels both sets cut, the chain's edges and the mark's polylines, rule
//! and edges. Nothing the walk does afterwards changes any of them, and nothing the walk does
//! afterwards reads the bytes they decide: the tile is packed onto the sheet and drawn from
//! it, and the sheet is read once, by the device, after the walk's `finish`. So the walk
//! records each such meet beside the place its tile was packed at and goes on; helper threads
//! make the areas meanwhile ([`super::Helpers`]), and the settle writes them where the walk put
//! each tile. A frame whose meets never reach [`EXACT_FLOOR_PIXELS`], or a host that allowed
//! one thread, starts no helper and makes the areas at the settle on the walk's thread.
//!
//! **The bytes are the bytes the walk would have written**, by construction: the same
//! function of the same inputs, written over the same `min`, whichever thread made them and in
//! whatever order. What moves is when the meet's result is kept for the next render
//! ([`super::KeptMeets`]): at the settle, in encounter order, rather than at the meet. A meet
//! asked again within one render before its first asking settled is computed again rather than
//! found, which costs a thread time and changes no byte, and the second keep finds the key
//! already held and charges nothing.
//!
//! **Held, then settled.** What the recorded meets hold — each tile, its cut, the mark's
//! polylines and edges, and the chain's edges where the meet made them for itself — is counted
//! against the queue's own limit (`parallel::in_flight_limit`), and past it they are settled
//! before the walk goes on, so a page of a million meets holds a batch of them and never all.
//! Settling is the same at every thread count, so a frame keeps the same answers whatever the
//! host allowed.

use std::sync::Arc;

use super::super::Encoder;
use super::helpers::Helpers;
use super::{Mark, MarkInputs, exact_areas};
use crate::raster::{CoverageMask, Polyline, RowEdges, Rule};

/// The fewest pixels a frame's recorded meets reach before helpers are started to make their
/// areas beside the walk; a frame below it makes them on the walk's thread when it settles.
///
/// A pixel cut by both sets costs between a few thousand instructions and tens of thousands
/// — 34.5 k on `bug1721218_reduced.pdf`, whose dots put eighteen edges through each — so 64
/// of them are a few hundred microseconds of one thread's work, against a spawn's tens of
/// microseconds (ADR 1541).
const EXACT_FLOOR_PIXELS: usize = 64;

/// A meet whose exact pixels are made when the frame settles them (ADR 1541): what its areas
/// are made from, and the words to keep the finished tile under.
#[derive(Debug)]
pub(in crate::encode) struct ExactMeet {
    inputs: Arc<ExactInputs>,
    /// The words [`Encoder::meet_words`] named this meet by, where it is kept.
    meet_words: Option<Box<[u32]>>,
    /// The words [`Encoder::tile_words`] named the whole tile by, where it is kept.
    tile_words: Option<Box<[u32]>>,
}

/// Everything a meet's exact areas are a function of: the tile as `min` left it, the pixels
/// both sets cut, the chain's edges and the mark's polylines, rule and edges. Owned, so that a
/// helper thread can make the areas while the walk goes on.
#[derive(Debug)]
pub(super) struct ExactInputs {
    tile: CoverageMask,
    cut: Vec<usize>,
    links: Arc<[RowEdges]>,
    polylines: Vec<Polyline>,
    rule: Rule,
    edges: Option<RowEdges>,
}

impl ExactInputs {
    /// The intersection's area in each cut pixel, as bytes, or `None` where the mark's edges
    /// pass their bound and the tile keeps `min` — [`exact_areas`] of what the walk recorded.
    pub(super) fn areas(&self) -> Option<Vec<u8>> {
        let mark = Mark {
            polylines: &self.polylines,
            rule: self.rule,
            edges: self.edges.as_ref(),
        };
        exact_areas(&self.tile, &self.cut, &self.links, mark)
    }
}

impl ExactMeet {
    /// A meet of `tile`, already met by `min`, whose `cut` pixels take the area of the
    /// intersection of the mark `inputs` describe with the chain whose edges are `links`;
    /// borrowed inputs are copied and owned ones taken.
    pub(super) fn new(
        tile: &CoverageMask,
        cut: Vec<usize>,
        links: Arc<[RowEdges]>,
        inputs: MarkInputs<'_>,
    ) -> Self {
        let (polylines, rule, edges) = match inputs {
            MarkInputs::Borrowed(mark) => (mark.polylines.to_vec(), mark.rule, mark.edges.cloned()),
            MarkInputs::Owned(polylines, rule, edges) => (polylines, rule, edges.map(|e| *e)),
        };
        Self {
            inputs: Arc::new(ExactInputs {
                tile: tile.clone(),
                cut,
                links,
                polylines,
                rule,
                edges,
            }),
            meet_words: None,
            tile_words: None,
        }
    }

    /// Keep the finished tile under the meet's own words when it settles.
    pub(super) fn keep_as_meet(&mut self, words: Vec<u32>) {
        self.meet_words = Some(words.into_boxed_slice());
    }

    /// Keep the finished tile under the whole tile's words when it settles.
    pub(in crate::encode) fn keep_as_tile(&mut self, words: Vec<u32>) {
        self.tile_words = Some(words.into_boxed_slice());
    }

    /// The host memory this meet holds until it settles, counted as the queue counts a job's
    /// (`Job::held`): every buffer it owns, and the chain's edges where it alone holds them.
    ///
    /// A chain's edges the frame kept are held by the frame's residue cache too, under the
    /// edge budget that admitted them (ADR 1467), and every meet of that chain shares them: on
    /// `bug1721218_reduced.pdf` one chain's are 4.98 MB, so counting them again per meet
    /// settled every meet on its own. Edges remade for one meet's rows are that meet's alone.
    fn held(&self) -> u64 {
        let inputs = &self.inputs;
        let words = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
        let points: usize = inputs.polylines.iter().map(|p| p.points.len()).sum();
        let own_links = if Arc::strong_count(&inputs.links) == 1 {
            inputs.links.iter().map(RowEdges::bytes).sum::<u64>()
        } else {
            0
        };
        words(inputs.tile.coverage.len())
            .saturating_add(words(inputs.cut.len()).saturating_mul(8))
            .saturating_add(words(points).saturating_mul(8))
            .saturating_add(inputs.edges.as_ref().map_or(0, RowEdges::bytes))
            .saturating_add(own_links)
    }
}

impl Encoder<'_> {
    /// Record `exact`, whose tile the walk just packed at `at`, to be settled with the
    /// frame's other meets; hand it to the helpers where the frame has them, start them where
    /// the recorded meets have just reached [`EXACT_FLOOR_PIXELS`], and settle every recorded
    /// meet now where what they hold has passed the queue's limit.
    pub(in crate::encode) fn place_exact(&mut self, at: (u32, u32), exact: ExactMeet) {
        self.exact_held = self.exact_held.saturating_add(exact.held());
        self.exact_pixels = self.exact_pixels.saturating_add(exact.inputs.cut.len());
        let index = self.exact_meets.len();
        if let Some(helpers) = &self.exact_helpers {
            helpers.give(index, Arc::clone(&exact.inputs));
        }
        self.exact_meets.push((at, exact));
        if self.exact_helpers.is_none()
            && self.threads > 1
            && self.exact_pixels >= EXACT_FLOOR_PIXELS
        {
            let helpers = Helpers::start(self.threads.saturating_sub(1));
            for (index, (_, meet)) in self.exact_meets.iter().enumerate() {
                helpers.give(index, Arc::clone(&meet.inputs));
            }
            self.exact_helpers = Some(helpers);
        }
        if self.exact_held > self.in_flight_limit {
            self.settle_exact();
        }
    }

    /// Make every recorded meet's exact pixels that the helpers have not, write each finished
    /// tile where the walk packed it, and keep it for the next render, in the order the walk
    /// met them.
    pub(in crate::encode) fn settle_exact(&mut self) {
        let meets = std::mem::take(&mut self.exact_meets);
        self.exact_held = 0;
        if meets.is_empty() {
            return;
        }
        let span = self.clock.start();
        let areas = match &self.exact_helpers {
            Some(helpers) => helpers.collect(meets.len()),
            None => meets.iter().map(|(_, meet)| meet.inputs.areas()).collect(),
        };
        for ((at, meet), areas) in meets.into_iter().zip(areas) {
            let mut tile = meet.inputs.tile.clone();
            if let Some(areas) = areas {
                for (&index, value) in meet.inputs.cut.iter().zip(areas) {
                    // Never above either set's own coverage: the bytes are each the set's area
                    // rounded, and the intersection is inside both.
                    let met = &mut tile.coverage[index];
                    *met = value.min(*met);
                }
            }
            self.scratch.rewrite(at, &tile);
            if meet.meet_words.is_some() || meet.tile_words.is_some() {
                let finished: Arc<[u8]> = Arc::from(tile.coverage.as_slice());
                if let Some(words) = meet.meet_words {
                    self.kept.keep(words, &finished);
                }
                if let Some(words) = meet.tile_words {
                    self.kept.keep_tile(words, &finished);
                }
            }
        }
        self.clock.geometry(span);
    }

    /// Settle the frame's last meets and let its helpers go, so that no thread this frame
    /// started outlives it (`parallel`'s "a scope and not a pool").
    pub(in crate::encode) fn finish_exact(&mut self) {
        self.settle_exact();
        self.exact_helpers = None;
    }
}
