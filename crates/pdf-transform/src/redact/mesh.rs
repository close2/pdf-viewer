//! A mesh shading's data under a redaction region, ISO 32000-2 §8.7.4.5.5 to §8.7.4.5.8 and
//! §12.5.6.23 (ADR 1363).
//!
//! A mesh states its colours at places: "[t]he colour at each vertex of the triangles is
//! specified" (§8.7.4.5.5), and a patch's "[c]olour values shall be given for the four corners of
//! the patch" (§8.7.4.5.7). Those are data located in the plane, as an image's samples are, so a
//! redaction destroys the ones in the region rather than clipping them away — "clipping or image
//! masks shall not be used to hide that data" — and keeps the ones outside it, which are marks the
//! annotation did not identify.
//!
//! # Triangles are cut exactly
//!
//! A Gouraud triangle's colour is the linear interpolation of its three vertices' colours — "a
//! technique known as Gouraud interpolation is used to colour the interiors", done on the
//! parameter where a `/Function` is present ("[a]ll linear interpolation within the triangle mesh
//! shall be done using the t values") — so it is an affine function over the triangle, and any
//! piece of the triangle is the same function over a smaller triangle. Each triangle is therefore
//! cut to the region's complement by the nine-cell construction a painted path is cut by
//! ([`super::paths::subtract`]), each surviving convex piece fanned into triangles, and each new
//! vertex given the colour the source triangle's own interpolation states there. Outside the region
//! every point is coloured exactly as before; inside it no triangle remains.
//!
//! # Patches are removed a sub-patch at a time
//!
//! A patch's geometry is a bicubic surface and the region's edge is not a curve of constant
//! parameter on it, so no finite set of patches is the patch minus the region. A tensor-product
//! patch is, though, exactly the union of the four sub-patches de Casteljau's construction splits
//! it into at `u = ½` and `v = ½`, each carrying the bilinear interpolation of the parent's corner
//! colours at its own corners ("bilinear interpolation is used to fill in colours over the entire
//! unit square"). So a patch whose control points' box meets the region is split, and split
//! again, and a sub-patch is kept whole where its box is clear of the region, dropped where it is
//! not, and split further while it is larger than [`LEAF_EXTENT`]. The box contains the surface,
//! which lies in its control points' convex hull, so nothing kept reaches the region. What is
//! removed past the region's edge is at most one leaf wide: the sub-patch is the unit of removal
//! here as a code is for text, and "within the region" is the same bounding-box intersection
//! (ADR 1124). Type 6 is written back as type 7 — "[t]he Coons patch (Type 6) is actually a special
//! case of the tensor-product patch (Type 7)" — because a sub-patch of a Coons patch is a tensor
//! patch whose interior points the Coons formula need not give.
//!
//! §8.7.4.5.7 settles a patch that folds over itself by "the largest value of v", then of `u`, and
//! the leaves of one patch are written in that order, so the precedence is kept to a leaf's
//! resolution: a fold inside one leaf is still that leaf's, and one across two is ordered by the
//! leaves' own lowest `v` and `u`.
//!
//! # How the survivors are written
//!
//! A cut creates vertices the source did not hold, at places and colours its coordinate and
//! component grids may not reach. So the mesh is written back as a stream of the same family — a
//! lattice (type 5) as the free-form mesh (type 4) it is a special case of, since a lattice cannot
//! state a hole — at 32 bits a coordinate and 16 a component, under the producer's own `/Decode`.
//! A coordinate then moves by at most half a step of a 32-bit grid, and [`Encoded::margin_holds`]
//! refuses the page unless that, and a reader's single-precision decode, stay inside the region's
//! pad (ADR 1195's margin). Every entry of the dictionary but the grid widths, the flag width and
//! the type is the producer's.

use kurbo::{BezPath, PathEl, Point, Shape};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, Stream};

use super::paths::{self, Mapping, REGION_PAD};

/// The largest a sub-patch is left before it is kept or dropped whole, in the display list's
/// units: a quarter of a point, half a pixel at the 150 dots to the inch the command line renders
/// at, so what a patch loses past the region's edge is below what a device resolves.
const LEAF_EXTENT: f64 = 0.25;

/// How many times a patch is halved in each direction before a sub-patch still meeting the region
/// is dropped whatever its size.
const MAX_SPLITS: usize = 12;

/// The coordinate width the survivors are written at.
const COORDINATE_BITS: u32 = 32;

