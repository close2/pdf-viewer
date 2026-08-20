//! Reading mesh shadings (PDF types 4, 5, 6 and 7) into triangles.
//!
//! All four describe the same thing — an area of smoothly varying colour — and differ only
//! in how the file writes it down. Types 4 and 5 give triangles directly, as a strip with
//! edge flags or as a lattice of rows. Types 6 and 7 give Bézier *patches*: a Coons patch
//! is bounded by four cubic curves, and a tensor-product patch adds four interior control
//! points. A Coons patch is exactly a tensor patch whose interior points are implied by its
//! boundary, so both are converted to the tensor form and evaluated once.
//!
//! Everything leaves here as triangles, which is the one representation a rasteriser can
//! actually draw — carrying a colour at each corner, or, where the shading states a
//! `/Function`, the parametric value at each corner beside the sampled function.
//!
//! # Which of the two a mesh carries is a `shall` about an order
//!
//! §8.7.4.5.5: "[a]ll linear interpolation within the triangle mesh shall be done using the t
//! values. After interpolation, the results shall be passed to the function(s) specified in
//! the Function entry to determine the colour at each point." Evaluating the function at each
//! vertex and interpolating the colours it returns is the same picture only where the function
//! is a straight line, and nothing about the result says which was done — so the parameter is
//! what leaves here, and [`pdf_render::Corners`] is where the distinction is kept.
//!
//! # The data is a bit stream, not a byte stream
//!
//! Coordinates, colour components and edge flags each occupy a width the dictionary
//! chooses, and they are packed without regard for byte boundaries — except that each
//! *vertex* in a triangle mesh is padded to a whole number of bytes, and each patch is not.
//! Getting that padding wrong shifts every subsequent value by a few bits, which produces a
//! mesh that is plausible and wrong rather than one that fails.

use pdf_render::{Color, Corners, Point, Ramp, Triangle};
use pdf_syntax::{Dictionary, Document, Stream};

use crate::colour::{ColourSpace, Conversion};
use crate::function::{BitReader, Function};

/// How finely a Bézier patch is evaluated along each axis.
///
/// The geometry's accuracy is set here, because a backend that subdivides these triangles
/// further does so linearly and cannot recover curvature. Ten steps puts the error of a
/// patch spanning a whole page well under a pixel, at two hundred triangles per patch.
const PATCH_STEPS: usize = 10;

/// Most triangles one shading may produce.
///
/// A mesh stream is compressed, so a few kilobytes can describe an unbounded number of
/// patches. This is the decompression-bomb bound for shadings.
const MAX_TRIANGLES: usize = 1 << 18;

/// Reads a mesh shading's stream into triangles, with the ramp a parametric mesh needs.
///
/// The ramp is `Some` exactly where the shading states a `/Function`, which is exactly where
/// the triangles carry [`Corners::Parameters`]: §8.7.4.5.5 interpolates the parameter and
/// calls the function afterwards, so the function crosses into the display list as the
/// samples of itself that [`Ramp`] already is for an axial or a radial shading.
///
/// Returns `None` when the stream is unreadable or describes no triangles, which the
/// caller reports rather than drawing an empty shading.
pub(crate) fn read(
    document: &Document,
    stream: &Stream,
    kind: i64,
    space: &ColourSpace,
    functions: &[Function],
    resolution: usize,
    into: &Conversion,
) -> Option<(Vec<Triangle>, Option<Ramp>)> {
    let dict = &stream.dict;
    let data = document.decoded_stream_data(stream)?;

    let coordinate_bits = bits(
        document,
        dict,
        "BitsPerCoordinate",
        &[1, 2, 4, 8, 12, 16, 24, 32],
    )?;
    let component_bits = bits(document, dict, "BitsPerComponent", &[1, 2, 4, 8, 12, 16])?;
    // Type 5 is a lattice and carries no flags; the others need one per vertex or patch.
    let flag_bits = if kind == 5 {
        0
    } else {
        bits(document, dict, "BitsPerFlag", &[2, 4, 8])?
    };

    // When a shading has functions, the stream carries a single parameter per vertex
    // rather than a full colour, and the functions turn it into one.
    let components = if functions.is_empty() {
        space.components()
    } else {
        1
    };

    let decode = decode_ranges(document, dict)?;
    if decode.len() < components.checked_add(2)? {
        return None;
    }

    let reader = MeshReader {
        decode,
        components,
        coordinate_bits,
        component_bits,
        flag_bits,
        space,
        functions,
        into,
    };

    // Type 5's lattice width is read before the vertices, whichever of the two things a
    // vertex carries.
    let per_row = if kind == 5 {
        let per_row = usize::try_from(
            document
                .get_key(dict, "VerticesPerRow")
                .as_integer()
                .unwrap_or(0),
        )
        .ok()?;
        if per_row < 2 {
            return None;
        }
        per_row
    } else {
        0
    };

    let mut bits = BitReader::new(&data);
    // The two readings differ only in what a vertex carries, which is what the clause makes
    // the whole question: components, or the one parametric value the function takes.
    let (triangles, ramp) = if functions.is_empty() {
        (reader.triangles::<Color>(&mut bits, kind, per_row)?, None)
    } else {
        (
            reader.triangles::<f32>(&mut bits, kind, per_row)?,
            Some(reader.ramp(resolution)),
        )
    };

    (!triangles.is_empty()).then_some((triangles, ramp))
}

