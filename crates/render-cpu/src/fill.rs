//! The fill kernel — ISO 32000-2 §8.5.3.3's filled area, painted with a colour, a shading or a
//! paint no shader expresses.
//!
//! The scan conversion itself is `crate::scan`'s and `crate::area`'s; this module decides what is
//! drawn into it.

use pdf_render::{Paint, Path, Transform};

use crate::compose::carries_coverage_as_alpha;
use crate::mask_cache::crop_to_mask;
use crate::surface::ToDevice;
use crate::{CpuRasterError, CpuRasterizer, convert, scan, shading};

impl CpuRasterizer {
    /// Draws `path` with a paint no shader can express, and says whether it did.
    ///
    /// Three of them, and each is a *paint* that becomes pixels rather than a gradient:
    ///
    /// - **A shading stating ISO 32000-2 §8.7.4.3 Table 77's `/Background`** answers a colour
    ///   outside its own bounds, which no spread mode expresses. §11.6.7 makes the wash and the
    ///   shading one painting operation — the pattern's implicit group is "filled with the
    ///   specified background colour before the sh operator is invoked" — so
    ///   [`pdf_render::ShadingRaster`] answers both per device pixel and the shape's own edge
    ///   does the compositing once. It is asked first because it holds for every kind.
    ///
    /// - **A mesh** carries a colour — or §8.7.4.5.5's parametric value — per triangle corner,
    ///   so it is rasterised by [`pdf_render::MeshRaster`] and drawn inside the shape rather
    ///   than as a paint over it.
    /// - **A radial whose two circles do not contain one another** is ISO 32000-2 §8.7.4.5.4's
    ///   cone, where a point can lie on two blend circles and the clause's "greatest value of
    ///   s" decides between them — including the case where the greater root is one `/Extend`
    ///   refuses. No two-point conical gradient expresses that, so the cone is evaluated
    ///   exactly and drawn as a raster; every other radial keeps the native gradient, which
    ///   [`shading::is_a_cone`] proves is enough there.
    ///
    /// Split out of [`CpuRasterizer::draw_fill`] because it is one question — is this paint a
    /// raster? — asked twice, and neither answer shares a line with what follows it there.
    fn filled_as_a_raster(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        (path, fill_rule): (&tiny_skia::Path, pdf_render::FillRule),
        (paint, blend): (&Paint, tiny_skia::BlendMode),
        to_device: ToDevice,
        (transform, clip): (Transform, scan::Clip<'_>),
    ) -> bool {
        let Paint::Shading(shading) = paint else {
            return false;
        };
        let at = convert::transform(to_device.of(transform));
        if shading.background.is_some() {
            shading::fill_with_background(
                pixmap,
                path,
                shading,
                to_device.of(Transform::IDENTITY),
                convert::fill_rule(fill_rule),
                at,
                clip,
                blend,
                self.anti_alias,
            );
            // Answered whether or not a raster came back: an empty one is a shape covering no
            // device pixel, and falling through to a shader that has refused this paint (see
            // `shading::shader`) would turn that into an error on the page.
            return true;
        }
        match shading.kind.as_ref() {
            pdf_render::ShadingKind::Mesh {
                triangles,
                patches,
                ramp,
            } => {
                shading::fill_mesh(
                    pixmap,
                    path,
                    triangles,
                    patches.as_ref(),
                    ramp.as_ref(),
                    to_device.of(shading.transform),
                    convert::fill_rule(fill_rule),
                    at,
                    clip,
                    blend,
                    self.anti_alias,
                );
                true
            }
            pdf_render::ShadingKind::Radial {
                start,
                start_radius,
                end,
                end_radius,
                ramp,
                extend,
            } => {
                shading::is_a_cone(*start, *start_radius, *end, *end_radius)
                    && shading::fill_radial(
                        pixmap,
                        path,
                        pdf_render::Radial {
                            start: *start,
                            start_radius: *start_radius,
                            end: *end,
                            end_radius: *end_radius,
                            ramp,
                            extend: *extend,
                        },
                        to_device.of(shading.transform),
                        convert::fill_rule(fill_rule),
                        at,
                        clip,
                        blend,
                        self.anti_alias,
                    )
            }
            _ => false,
        }
    }

    /// Draws a filled path, including the marks a shape with no area cannot make itself.
    ///
    /// Split out of [`CpuRasterizer::draw`] because ISO 32000-2 §10.7.4 turns one command into
    /// two draws: a subpath with no extent along one axis has zero area, so this rasteriser
    /// computes zero coverage for it at every placement and every scale, and the clause says
    /// no shape may disappear. What it marks instead is `pdf-render`'s geometry rather than
    /// this backend's hairline, so that the two backends cannot answer it differently — and
    /// the marks are filled under the non-zero rule whatever the command's own rule is,
    /// because a mark is a shape in its own right rather than part of the path's winding.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for a path the rasteriser rejects or a paint this backend
    /// does not implement.
    #[expect(
        clippy::too_many_arguments,
        reason = "the fields of one display list command, passed through rather than \
                  regrouped: a struct here would be a second spelling of `Command::Fill`"
    )]
    pub(crate) fn draw_fill(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        (source, fill_rule): (&Path, pdf_render::FillRule),
        transform: Transform,
        paint: &Paint,
        blend: tiny_skia::BlendMode,
        to_device: ToDevice,
        (clip, admits): (scan::Clip<'_>, Option<tiny_skia::Rect>),
    ) -> Result<(), CpuRasterError> {
        // §8.3.4 NOTE 3: a matrix with no inverse carries this path onto a line or a point, so
        // the *mark* is refused and the rest of the page is drawn — see `pdf_render::paint_space`,
        // which states that for all three backends (trap 2). Before any geometry, because none of
        // it would be used and because the raster route below would otherwise answer separately.
        let Some(page_to_path) = pdf_render::paint_space(transform) else {
            return Ok(());
        };
        let at = to_device.of(transform);
        let cropped = crop_to_mask(source, transform, to_device, admits);
        let source = cropped.as_ref().unwrap_or(source);
        let path = convert::path(source).ok_or(CpuRasterError::InvalidPath)?;

        if self.filled_as_a_raster(
            pixmap,
            (&path, fill_rule),
            (paint, blend),
            to_device,
            (transform, clip),
        ) {
            return Ok(());
        }

        // A sampled shading's pixels are borrowed by its shader, so they need somewhere to
        // live for exactly as long as this call.
        let mut scratch = None;
        let brush = self.paint(
            paint,
            blend,
            page_to_path,
            to_device.of(Transform::IDENTITY),
            (pixmap.width(), pixmap.height()),
            &mut scratch,
        )?;
        let split = pdf_render::split_collapsed_fill(source, at);
        if let Some(split) = &split
            && !split.marks.is_empty()
            && let Some(marks) = convert::path(&split.marks)
        {
            scan::fill(
                pixmap,
                &marks,
                &brush,
                tiny_skia::FillRule::Winding,
                convert::transform(at),
                clip,
            );
        }
        let remaining = match &split {
            // Every subpath collapsed, so the marks are the whole of the drawing.
            Some(split) if split.filled.is_empty() => return Ok(()),
            Some(split) => &split.filled,
            None => source,
        };
        // §10.7.4 again, for a shape this rasteriser's coverage quantum would round to nothing.
        if carries_coverage_as_alpha(brush.anti_alias, brush.blend_mode)
            && let Some(bands) = pdf_render::sub_pixel_bands(remaining, at, fill_rule)
        {
            for band in &bands {
                let shape = convert::path(&band.shape).ok_or(CpuRasterError::InvalidPath)?;
                let mut faint = brush.clone();
                faint.shader.apply_opacity(band.coverage);
                scan::fill(
                    pixmap,
                    &shape,
                    &faint,
                    tiny_skia::FillRule::Winding,
                    convert::transform(at),
                    clip,
                );
            }
            return Ok(());
        }
        let path = match &split {
            Some(split) => convert::path(&split.filled).ok_or(CpuRasterError::InvalidPath)?,
            None => path,
        };
        // §10.7.4 a third time, for the *edge* of a shape thicker than a pixel: an axis-aligned
        // rectangle's coverage is the product of its two overlaps, which `pdf_render::edge`
        // derives from the clause's own definition of a pixel and `scan::fill_rectangles` draws
        // exactly where this converter would round it to a quarter. §11.6.2 is what admits a path
        // stating *several* — they are portions of one object, so they may be drawn one at a time
        // only while no device pixel receives two of them (ADR 0583).
        let exact = rectangular_mark(remaining, at);
        if exact.is_some() {
            scan::fill_rectangles(
                pixmap,
                (&path, &exact),
                &brush,
                convert::transform(at),
                clip,
            );
            return Ok(());
        }
        scan::fill(
            pixmap,
            &path,
            &brush,
            convert::fill_rule(fill_rule),
            convert::transform(at),
            clip,
        );
        Ok(())
    }
}