/// The component width the survivors are written at.
const COMPONENT_BITS: u32 = 16;

/// One vertex of a triangle: where it is in the shading's space, and its decoded colour
/// components, or its one parameter where the shading states a `/Function`.
#[derive(Clone, Debug)]
pub(super) struct Vertex {
    pub(super) point: Point,
    pub(super) colour: Vec<f64>,
}

/// One tensor-product patch: its control net `grid[row][column]`, the order the stream's points
/// are laid into it the same as the reader's, and its corners' colours at `(u, v)` = (0, 0),
/// (0, 1), (1, 1) and (1, 0).
#[derive(Clone, Debug)]
pub(super) struct Patch {
    pub(super) grid: [[Point; 4]; 4],
    pub(super) corners: [Vec<f64>; 4],
}

/// A mesh shading's data, decoded.
pub(super) struct Mesh {
    /// Table 77's `/ShadingType`: 4, 5, 6 or 7.
    pub(super) kind: i64,
    /// The `/Decode` pairs: `x`, `y`, then one per colour value a vertex carries.
    decode: Vec<(f64, f64)>,
    /// The triangles, for types 4 and 5, in the stream's order.
    pub(super) triangles: Vec<[Vertex; 3]>,
    /// The patches, for types 6 and 7, in the stream's order.
    pub(super) patches: Vec<Patch>,
}

/// Where the region is, for one placement of the shading: the map from the shading's space into
/// the display list's, and the region boxes in the display list's space.
pub(super) struct Placement {
    pub(super) to_display: Mapping,
    pub(super) regions: Vec<[f64; 4]>,
}

/// Reads a mesh shading's stream, or the refusal by name.
pub(super) fn read(document: &Document, stream: &Stream) -> Result<Mesh, String> {
    let dict = &stream.dict;
    let refused = |why: &str| {
        format!(
            "§8.7.4.5.5: a mesh shading meets the region and {why}; the page is refused rather \
             than its data left in place"
        )
    };
    let kind = document
        .get_key(dict, "ShadingType")
        .as_integer()
        .ok_or_else(|| refused("states no /ShadingType"))?;
    let width = |key: &str, allowed: &[i64]| {
        document
            .get_key(dict, key)
            .as_integer()
            .filter(|bits| allowed.contains(bits))
            .and_then(|bits| u32::try_from(bits).ok())
    };
    let coordinate = width("BitsPerCoordinate", &[1, 2, 4, 8, 12, 16, 24, 32])
        .ok_or_else(|| refused("its /BitsPerCoordinate is not one Table 81 allows"))?;
    let component = width("BitsPerComponent", &[1, 2, 4, 8, 12, 16])
        .ok_or_else(|| refused("its /BitsPerComponent is not one Table 81 allows"))?;
    let flag = if kind == 5 {
        0
    } else {
        width("BitsPerFlag", &[2, 4, 8])
            .ok_or_else(|| refused("its /BitsPerFlag is not one Table 81 allows"))?
    };
    let decode: Vec<(f64, f64)> = document
        .get_key(dict, "Decode")
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| document.resolve(item).as_number())
                .collect::<Vec<f64>>()
        })
        .unwrap_or_default()
        .chunks_exact(2)
        .map(|pair| (pair[0], pair[1]))
        .collect();
    if decode.len() < 3 {
        return Err(refused(
            "its /Decode does not state a pair for x, y and a colour",
        ));
    }
    let data = document
        .decoded_stream_data(stream)
        .ok_or_else(|| refused("its stream does not decode"))?;
    let mut reader = Reader {
        data: &data,
        at: 0,
        coordinate,
        component,
        decode: &decode,
    };
    let mut mesh = Mesh {
        kind,
        decode: decode.clone(),
        triangles: Vec::new(),
        patches: Vec::new(),
    };
    match kind {
        4 => read_free_form(&mut reader, flag, &mut mesh.triangles)?,
        5 => {
            let per_row = document
                .get_key(dict, "VerticesPerRow")
                .as_integer()
                .and_then(|count| usize::try_from(count).ok())
                .filter(|count| *count >= 2)
                .ok_or_else(|| refused("its /VerticesPerRow is not a count of two or more"))?;
            read_lattice(&mut reader, per_row, &mut mesh.triangles)?;
        }
        6 | 7 => read_patches(&mut reader, flag, kind == 7, &mut mesh.patches)?,
        _ => return Err(refused("its /ShadingType is not a mesh")),
    }
    Ok(mesh)
}