/// Reads a bit width, checking it against the values the specification permits.
fn bits(document: &Document, dict: &Dictionary, key: &str, allowed: &[i64]) -> Option<u32> {
    let value = document.get_key(dict, key).as_integer()?;
    allowed
        .contains(&value)
        .then(|| u32::try_from(value).ok())?
}

/// Reads `/Decode` as low/high pairs.
fn decode_ranges(document: &Document, dict: &Dictionary) -> Option<Vec<(f32, f32)>> {
    let array = document.get_key(dict, "Decode");
    let items = array.as_array()?;
    let values: Vec<f32> = items
        .iter()
        .filter_map(|item| document.resolve(item).as_number())
        .map(|value| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a decode bound outside f32's range is not a bound"
            )]
            {
                value as f32
            }
        })
        .collect();
    if values.is_empty() || !values.len().is_multiple_of(2) {
        return None;
    }
    Some(
        values
            .chunks_exact(2)
            .filter_map(|pair| Some((*pair.first()?, *pair.get(1)?)))
            .collect(),
    )
}

/// One vertex: where it is and what it carries — a colour, or §8.7.4.5.5's parameter.
#[derive(Debug, Clone, Copy)]
struct Vertex<C> {
    point: Point,
    corner: C,
}

/// What a vertex carries, and the two things the reader does with it.
///
/// A mesh states either a colour per vertex or one parametric value per vertex, and the
/// clause makes that a choice about *what is interpolated* rather than about a format. The
/// reading of the stream is identical either way — the flags, the lattice, the patches and
/// their shared edges — so the difference between the two lives here and nowhere else.
trait Corner: Copy {
    /// Reads one vertex's worth of the stream.
    fn read(reader: &MeshReader<'_>, bits: &mut BitReader<'_>) -> Option<Self>;

    /// The value `t` of the way from `self` to `other`, which a patch's interior needs.
    fn mix(self, other: Self, t: f32) -> Self;

    /// Three of these as the display list carries them.
    fn corners(values: [Self; 3]) -> Corners;

    /// A value for a patch slot the stream is about to fill, so that a patch's four corners
    /// can be an array before all four have been read.
    fn placeholder() -> Self;
}

impl Corner for Color {
    fn read(reader: &MeshReader<'_>, bits: &mut BitReader<'_>) -> Option<Self> {
        let mut values = Vec::with_capacity(reader.components);
        for index in 0..reader.components {
            let raw = bits.read(reader.component_bits)?;
            values.push(reader.decode_at(index.checked_add(2)?, raw, reader.component_bits));
        }
        Some(reader.into.paint(reader.space, &values))
    }

    fn mix(self, other: Self, t: f32) -> Self {
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    fn corners(values: [Self; 3]) -> Corners {
        Corners::Colours(values)
    }

    fn placeholder() -> Self {
        Color::BLACK
    }
}

impl Corner for f32 {
    fn read(reader: &MeshReader<'_>, bits: &mut BitReader<'_>) -> Option<Self> {
        let raw = bits.read(reader.component_bits)?;
        Some(reader.fraction_of_range(reader.decode_at(2, raw, reader.component_bits)))
    }

