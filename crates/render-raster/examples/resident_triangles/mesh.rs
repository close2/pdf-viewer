//! A page's display list as resident triangles: what is carried, and the geometry it becomes.
//!
//! Every carried mark becomes page-space triangles once, here, and nothing in this module
//! knows a magnification. A fill is the stencil-then-cover construction (Kokojima, Takahashi,
//! Ito and Takahashi, "Resolution independent rendering of deformable vector objects using
//! graphics hardware", 2006): a fan from each subpath's first point over its chords, one
//! triangle per quadratic piece whose inside is decided per sample by Loop and Blinn's
//! implicit form `u² − v` (2005), and one rectangle that paints where the stencil says the
//! path's winding number passes the fill rule of ISO 32000-2 §8.5.3.3.
//!
//! A cubic is not given to the device as a cubic: it is cut into quadratics at a page-space
//! tolerance fixed for the highest magnification the probe draws, so the triangles are the
//! same at every view and a view past that magnification would show the cut (RFC 0013 §4).

use std::sync::Arc;

use kurbo::{BezPath, PathEl};
use pdf_render::{
    BlendMode, Command, DisplayList, FillRule, ImageSource, Paint, Path, PathCommand, Transform,
};

/// One vertex: a page-space position, Loop–Blinn's `(u, v)`, and a straight-alpha colour.
pub(crate) const STRIDE: usize = 20;

/// The `(u, v)` a chord's fan triangle carries: `0² − 1 < 0`, so no sample is discarded.
const SOLID_UV: [f32; 2] = [0.0, 1.0];

/// What the walk found, by why each mark was or was not carried.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Census {
    pub(crate) fills: u32,
    pub(crate) even_odd: u32,
    pub(crate) strokes: u32,
    pub(crate) clipped: u32,
    pub(crate) translucent: u32,
    pub(crate) flattened_groups: u32,
    pub(crate) zero_width_strokes: u32,
    pub(crate) shaded: u32,
    pub(crate) images: u32,
    /// The decoded images' samples, which a textured quad would hold resident.
    pub(crate) image_samples: u64,
    /// Images decoded at the placement's own scale (§8.9.5.3's `/Interpolate` is not this),
    /// whose samples depend on the view.
    pub(crate) images_at_device_scale: u32,
    pub(crate) other_groups: u32,
    /// Why each of those groups was not a run: its alpha, a soft mask, a blend mode other
    /// than Normal, knockout, a blending colour space of its own.
    pub(crate) group_reasons: [u32; 5],
    /// Groups walked as runs at the caller's word although their compositing is not carried.
    pub(crate) groups_as_runs: u32,
    pub(crate) other_blend_or_mask: u32,
    pub(crate) other_commands: u32,
}

/// One run of marks with one paint and one rule whose rectangles are pairwise disjoint, so
/// that one stencil pass and one cover pass draw the run exactly as one pass per mark would.
#[derive(Debug, Clone)]
pub(crate) struct Batch {
    pub(crate) even_odd: bool,
    pub(crate) fan: std::ops::Range<u32>,
    pub(crate) curve: std::ops::Range<u32>,
    pub(crate) cover: std::ops::Range<u32>,
    /// An image's textured quad, drawn from `cover` with this texture, and no stencil.
    pub(crate) image: Option<u32>,
}

/// Everything the device holds for a page or a column of pages.
#[derive(Default)]
pub(crate) struct Mesh {
    pub(crate) fan: Vec<u8>,
    pub(crate) curve: Vec<u8>,
    pub(crate) cover: Vec<u8>,
    pub(crate) batches: Vec<Batch>,
    pub(crate) census: Census,
    pub(crate) quads: u64,
    /// The carried commands, clips removed, for the oracle to draw the same marks.
    pub(crate) carried: Vec<Command>,
    /// Walk into a group whose compositing is not carried, to price its geometry; such a
    /// page's oracle figure then measures geometry only (RFC 0013 §4).
    pub(crate) into_groups: bool,
    /// The decoded images, each a texture the device holds once.
    pub(crate) images: Vec<pdf_render::Image>,
}

impl Mesh {
    pub(crate) fn vertex_bytes(&self) -> usize {
        self.fan.len() + self.curve.len() + self.cover.len()
    }
}

/// A mark ready to tessellate: page-space geometry, rule and colour.
struct Mark {
    geometry: BezPath,
    even_odd: bool,
    colour: u32,
    /// An image's index in [`Mesh::images`]; `geometry` is then its quad, `colour` its alpha.
    image: Option<u32>,
}