/// §8.7.4.5.5's free-form stream: each vertex's edge flag says which earlier two it joins.
fn read_free_form(
    reader: &mut Reader<'_>,
    flag_bits: u32,
    out: &mut Vec<[Vertex; 3]>,
) -> Result<(), String> {
    let mut last: Option<[Vertex; 3]> = None;
    while let Some((flag, vertex)) = reader.vertex(flag_bits) {
        let triangle = match (flag, last.take()) {
            (0, _) | (_, None) => {
                let (Some((_, b)), Some((_, c))) =
                    (reader.vertex(flag_bits), reader.vertex(flag_bits))
                else {
                    break;
                };
                [vertex, b, c]
            }
            (1, Some([_, b, c])) => [b, c, vertex],
            (_, Some([a, _, c])) => [a, c, vertex],
        };
        if out.len() >= pdf_model::mesh::MAX_TRIANGLES {
            return Err(
                "§8.7.4.5.5: a mesh shading meeting the region states more triangles than this \
                 build reads; the page is refused"
                    .to_owned(),
            );
        }
        out.push(triangle.clone());
        last = Some(triangle);
    }
    Ok(())
}

/// §8.7.4.5.6's lattice: rows of `/VerticesPerRow` vertices, each quad of neighbours two
/// triangles.
fn read_lattice(
    reader: &mut Reader<'_>,
    per_row: usize,
    out: &mut Vec<[Vertex; 3]>,
) -> Result<(), String> {
    let mut rows: Vec<Vec<Vertex>> = Vec::new();
    'rows: loop {
        let mut row = Vec::with_capacity(per_row);
        for _ in 0..per_row {
            let Some((_, vertex)) = reader.vertex(0) else {
                break 'rows;
            };
            row.push(vertex);
        }
        rows.push(row);
    }
    for pair in rows.windows(2) {
        let (above, below) = (&pair[0], &pair[1]);
        for column in 0..per_row.saturating_sub(1) {
            if out.len().saturating_add(2) > pdf_model::mesh::MAX_TRIANGLES {
                return Err(
                    "§8.7.4.5.6: a lattice shading meeting the region states more triangles than \
                     this build reads; the page is refused"
                        .to_owned(),
                );
            }
            let next = column.saturating_add(1);
            out.push([
                above[column].clone(),
                above[next].clone(),
                below[column].clone(),
            ]);
            out.push([
                above[next].clone(),
                below[column].clone(),
                below[next].clone(),
            ]);
        }
    }
    Ok(())
}

/// §8.7.4.5.7's and §8.7.4.5.8's patch streams: twelve or sixteen points and four corners, of
/// which a continuation inherits four and two from the previous patch's named edge (Tables 84
/// and 85). A patch is not padded to a byte, as a triangle's vertex is.
fn read_patches(
    reader: &mut Reader<'_>,
    flag_bits: u32,
    tensor: bool,
    out: &mut Vec<Patch>,
) -> Result<(), String> {
    let total = if tensor { 16 } else { 12 };
    let mut previous: Option<([Point; 16], [Vec<f64>; 4])> = None;
    while let Some(flag) = reader.bits(flag_bits) {
        let flag = flag & 0b11;
        let mut points = [Point::ZERO; 16];
        let mut corners: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::new());
        let (start, corner_start) = match (flag, previous.as_ref()) {
            (0, _) => (0, 0),
            (_, Some((last, last_corners))) => {
                let (edge, shared) = match flag {
                    1 => ([3, 4, 5, 6], [1, 2]),
                    2 => ([6, 7, 8, 9], [2, 3]),
                    _ => ([9, 10, 11, 0], [3, 0]),
                };
                for (slot, index) in points.iter_mut().zip(edge) {
                    *slot = last[index];
                }
                corners[0].clone_from(&last_corners[shared[0]]);
                corners[1].clone_from(&last_corners[shared[1]]);
                (4, 2)
            }
            (_, None) => break,
        };
        let mut complete = true;
        for slot in points.iter_mut().take(total).skip(start) {
            if let Some(point) = reader.point() {
                *slot = point;
            } else {
                complete = false;
                break;
            }
        }
        for slot in corners.iter_mut().skip(corner_start) {
            if !complete {
                break;
            }
            match reader.colour() {
                Some(colour) => *slot = colour,
                None => complete = false,
            }
        }
        if !complete {
            break;
        }
        if out.len() >= pdf_model::mesh::MAX_PATCHES {
            return Err(
                "§8.7.4.5.7: a patch mesh meeting the region states more patches than this build \
                 reads; the page is refused"
                    .to_owned(),
            );
        }
        out.push(Patch {
            grid: control_grid(&points, tensor),
            corners: corners.clone(),
        });
        previous = Some((points, corners));
    }
    Ok(())
}