    fn mix(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    fn corners(values: [Self; 3]) -> Corners {
        Corners::Parameters(values)
    }

    fn placeholder() -> Self {
        0.0
    }
}

/// The parsed shape of a mesh stream, ready to read vertices from.
struct MeshReader<'a> {
    decode: Vec<(f32, f32)>,
    components: usize,
    coordinate_bits: u32,
    component_bits: u32,
    flag_bits: u32,
    space: &'a ColourSpace,
    functions: &'a [Function],
    /// How the mesh's vertex colours are converted (`crate::colour::Conversion`).
    into: &'a Conversion,
}

impl MeshReader<'_> {
    /// Reads the whole stream as one of the four mesh types, whichever thing a vertex carries.
    ///
    /// `per_row` is type 5's `/VerticesPerRow` and is read by nothing else.
    fn triangles<C: Corner>(
        &self,
        bits: &mut BitReader<'_>,
        kind: i64,
        per_row: usize,
    ) -> Option<Vec<Triangle>> {
        match kind {
            4 => Some(self.free_form::<C>(bits)),
            5 => Some(self.lattice::<C>(bits, per_row)),
            6 | 7 => Some(self.patches::<C>(bits, kind == 7)),
            _ => None,
        }
    }

    /// Maps a raw sample onto the range `/Decode` gives for that position.
    fn decode_at(&self, index: usize, raw: u32, width: u32) -> f32 {
        let (low, high) = self.decode.get(index).copied().unwrap_or((0.0, 1.0));
        let max = if width >= 32 {
            f64::from(u32::MAX)
        } else {
            f64::from((1u32 << width).saturating_sub(1))
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a sample is at most 32 bits, exact in f64 and bounded after scaling"
        )]
        let fraction = (f64::from(raw) / max) as f32;
        low + fraction * (high - low)
    }

    fn read_point(&self, bits: &mut BitReader<'_>) -> Option<Point> {
        let x = bits.read(self.coordinate_bits)?;
        let y = bits.read(self.coordinate_bits)?;
        Some(Point::new(
            self.decode_at(0, x, self.coordinate_bits),
            self.decode_at(1, y, self.coordinate_bits),
        ))
    }

    /// Where a decoded parametric value sits in the range `/Decode` gives it.
    ///
    /// Table 81 makes that range the function's input interval — "[e]ach input value shall be
    /// forced into the range interval specified for the corresponding colour component in the
    /// shading dictionary's Decode array" — and [`Self::ramp`] samples the function across it,
    /// so a corner carries its position in the range rather than the value itself. A range of
    /// zero width states one colour for the whole mesh, and every corner is at its start.
    fn fraction_of_range(&self, value: f32) -> f32 {
        let (low, high) = self.parameter_range();
        let span = high - low;
        if span.abs() <= f32::EPSILON {
            return 0.0;
        }
        (value - low) / span
    }

    /// The `/Decode` pair the parametric value is mapped onto, which is the third.
    fn parameter_range(&self) -> (f32, f32) {
        self.decode.get(2).copied().unwrap_or((0.0, 1.0))
    }

    /// The shading's `/Function`, sampled across the range `/Decode` gives its input.
    ///
    /// The same construction as an axial or radial shading's ramp, and for the same reason:
    /// a display list holds no PDF functions, so the function is evaluated here and crosses
    /// as samples. Its own discontinuities are sampled *across* rather than averaged over,
    /// which is what a type 3 stitching function with equal `/Bounds` needs.
    fn ramp(&self, resolution: usize) -> Ramp {
        let (low, high) = self.parameter_range();
        let span = high - low;
        let breaks = crate::shading::breakpoints_over(self.functions, low, high);
        Ramp::sample_across_at(resolution, &breaks, |t| {
            self.colour_of_parameter(low + t * span)
        })
    }

    /// The colour the shading's functions give one parametric value.
    fn colour_of_parameter(&self, parameter: f32) -> Color {
        let mut components = Vec::new();
        for function in self.functions {
            components.extend(function.eval(&[parameter]));
        }
        self.into.paint(self.space, &components)
    }

    /// Reads a vertex, including the byte padding each one carries in a triangle mesh.
    fn read_vertex<C: Corner>(
        &self,
        bits: &mut BitReader<'_>,
        with_flag: bool,
    ) -> Option<(u8, Vertex<C>)> {
        let flag = if with_flag {
            // Only the low two bits of an edge flag are meaningful, whatever width the
            // dictionary gave it.
            u8::try_from(bits.read(self.flag_bits)? & 0b11).unwrap_or(0)
        } else {
            0
        };
        let point = self.read_point(bits)?;
        let corner = C::read(self, bits)?;
        // "Each set of vertex data shall occupy a whole number of bytes."
        bits.align();
        Some((flag, Vertex { point, corner }))
    }

    /// Type 4: a strip whose edge flags say which two earlier vertices each triangle keeps.
    fn free_form<C: Corner>(&self, bits: &mut BitReader<'_>) -> Vec<Triangle> {
        let mut triangles = Vec::new();
        // The previous two triangles' vertices, in the specification's `va`, `vb`, `vc`.
        let mut previous: Option<(Vertex<C>, Vertex<C>, Vertex<C>)> = None;

        while triangles.len() < MAX_TRIANGLES {
            let Some((flag, vertex)) = self.read_vertex(bits, true) else {
                break;
            };
            let corners = match (flag, previous) {
                // A new triangle needs two more vertices, whose own flags are ignored.
                (0, _) => {
                    let Some((_, second)) = self.read_vertex(bits, true) else {
                        break;
                    };
                    let Some((_, third)) = self.read_vertex(bits, true) else {
                        break;
                    };
                    (vertex, second, third)
                }
                // Flag 1 keeps the previous triangle's `vb` and `vc`; flag 2 keeps `va`
                // and `vc`. Reversing these produces a mesh with folded triangles.
                (1, Some((_, b, c))) => (b, c, vertex),
                (2, Some((a, _, c))) => (a, c, vertex),
                // A continuation with nothing to continue is malformed.
                _ => break,
            };
            triangles.push(triangle(corners));
            previous = Some(corners);
        }
        triangles
    }

    /// Type 5: rows of a lattice, with triangles between consecutive rows.
    fn lattice<C: Corner>(&self, bits: &mut BitReader<'_>, per_row: usize) -> Vec<Triangle> {
        let mut rows: Vec<Vec<Vertex<C>>> = Vec::new();
        loop {
            let mut row = Vec::with_capacity(per_row);
            for _ in 0..per_row {
                let Some((_, vertex)) = self.read_vertex(bits, false) else {
                    break;
                };
                row.push(vertex);
            }
            if row.len() < per_row {
                break;
            }
            rows.push(row);
            if rows.len().saturating_mul(per_row) > MAX_TRIANGLES {
                break;
            }
        }

        let mut triangles = Vec::new();
        for pair in rows.windows(2) {
            let (upper, lower) = (&pair[0], &pair[1]);
            for column in 0..per_row.saturating_sub(1) {
                let next = column.saturating_add(1);
                let (Some(a), Some(b), Some(c), Some(d)) = (
                    upper.get(column),
                    upper.get(next),
                    lower.get(column),
                    lower.get(next),
                ) else {
                    continue;
                };
                triangles.push(triangle((*a, *b, *c)));
                triangles.push(triangle((*b, *d, *c)));
            }
        }
        triangles
    }

    /// Types 6 and 7: Bézier patches, evaluated into triangles.
    fn patches<C: Corner>(&self, bits: &mut BitReader<'_>, tensor: bool) -> Vec<Triangle> {
        let boundary = 12usize;
        let total = if tensor { 16 } else { boundary };

        let mut triangles = Vec::new();
        let mut previous: Option<([Point; 16], [C; 4])> = None;

        while triangles.len() < MAX_TRIANGLES {
            let Some(flag) = bits.read(self.flag_bits) else {
                break;
            };
            let flag = flag & 0b11;

            // A continuation reuses four points and two corners from the previous patch's
            // named edge, so only the rest is in the stream.
            let (mut points, mut corners, start, corner_start) = match (flag, previous) {
                (0, _) => ([Point::new(0.0, 0.0); 16], [C::placeholder(); 4], 0, 0),
                (_, Some((last, last_corners))) => {
                    let (edge, shared) = shared_edge(flag, &last, &last_corners);
                    let mut points = [Point::new(0.0, 0.0); 16];
                    for (slot, point) in points.iter_mut().zip(edge.iter()) {
                        *slot = *point;
                    }
                    let mut corners = [C::placeholder(); 4];
                    corners[0] = shared[0];
                    corners[1] = shared[1];
                    (points, corners, 4, 2)
                }
                // A continuation with no previous patch is malformed.
                _ => break,
            };

            let mut complete = true;
            for slot in points.iter_mut().take(total).skip(start) {
                let Some(point) = self.read_point(bits) else {
                    complete = false;
                    break;
                };
                *slot = point;
            }
            if !complete {
                break;
            }
            for slot in corners.iter_mut().skip(corner_start) {
                let Some(corner) = C::read(self, bits) else {
                    complete = false;
                    break;
                };
                *slot = corner;
            }
            if !complete {
                break;
            }
            // A patch's data is *not* padded to a byte boundary, unlike a vertex's.

            let grid = control_grid(&points, tensor);
            triangles.extend(tessellate(&grid, &corners));
            previous = Some((points, corners));
        }
        triangles
    }
}

