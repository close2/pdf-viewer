//! The pieces of a tightly bent stroke, made disjoint so that each point of the set is
//! covered once (ADR 1375).
//!
//! [`fill`](super::super::fill) integrates winding over a pixel and clamps it afterwards,
//! so two pieces that overlap inside one pixel on the stroke's rim count the overlap
//! twice there (trap 58). [`inner_cut`](super::inner_cut) makes neighbouring pieces meet
//! edge to edge where the half-width allows it. Where the path bends more tightly than
//! the half-width it does not: the inner offset folds over itself, each segment's
//! rectangle reaches past the centre of the bend, and a run of rectangles overlaps in a
//! star whose points are the rim. ISO 32000-2 §8.4.3.2 says what the union of those
//! pieces is:
//!
//! > stroking a path shall entail painting all points whose perpendicular distance from
//! > the path in user space is less than or equal to half the line width
//!
//! — a set, in which a point painted twice is painted exactly as much as a point painted
//! once. This module states that set as a tiling instead of a covering: each piece loses
//! whatever the pieces before it already hold, `P_k − (P_0 ∪ … ∪ P_{k−1})`, which leaves
//! the union exactly as it was and no two fragments sharing any area.
//!
//! Every piece [`stroke_polylines`](super::stroke_polylines) emits is convex, and a convex
//! piece less another is a set of convex fragments ([`convex`](super::convex)), so the tiling
//! needs no boolean library.
//!
//! Only the pieces that meet a tight bend take part — the segments and joins at a vertex
//! [`inner_cut`](super::inner_cut) declined, the pieces of one subpath that overlap a piece
//! of another, and whatever piece's box meets one of theirs; the rest already meet their
//! neighbours edge to edge. A piece is cut only by earlier pieces whose boxes meet what is
//! left of it, but where a bend is tight that is nearly every piece of the bend — each body
//! reaches past the centre of curvature — so the work is quadratic in them and bounded:
//! [`MAX_PIECES`] pieces in, [`MAX_FRAGMENTS`] fragments out. Past either bound the stroke
//! keeps its overlapping pieces — the same set, dearer only in the rim pixels an overlap
//! touches — because a width and a path are a document's numbers and an allocation is not
//! (CLAUDE.md principle 3).

use std::ops::ControlFlow;

use super::super::flatten::Polyline;
use super::convex::{Convex, Scratch};
use super::sweep::{Bounds, meeting};

/// The most pieces one tiling may be asked to seed from, and the most it may re-cut: the
/// cutting is quadratic in the pieces of one bend that meet. A thick curve's tight run is a
/// few pieces per flattened vertex — 65 for `stroke_set`'s hook at 8× — and a glyph outline
/// stroked wider than itself about a hundred and fifty.
const MAX_PIECES: usize = 1024;

/// The most fragments one tiling may produce before it is abandoned for the overlapping
/// pieces it came from; and the most pairs of boxes a sweep compares in answering whether
/// pieces stand apart.
const MAX_FRAGMENTS: usize = 65_536;

/// One subpath's pieces as [`stroke_subpath`](super::stroke_subpath) built them, before any
/// tiling.
pub(super) struct Subpath {
    /// The closed polygons, all wound one way.
    pub(super) pieces: Vec<Polyline>,
    /// Beside each piece, whether it meets a vertex [`inner_cut`](super::inner_cut) declined.
    pub(super) at_a_tight_bend: Vec<bool>,
    /// Whether the pieces share no area among themselves by their construction alone
    /// ([`Stroked::tiles`](super::Stroked::tiles)).
    pub(super) tiles: bool,
}

