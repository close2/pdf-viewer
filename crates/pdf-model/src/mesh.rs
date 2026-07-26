//! Reading mesh shadings (PDF types 4, 5, 6 and 7) into triangles.
//!
//! All four describe the same thing — an area of smoothly varying colour — and differ only
//! in how the file writes it down. Types 4 and 5 give triangles directly, as a strip with
//! edge flags or as a lattice of rows. Types 6 and 7 give Bézier *patches*: a Coons patch
//! is bounded by four cubic curves, and a tensor-product patch adds four interior control
//! points. A Coons patch is exactly a tensor patch whose interior points are implied by its
//! boundary, so both are converted to the tensor form and evaluated once.
//!
//! Everything leaves here as triangles with a colour at each corner, which is the one
//! representation a rasteriser can actually draw.
//!
//! # The data is a bit stream, not a byte stream
//!
//! Coordinates, colour components and edge flags each occupy a width the dictionary
//! chooses, and they are packed without regard for byte boundaries — except that each
//! *vertex* in a triangle mesh is padded to a whole number of bytes, and each patch is not.
//! Getting that padding wrong shifts every subsequent value by a few bits, which produces a
//! mesh that is plausible and wrong rather than one that fails.

use pdf_render::{Color, Point, Triangle};
use pdf_syntax::{Dictionary, Document, Stream};

use crate::colour::ColourSpace;
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

/// Reads a mesh shading's stream into triangles.
///
/// Returns `None` when the stream is unreadable or describes no triangles, which the
/// caller reports rather than drawing an empty shading.
pub(crate) fn read(
    document: &Document,
    stream: &Stream,
    kind: i64,
    space: &ColourSpace,
    functions: &[Function],
) -> Option<Vec<Triangle>> {
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
    };

    let mut bits = BitReader::new(&data);
    let triangles = match kind {
        4 => reader.free_form(&mut bits),
        5 => {
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
            reader.lattice(&mut bits, per_row)
        }
        6 | 7 => reader.patches(&mut bits, kind == 7),
        _ => return None,
    };

    (!triangles.is_empty()).then_some(triangles)
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

/// One vertex: where it is and what colour it carries.
#[derive(Debug, Clone, Copy)]
struct Vertex {
    point: Point,
    colour: Color,
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
}