impl BitReader<'_> {
    /// Advances to the next byte boundary.
    fn align(&mut self) {
        let over = self.position() % 8;
        if over != 0 {
            let padding = 8usize.saturating_sub(over);
            let _ = self.read(u32::try_from(padding).unwrap_or(0));
        }
    }
}

/// The points and corners a continuation patch inherits from its predecessor.
///
/// The indices come straight from Table 84: flag 1 continues along the previous patch's
/// second edge, flag 2 its third, flag 3 its fourth.
fn shared_edge<C: Corner>(
    flag: u32,
    points: &[Point; 16],
    corners: &[C; 4],
) -> ([Point; 4], [C; 2]) {
    let pick = |indices: [usize; 4]| {
        [
            points[indices[0]],
            points[indices[1]],
            points[indices[2]],
            points[indices[3]],
        ]
    };
    match flag {
        1 => (pick([3, 4, 5, 6]), [corners[1], corners[2]]),
        2 => (pick([6, 7, 8, 9]), [corners[2], corners[3]]),
        _ => (pick([9, 10, 11, 0]), [corners[3], corners[0]]),
    }
}

/// Arranges a patch's control points into the 4×4 grid a tensor surface is evaluated over.
///
/// The stream gives the twelve boundary points anticlockwise from one corner, then — for a
/// tensor patch — the four interior ones. A Coons patch has no interior points, and the
/// specification defines them in terms of the boundary: the surface a Coons patch describes
/// *is* a tensor patch with these interiors, so both are drawn by one piece of code.
fn control_grid(points: &[Point; 16], tensor: bool) -> [[Point; 4]; 4] {
    let mut grid = [[Point::new(0.0, 0.0); 4]; 4];
    // The boundary, in the order the stream gives it.
    let edge = [
        (0, 0, 0),
        (1, 0, 1),
        (2, 0, 2),
        (3, 0, 3),
        (4, 1, 3),
        (5, 2, 3),
        (6, 3, 3),
        (7, 3, 2),
        (8, 3, 1),
        (9, 3, 0),
        (10, 2, 0),
        (11, 1, 0),
    ];
    for (source, row, column) in edge {
        grid[row][column] = points[source];
    }

    if tensor {
        grid[1][1] = points[12];
        grid[1][2] = points[13];
        grid[2][2] = points[14];
        grid[2][1] = points[15];
        return grid;
    }

    // The Coons interior, from the specification's own construction.
    let blend = |a: Point, b: Point, c: Point, d: Point, e: Point, f: Point, g: Point| {
        Point::new(
            (-4.0 * a.x + 6.0 * (b.x + c.x) - 2.0 * (d.x + e.x) + 3.0 * (f.x + g.x)) / 9.0,
            (-4.0 * a.y + 6.0 * (b.y + c.y) - 2.0 * (d.y + e.y) + 3.0 * (f.y + g.y)) / 9.0,
        )
    };
    let subtract = |point: Point, other: Point| Point::new(point.x - other.x, point.y - other.y);

    grid[1][1] = subtract(
        blend(
            grid[0][0], grid[0][1], grid[1][0], grid[0][3], grid[3][0], grid[3][1], grid[1][3],
        ),
        Point::new(grid[3][3].x / 9.0, grid[3][3].y / 9.0),
    );
    grid[1][2] = subtract(
        blend(
            grid[0][3], grid[0][2], grid[1][3], grid[0][0], grid[3][3], grid[3][2], grid[1][0],
        ),
        Point::new(grid[3][0].x / 9.0, grid[3][0].y / 9.0),
    );
    grid[2][1] = subtract(
        blend(
            grid[3][0], grid[3][1], grid[2][0], grid[3][3], grid[0][0], grid[0][1], grid[2][3],
        ),
        Point::new(grid[0][3].x / 9.0, grid[0][3].y / 9.0),
    );
    grid[2][2] = subtract(
        blend(
            grid[3][3], grid[3][2], grid[2][3], grid[3][0], grid[0][3], grid[0][2], grid[2][0],
        ),
        Point::new(grid[0][0].x / 9.0, grid[0][0].y / 9.0),
    );
    grid
}