/// A whole stroke's pieces, and whether no point is inside two of them.
///
/// §8.4.3.2's set is the stroke's, not a subpath's — the points within half the line width
/// of "the path", every subpath of it — so two subpaths whose pieces overlap count that
/// overlap twice in a rim pixel exactly as two pieces of one subpath do. Each subpath is
/// tiled alone where nothing of it overlaps another subpath, which leaves its pieces as they
/// were; where pieces of two subpaths overlap, those pieces seed one tiling of the whole
/// stroke, and the stroke tiles where that tiling vouches for every piece (ADR 1431).
pub(super) fn tile_stroke(mut subpaths: Vec<Subpath>) -> (Vec<Polyline>, bool) {
    if subpaths.len() < 2 {
        return subpaths
            .pop()
            .map_or((Vec::new(), false), |subpath| alone(subpath, None));
    }
    let boxes: Vec<Bounds> = subpaths
        .iter()
        .flat_map(|subpath| subpath.pieces.iter().map(|piece| Bounds::of(&piece.points)))
        .collect();
    let owner: Vec<usize> = subpaths
        .iter()
        .enumerate()
        .flat_map(|(k, subpath)| std::iter::repeat_n(k, subpath.pieces.len()))
        .collect();
    let mut across = Vec::new();
    let swept = meeting(&boxes, MAX_FRAGMENTS, |a, b| {
        if owner[a] != owner[b] {
            across.push((a, b));
        }
        ControlFlow::Continue(())
    });
    if swept.is_break() {
        return each_alone(subpaths, None, false);
    }
    if across.is_empty() {
        return each_alone(subpaths, None, true);
    }
    let convex: Vec<Option<Convex>> = subpaths
        .iter()
        .flat_map(|subpath| subpath.pieces.iter().map(Convex::new))
        .collect();
    let mut crossing = vec![false; convex.len()];
    for (a, b) in across {
        // A pair whose pieces are both marked already would mark nothing new.
        if crossing[a] && crossing[b] {
            continue;
        }
        if let (Some(piece_a), Some(piece_b)) = (&convex[a], &convex[b])
            && !piece_a.apart_from(piece_b)
        {
            crossing[a] = true;
            crossing[b] = true;
        }
    }
    if !crossing.contains(&true) {
        return each_alone(subpaths, Some(&convex), true);
    }
    let groups: Vec<Option<usize>> = owner
        .iter()
        .map(|&k| subpaths[k].tiles.then_some(k))
        .collect();
    let seeds: Vec<bool> = subpaths
        .iter()
        .flat_map(|subpath| subpath.at_a_tight_bend.iter().copied())
        .zip(&crossing)
        .map(|(tight, crossing)| tight || *crossing)
        .collect();
    let lengths: Vec<usize> = subpaths
        .iter()
        .map(|subpath| subpath.pieces.len())
        .collect();
    let (pieces, tight): (Vec<Vec<Polyline>>, Vec<Vec<bool>>) = subpaths
        .into_iter()
        .map(|subpath| (subpath.pieces, subpath.at_a_tight_bend))
        .unzip();
    let tiles_alone: Vec<bool> = groups.iter().map(Option::is_some).collect();
    match tile(
        pieces.into_iter().flatten().collect(),
        &convex,
        &seeds,
        &groups,
    ) {
        Ok(tiling) => (tiling.pieces, tiling.whole),
        // Abandoned at a bound: the pieces overlap across subpaths, so the stroke does not
        // tile, and each subpath is tiled alone as though nothing crossed it.
        Err(mut pieces) => {
            let mut subpaths = Vec::with_capacity(lengths.len());
            let mut first = 0_usize;
            for (length, at_a_tight_bend) in lengths.into_iter().zip(tight) {
                let rest = pieces.split_off(length);
                subpaths.push(Subpath {
                    pieces,
                    at_a_tight_bend,
                    tiles: tiles_alone[first],
                });
                pieces = rest;
                first = first.saturating_add(length);
            }
            each_alone(subpaths, Some(&convex), false)
        }
    }
}

/// Each subpath tiled [`alone`], in order; the stroke tiles where `apart` says the subpaths'
/// pieces share no area with each other's and every subpath's own pieces tile.
fn each_alone(
    subpaths: Vec<Subpath>,
    convex: Option<&[Option<Convex>]>,
    apart: bool,
) -> (Vec<Polyline>, bool) {
    let mut out = Vec::new();
    let (mut tiles, mut first) = (apart, 0_usize);
    for subpath in subpaths {
        let last = first.saturating_add(subpath.pieces.len());
        let (pieces, own) = alone(subpath, convex.map(|convex| &convex[first..last]));
        out.extend(pieces);
        tiles &= own;
        first = last;
    }
    (out, tiles)
}