/// The stream's points laid into the control net, as the reader lays them; a Coons patch's four
/// interior points are the ones §8.7.4.5.8 defines from its boundary.
fn control_grid(points: &[Point; 16], tensor: bool) -> [[Point; 4]; 4] {
    let mut grid = [[Point::ZERO; 4]; 4];
    for (source, (row, column)) in BOUNDARY.iter().enumerate() {
        grid[*row][*column] = points[source];
    }
    if tensor {
        for ((row, column), point) in INTERIOR.iter().zip(points.iter().skip(12)) {
            grid[*row][*column] = *point;
        }
        return grid;
    }
    let p = |row: usize, column: usize| grid[row][column];
    // §8.7.4.5.8's interior points, each `(−4a + 6(b + c) − 2(d + e) + 3(f + g) − h) / 9` of
    // boundary points, taken one coordinate at a time.
    let interior = |points: [Point; 8]| {
        let blend = |value: &dyn Fn(Point) -> f64| {
            let v = points.map(value);
            ((-4.0f64).mul_add(v[0], 6.0 * (v[1] + v[2])) - 2.0 * (v[3] + v[4])
                + 3.0 * (v[5] + v[6])
                - v[7])
                / 9.0
        };
        Point::new(blend(&|point| point.x), blend(&|point| point.y))
    };
    let p11 = interior([
        p(0, 0),
        p(0, 1),
        p(1, 0),
        p(0, 3),
        p(3, 0),
        p(3, 1),
        p(1, 3),
        p(3, 3),
    ]);
    let p12 = interior([
        p(0, 3),
        p(0, 2),
        p(1, 3),
        p(0, 0),
        p(3, 3),
        p(3, 2),
        p(1, 0),
        p(3, 0),
    ]);
    let p21 = interior([
        p(3, 0),
        p(3, 1),
        p(2, 0),
        p(3, 3),
        p(0, 0),
        p(0, 1),
        p(2, 3),
        p(0, 3),
    ]);
    let p22 = interior([
        p(3, 3),
        p(3, 2),
        p(2, 3),
        p(3, 0),
        p(0, 3),
        p(0, 2),
        p(2, 0),
        p(0, 0),
    ]);
    grid[1][1] = p11;
    grid[1][2] = p12;
    grid[2][1] = p21;
    grid[2][2] = p22;
    grid
}

/// Where each of the twelve boundary points a patch stream states lands in the control net.
const BOUNDARY: [(usize, usize); 12] = [
    (0, 0),
    (0, 1),
    (0, 2),
    (0, 3),
    (1, 3),
    (2, 3),
    (3, 3),
    (3, 2),
    (3, 1),
    (3, 0),
    (2, 0),
    (1, 0),
];

/// Where each of a tensor patch's four interior points lands.
const INTERIOR: [(usize, usize); 4] = [(1, 1), (1, 2), (2, 2), (2, 1)];

/// A most-significant-bit-first reader over a mesh's stream, decoding values through `/Decode`.
struct Reader<'a> {
    data: &'a [u8],
    /// The bit position.
    at: usize,
    coordinate: u32,
    component: u32,
    decode: &'a [(f64, f64)],
}