/// Evaluates a bicubic Bézier surface into triangles.
fn tessellate<C: Corner>(grid: &[[Point; 4]; 4], patch: &[C; 4]) -> Vec<Triangle> {
    let mut points = Vec::with_capacity(
        PATCH_STEPS
            .saturating_add(1)
            .saturating_mul(PATCH_STEPS.saturating_add(1)),
    );
    let mut corners = Vec::with_capacity(points.capacity());

    for row in 0..=PATCH_STEPS {
        for column in 0..=PATCH_STEPS {
            #[expect(
                clippy::cast_precision_loss,
                reason = "PATCH_STEPS is a small constant"
            )]
            let (u, v) = (
                row as f32 / PATCH_STEPS as f32,
                column as f32 / PATCH_STEPS as f32,
            );
            points.push(surface(grid, u, v));
            // The corners are `c1` at (0,0), `c2` at (0,1), `c3` at (1,1), `c4` at (1,0),
            // matching the order the control points visit them.
            corners.push(bilinear(patch, u, v));
        }
    }

    let stride = PATCH_STEPS.saturating_add(1);
    let mut triangles =
        Vec::with_capacity(PATCH_STEPS.saturating_mul(PATCH_STEPS).saturating_mul(2));
    for row in 0..PATCH_STEPS {
        for column in 0..PATCH_STEPS {
            let at = |r: usize, c: usize| r.saturating_mul(stride).saturating_add(c);
            let (a, b, c, d) = (
                at(row, column),
                at(row, column.saturating_add(1)),
                at(row.saturating_add(1), column),
                at(row.saturating_add(1), column.saturating_add(1)),
            );
            let corner = |index: usize| Vertex {
                point: points.get(index).copied().unwrap_or(Point::new(0.0, 0.0)),
                corner: corners.get(index).copied().unwrap_or_else(C::placeholder),
            };
            triangles.push(triangle((corner(a), corner(b), corner(c))));
            triangles.push(triangle((corner(b), corner(d), corner(c))));
        }
    }
    triangles
}