/// One subpath's pieces, re-cut where it bends more tightly than the half-width, and whether
/// they tile its set. `convex` is each piece's convex form where the caller has it already.
fn alone(subpath: Subpath, convex: Option<&[Option<Convex>]>) -> (Vec<Polyline>, bool) {
    if !subpath.at_a_tight_bend.contains(&true) {
        return (subpath.pieces, subpath.tiles);
    }
    let owned: Vec<Option<Convex>>;
    let convex = if let Some(convex) = convex {
        convex
    } else {
        owned = subpath.pieces.iter().map(Convex::new).collect();
        &owned
    };
    let groups = vec![None; subpath.pieces.len()];
    match tile(subpath.pieces, convex, &subpath.at_a_tight_bend, &groups) {
        // No point inside two pieces, whatever the path does: the fill need not ask (ADR 1421).
        Ok(tiling) => (tiling.pieces, subpath.tiles || tiling.whole),
        Err(pieces) => (pieces, subpath.tiles),
    }
}

/// The pieces that take part — those `seeds` names, and every piece whose box meets one of
/// theirs — re-cut so that no two of them share any area and their union is unchanged; the
/// rest as they came, since they meet their neighbours edge to edge already. Or, as `Err`,
/// every piece as it came, where the tiling would pass [`MAX_PIECES`] or [`MAX_FRAGMENTS`].
///
/// A piece with no area deposits nothing and holds nothing, and is dropped from the tiling.
/// `groups` names, beside each piece, the subpath whose own pieces are known to share no
/// area, where it is one ([`the_rest_stand_apart`]).
fn tile(
    pieces: Vec<Polyline>,
    convex: &[Option<Convex>],
    seeds: &[bool],
    groups: &[Option<usize>],
) -> Result<Tiled, Vec<Polyline>> {
    if seeds.iter().filter(|seed| **seed).count() > MAX_PIECES {
        return Err(pieces);
    }
    let takes_part = takes_part(convex, seeds);
    let tiled: Vec<&Convex> = convex
        .iter()
        .zip(&takes_part)
        .filter_map(|(piece, takes_part)| piece.as_ref().filter(|_| *takes_part))
        .collect();
    if tiled.len() > MAX_PIECES {
        return Err(pieces);
    }
    // The boxes side by side, for the scan below that reads nothing else of most pieces.
    let boxes: Vec<Bounds> = tiled.iter().map(|piece| piece.bounds).collect();
    let mut scratch = Scratch::default();
    let mut fragments = Vec::new();
    let mut out: Vec<Polyline> = Vec::with_capacity(pieces.len());
    for (k, piece) in tiled.iter().enumerate() {
        fragments.clear();
        fragments.push((piece.points.clone(), piece.bounds));
        // The box round what is left of this piece. An earlier piece whose box misses it
        // misses every fragment's box, which lies inside it, so it would hand the fragments
        // back unchanged ([`Convex::subtract_from_each`]) and is passed over without asking
        // (ADR 1421). A box test is all the scan costs a piece it passes over: a sweep that
        // found only the pairs whose boxes meet cost more in its sorting than it saved
        // (ADR 1431).
        let mut reach = piece.bounds;
        // Nearest first: a piece's neighbours hold most of what it shares, so taking them
        // away first leaves little for the rest to cut.
        for j in (0..k).rev() {
            if boxes[j].meets(reach)
                && tiled[j].subtract_from_each(&mut fragments, piece.orientation, &mut scratch)
            {
                reach = Bounds::round(fragments.iter().map(|(_, bounds)| *bounds));
            }
            if fragments.is_empty() || out.len().saturating_add(fragments.len()) > MAX_FRAGMENTS {
                break;
            }
        }
        if out.len().saturating_add(fragments.len()) > MAX_FRAGMENTS {
            return Err(pieces);
        }
        out.extend(
            fragments
                .drain(..)
                .map(|(points, _)| Polyline::polygon(points)),
        );
    }
    out.extend(
        pieces
            .into_iter()
            .zip(convex)
            .zip(&takes_part)
            .filter(|((_, piece), takes_part)| piece.is_some() && !**takes_part)
            .map(|((piece, _), _)| piece),
    );
    let whole = tiled.len() == convex.iter().flatten().count()
        || the_rest_stand_apart(convex, &takes_part, groups);
    Ok(Tiled { pieces: out, whole })
}

