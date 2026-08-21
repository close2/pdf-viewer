//! Path construction and painting: §8.5's subpaths, painting operators and clips.
//!
//! [`Interpreter::end_path`] is where a completed path becomes fill, stroke and clip
//! commands, and the free functions carry §8.5.2's subpath rules that every painting route
//! shares.

use std::sync::Arc;

use pdf_render::display_list::Clip;
use pdf_render::{
    BlendMode, ClipId, Command, FillRule, Paint, Path, PathCommand, Point, Transform,
};

use super::pattern::PatternPaint;
use super::report::Unsupported;
use super::transparency::{Painted, knockout_group_elements};
use super::{GraphicsState, Interpreter};

impl Interpreter<'_> {
    /// Emits the drawing for a completed path and resets it.
    ///
    /// `fill` and `stroke` say what to paint; `close_before_stroke` is already applied by
    /// the caller. A pending `W` takes effect here, which is what the specification
    /// requires: the clip changes *after* the current path is painted.
    pub(super) fn end_path(
        &mut self,
        path: &mut Path,
        pending_clip: &mut Option<FillRule>,
        state: &mut GraphicsState,
        fill: Option<FillRule>,
        stroke: Option<bool>,
    ) {
        // Whether the content stream stated a path at all, which is asked *before* the
        // trailing `m` goes: a path the clause disregards down to nothing is not the same
        // thing as a painting operator with no path in front of it, and the two get
        // different answers below.
        let stated = !path.is_empty();

        // §8.5.3.3.1's trailing `m`, removed before anything reads the path: filling,
        // stroking and clipping all disregard it, and this is the one place all three meet.
        drop_trailing_point(path);

        // Hidden optional content still builds its clip below — §8.11.3.1 puts clipping
        // among the graphics state operations that "shall still be applied" — but marks
        // nothing.
        if !path.is_empty() && (fill.is_some() || stroke.is_some()) && !self.is_hidden() {
            // `B` fills *and* strokes one path, and both commands then describe the same
            // geometry; sharing it means the copy happens once rather than twice.
            let shared = Arc::new(path.clone());
            // Where the two portions start, for §11.6.2 below.
            let mark = self.list.command_count();

            // A tiling pattern is not a paint: its cell is a content stream, replayed
            // across the area the path covers. Doing that here rather than in the display
            // list keeps the list flat — no backend needs to know what a pattern is.
            let fill_clip = self.paint_clip(state, true);
            let stroke_clip = self.paint_clip(state, false);
            if let (Some(rule), Some(PatternPaint::Tiling(tiling))) =
                (fill, state.fill_pattern.clone())
            {
                self.tile(&shared, state.transform, rule, &tiling, state);
            } else if let Some(rule) = fill {
                self.note_transfer(state, Painted::of(state.fill_pattern.as_ref(), false));
                self.list.push(Command::Fill {
                    path: Arc::clone(&shared),
                    transform: state.transform,
                    fill_rule: rule,
                    paint: state.fill_paint(),
                    clip: fill_clip,
                    mask: state.soft_mask,
                    blend: state.blend,
                });
            }
            // A stroke whose colour is a *tiling* pattern would be the cell replayed across
            // the stroked outline, and this tree does not compute that outline — the backends
            // stroke a path themselves (§8.4.3 and ADR 0028). §8.7.2 makes a pattern a colour
            // for `SCN` exactly as for `scn`, so this is a gap rather than a permission, and
            // it is named rather than drawn in the last solid colour that happened to be set.
            if stroke.is_some() && matches!(state.stroke_pattern, Some(PatternPaint::Tiling(_))) {
                self.note(Unsupported::Shading {
                    name: "a stroke whose colour is a tiling pattern".to_owned(),
                });
            }
            if stroke.is_some() {
                self.note_transfer(state, Painted::of(state.stroke_pattern.as_ref(), true));
                self.list.push(Command::Stroke {
                    path: Arc::clone(&shared),
                    transform: state.transform,
                    stroke: state.stroke.clone(),
                    paint: state.stroke_paint(),
                    clip: stroke_clip,
                    mask: state.soft_mask,
                    blend: state.blend,
                });
            }
            // §11.6.2: the fill and the stroke are two parts of one object, and "[p]ortions
            // of an object shall not be composited with one another". They are two commands
            // here, so the band the stroke shares with the fill — half its width, for any
            // path with an interior — composites twice.
            //
            // Two conditions narrow that to the pages where it can be seen, and the second
            // one is not obvious: the paint has to composite at all, since opaque Normal
            // painting puts the stroke over the fill either way, and *both* parts have to
            // mark the page. A `B` whose fill or stroke alpha is zero is one object painted
            // once, and three of the six corpus documents that reach this line are exactly
            // that — `issue11045.pdf` fills at alpha 0 and strokes opaque, `issue3458.pdf`
            // strokes at alpha 0 and fills. Reporting them would name pages whose pixels are
            // the same under either model, which costs them their place in the oracle's
            // comparison and buys nothing.
            let fill_marks = fill.is_some()
                && (matches!(state.fill_pattern, Some(PatternPaint::Tiling(_)))
                    || marks(&state.fill_paint()));
            if fill_marks
                && stroke.is_some()
                && marks(&state.stroke_paint())
                && state.paint_composites()
            {
                // The clause's own answer to "not composited with one another" is §11.4.6's:
                // at any point the topmost portion contributes and the ones under it do not,
                // which is what a knockout group of the two portions computes. `B` strokes
                // after it fills, so the stroke is the topmost portion — the order the two
                // commands are already in. The group is the object, so it takes the alpha
                // and the blend mode that would have been applied to each portion: they are
                // on the elements, and the group composites once at 1.0 under Normal.
                //
                // The `/AIS` reading asked for is the one accumulated over the content so
                // far rather than `state`'s own, and the difference matters where a part is
                // a tiling pattern's group: §11.3.7.2 gives a group object the opacity of
                // "all of the objects it contains", so the reading its *contents* ran under
                // is what decides whether its alpha is its shape.
                let parts = self.list.split_off_commands(mark);
                if let Some(elements) =
                    knockout_group_elements(&parts, self.alpha_sources.settled())
                {
                    self.list.push(Command::Group {
                        commands: elements,
                        alpha: 1.0,
                        clip: None,
                        mask: None,
                        blend: BlendMode::Normal,
                        isolated: true,
                        knockout: true,
                        blending: None,
                    });
                } else {
                    for part in parts {
                        self.list.push(part);
                    }
                    self.note(Unsupported::CompositedInParts {
                        detail: "a path filled and stroked by one operator",
                    });
                }
            }
        }

        // A pending `W` takes effect now: the specification says the clip changes *after*
        // the current path is painted, so the fill and stroke above used the old clip and
        // everything following uses the new one. The new clip becomes a child of the
        // current one, since clipping intersects rather than replaces.
        //
        // A path §8.5.3.3.1 has just disregarded down to nothing still becomes a clip, and
        // that clip admits nothing: §8.5.4 defines the region as "the same area that would
        // be filled by the f operator", and an empty path fills none. `issue9017_reduced.pdf`
        // writes `568.938 673.022 m W n` around a shading, and all three reference renderers
        // leave that shading undrawn. A painting operator with *no* path in front of it is
        // the other case — §8.5.3.1 calls it an error — and leaves the clip alone rather
        // than blanking everything after it, which is the recovery a viewer owes a
        // malformed file.
        if let Some(rule) = pending_clip.take()
            && stated
        {
            let clip = Clip {
                path: path.clone(),
                transform: state.transform,
                fill_rule: rule,
                parent: state.clip,
            };
            match self.list.add_clip(clip) {
                Ok(id) => state.clip = Some(id),
                Err(_) => self.note(Unsupported::LimitReached { limit: "max_clips" }),
            }
        }

        *path = Path::new();
    }

    /// Registers a clip shaped like a rectangle, nested inside `parent`.
    ///
    /// `None` only when the display list is full of clips, which the caller reports.
    pub(super) fn rect_clip(
        &mut self,
        corners: [f32; 4],
        transform: Transform,
        parent: Option<ClipId>,
    ) -> Option<ClipId> {
        let [x0, y0, x1, y1] = corners;
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x0, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y1)));
        path.push(PathCommand::LineTo(Point::new(x0, y1)));
        path.push(PathCommand::Close);
        self.list
            .add_clip(Clip {
                path,
                transform,
                fill_rule: FillRule::NonZero,
                parent,
            })
            .ok()
    }
}