impl Reader<'_> {
    /// The next `count` bits as an integer, or `None` past the end.
    fn bits(&mut self, count: u32) -> Option<u64> {
        let mut value = 0u64;
        for _ in 0..count {
            let byte = self.data.get(self.at / 8)?;
            let bit = (byte >> 7usize.saturating_sub(self.at & 7)) & 1;
            value = (value << 1) | u64::from(bit);
            self.at = self.at.saturating_add(1);
        }
        Some(value)
    }

    /// A raw value mapped onto its `/Decode` pair (§8.9.5.2's formula).
    fn decoded(&mut self, bits: u32, pair: usize) -> Option<f64> {
        let raw = self.bits(bits)?;
        let (low, high) = self.decode.get(pair).copied().unwrap_or((0.0, 1.0));
        #[expect(
            clippy::cast_precision_loss,
            reason = "a value of at most 32 bits is exact in f64"
        )]
        let fraction = raw as f64 / (1u64 << bits).saturating_sub(1) as f64;
        Some(fraction.mul_add(high - low, low))
    }

    fn point(&mut self) -> Option<Point> {
        let x = self.decoded(self.coordinate, 0)?;
        let y = self.decoded(self.coordinate, 1)?;
        Some(Point::new(x, y))
    }

    fn colour(&mut self) -> Option<Vec<f64>> {
        (2..self.decode.len())
            .map(|pair| self.decoded(self.component, pair))
            .collect()
    }

    /// One triangle vertex and its flag, then the padding to a byte "[e]ach set of vertex data
    /// shall occupy a whole number of bytes".
    fn vertex(&mut self, flag_bits: u32) -> Option<(u64, Vertex)> {
        let flag = self.bits(flag_bits)? & 0b11;
        let point = self.point()?;
        let colour = self.colour()?;
        self.at = self.at.div_ceil(8).saturating_mul(8);
        Some((flag, Vertex { point, colour }))
    }
}

impl Mesh {
    /// Cuts every triangle to the complement of one placement's regions, exactly (types 4, 5),
    /// answering whether any triangle met them.
    pub(super) fn cut_triangles(&mut self, placement: &Placement) -> Result<bool, String> {
        let mut out: Vec<[Vertex; 3]> = Vec::with_capacity(self.triangles.len());
        let mut met = false;
        for triangle in &self.triangles {
            let outline = ring(&[triangle[0].point, triangle[1].point, triangle[2].point]);
            if !meets(&outline, &placement.regions, placement.to_display) {
                out.push(triangle.clone());
                continue;
            }
            met = true;
            let cut = paths::subtract(&[outline], &placement.regions, placement.to_display)
                .ok_or_else(|| {
                    "§12.5.6.23: cutting a mesh triangle against these regions exceeds this \
                     build's bound on the surviving pieces; the page is refused"
                        .to_owned()
                })?;
            for piece in &cut.polygons {
                let corners = vertices_of(piece);
                let Some((&hub, rest)) = corners.split_first() else {
                    continue;
                };
                for pair in rest.windows(2) {
                    let fan = [hub, pair[0], pair[1]];
                    if ring(&fan).area().abs() <= f64::EPSILON {
                        continue;
                    }
                    out.push(fan.map(|point| Vertex {
                        point,
                        colour: interpolate(triangle, point),
                    }));
                }
            }
            if out.len() > pdf_model::mesh::MAX_TRIANGLES {
                return Err(
                    "§12.5.6.23: the cut mesh would state more triangles than a reader of it is \
                     bound to; the page is refused"
                        .to_owned(),
                );
            }
        }
        self.triangles = out;
        Ok(met)
    }