/// Walks `list`, whose page space is mapped by `placement` into the mesh's space (a column
/// places its later pages below the first), appending what it carries.
pub(crate) fn append(mesh: &mut Mesh, list: &DisplayList, placement: Transform, tolerance: f64) {
    let mut marks = Vec::new();
    walk(list.commands(), placement, tolerance, &mut marks, mesh);
    let mut open: Option<(bool, u32, Vec<kurbo::Rect>)> = None;
    for mark in marks {
        if let Some(image) = mark.image {
            let at = count(&mesh.cover);
            quad(mesh, &mark);
            mesh.batches.push(Batch {
                even_odd: false,
                fan: 0..0,
                curve: 0..0,
                cover: at..count(&mesh.cover),
                image: Some(image),
            });
            open = None;
            continue;
        }
        let bounds = mark.geometry.control_box();
        let joins = open.as_ref().is_some_and(|(even_odd, colour, rects)| {
            *even_odd == mark.even_odd
                && *colour == mark.colour
                && rects.len() < 4096
                && rects.iter().all(|r| !touches(*r, bounds))
        });
        if !joins {
            mesh.batches.push(Batch {
                even_odd: mark.even_odd,
                fan: count(&mesh.fan)..count(&mesh.fan),
                curve: count(&mesh.curve)..count(&mesh.curve),
                cover: count(&mesh.cover)..count(&mesh.cover),
                image: None,
            });
            open = Some((mark.even_odd, mark.colour, Vec::new()));
        }
        tessellate(mesh, &mark, tolerance);
        if let Some((_, _, rects)) = open.as_mut() {
            rects.push(bounds);
        }
        let (fan, curve, cover) = (count(&mesh.fan), count(&mesh.curve), count(&mesh.cover));
        let batch = mesh.batches.last_mut().expect("a batch was opened above");
        batch.fan.end = fan;
        batch.curve.end = curve;
        batch.cover.end = cover;
    }
}

/// Whether two closed rectangles meet at all — a shared edge included, where a sample on it
/// could belong to both covers.
fn touches(a: kurbo::Rect, b: kurbo::Rect) -> bool {
    a.x0 <= b.x1 && b.x0 <= a.x1 && a.y0 <= b.y1 && b.y0 <= a.y1
}

#[expect(clippy::cast_possible_truncation)] // a page holds far fewer than 2^32 images
fn count_of(images: &[pdf_render::Image]) -> u32 {
    images.len() as u32
}

/// An image's two triangles. Its first sample row is the unit square's top edge (§8.9.4's
/// image space), so the texture's `v` runs opposite to the square's `y`.
fn quad(mesh: &mut Mesh, mark: &Mark) {
    let corners: Vec<kurbo::Point> = mark
        .geometry
        .elements()
        .iter()
        .filter_map(|element| match *element {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => Some(p),
            _ => None,
        })
        .collect();
    let uv = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    for index in [0, 1, 2, 0, 2, 3] {
        push(&mut mesh.cover, corners[index], uv[index], mark.colour);
    }
}

#[expect(clippy::cast_possible_truncation)] // a probe's buffers stay far under 4 GiB
fn count(bytes: &[u8]) -> u32 {
    (bytes.len() / STRIDE) as u32
}