/// A point on the bicubic Bézier surface the control grid defines.
fn surface(grid: &[[Point; 4]; 4], u: f32, v: f32) -> Point {
    let bu = bernstein(u);
    let bv = bernstein(v);
    let mut x = 0.0;
    let mut y = 0.0;
    for (row, weights) in grid.iter().zip(bu.iter()) {
        for (point, weight) in row.iter().zip(bv.iter()) {
            x += point.x * weights * weight;
            y += point.y * weights * weight;
        }
    }
    Point::new(x, y)
}

/// The four cubic Bernstein basis values at `t`.
fn bernstein(t: f32) -> [f32; 4] {
    let s = 1.0 - t;
    [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t]
}

/// What a patch's interior carries, interpolated between its four corners.
///
/// Bilinear in whichever quantity the corners hold, which is the same rule §8.7.4.5.5 states
/// for a triangle: where the corners are parameters, this interpolates the parameter and the
/// function is called afterwards, at each device pixel, by the rasteriser.
fn bilinear<C: Corner>(corners: &[C; 4], u: f32, v: f32) -> C {
    let top = corners[0].mix(corners[1], v);
    let bottom = corners[3].mix(corners[2], v);
    top.mix(bottom, u)
}

fn triangle<C: Corner>(vertices: (Vertex<C>, Vertex<C>, Vertex<C>)) -> Triangle {
    let (a, b, c) = vertices;
    Triangle {
        points: [a.point, b.point, c.point],
        corners: C::corners([a.corner, b.corner, c.corner]),
    }
}