    /// Removes every sub-patch meeting one placement's regions (types 6, 7), answering whether
    /// any patch met them.
    pub(super) fn cut_patches(&mut self, placement: &Placement) -> Result<bool, String> {
        let mut out: Vec<Patch> = Vec::with_capacity(self.patches.len());
        let mut met = false;
        for patch in &self.patches {
            let mut leaves: Vec<((f64, f64), Patch)> = Vec::new();
            split(patch, (0.0, 0.0), 1.0, 0, placement, &mut leaves);
            met |=
                !(leaves.len() == 1 && leaves[0].0 == (0.0, 0.0) && leaves[0].1.grid == patch.grid);
            // §8.7.4.5.7's precedence within a patch: the largest v, then the largest u, wins,
            // so the leaves are written in that order and the later paints over the earlier.
            leaves.sort_by(|(a, _), (b, _)| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
            out.extend(leaves.into_iter().map(|(_, leaf)| leaf));
            if out.len() > pdf_model::mesh::MAX_PATCHES {
                return Err(
                    "§12.5.6.23: removing the region from a patch mesh would state more patches \
                     than a reader of it is bound to; the page is refused"
                        .to_owned(),
                );
            }
        }
        self.patches = out;
        Ok(met)
    }

    /// The parameter interval each triangle or patch spans, for a mesh whose vertices carry a
    /// `/Function`'s parameter: a triangle's parameter is affine over it and a patch's bilinear,
    /// so each spans exactly the least to the greatest of its corners'.
    pub(super) fn parameter_spans(&self) -> Vec<(f64, f64)> {
        let span = |values: &mut dyn Iterator<Item = f64>| {
            values.fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), value| {
                (low.min(value), high.max(value))
            })
        };
        let mut out: Vec<(f64, f64)> = self
            .triangles
            .iter()
            .map(|triangle| {
                span(
                    &mut triangle
                        .iter()
                        .map(|vertex| vertex.colour.first().copied().unwrap_or(0.0)),
                )
            })
            .collect();
        out.extend(self.patches.iter().map(|patch| {
            span(
                &mut patch
                    .corners
                    .iter()
                    .map(|corner| corner.first().copied().unwrap_or(0.0)),
            )
        }));
        out
    }

    /// The mesh written back as a stream of its family at [`COORDINATE_BITS`] and
    /// [`COMPONENT_BITS`], under the producer's dictionary with the widths and type restated.
    pub(super) fn encode(&self, source: &Dictionary) -> Encoded {
        let mut writer = Writer::default();
        let mut largest = 0.0f64;
        let mut note = |point: Point| largest = largest.max(point.x.abs()).max(point.y.abs());
        let kind = if self.kind == 5 {
            4
        } else if self.kind == 6 {
            7
        } else {
            self.kind
        };
        if kind == 4 {
            for triangle in &self.triangles {
                for vertex in triangle {
                    // Every triangle is stated whole, flag 0 then two vertices whose flags "shall
                    // be ignored".
                    writer.bits(0, 8);
                    self.write_point(&mut writer, vertex.point);
                    note(vertex.point);
                    self.write_colour(&mut writer, &vertex.colour);
                }
            }
        } else {
            for patch in &self.patches {
                writer.bits(0, 8);
                for (row, column) in BOUNDARY.iter().chain(INTERIOR.iter()) {
                    let point = patch.grid[*row][*column];
                    self.write_point(&mut writer, point);
                    note(point);
                }
                for corner in &patch.corners {
                    self.write_colour(&mut writer, corner);
                }
            }
        }
        let mut dict = Dictionary::new();
        for (key, value) in source.iter() {
            if !matches!(
                key.as_bytes(),
                b"Filter"
                    | b"DecodeParms"
                    | b"DP"
                    | b"Length"
                    | b"BitsPerCoordinate"
                    | b"BitsPerComponent"
                    | b"BitsPerFlag"
                    | b"VerticesPerRow"
                    | b"ShadingType"
            ) {
                dict.insert(key.clone(), value.clone());
            }
        }
        let integer = |value: i64| Object::Integer(value);
        dict.insert(Name::new(&b"ShadingType"[..]), integer(kind));
        dict.insert(
            Name::new(&b"BitsPerCoordinate"[..]),
            integer(i64::from(COORDINATE_BITS)),
        );
        dict.insert(
            Name::new(&b"BitsPerComponent"[..]),
            integer(i64::from(COMPONENT_BITS)),
        );
        dict.insert(Name::new(&b"BitsPerFlag"[..]), integer(8));
        let step = self
            .decode
            .iter()
            .take(2)
            .map(|(low, high)| (high - low).abs())
            .fold(0.0f64, f64::max)
            / f64::from(u32::MAX);
        Encoded {
            dict,
            data: writer.finish(),
            largest,
            step,
        }
    }

    fn write_point(&self, writer: &mut Writer, point: Point) {
        writer.bits(
            quantise(point.x, self.decode[0], COORDINATE_BITS),
            COORDINATE_BITS,
        );
        writer.bits(
            quantise(point.y, self.decode[1], COORDINATE_BITS),
            COORDINATE_BITS,
        );
    }

    fn write_colour(&self, writer: &mut Writer, colour: &[f64]) {
        for (index, value) in colour.iter().enumerate() {
            let pair = self
                .decode
                .get(index.saturating_add(2))
                .copied()
                .unwrap_or((0.0, 1.0));
            writer.bits(quantise(*value, pair, COMPONENT_BITS), COMPONENT_BITS);
        }
    }
}

/// A mesh written back: the dictionary entries, the raw (unfiltered) data, and what the margin
/// guard is checked against.
pub(super) struct Encoded {
    pub(super) dict: Dictionary,
    pub(super) data: Vec<u8>,
    /// The largest coordinate magnitude written.
    largest: f64,
    /// One step of the 32-bit coordinate grid, the wider of the two axes'.
    step: f64,
}