fn walk(
    commands: &[Command],
    placement: Transform,
    tolerance: f64,
    marks: &mut Vec<Mark>,
    mesh: &mut Mesh,
) {
    for command in commands {
        if command.clip().is_some() {
            mesh.census.clipped += 1;
        }
        match command {
            Command::Fill {
                path,
                transform,
                fill_rule,
                paint,
                mask,
                blend,
                ..
            } => {
                let Some(colour) = solid(paint, *mask, *blend, &mut mesh.census) else {
                    continue;
                };
                mesh.census.fills += 1;
                let even_odd = *fill_rule == FillRule::EvenOdd;
                mesh.census.even_odd += u32::from(even_odd);
                marks.push(Mark {
                    geometry: bez(path, transform.then(placement)),
                    even_odd,
                    colour,
                    image: None,
                });
                keep(mesh, command);
            }
            Command::Stroke {
                path,
                transform,
                stroke,
                paint,
                mask,
                blend,
                ..
            } => {
                let Some(colour) = solid(paint, *mask, *blend, &mut mesh.census) else {
                    continue;
                };
                if stroke.width <= 0.0 {
                    // §8.4.3.2's thinnest line is a width in device pixels, so no page-space
                    // outline can carry it (RFC 0013 §4).
                    mesh.census.zero_width_strokes += 1;
                    continue;
                }
                mesh.census.strokes += 1;
                let to_page = transform.then(placement);
                let stretch = f64::from(to_page.max_stretch()).max(f64::MIN_POSITIVE);
                let local = tolerance / stretch;
                let source = bez(path, Transform::IDENTITY);
                let dashed = if stroke.dash_array.iter().sum::<f32>() > 0.0 {
                    let pattern: Vec<f64> =
                        stroke.dash_array.iter().copied().map(f64::from).collect();
                    let cut: BezPath = kurbo::dash(
                        source.elements().iter().copied(),
                        stroke.dash_phase.into(),
                        &pattern,
                    )
                    .collect();
                    cut
                } else {
                    source
                };
                let style = kurbo::Stroke::new(f64::from(stroke.width))
                    .with_caps(cap(stroke.cap))
                    .with_join(join(stroke.join))
                    .with_miter_limit(f64::from(stroke.miter_limit.max(1.0)));
                let mut outline =
                    kurbo::stroke(dashed.iter(), &style, &kurbo::StrokeOpts::default(), local);
                outline.apply_affine(affine(to_page));
                marks.push(Mark {
                    geometry: outline,
                    even_odd: false,
                    colour,
                    image: None,
                });
                keep(mesh, command);
            }
            Command::Group {
                commands,
                alpha,
                mask,
                blend,
                knockout,
                blending,
                ..
            } if *alpha >= 1.0
                && mask.is_none()
                && *blend == BlendMode::Normal
                && !*knockout
                && blending.is_none() =>
            {
                // §11.4's group with every parameter at its default composites as its
                // elements would alone: Normal over an isolated or a non-isolated backdrop
                // gives one result, so the group is walked as a run.
                mesh.census.flattened_groups += 1;
                walk(commands, placement, tolerance, marks, mesh);
            }
            Command::Group {
                commands,
                alpha,
                mask,
                blend,
                knockout,
                blending,
                ..
            } => {
                mesh.census.other_groups += 1;
                let reasons = [
                    *alpha < 1.0,
                    mask.is_some(),
                    *blend != BlendMode::Normal,
                    *knockout,
                    blending.is_some(),
                ];
                for (count, reason) in mesh.census.group_reasons.iter_mut().zip(reasons) {
                    *count += u32::from(reason);
                }
                if mesh.into_groups {
                    mesh.census.groups_as_runs += 1;
                    walk(commands, placement, tolerance, marks, mesh);
                }
            }
            Command::Image {
                image,
                transform,
                alpha,
                mask,
                blend,
                ..
            } => {
                mesh.census.images += 1;
                match image {
                    ImageSource::Decoded(decoded)
                        if mask.is_none() && *blend == BlendMode::Normal =>
                    {
                        mesh.census.image_samples +=
                            u64::from(decoded.width) * u64::from(decoded.height);
                        // §8.9.4: the image occupies its unit square, mapped by the CTM.
                        let to_page = affine(transform.then(placement));
                        let mut square = BezPath::new();
                        for (x, y) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
                            let corner = to_page * kurbo::Point::new(x, y);
                            if square.elements().is_empty() {
                                square.move_to(corner);
                            } else {
                                square.line_to(corner);
                            }
                        }
                        marks.push(Mark {
                            geometry: square,
                            even_odd: false,
                            colour: u32::from(unit_byte(*alpha)) << 24,
                            image: Some(count_of(&mesh.images)),
                        });
                        mesh.images.push(decoded.clone());
                        keep(mesh, command);
                    }
                    ImageSource::Decoded(_) => mesh.census.other_blend_or_mask += 1,
                    // `ImageSource` is non-exhaustive; every other source is sampled per view.
                    _ => mesh.census.images_at_device_scale += 1,
                }
            }
            _ => mesh.census.other_commands += 1,
        }
    }
}

/// The oracle draws exactly the carried marks, each without its clip, because the probe
/// draws none of them clipped (RFC 0013 §4 prices the clip apart).
fn keep(mesh: &mut Mesh, command: &Command) {
    let mut kept = command.clone();
    kept.set_clip(None);
    mesh.carried.push(kept);
}