/// Which device rectangles a fill's path is, if any — ISO 32000-2 §10.7.4 and §11.6.2.
///
/// One walk answers both questions: `pdf_render::device_rectangles` returns its one-rectangle
/// variant without allocating, which is the great majority of every corpus here
/// (`pdf-model/examples/rectangular_path_census`), and a `Vec` only for the paths that state
/// several. Asking two functions instead cost +0.18% of the rasteriser on a page of text, which is
/// the second walk over every fill the first one declined.
///
/// **The second half is `DeviceRectangles::share_a_device_pixel`**: §11.6.2 makes a path's subpaths
/// portions of one object and forbids compositing portions with one another, so a pixel reached by
/// two of them may not receive two compositing steps. That question decides which of the two
/// multi-rectangle variants the mark is rather than whether it is one at all — the portions'
/// coverages are summed into one buffer and blitted once (ADR 0590) where they may not be drawn
/// separately. Both questions live in the shared crate because both are decisions about the device,
/// which is trap 2's rule and [`scan::Exact`]'s own comment. ADR 0583.
pub(crate) fn rectangular_mark(path: &Path, at: Transform) -> scan::Exact {
    let Some(rectangles) = pdf_render::device_rectangles(path, at) else {
        return scan::Exact::Unknown;
    };
    let shared = rectangles.share_a_device_pixel();
    match rectangles {
        pdf_render::DeviceRectangles::One(rect) => {
            convert::to_skia_rect(rect).map_or(scan::Exact::Unknown, scan::Exact::One)
        }
        pdf_render::DeviceRectangles::Several(rects) => rects
            .into_iter()
            .map(convert::to_skia_rect)
            .collect::<Option<Vec<_>>>()
            .map_or(scan::Exact::Unknown, |rects| {
                if shared {
                    scan::Exact::Shared(rects)
                } else {
                    scan::Exact::Several(rects)
                }
            }),
    }
}