impl Encoded {
    /// Whether writing a coordinate on the 32-bit grid, and a reader decoding it at single
    /// precision (§7.3.3's reals are what a conforming reader holds), together move it by less
    /// than [`REGION_PAD`] in the display list's space — the argument
    /// [`super::paths::Cut::margin_holds`] makes for a path.
    pub(super) fn margin_holds(&self, norm: f64) -> bool {
        let single = self.largest * f64::from(f32::EPSILON) / 2.0;
        (single + self.step / 2.0) * norm < REGION_PAD
    }
}

/// A value's integer on a `bits`-wide grid over its `/Decode` pair, rounded to the nearest.
fn quantise(value: f64, (low, high): (f64, f64), bits: u32) -> u64 {
    let top = (1u64 << bits).saturating_sub(1);
    if (high - low).abs() <= f64::EPSILON {
        return 0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a grid of at most 32 bits is exact in f64"
    )]
    let scaled = ((value - low) / (high - low) * top as f64).round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "clamped to the grid's own range first, which is at most 32 bits and exact"
    )]
    let clamped = scaled.clamp(0.0, top as f64) as u64;
    clamped
}

/// A most-significant-bit-first writer.
#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
    /// Bits used in the last byte.
    used: u32,
}

impl Writer {
    fn bits(&mut self, value: u64, count: u32) {
        for shift in (0..count).rev() {
            if self.used == 0 {
                self.bytes.push(0);
            }
            let bit = u8::from((value >> shift) & 1 == 1);
            if let Some(last) = self.bytes.last_mut() {
                *last |= bit << 7u32.saturating_sub(self.used);
            }
            self.used = self.used.saturating_add(1) & 7;
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// A closed polygon through the given points.
fn ring(points: &[Point]) -> BezPath {
    let mut path = BezPath::new();
    for (index, point) in points.iter().enumerate() {
        if index == 0 {
            path.move_to(*point);
        } else {
            path.line_to(*point);
        }
    }
    path.close_path();
    path
}

/// The corners of a straight-edged polygon the cut produced, in order, without the closing repeat.
fn vertices_of(polygon: &BezPath) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    for element in polygon.elements() {
        match element {
            PathEl::MoveTo(point) | PathEl::LineTo(point) => {
                if out.last() != Some(point) {
                    out.push(*point);
                }
            }
            PathEl::QuadTo(_, point) | PathEl::CurveTo(_, _, point) => out.push(*point),
            PathEl::ClosePath => {}
        }
    }
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// Whether a polygon's display-space box meets any padded region box.
fn meets(polygon: &BezPath, regions: &[[f64; 4]], to_display: Mapping) -> bool {
    let points = vertices_of(polygon);
    let mapped: Vec<Point> = points
        .iter()
        .map(|point| to_display.apply(*point))
        .collect();
    let box_of = bounds(&mapped);
    regions
        .iter()
        .any(|region| overlaps_padded(*region, box_of))
}

/// The box of a set of points.
fn bounds(points: &[Point]) -> [f64; 4] {
    points.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |[x0, y0, x1, y1], point| {
            [
                x0.min(point.x),
                y0.min(point.y),
                x1.max(point.x),
                y1.max(point.y),
            ]
        },
    )
}

/// Whether a box meets a region widened by [`REGION_PAD`].
fn overlaps_padded(region: [f64; 4], other: [f64; 4]) -> bool {
    other[0] < region[2] + REGION_PAD
        && other[2] > region[0] - REGION_PAD
        && other[1] < region[3] + REGION_PAD
        && other[3] > region[1] - REGION_PAD
}

/// Whether a box lies inside a region's own bounds.
fn inside(region: [f64; 4], other: [f64; 4]) -> bool {
    other[0] >= region[0] && other[2] <= region[2] && other[1] >= region[1] && other[3] <= region[3]
}

/// The colour a triangle's own linear interpolation states at a point of its plane.
fn interpolate(triangle: &[Vertex; 3], at: Point) -> Vec<f64> {
    let [a, b, c] = [triangle[0].point, triangle[1].point, triangle[2].point];
    let cross = |from: Point, one: Point, two: Point| {
        (one.x - from.x).mul_add(two.y - from.y, -((one.y - from.y) * (two.x - from.x)))
    };
    let area = cross(a, b, c);
    if area.abs() <= f64::EPSILON {
        return triangle[0].colour.clone();
    }
    let wb = cross(a, at, c) / area;
    let wc = cross(a, b, at) / area;
    let wa = 1.0 - wb - wc;
    triangle[0]
        .colour
        .iter()
        .zip(&triangle[1].colour)
        .zip(&triangle[2].colour)
        .map(|((ca, cb), cc)| wa * ca + wb * cb + wc * cc)
        .collect()
}

/// Splits a patch until each piece is clear of the regions, inside one, or small enough to be
/// the unit of removal, keeping the clear ones with the parameter corner `(u, v)` they start at.
fn split(
    patch: &Patch,
    origin: (f64, f64),
    size: f64,
    depth: usize,
    placement: &Placement,
    out: &mut Vec<((f64, f64), Patch)>,
) {
    let mapped: Vec<Point> = patch
        .grid
        .iter()
        .flatten()
        .map(|point| placement.to_display.apply(*point))
        .collect();
    let box_of = bounds(&mapped);
    if !placement
        .regions
        .iter()
        .any(|region| overlaps_padded(*region, box_of))
    {
        out.push((origin, patch.clone()));
        return;
    }
    let extent = (box_of[2] - box_of[0]).max(box_of[3] - box_of[1]);
    let within = placement
        .regions
        .iter()
        .any(|region| inside(*region, box_of));
    if within || extent <= LEAF_EXTENT || depth >= MAX_SPLITS {
        return;
    }
    let half = size / 2.0;
    let [low_u, high_u] = halve_rows(patch);
    for (u_offset, rows) in [(0.0, low_u), (half, high_u)] {
        let [low_v, high_v] = halve_columns(&rows);
        for (v_offset, piece) in [(0.0, low_v), (half, high_v)] {
            split(
                &piece,
                (origin.0 + u_offset, origin.1 + v_offset),
                half,
                depth.saturating_add(1),
                placement,
                out,
            );
        }
    }
}

/// The two halves of a patch at `u = ½`: de Casteljau along each column's four rows.
fn halve_rows(patch: &Patch) -> [Patch; 2] {
    let mut low = patch.clone();
    let mut high = patch.clone();
    for column in 0..4 {
        let [a, b] = halve([
            patch.grid[0][column],
            patch.grid[1][column],
            patch.grid[2][column],
            patch.grid[3][column],
        ]);
        for row in 0..4 {
            low.grid[row][column] = a[row];
            high.grid[row][column] = b[row];
        }
    }
    // Corners at (u, v): 0 = (0,0), 1 = (0,1), 2 = (1,1), 3 = (1,0).
    let middle_at_v0 = mix(&patch.corners[0], &patch.corners[3], 0.5);
    let middle_at_v1 = mix(&patch.corners[1], &patch.corners[2], 0.5);
    low.corners = [
        patch.corners[0].clone(),
        patch.corners[1].clone(),
        middle_at_v1.clone(),
        middle_at_v0.clone(),
    ];
    high.corners = [
        middle_at_v0,
        middle_at_v1,
        patch.corners[2].clone(),
        patch.corners[3].clone(),
    ];
    [low, high]
}

/// The two halves of a patch at `v = ½`: de Casteljau along each row's four columns.
fn halve_columns(patch: &Patch) -> [Patch; 2] {
    let mut low = patch.clone();
    let mut high = patch.clone();
    for row in 0..4 {
        let [a, b] = halve(patch.grid[row]);
        low.grid[row] = a;
        high.grid[row] = b;
    }
    let middle_at_u0 = mix(&patch.corners[0], &patch.corners[1], 0.5);
    let middle_at_u1 = mix(&patch.corners[3], &patch.corners[2], 0.5);
    low.corners = [
        patch.corners[0].clone(),
        middle_at_u0.clone(),
        middle_at_u1.clone(),
        patch.corners[3].clone(),
    ];
    high.corners = [
        middle_at_u0,
        patch.corners[1].clone(),
        patch.corners[2].clone(),
        middle_at_u1,
    ];
    [low, high]
}

/// A cubic's control points split at its midpoint, exactly, by de Casteljau.
fn halve(points: [Point; 4]) -> [[Point; 4]; 2] {
    let [first, second, third, last] = points;
    let near = first.midpoint(second);
    let between = second.midpoint(third);
    let far = third.midpoint(last);
    let near_inner = near.midpoint(between);
    let far_inner = between.midpoint(far);
    let middle = near_inner.midpoint(far_inner);
    [
        [first, near, near_inner, middle],
        [middle, far_inner, far, last],
    ]
}

/// The componentwise mix of two colours.
fn mix(from: &[f64], to: &[f64], t: f64) -> Vec<f64> {
    from.iter()
        .zip(to)
        .map(|(a, b)| (b - a).mul_add(t, *a))
        .collect()
}