/// Begins a subpath at `at`, ISO 32000-2 §8.5.2.1 Table 58's `m`.
///
/// Table 58's `m` overrides an `m` immediately before it, leaving
///
/// > no vestige of the previous m operation remains in the path.
///
/// Six corpus documents write consecutive `m` operators and one of them, `bug1743245.pdf`,
/// writes 205 of them on its first page. Keeping them would leave 205 single-point subpaths
/// in the path, and §8.5.3.2 turns a single-point subpath under round caps into a *dot* — so
/// the sentence above is what stands between that clause and 205 marks the file never asked
/// for. It also makes the only single-point open subpath a path can hold a *trailing* one,
/// which is exactly the shape §8.5.3.3.1 names.
pub(super) fn begin_subpath(path: &mut Path, at: Point) {
    if matches!(path.commands().last(), Some(PathCommand::MoveTo(_))) {
        path.replace_last(PathCommand::MoveTo(at));
    } else {
        path.push(PathCommand::MoveTo(at));
    }
}

/// Closes the current subpath, ISO 32000-2 §8.5.2.1 Table 58's `h`.
///
/// > If the current subpath is already closed, h shall do nothing.
///
/// A subpath is already closed when the previous command closed it, and there is no current
/// subpath to close before the first `m`. A lone `m` followed by `h` is neither: it is
/// §8.5.3.2's single-point closed path, which is a mark rather than a no-op.
pub(super) fn close_subpath(path: &mut Path) {
    match path.commands().last() {
        None | Some(PathCommand::Close) => {}
        Some(_) => path.push(PathCommand::Close),
    }
}

