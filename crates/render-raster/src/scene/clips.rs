//! A display-list clip chain on the scene: resolving it once, what it admits, the clip regions a
//! frame uploads, and §14.11.2.1's crop of the page.
//!
//! The encoder's walk asks one question of every mark — *what is this drawn under?* — and the
//! answer is here: a chain that admits nothing skips the mark whole, a chain the mark does not
//! cut is left off it ([`Encoder::cuts_nothing`]), and any other is built onto the scene once and
//! reused. [`rectangle`] is the test the first two share.

use pdf_render::{ClipId, FillRule, Path, PathCommand, Point, Rect, Transform};
use raster_scene::SceneBuilder;

use crate::QuorraRasterError;

use super::{Admitted, ClipOutline, Encoder, ResolvedClip, fill_rule};

/// The page-space rectangle `path` states under `transform`, or `None` where it states
/// something else.
///
/// One subpath of four corners whose sides each run along an axis — which is what a `/BBox`,
/// a `re W n` and §14.11.2.1's crop are, and it is the population [`Encoder::cuts_nothing`]
/// exists for. The interior of such a path is the same region under either of §8.5.3.3's
/// rules, so the fill rule is not read here: four corners have no second subpath to wind
/// against.
///
/// Axis-alignment is tested **after** `transform` rather than before, so a form whose matrix
/// turns by a quarter still states a rectangle here. A bowtie is excluded by the corner test:
/// its two crossing sides run along neither axis.
fn rectangle(path: &Path, transform: Transform) -> Option<Rect> {
    let mut corners: [Point; 4] = [Point::new(0.0, 0.0); 4];
    let mut seen = 0_usize;
    for command in path.commands() {
        let point = match command {
            PathCommand::MoveTo(point) if seen == 0 => *point,
            PathCommand::LineTo(point) => *point,
            // `Close` states no point of its own; a curve, or a second `MoveTo`, is a path
            // this cannot read as one rectangle.
            PathCommand::Close => continue,
            _ => return None,
        };
        let point = transform.apply(point);
        // A fifth point is admitted only where it repeats the first, which is how a rectangle
        // written `m l l l l h` closes: anything else is not four corners.
        if seen == 4 {
            #[expect(
                clippy::float_cmp,
                reason = "exactly the first corner again, which is what a closing point is; a \
                          near miss is a pentagon and keeps its clip"
            )]
            let closes = point.x == corners[0].x && point.y == corners[0].y;
            return closes.then(|| Rect::from_corners(corners[0], corners[2]));
        }
        corners[seen] = point;
        seen = seen.saturating_add(1);
    }
    if seen != 4 {
        return None;
    }
    // The four sides, the closing one included: each must run along an axis.
    let sides = [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ];
    for (a, b) in sides {
        #[expect(
            clippy::float_cmp,
            reason = "a side runs along an axis or it does not, and a side that is a hair off \
                      one states a shape this must not read as a rectangle"
        )]
        let turned = a.x != b.x && a.y != b.y;
        if turned {
            return None;
        }
    }
    Some(Rect::from_corners(corners[0], corners[2]))
}

/// Chains are acyclic by the display list's construction; the cap exists so a
/// hostile or corrupted list is an error rather than a stack overflow. 4096 links
/// is far beyond any real chain (the corpus's worst page nests clips a handful
/// deep across 3 608 chains).
const MAX_CLIP_DEPTH: usize = 4096;

