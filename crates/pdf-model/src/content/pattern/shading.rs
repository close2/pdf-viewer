//! §8.7.4: the `sh` operator and a shading pattern's clip: the `/BBox`, the shading's domain, and the
//! outline a mesh is painted through (ADR 2081).

use std::sync::Arc;

use pdf_render::display_list::Clip;
use pdf_render::{
    ClipId, Command, FillRule, Paint, Path, PathCommand, Point, Shading, ShadingKind, Transform,
};
use pdf_syntax::{Dictionary, Name};

use crate::content::report::Unsupported;
use crate::content::transparency::Painted;
use crate::content::{GraphicsState, Interpreter};

use super::PatternPaint;
use super::colouring::shading_with_alpha;

impl Interpreter<'_> {
    /// The clip in force for a paint, including a shading pattern's own `/BBox`.
    ///
    /// ISO 32000-2 §8.7.4.3 Table 77 makes `/BBox` "a temporary clipping boundary … in
    /// addition to the current clipping path and any other clipping boundaries in effect at
    /// that time", so it nests *inside* whatever the graphics state already has rather than
    /// replacing it. A pattern with no `/BBox`, or a paint that is not a shading pattern,
    /// gets the state's clip unchanged and costs nothing.
    ///
    /// `None` for the clip is "no clip"; the error case — a display list already holding as
    /// many clips as it can — falls back to the state's own, because losing a bounding box
    /// draws too much and losing the whole command draws nothing.
    pub(in crate::content) fn paint_clip(
        &mut self,
        state: &GraphicsState,
        fill: bool,
    ) -> Option<ClipId> {
        let pattern = if fill {
            state.fill_pattern.as_ref()
        } else {
            state.stroke_pattern.as_ref()
        };
        let Some(PatternPaint::Shading(pattern)) = pattern else {
            return state.clip;
        };
        // The geometry, which is the half of a shading no colouring can move: a rebuild at the
        // mark states the same domain and the same matrix, so the clip is the selection's.
        let shading = Arc::clone(&pattern.shading);
        let bbox = pattern.bbox;
        let clip = match bbox {
            Some((corners, transform)) => self
                .rect_clip(corners, transform, state.clip)
                .or(state.clip),
            None => state.clip,
        };
        // §8.7.4.5.2's domain is where a type 1 shading marks *and nothing else* only where
        // the shading has no `/Background`. Where it has one, the same sentence says what
        // happens outside instead — those points "shall be painted with the shading's
        // background colour" — and `pdf_render::ShadingRaster` answers it per pixel, so a clip
        // here would cut away the very wash the entry asks for.
        if pattern.shading.background.is_some() {
            return clip;
        }
        self.domain_clip(&shading, clip)
    }

    /// The parallelogram a function-based shading's domain occupies, as a clip.
    ///
    /// ISO 32000-2 §8.7.4.5.2 states where a type 1 shading marks and where it does not:
    ///
    /// > The transformation matrix ( Matrix ) then maps the domain rectangle into a
    /// > corresponding rectangle or parallelogram in the target coordinate space. Points wi
    /// > thin the shading's bounding box ( BBox ) that fall outside this transformed domain
    /// > rectangle shall be painted with the shading's background colour ( Background ); if
    /// > the shading dictionary has no Background entry, such points shall be left unpainted.
    ///
    /// **"Or parallelogram" is the whole of it**, and it is why this is a clip rather than a
    /// property of the sampled grid. `function_based_shading.pdf` states
    /// `/Matrix [85 85 -85 85 515 382]` — a rotation — so its domain occupies a diamond, and
    /// this reader painted a *square* against four references' diamond for the project's whole
    /// life. The backend's pattern is padded outside its grid, which is right for the
    /// interpolation and says nothing about where the shading ends; where a shading ends is
    /// this clause's answer and so the interpreter's to compose, exactly as Table 77's
    /// `/BBox` is.
    ///
    /// **This is the clause's branch for a shading that states no `/Background`**, and it is the
    /// only branch that reaches here: a shading that states one is answered per device pixel by
    /// [`pdf_render::ShadingRaster`] instead, so [`Interpreter::paint_clip`] returns before
    /// calling this rather than clipping away the very wash Table 77 asks for (ADR 0529). A
    /// *stroking* selection is where the wash is still owed, and that is named by
    /// [`Unsupported::ShadingBackground`] at [`Interpreter::note_unpainted_background`].
    ///
    /// Nothing happens for any other shading type: an axial or radial shading says where it
    /// stops through `/Extend`, which its ramp already carries, and a mesh through its
    /// triangles.
    fn domain_clip(&mut self, shading: &Shading, parent: Option<ClipId>) -> Option<ClipId> {
        let ShadingKind::Sampled { domain, .. } = shading.kind.as_ref() else {
            return parent;
        };
        // Table 78's order is [x min x max y min y max], which is not `rect_clip`'s.
        let [x0, x1, y0, y1] = *domain;
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x0, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y1)));
        path.push(PathCommand::LineTo(Point::new(x0, y1)));
        path.push(PathCommand::Close);
        self.list
            .add_clip(Clip {
                path,
                // The shading's own `/Matrix` is composed into this already, which is what
                // makes the domain rectangle's corners the right four points to send.
                transform: shading.transform,
                fill_rule: FillRule::NonZero,
                parent,
            })
            .ok()
            .or(parent)
    }

    /// Paints a shading across the current clip, for the `sh` operator.
    ///
    /// `sh` covers the whole clipping region rather than a path, so the geometry drawn is
    /// the page itself and the clip does the shaping. Where the shading does not extend,
    /// it paints nothing, so the covered area is only ever as large as the shading says.
    pub(in crate::content) fn paint_shading(
        &mut self,
        name: &Name,
        resources: &Dictionary,
        state: &GraphicsState,
    ) {
        // The name's bytes do the finding (§7.3.5, `resources.rs`); the text says so afterwards.
        // Looked up before the visibility is asked, because the operator selects its shading
        // whether or not the layer it stands in is shown — which is what the ledger records.
        let entry = self.resource_entry(resources, "Shading", name);
        // `sh` marks the page and changes nothing else, so a hidden layer skips it whole —
        // including the report a shading we cannot build would otherwise make about a
        // shading that was never going to be drawn.
        if self.is_hidden() {
            return;
        }
        let label = String::from_utf8_lossy(name.as_bytes()).into_owned();
        let Some(object) = entry else {
            self.note(Unsupported::Shading {
                name: format!("/{label} is not in /Shading"),
            });
            return;
        };

        // `sh` is drawn in the current user space, unlike a pattern.
        //
        // Table 77's `/BBox` is read here rather than inside `shading::build`, because it is
        // stated "in the shading's target coordinate space" — which is the space the caller
        // paints into, not the shading's own — and because the clause makes it a *clip*
        // rather than a property of the gradient: "this bounding box shall be applied as a
        // temporary clipping boundary when the shading is painted, in addition to the
        // current clipping path and any other clipping boundaries in effect at that time".
        let clip = crate::shading::bbox_of(self.document, &object).map_or(state.clip, |corners| {
            self.rect_clip(corners, state.transform, state.clip)
                .or(state.clip)
        });
        // §10.5's transfer function. An `sh` builds its shading at the mark, so the state here is
        // the state the clause asks about and nothing can have moved between them — and §11.6.4.4
        // makes `sh` a non-stroking painting operation, which is which of Table 51's two alpha
        // constants §11.7.5.2's first condition reads.
        let transfer = self.mark_transfer(state, Painted::Shading { stroking: false });
        let conversion = self.conversion(state);
        // §11.7.5.3's NOTE puts §10.5's values "only when all colour compositing has been
        // completed and rasterization is being performed", so the ramp is sampled raw and the
        // function rides on the mark below (ADR 1266).
        let colouring = crate::shading::Colouring::new(state.smoothness, &conversion);
        match self.shadings.build(
            self.document,
            &object,
            resources,
            state.transform,
            colouring,
        ) {
            Ok(built) => {
                self.black_generation_painted |= built.black_generation_bypassed;
                // §10.7.3 permits the bound and `crate::content`'s module documentation forbids
                // drawing less than the document states in silence, so a mesh the bound cut
                // short is said out loud before its remaining triangles are painted.
                if let Some(limit) = built.truncated {
                    self.note(Unsupported::LimitReached { limit });
                }
                // And a mesh whose colours §8.7.4.4's subdivision could not bring within
                // §10.7.3's tolerance before a bound stopped it is said the same way: the
                // triangles are all there, and what is coarser than the clause asks is the
                // colour between them.
                if built.coarse {
                    self.note(Unsupported::LimitReached {
                        limit: "max_mesh_refinement",
                    });
                }
                let shading = built.shading;
                // Table 77's `/Background` "shall be applied only when the shading is used as
                // part of a shading pattern, not when painted directly with the sh operator";
                // §8.7.4.2 says the same of this operator — "[t]he Background entry, if
                // present, is ignored" — and §11.6.4.2 a third time, of the shape a `sh`
                // contributes: "1.0 inside and 0.0 outside the bounds of the shading's painti
                // ng geometry, disregarding the Background entry". `shading::Cache` is keyed
                // by the object and one object can be both a pattern's shading and an `sh`'s,
                // so the entry is resolved once where the colour space is and dropped here.
                let shading = Shading {
                    background: None,
                    ..shading
                };
                // §8.7.4.5.2's domain, which for a type 1 shading is where it marks at all.
                let clip = self.domain_clip(&shading, clip);
                let (path, transform) = self.shading_surface(&shading);

                // §11.7.5.2's channel carries the function, and the shape it records is
                // §11.6.4.2's — where the shading paints rather than the whole region the `sh`
                // was clipped to (`pdf_render::shape_of`, ADR 1255).
                self.draw_mark(
                    Command::Fill {
                        path: Arc::new(path),
                        transform,
                        fill_rule: FillRule::NonZero,
                        // §11.6.4.4's non-stroking constant applies to `sh` as to any other
                        // non-stroking painting operation.
                        paint: Paint::Shading(shading_with_alpha(
                            &Arc::new(shading),
                            state.fill_alpha,
                        )),
                        clip,
                        mask: state.soft_mask,
                        blend: state.blend,
                    },
                    transfer,
                );
            }
            Err(error) => self.note(Unsupported::Shading {
                name: format!("/{label}: {error}"),
            }),
        }
    }

    /// The surface a `sh` fills, and the transform that places it.
    ///
    /// ISO 32000-2 §8.7.4.2 gives the operator no path at all:
    ///
    /// > This operator does not require the creation of a pattern dictionary or a path and
    /// > works without reference to the current colour in the graphics state.
    ///
    /// A display list fills paths, so one has to stand in for "wherever the shading marks".
    /// [`pdf_render::Shading::painting_bounds`] is §11.6.4.2's answer to where that is, and it
    /// is used wherever the shading has one; the page rectangle is the fallback for the
    /// shadings whose geometry is unbounded — axial and radial — where nothing smaller is
    /// true.
    ///
    /// # Why the page is not good enough on its own
    ///
    /// A `sh` inside a tiling pattern's cell is part of a figure the lattice repeats, and
    /// [`pdf_render::Cell::repeat`] displaces each copy's geometry — the shading with it, so
    /// that "every site would show the first site's gradient" does not happen. A page
    /// rectangle is displaced by the same lattice step while being no part of the figure, so
    /// the site whose shading arrives on the page is the site whose rectangle has just left
    /// it, and the tiling draws nothing at all. `0423269.pdf` in the `SafeDocs` crawl is two
    /// mesh-filled backgrounds lost that way, silently, because a shading that paints outside
    /// its own surface is not a refusal anything can report.
    ///
    /// # Why the rectangle is grown
    ///
    /// The mesh's own bounds pass through its outermost vertices, so an outline laid exactly
    /// on them shares an edge with the triangles it is there to admit — and a shared edge is
    /// counted twice, because coverage multiplies: two halves composite as a quarter, which is
    /// the arithmetic [`Interpreter::unclip_redundant`] documents for a cell's box. The margin
    /// only has to be non-zero, since it is the shading's own alpha and the current clip that
    /// bound the mark; a sixteenth of the geometry's own extent is that in whatever units the
    /// shading states, and costs nothing because it paints nothing.
    fn shading_surface(&self, shading: &Shading) -> (Path, Transform) {
        /// The share of a mesh's own extent the outline is grown by on each side.
        const MARGIN: f32 = 1.0 / 16.0;

        let (corners, transform) = match shading.painting_bounds() {
            Some([x0, y0, x1, y1]) => {
                let margin = (x1 - x0).abs().max((y1 - y0).abs()) * MARGIN;
                (
                    [x0 - margin, y0 - margin, x1 + margin, y1 + margin],
                    shading.transform,
                )
            }
            None => (
                [0.0, 0.0, self.page.width, self.page.height],
                Transform::IDENTITY,
            ),
        };
        let [x0, y0, x1, y1] = corners;
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x0, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y1)));
        path.push(PathCommand::LineTo(Point::new(x0, y1)));
        path.push(PathCommand::Close);
        (path, transform)
    }
}