/// Drops a path's trailing single-point open subpath, ISO 32000-2 §8.5.3.3.1.
///
/// > Any subpaths that are open shall be implicitly closed before being filled, except that
/// > if the last subpath in the path is a single-point open subpath (specified by a trailing
/// > m operator), it shall be disregarded and not considered to be part of the path.
///
/// §8.5.3.2 says the same of stroking — "a single-point open subpath (specified by a
/// trailing m operator) shall produce no output" — and §8.5.4 defines a clip as the area
/// `f` would fill, so all three painting routes want the same thing and it is done once,
/// here, before any of them looks at the path.
///
/// Eleven corpus documents' first pages end a path this way. It changes no pixel by itself,
/// since a point encloses nothing; what it changes is that the path handed to the backends
/// no longer states a subpath the standard has just said is not part of it — and a path
/// consisting only of `m` becomes an empty path rather than one `tiny-skia` refuses.
fn drop_trailing_point(path: &mut Path) {
    if matches!(path.commands().last(), Some(PathCommand::MoveTo(_))) {
        path.pop();
    }
}

/// Whether a paint puts anything on the page.
///
/// A shading always does — its own colours decide where, and a shading with no coverage is a
/// question for the rasteriser rather than for a report. A solid colour with zero alpha does
/// not, which is what `1 0 0 rg /GS gs` with a `ca` of 0 amounts to: a part of an object that
/// paints nothing cannot composite with the part that does.
pub(super) fn marks(paint: &Paint) -> bool {
    match paint {
        Paint::Solid(colour) => colour.a > 0.0,
        // `Paint` is `#[non_exhaustive]`, and a paint this function has not been taught about
        // is one that may well mark the page — which is the safe direction for a report.
        _ => true,
    }
}