fn solid(
    paint: &Paint,
    mask: Option<pdf_render::SoftMaskId>,
    blend: BlendMode,
    census: &mut Census,
) -> Option<u32> {
    let Paint::Solid(colour) = paint else {
        census.shaded += 1;
        return None;
    };
    if mask.is_some() || blend != BlendMode::Normal {
        census.other_blend_or_mask += 1;
        return None;
    }
    if colour.a < 1.0 {
        census.translucent += 1;
    }
    let byte = |channel: f32| u32::from(unit_byte(channel));
    Some(byte(colour.r) | byte(colour.g) << 8 | byte(colour.b) << 16 | byte(colour.a) << 24)
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to 0..=255 first
fn unit_byte(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn affine(t: Transform) -> kurbo::Affine {
    kurbo::Affine::new([t.a, t.b, t.c, t.d, t.e, t.f].map(f64::from))
}

fn bez(path: &Arc<Path>, to_page: Transform) -> BezPath {
    let map = |p: pdf_render::Point| {
        let q = to_page.apply(p);
        kurbo::Point::new(f64::from(q.x), f64::from(q.y))
    };
    let mut out = BezPath::new();
    for command in path.commands() {
        match *command {
            PathCommand::MoveTo(p) => out.move_to(map(p)),
            PathCommand::LineTo(p) => out.line_to(map(p)),
            PathCommand::CurveTo(a, b, c) => out.curve_to(map(a), map(b), map(c)),
            PathCommand::Close => out.close_path(),
        }
    }
    out
}

fn cap(cap: pdf_render::LineCap) -> kurbo::Cap {
    match cap {
        pdf_render::LineCap::Butt => kurbo::Cap::Butt,
        pdf_render::LineCap::Round => kurbo::Cap::Round,
        pdf_render::LineCap::Square => kurbo::Cap::Square,
    }
}

fn join(join: pdf_render::LineJoin) -> kurbo::Join {
    match join {
        pdf_render::LineJoin::Miter => kurbo::Join::Miter,
        pdf_render::LineJoin::Round => kurbo::Join::Round,
        pdf_render::LineJoin::Bevel => kurbo::Join::Bevel,
    }
}

#[expect(clippy::cast_possible_truncation)] // page-space coordinates, far inside f32's range
fn push(buffer: &mut Vec<u8>, p: kurbo::Point, uv: [f32; 2], colour: u32) {
    for value in [p.x as f32, p.y as f32, uv[0], uv[1]] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    buffer.extend_from_slice(&colour.to_le_bytes());
}

/// One mark's fan, curve triangles and cover rectangle.
///
/// Every subpath is closed, as §8.5.3.1 closes an open subpath before filling it; a stroke's
/// outline arrives closed already.
fn tessellate(mesh: &mut Mesh, mark: &Mark, tolerance: f64) {
    // The chord that closes a subpath ends at its anchor, so its fan triangle has no area and
    // no closing edge is drawn.
    let mut anchor = kurbo::Point::ZERO;
    let mut last = kurbo::Point::ZERO;
    let fan = |mesh: &mut Mesh, anchor: kurbo::Point, a: kurbo::Point, b: kurbo::Point| {
        if a != b && a != anchor && b != anchor {
            for p in [anchor, a, b] {
                push(&mut mesh.fan, p, SOLID_UV, 0);
            }
        }
    };
    let curve = |mesh: &mut Mesh, a: kurbo::Point, c: kurbo::Point, b: kurbo::Point| {
        mesh.quads += 1;
        push(&mut mesh.curve, a, [0.0, 0.0], 0);
        push(&mut mesh.curve, c, [0.5, 0.0], 0);
        push(&mut mesh.curve, b, [1.0, 1.0], 0);
    };
    for element in mark.geometry.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                anchor = p;
                last = p;
            }
            PathEl::LineTo(p) => {
                fan(mesh, anchor, last, p);
                last = p;
            }
            PathEl::QuadTo(c, p) => {
                fan(mesh, anchor, last, p);
                curve(mesh, last, c, p);
                last = p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                for (_, _, quad) in kurbo::CubicBez::new(last, c1, c2, p).to_quads(tolerance) {
                    fan(mesh, anchor, quad.p0, quad.p2);
                    curve(mesh, quad.p0, quad.p1, quad.p2);
                }
                last = p;
            }
            PathEl::ClosePath => last = anchor,
        }
    }
    let r = mark.geometry.control_box();
    let corners = [
        kurbo::Point::new(r.x0, r.y0),
        kurbo::Point::new(r.x1, r.y0),
        kurbo::Point::new(r.x1, r.y1),
        kurbo::Point::new(r.x0, r.y1),
    ];
    for index in [0, 1, 2, 0, 2, 3] {
        push(&mut mesh.cover, corners[index], SOLID_UV, mark.colour);
    }
}