impl MeshReader<'_> {
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

    fn read_colour(&self, bits: &mut BitReader<'_>) -> Option<Color> {
        let mut values = Vec::with_capacity(self.components);
        for index in 0..self.components {
            let raw = bits.read(self.component_bits)?;
            values.push(self.decode_at(index.checked_add(2)?, raw, self.component_bits));
        }
        Some(self.to_colour(&values))
    }

    /// Turns the stream's components into a colour, through the functions if there are any.
    fn to_colour(&self, values: &[f32]) -> Color {
        if self.functions.is_empty() {
            return self.space.to_rgb(values);
        }
        let mut components = Vec::new();
        for function in self.functions {
            components.extend(function.eval(values));
        }
        self.space.to_rgb(&components)
    }

    /// Reads a vertex, including the byte padding each one carries in a triangle mesh.
    fn read_vertex(&self, bits: &mut BitReader<'_>, with_flag: bool) -> Option<(u8, Vertex)> {
        let flag = if with_flag {
            // Only the low two bits of an edge flag are meaningful, whatever width the
            // dictionary gave it.
            u8::try_from(bits.read(self.flag_bits)? & 0b11).unwrap_or(0)
        } else {
            0
        };
        let point = self.read_point(bits)?;
        let colour = self.read_colour(bits)?;
        // "Each set of vertex data shall occupy a whole number of bytes."
        bits.align();
        Some((flag, Vertex { point, colour }))
    }

    /// Type 4: a strip whose edge flags say which two earlier vertices each triangle keeps.
    fn free_form(&self, bits: &mut BitReader<'_>) -> Vec<Triangle> {
        let mut triangles = Vec::new();
        // The previous two triangles' vertices, in the specification's `va`, `vb`, `vc`.
        let mut previous: Option<(Vertex, Vertex, Vertex)> = None;

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
    fn lattice(&self, bits: &mut BitReader<'_>, per_row: usize) -> Vec<Triangle> {
        let mut rows: Vec<Vec<Vertex>> = Vec::new();
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
    fn patches(&self, bits: &mut BitReader<'_>, tensor: bool) -> Vec<Triangle> {
        let boundary = 12usize;
        let total = if tensor { 16 } else { boundary };

        let mut triangles = Vec::new();
        let mut previous: Option<([Point; 16], [Color; 4])> = None;

        while triangles.len() < MAX_TRIANGLES {
            let Some(flag) = bits.read(self.flag_bits) else {
                break;
            };
            let flag = flag & 0b11;

            // A continuation reuses four points and two colours from the previous patch's
            // named edge, so only the rest is in the stream.
            let (mut points, mut colours, start, colour_start) = match (flag, previous) {
                (0, _) => ([Point::new(0.0, 0.0); 16], [Color::BLACK; 4], 0, 0),
                (_, Some((last, last_colours))) => {
                    let (edge, shared) = shared_edge(flag, &last, &last_colours);
                    let mut points = [Point::new(0.0, 0.0); 16];
                    for (slot, point) in points.iter_mut().zip(edge.iter()) {
                        *slot = *point;
                    }
                    let mut colours = [Color::BLACK; 4];
                    colours[0] = shared[0];
                    colours[1] = shared[1];
                    (points, colours, 4, 2)
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
            for slot in colours.iter_mut().skip(colour_start) {
                let Some(colour) = self.read_colour(bits) else {
                    complete = false;
                    break;
                };
                *slot = colour;
            }
            if !complete {
                break;
            }
            // A patch's data is *not* padded to a byte boundary, unlike a vertex's.

            let grid = control_grid(&points, tensor);
            triangles.extend(tessellate(&grid, &colours));
            previous = Some((points, colours));
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

/// The points and colours a continuation patch inherits from its predecessor.
///
/// The indices come straight from Table 84: flag 1 continues along the previous patch's
/// second edge, flag 2 its third, flag 3 its fourth.
fn shared_edge(flag: u32, points: &[Point; 16], colours: &[Color; 4]) -> ([Point; 4], [Color; 2]) {
    let pick = |indices: [usize; 4]| {
        [
            points[indices[0]],
            points[indices[1]],
            points[indices[2]],
            points[indices[3]],
        ]
    };
    match flag {
        1 => (pick([3, 4, 5, 6]), [colours[1], colours[2]]),
        2 => (pick([6, 7, 8, 9]), [colours[2], colours[3]]),
        _ => (pick([9, 10, 11, 0]), [colours[3], colours[0]]),
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
fn tessellate(grid: &[[Point; 4]; 4], colours: &[Color; 4]) -> Vec<Triangle> {
    let mut points = Vec::with_capacity(
        PATCH_STEPS
            .saturating_add(1)
            .saturating_mul(PATCH_STEPS.saturating_add(1)),
    );
    let mut corner_colours = Vec::with_capacity(points.capacity());

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
            corner_colours.push(bilinear(colours, u, v));
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
                colour: corner_colours.get(index).copied().unwrap_or(Color::BLACK),
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

/// The colour inside a patch, interpolated between its four corner colours.
fn bilinear(colours: &[Color; 4], u: f32, v: f32) -> Color {
    let mix = |a: Color, b: Color, t: f32| Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    };
    let top = mix(colours[0], colours[1], v);
    let bottom = mix(colours[3], colours[2], v);
    mix(top, bottom, u)
}

fn triangle(corners: (Vertex, Vertex, Vertex)) -> Triangle {
    let (a, b, c) = corners;
    Triangle {
        points: [a.point, b.point, c.point],
        colours: [a.colour, b.colour, c.colour],
    }
}