impl Encoder<'_> {
    /// Hangs every clip chain in this list from §14.11.2.1's boundary.
    ///
    /// ISO 32000-2 §14.11.2.1, and it is a `shall`:
    ///
    /// > The crop box defines the region to which the contents of the page shall be clipped
    /// > (cropped) when displayed or printed.
    ///
    /// **The one backend that states this in the scene rather than in the pixels**, and the
    /// reason is the one [`crate::present::build`] gives for the medium: a window frame is drawn
    /// straight onto the swapchain and there is no raster afterwards to cut. The other two
    /// rasterisers, and this one's own readback, call `pdf_render::crop_to_page` on the finished
    /// page instead — see `pdf_render::medium` for why clipping an isolated group's result is
    /// clipping its contents.
    ///
    /// Called by the frame builder and never by [`Self::commands`], which recurses into a
    /// group's elements: the boundary is the list's and is hung once. A group's own result is
    /// composited under a chain that already ends here, so its elements meet the rectangle
    /// twice — which changes nothing except the coverage of the boundary pixel of a page's own
    /// edge, where two anti-aliased answers to one rectangle multiply.
    ///
    /// Does nothing where the list is not a page or its boundary already contains the target,
    /// which is every page-sized target this tree builds.
    pub(crate) fn crop_to_page(
        &mut self,
        builder: &mut SceneBuilder,
    ) -> Result<(), QuorraRasterError> {
        // The early-out reads the view — "does any pixel of *this* target lie outside
        // the boundary" — so a page-space scene, which must be true at every view,
        // states the clip unconditionally (ADR 0702). It is one rectangle.
        if !self.page_space && pdf_render::crop_area(self.list, self.target).is_none() {
            return Ok(());
        }
        let Some(region) = self.list.content_clip() else {
            return Ok(());
        };
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(region.min.x, region.min.y)));
        path.push(PathCommand::LineTo(Point::new(region.max.x, region.min.y)));
        path.push(PathCommand::LineTo(Point::new(region.max.x, region.max.y)));
        path.push(PathCommand::LineTo(Point::new(region.min.x, region.max.y)));
        path.push(PathCommand::Close);
        let outline = self.transient_outline(&path)?;
        self.root = Some(builder.clip(
            outline,
            self.placed(Transform::IDENTITY),
            fill_rule(FillRule::NonZero),
            None,
        )?);
        self.root_rect = Some(region);
        Ok(())
    }

    /// The raster clip chain for a display-list clip, memoised per frame — `None`
    /// when some link admits nothing, in which case the command under it draws
    /// nothing at all (an empty clip is a statement, not an accident).
    pub(crate) fn clip_chain(
        &mut self,
        builder: &mut SceneBuilder,
        clip: Option<ClipId>,
        marks: Option<Rect>,
    ) -> Result<Admitted, QuorraRasterError> {
        if self.cuts_nothing(clip, marks) {
            return Ok(Admitted::Chain(None));
        }
        let Some(id) = clip else {
            // An unclipped command still meets §14.11.2.1's boundary, which is what `root` is.
            return Ok(Admitted::Chain(self.root));
        };
        match self.resolve_clip(builder, id, 0)? {
            ResolvedClip::AdmitsNothing => Ok(Admitted::Nothing),
            ResolvedClip::Chain(chain) => Ok(Admitted::Chain(Some(chain))),
        }
    }

    /// Whether this chain takes nothing from a mark bounded by `marks`, so that the mark may be
    /// drawn with no clip at all.
    ///
    /// # The clause states a set, and a set that contains a mark takes nothing from it
    ///
    /// ISO 32000-2 §10.7.4:
    ///
    /// > For clipping, the clipping region consists of the set of pixels that would be included
    /// > by a fill operation. Subsequent painting operations shall affect a region that is the
    /// > intersection of the set of pixels defined by the clipping region with the set of pixels
    /// > for the region to be painted.
    ///
    /// and §8.5.4 says the same of the shape: "[t]he effective shape is the intersection of the
    /// object's intrinsic shape with the clipping path". An intersection with a set that
    /// contains the mark is the mark — `S n C = S` where `S` is inside `C` — at every pixel,
    /// the mark's own anti-aliased boundary included.
    ///
    /// **raster composes a clip with a mark by multiplying their coverages** (its
    /// `coverage.wgsl`: `cov * extent.x * extent.y`), and a product does not have that
    /// property: where a mark's own edge and its clip's fall in one pixel, the mark keeps the
    /// *square* of its coverage there. So the ink a containing clip costs is recoverable on
    /// this side without the device changing anything — by not stating the clip.
    /// `doc/QUORRA_FEEDBACK.md` section 24 is the ask for the case that remains, which is a
    /// clip that genuinely cuts.
    ///
    /// # Why the question is asked in page space
    ///
    /// Both rectangles here are page-space, before the target transform, so the answer is a
    /// property of the document and not of this view: every affine carries a containment to a
    /// containment, and §10.7.4's pixel region is a *superset* of the geometric one it is built
    /// from — "painting any pixel whose half-open square region intersects the shape" — so a
    /// containment that holds geometrically holds against the pixels the device clips with.
    /// That is what keeps this out of [`Encoder::consume_view`] and leaves a page-space scene
    /// true at every magnification (ADR 0702).
    ///
    /// `marks` is `None` for a command whose extent this cannot bound, and for one whose marks
    /// can be *enlarged* past their own geometry by §10.7.4's substitutions — a collapsed fill
    /// subpath or §8.5.3.2's dot, each one device pixel across and so a bound the view decides.
    /// Both answer `false`, which keeps the clip.
    fn cuts_nothing(&mut self, clip: Option<ClipId>, marks: Option<Rect>) -> bool {
        let Some(marks) = marks else { return false };
        self.admitted_rect(clip)
            .is_some_and(|admits| admits.contains(marks))
    }

    /// The page-space rectangle a whole chain admits, or `None` where any link of it is not a
    /// rectangle [`rectangle`] can read.
    ///
    /// **An *inner* bound, which is the opposite of `DisplayList::clip_bounds`**: that one is a
    /// rectangle the region lies inside, and a containment test needs one that lies inside the
    /// region. A curved or many-sided clip has an inner rectangle too, and this does not look
    /// for it — the population is rectangles, and the answer for everything else is to keep the
    /// clip.
    ///
    /// Memoised per chain, because a page states one chain per clipping path and asks this
    /// question once per command under it.
    fn admitted_rect(&mut self, clip: Option<ClipId>) -> Option<Rect> {
        let Some(id) = clip else {
            return self.root_rect;
        };
        if let Some(cached) = self.admits.get(&id.index()) {
            return *cached;
        }
        let rect = self.walk_admitted(id, 0);
        self.admits.insert(id.index(), rect);
        rect
    }

    /// [`Encoder::admitted_rect`]'s walk: this link's rectangle met with its parents'.
    ///
    /// Bounded by [`MAX_CLIP_DEPTH`] for the reason that constant states, and answering `None`
    /// there rather than erroring: a chain that deep keeps its clip, which is the conservative
    /// answer and not a refusal.
    fn walk_admitted(&mut self, id: ClipId, depth: usize) -> Option<Rect> {
        if depth > MAX_CLIP_DEPTH {
            return None;
        }
        let def = self.list.clip(id)?;
        let here = rectangle(&def.path, def.transform)?;
        let outer = match def.parent {
            Some(parent) => self.walk_admitted(parent, depth.saturating_add(1))?,
            None => match self.root_rect {
                Some(root) => root,
                // No §14.11.2.1 boundary in this scene: the chain ends at this link.
                None => return Some(here),
            },
        };
        here.intersection(outer)
    }

    fn resolve_clip(
        &mut self,
        builder: &mut SceneBuilder,
        id: ClipId,
        depth: usize,
    ) -> Result<ResolvedClip, QuorraRasterError> {
        if let Some(resolved) = self.clips.get(&id.index()) {
            return Ok(*resolved);
        }
        if depth > MAX_CLIP_DEPTH {
            return Err(QuorraRasterError::CyclicClip(id));
        }
        let def = self
            .list
            .clip(id)
            .ok_or(QuorraRasterError::UnknownClip(id))?;
        if def.admits_nothing() {
            self.clips.insert(id.index(), ResolvedClip::AdmitsNothing);
            return Ok(ResolvedClip::AdmitsNothing);
        }
        let parent = match def.parent {
            Some(parent) => match self.resolve_clip(builder, parent, depth.saturating_add(1))? {
                ResolvedClip::AdmitsNothing => {
                    self.clips.insert(id.index(), ResolvedClip::AdmitsNothing);
                    return Ok(ResolvedClip::AdmitsNothing);
                }
                ResolvedClip::Chain(chain) => Some(chain),
            },
            // The outermost link of every chain hangs from the page's own boundary.
            None => self.root,
        };
        let (outline, rule) = match self.clip_outlines.get(&id.index()).copied() {
            Some(kept) => {
                if kept.read_view {
                    self.consume_view();
                }
                (kept.outline, kept.rule)
            }
            None => self.clip_outline(id.index(), def)?,
        };
        let link = builder.clip(outline, self.placed(def.transform), fill_rule(rule), parent)?;
        let resolved = ResolvedClip::Chain(link);
        self.clips.insert(id.index(), resolved);
        Ok(resolved)
    }

    /// A clip's region uploaded for this target, and kept for every frame this one hands its
    /// clips to ([`Encoder::clip_outlines`]).
    fn clip_outline(
        &mut self,
        index: usize,
        def: &pdf_render::Clip,
    ) -> Result<(raster_scene::OutlineId, FillRule), QuorraRasterError> {
        // ISO 32000-2 §10.7.4: "For clipping, the clipping region consists of the set of pixels
        // that would be included by a fill operation", and the clause's own EXAMPLE says what a
        // fill of a flat rectangle includes — "[a] zero-width or zero-height rectangle paints a
        // line 1 pixel wide". `pdf_render::clip_region` builds that region for every backend, so
        // that a clip and a fill of one path cannot disagree and neither can two rasterisers
        // (ADR 1064). raster's own encode places a *fill*'s marks per viewport from the collapse
        // table resident on the outline; a clip's are baked here instead, which is a decision
        // about this target's pixel grid and so consumes the view (ADR 0702).
        let region = pdf_render::clip_region(
            &def.path,
            def.fill_rule,
            def.transform.then(self.target.transform),
        );
        let read_view = matches!(region, Some(pdf_render::ClipRegion::One(..)));
        let (outline, rule) = match &region {
            Some(pdf_render::ClipRegion::One(region, rule)) => {
                self.consume_view();
                (self.transient_outline(region)?, *rule)
            }
            // The region is the union of two fills under two different rules, and
            // `SceneBuilder::clip` takes one outline and one rule. `MaskKind::Alpha` states the
            // union — a group holding both fills has their combined alpha — but a mask
            // *multiplies* into a draw and every command already carries the one the document
            // gave it, so the two would have to be composed into a third the scene has no
            // vocabulary for. `QUORRA_FEEDBACK.md` section 51 is the ask; until then the frame
            // goes to the CPU backend, which composes the union into its own mask (ADR 1231).
            Some(pdf_render::ClipRegion::Union { .. }) => {
                return Err(QuorraRasterError::Unsupported(
                    "a clipping path whose region is the union of two fills under two rules                      (ISO 32000-2 §10.7.4, §8.5.4): a clip takes one outline and one rule, and                      a mask multiplies into a draw that already carries one"
                        .to_owned(),
                ));
            }
            None => (self.transient_outline(&def.path)?, def.fill_rule),
        };
        self.clip_outlines.insert(
            index,
            ClipOutline {
                outline,
                rule,
                read_view,
            },
        );
        Ok((outline, rule))
    }
}