/// Beside each piece, whether it takes part in the tiling: it has an area, and its box meets
/// a seed's, its own included. Asked once per piece rather than once for the tiling and again
/// for the rest.
fn takes_part(convex: &[Option<Convex>], seeds: &[bool]) -> Vec<bool> {
    let seeds: Vec<Bounds> = convex
        .iter()
        .zip(seeds)
        .filter_map(|(piece, seed)| piece.as_ref().filter(|_| *seed).map(|p| p.bounds))
        .collect();
    convex
        .iter()
        .map(|piece| {
            piece
                .as_ref()
                .is_some_and(|piece| seeds.iter().any(|seed| seed.meets(piece.bounds)))
        })
        .collect()
}

/// Whether every piece that took no part in the tiling shares no area with any other piece,
/// so that with the tiled fragments, which share none among themselves, no point of the
/// stroke is inside two pieces (ADR 1421).
///
/// Asked of the pieces as they came: a fragment lies inside the piece it was cut from, so a
/// piece apart from that piece is apart from every fragment of it. Two convex pieces are
/// apart where a line through an edge of either has the other wholly on its outside — a
/// neighbour meeting edge to edge is, on the edge they share. Two pieces of one subpath
/// that `groups` names share no area by that subpath's construction, and are not asked. The
/// pairs are found by a sweep along `x`, and past [`MAX_FRAGMENTS`] box comparisons the
/// question answers no, which leaves it to the fill (ADR 1389) as before.
#[expect(clippy::arithmetic_side_effects)] // a count below `MAX_FRAGMENTS` plus a length
fn the_rest_stand_apart(
    convex: &[Option<Convex>],
    takes_part: &[bool],
    groups: &[Option<usize>],
) -> bool {
    let pieces: Vec<(bool, Option<usize>, &Convex)> = convex
        .iter()
        .zip(takes_part)
        .zip(groups)
        .filter_map(|((piece, takes_part), group)| {
            piece.as_ref().map(|piece| (*takes_part, *group, piece))
        })
        .collect();
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_unstable_by(|&a, &b| {
        pieces[a]
            .2
            .bounds
            .min
            .x
            .total_cmp(&pieces[b].2.bounds.min.x)
    });
    // The sweep [`meeting`] makes, written out so that a pair known apart is passed over
    // before its boxes are compared: in a tight bend most pairs are two tiled pieces, and
    // comparing their boxes first cost a third of this question (ADR 1431).
    let (mut active, mut tests) = (Vec::<usize>::new(), 0_usize);
    for k in order {
        let (tiled_k, group_k, piece_k) = pieces[k];
        active.retain(|&m| pieces[m].2.bounds.max.x > piece_k.bounds.min.x);
        tests += active.len();
        if tests > MAX_FRAGMENTS {
            return false;
        }
        for &m in &active {
            let (tiled_m, group_m, piece_m) = pieces[m];
            // Two tiled pieces' fragments share nothing by construction, nor do two pieces
            // of a subpath that vouches for its own.
            let known = (tiled_k && tiled_m) || (group_k.is_some() && group_k == group_m);
            if !known && piece_k.bounds.meets(piece_m.bounds) && !piece_k.apart_from(piece_m) {
                return false;
            }
        }
        active.push(k);
    }
    true
}

/// What [`tile`] made of a stroke's pieces.
struct Tiled {
    /// The pieces, the tiled ones re-cut.
    pieces: Vec<Polyline>,
    /// Whether no point is inside two of the pieces, so that they tile the set: every piece
    /// with an area took part in the tiling, or those that did not stand apart from every
    /// other piece.
    whole: bool,
}
