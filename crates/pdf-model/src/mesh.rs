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
//! # Where the interpolation happens is a `shall` about a space
//!
//! §8.7.4.4 makes the shading's `/ColorSpace` the space "in which colour interpolation is
//! performed", and gives each family a rule. A device space may be converted "at any time
//! (before or after any interpolation on the colour values in the shading)", so a vertex in one
//! is converted as it is read and the rasteriser's linear interpolation between device colours is
//! the clause's own answer. A CIE-based, `Separation` or `DeviceN` space is not: "all gradient
//! fill calculations shall be performed in that space", and conversion "shall occur only after
//! all interpolation calculations have been performed". A display list carries device colours,
//! so a vertex in one of those spaces keeps its [`Components`] here and its triangle is
//! subdivided until linear interpolation between the converted corners stays within §10.7.3's
//! tolerance of the conversion of the interpolated components — which is that clause's own
//! licence, "PDF processors may actually compute colour values only for some subset of the
//! points in the target area", taken at the tolerance it names. [`Corner::emit`] is the seam.
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

use crate::colour::ColourSpace;
use crate::content::Transfer;
use crate::function::{BitReader, Function};
use crate::shading::{Colouring, transferred};

/// How finely a Bézier patch is evaluated along each axis.
///
/// The geometry's accuracy is set here, because a backend that subdivides these triangles
/// further does so linearly and cannot recover curvature. Ten steps is two hundred triangles
/// per patch.
///
/// **This comment used to end "puts the error of a patch spanning a whole page well under a
/// pixel", and that was a claim with no scale in it**: the surface is evaluated in the
/// shading's own space and the triangles are transformed afterwards, so whatever the chord
/// error is in page units, a device sees it multiplied by the magnification. What the sentence
/// is worth is what a measurement says, and the nine-hundred-and-forty-fifth session took one —
/// every corpus page holding a mesh rendered at this fineness and at 60, with
/// [`MAX_TRIANGLES`] lifted so that only the fineness moved. At the page's own scale the worst
/// page is `coons-allflags-withfunction.pdf` at a mean 0.0511 of 255 and a worst 32×32 tile of
/// 0.92; at four times the mean is unchanged and the worst tile rises to 10.18
/// (`issue18816.pdf`), because the departure is a seam at each patch's boundary whose *width*
/// does not shrink as the pixels arrive. So ten steps is adequate for a page and visibly not a
/// derivation, which is why §8.7.4.5.7's and §8.7.4.5.8's ledger rows stay `partial` on it.
///
/// **And raising it is not free, which is the thing this constant could not say before
/// [`MAX_TRIANGLES`] was public**: the bound is counted in triangles, so a document's patch
/// budget is `MAX_TRIANGLES / (2 · PATCH_STEPS²)`. The largest mesh any corpus this tree holds
/// paints with is `bug1703683_page2_reduced.pdf`'s 305 patches — 61 000 triangles, 23.3% of the
/// bound at this fineness — and it **crosses the bound at a fineness of 21**. So a session that
/// derives this number from §10.7.3's smoothness tolerance has to move the bound with it or
/// start dropping patches out of a real document, which is what makes the two constants one
/// decision rather than two.
const PATCH_STEPS: usize = 10;

/// How many times one triangle may be halved along each edge before §8.7.4.4's subdivision
/// stops and the triangle is emitted as it stands.
///
/// Each level quarters the interpolation error of a smooth conversion and multiplies the
/// triangles by four. A tint transform curving over its whole range — `t²`, the fixture in
/// `tests/shadings.rs` — starts a quarter of the component range out and is inside §10.7.3's
/// default tolerance of 1/256 after three levels, so six leaves the same margin again. A
/// triangle still outside the tolerance at this depth has a conversion that is not smooth at
/// the scale of the triangle, which is §10.7.3's NOTE 1 — a sampled function "sampled at too low
/// a frequency, in which case the accuracy defined by the smoothness tolerance cannot be
/// guaranteed" — and it is emitted and said, not pursued: [`Mesh::coarse`].
const MAX_REFINEMENT_DEPTH: usize = 6;

/// How many triangles a mesh may hold before §8.7.4.4's subdivision stops adding to it.
///
/// Half of [`MAX_TRIANGLES`], so that the document's own triangles always fit ahead of the ones
/// this crate adds to draw them accurately: a subdivision that ran a mesh into that bound would
/// have the document's later patches dropped and reported as `max_mesh_triangles`, which is the
/// wrong sentence for what happened. Past this the remaining triangles are emitted as the file
/// states them and [`Mesh::coarse`] says so. A subdivision already under way finishes, so the
/// count may pass this by fewer than `4^MAX_REFINEMENT_DEPTH` triangles, which is under the
/// other half.
const REFINED_TRIANGLES: usize = MAX_TRIANGLES / 2;

/// Most triangles one shading may produce.
///
/// A mesh stream is compressed, so a few kilobytes can describe an unbounded number of
/// patches. This is the decompression-bomb bound for shadings, and ISO 32000-2 §10.7.3 is
/// where a bound of this kind is licensed — "[e]ach output device may have internal limits" —
/// the same sentence [`crate::shading`]'s `MAX_FUNCTION_CELLS` rests on.
///
/// **It is counted in this program's triangles rather than in the document's patches**, and
/// that is worth saying out loud because it is the reason this constant is public: a type 6 or
/// 7 patch becomes `PATCH_STEPS`² quadrilaterals here, so how many patches a document is
/// allowed depends on a number that is nothing to do with the document, and raising the
/// tessellation's fineness lowers it. `examples/mesh_triangle_census` is what keeps that
/// relation measured rather than assumed: over `doc/pdf.js/test/pdfs`, the four `doc/corpora/`
/// submodules and `corpus-cache/openpreserve` — 1516 files, 40 mesh paints — the largest is
/// 23.3% of this bound and no page reports it.
pub const MAX_TRIANGLES: usize = 1 << 18;

/// Reads a mesh shading's stream into triangles, with the ramp a parametric mesh needs.
///
/// [`Mesh::truncated`] is set where [`MAX_TRIANGLES`] stopped the reading with more of the
/// stream to come, so the caller can say that the page is drawing less than the document
/// states.
///
/// The ramp is `Some` exactly where the shading states a `/Function`, which is exactly where
/// the triangles carry [`Corners::Parameters`]: §8.7.4.5.5 interpolates the parameter and
/// calls the function afterwards, so the function crosses into the display list as the
/// samples of itself that [`Ramp`] already is for an axial or a radial shading.
///
/// `colouring` carries ISO 32000-2 §10.5's transfer function where the graphics state states one;
/// see [`transferred_corners`] for where it reaches a mesh's own colours and [`MeshReader::ramp`]
/// for where it reaches a parametric one's.
///
/// Returns `None` when the stream is unreadable or describes no triangles, which the
/// caller reports rather than drawing an empty shading.
pub(crate) fn read(
    document: &Document,
    stream: &Stream,
    kind: i64,
    space: &ColourSpace,
    functions: &[Function],
    colouring: Colouring<'_>,
) -> Option<Mesh> {
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

    // §8.7.4.4's rule for the family, answered once: `Some` is the space the gradient is
    // calculated in, and `None` is a device space converted as it is read.
    let interpolation = space.interpolates_in();
    let reader = MeshReader {
        decode,
        components,
        coordinate_bits,
        component_bits,
        flag_bits,
        space,
        interpolation: interpolation.unwrap_or(space),
        functions,
        colouring,
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
    let mut refinement = Refinement::default();
    // The three readings differ only in what a vertex carries, which is what the two clauses
    // make the whole question: the one parametric value the function takes (§8.7.4.5.5), the
    // components of a space the gradient shall be calculated in (§8.7.4.4), or a device colour.
    let ((triangles, truncated), ramp) = if !functions.is_empty() {
        (
            reader.triangles::<f32>(&mut bits, kind, per_row, &mut refinement)?,
            Some(reader.ramp()),
        )
    } else if interpolation.is_some() {
        // §10.5's transfer is inside `Components`'s conversion, so it is inside the tolerance.
        (
            reader.triangles::<Components>(&mut bits, kind, per_row, &mut refinement)?,
            None,
        )
    } else {
        let (triangles, truncated) =
            reader.triangles::<Color>(&mut bits, kind, per_row, &mut refinement)?;
        (
            (
                transferred_corners(triangles, colouring.transfer),
                truncated,
            ),
            None,
        )
    };

    (!triangles.is_empty()).then_some(Mesh {
        triangles,
        ramp,
        truncated,
        coarse: refinement.coarse,
    })
}

/// What reading a mesh stream produced, and what a bound cost it.
///
/// A struct rather than the pair this returned until the nine-hundred-and-forty-fifth session,
/// because the third member is the one a caller must not be able to ignore by accident:
/// [`MAX_TRIANGLES`] stops a mesh part-way, and a page that drew the part without saying so is
/// exactly the silent drop `crate::content` forbids. Every neighbouring bound in this crate is
/// already reported as [`crate::Unsupported::LimitReached`]; this one was not, from the day it
/// was written until that session.
pub(crate) struct Mesh {
    /// The triangles, in the order §8.7.4.5.7's overlap rule needs them painted.
    pub(crate) triangles: Vec<Triangle>,
    /// The ramp a parametric mesh needs, `Some` exactly where the shading states a `/Function`.
    pub(crate) ramp: Option<Ramp>,
    /// [`MAX_TRIANGLES`] stopped the reading with a vertex, a row or a patch still to come.
    pub(crate) truncated: bool,
    /// [`MAX_REFINEMENT_DEPTH`] or [`REFINED_TRIANGLES`] stopped §8.7.4.4's subdivision with a
    /// triangle still outside §10.7.3's tolerance, so somewhere in this mesh a rasteriser's
    /// linear interpolation between device colours stands in for the clause's interpolation in
    /// the shading's own space by more than the tolerance allows.
    pub(crate) coarse: bool,
}

/// What §8.7.4.4's subdivision has to say for itself once a mesh has been read.
#[derive(Debug, Default)]
struct Refinement {
    /// A bound stopped a subdivision short of the tolerance; see [`Mesh::coarse`].
    coarse: bool,
}

/// Every corner colour through ISO 32000-2 §10.5's transfer function, on the device-space route.
///
/// A mesh in a space §8.7.4.4 has the gradient calculated in does not come this way: its
/// [`Components`] are converted after the subdivision and the transfer is inside that
/// conversion, so the tolerance the subdivision is measured against includes it.
///
/// **After the subdivision rather than before it**, which is the whole reason this is a pass over
/// the finished triangles instead of a line inside `Corner::read`. §8.7.4.5.7 makes the colour
/// inside a Coons or tensor patch a bilinear mix of the patch's four stated corner colours, so the
/// colours mapped here are [`PATCH_STEPS`]² samples of that mix rather than the four the file
/// wrote down — and the clause's input is "the value of a colour component" at a point, not the
/// corners a point's colour was mixed from. For types 4 and 5, which state their triangles
/// directly, the two orders are the same arithmetic.
///
/// **What remains approximate, and it is §8.7.4.4's own approximation.** A rasteriser interpolates
/// linearly between the three corners of each triangle it is given, so what is drawn between them
/// is a mix of transferred colours where the clause asks for the transfer of the mixed colour. The
/// clause that permits it is the one that permits interpolating a shading at all: "PDF processors
/// may actually compute colour values only for some subset of the points in the target area, with
/// the colours of the intervening points determined by interpolation between the ones computed."
/// Closing it entirely would need a per-pixel pass, which is `doc/todo/13`'s per-region model and
/// is priced there.
///
/// A corner holding §8.7.4.5.5's parameter has no colour to map; the function that turns it into
/// one is the ramp, and `MeshReader::ramp` maps that.
fn transferred_corners(triangles: Vec<Triangle>, transfer: Option<&Transfer>) -> Vec<Triangle> {
    let Some(transfer) = transfer else {
        return triangles;
    };
    triangles
        .into_iter()
        .map(|triangle| Triangle {
            points: triangle.points,
            corners: match triangle.corners {
                Corners::Colours(colours) => {
                    Corners::Colours(colours.map(|colour| transfer.apply(colour)))
                }
                parameters @ Corners::Parameters(_) => parameters,
            },
        })
        .collect()
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
#[derive(Debug, Clone)]
struct Vertex<C> {
    point: Point,
    corner: C,
}

impl<C: Corner> Vertex<C> {
    /// The vertex halfway to `other`, in position and in what it carries.
    ///
    /// Straight in both: the geometry inside a mesh triangle is linear by §8.7.4.5.5, and the
    /// carried quantity is linear in the space this reader interpolates it in.
    fn midpoint(&self, other: &Self) -> Self {
        Self {
            point: Point::new(
                (self.point.x + other.point.x) * 0.5,
                (self.point.y + other.point.y) * 0.5,
            ),
            corner: self.corner.mix(&other.corner, 0.5),
        }
    }
}

/// What a vertex carries, and the three things the reader does with it.
///
/// A mesh states either a colour per vertex or one parametric value per vertex, and the
/// clause makes that a choice about *what is interpolated* rather than about a format — and
/// §8.7.4.4 then makes a colour's own space a third answer to the same question. The reading
/// of the stream is identical in all three cases — the flags, the lattice, the patches and
/// their shared edges — so the difference between them lives here and nowhere else.
trait Corner: Clone {
    /// Reads one vertex's worth of the stream.
    fn read(reader: &MeshReader<'_>, bits: &mut BitReader<'_>) -> Option<Self>;

    /// The value `t` of the way from `self` to `other`, which a patch's interior needs.
    fn mix(&self, other: &Self, t: f32) -> Self;

    /// A value for a patch slot the stream is about to fill, so that a patch's four corners
    /// can be an array before all four have been read.
    fn placeholder() -> Self;

    /// Appends the triangles a rasteriser draws for this one.
    ///
    /// One triangle wherever a rasteriser's linear interpolation between its corners *is* the
    /// clause's answer — device colours, or §8.7.4.5.5's parameter — and, for [`Components`],
    /// as many as §8.7.4.4 needs. `refinement` is where the last of those says a bound stopped
    /// it. Every triangle goes through here so that the bound the callers count,
    /// [`MAX_TRIANGLES`], counts what was actually emitted.
    fn emit(
        reader: &MeshReader<'_>,
        vertices: [Vertex<Self>; 3],
        out: &mut Vec<Triangle>,
        refinement: &mut Refinement,
    );

    /// Appends the triangles of one tessellated patch: `points` and `corners` are the
    /// [`PATCH_STEPS`]` + 1` square grid [`tessellate`] evaluated, `u`-major.
    ///
    /// Two triangles per cell through [`Self::emit`], which is the whole of it for a colour or
    /// a parameter. [`Components`] overrides it, because a patch is where §8.7.4.4's
    /// conversion-after-interpolation has a cost worth measuring — 121 conversions a patch where
    /// the file's own route needed four — and a patch is also where one question, asked once,
    /// answers for two hundred triangles.
    fn emit_patch(
        reader: &MeshReader<'_>,
        points: &[Point],
        corners: &[Self],
        out: &mut Vec<Triangle>,
        refinement: &mut Refinement,
    ) {
        for (_, _, a, b, c, d) in patch_cells() {
            let corner = |index: usize| Vertex {
                point: points.get(index).copied().unwrap_or(Point::new(0.0, 0.0)),
                corner: corners
                    .get(index)
                    .cloned()
                    .unwrap_or_else(Self::placeholder),
            };
            Self::emit(reader, [corner(a), corner(b), corner(c)], out, refinement);
            Self::emit(reader, [corner(b), corner(d), corner(c)], out, refinement);
        }
    }
}

/// The cells of a tessellated patch as `(u, v)` and the grid indices of their four corners —
/// `(u, v)`, `(u, v+1)`, `(u+1, v)`, `(u+1, v+1)` — in the order [`tessellate`] documents: `v`
/// outer, so that the last cell written over any point is the one with the largest `v`
/// (ADR 0778).
fn patch_cells() -> impl Iterator<Item = (usize, usize, usize, usize, usize, usize)> {
    let stride = PATCH_STEPS.saturating_add(1);
    let at = move |u: usize, v: usize| u.saturating_mul(stride).saturating_add(v);
    (0..PATCH_STEPS).flat_map(move |v_step| {
        (0..PATCH_STEPS).map(move |u_step| {
            (
                u_step,
                v_step,
                at(u_step, v_step),
                at(u_step, v_step.saturating_add(1)),
                at(u_step.saturating_add(1), v_step),
                at(u_step.saturating_add(1), v_step.saturating_add(1)),
            )
        })
    })
}

impl Corner for Color {
    fn read(reader: &MeshReader<'_>, bits: &mut BitReader<'_>) -> Option<Self> {
        let mut values = Vec::with_capacity(reader.components);
        for index in 0..reader.components {
            let raw = bits.read(reader.component_bits)?;
            values.push(reader.decode_at(index.checked_add(2)?, raw, reader.component_bits));
        }
        Some(reader.colouring.into.paint(reader.space, &values))
    }

    fn mix(&self, other: &Self, t: f32) -> Self {
        mix_colour(*self, *other, t)
    }

    fn placeholder() -> Self {
        Color::BLACK
    }

    fn emit(
        _: &MeshReader<'_>,
        vertices: [Vertex<Self>; 3],
        out: &mut Vec<Triangle>,
        _: &mut Refinement,
    ) {
        let [a, b, c] = vertices;
        out.push(Triangle {
            points: [a.point, b.point, c.point],
            corners: Corners::Colours([a.corner, b.corner, c.corner]),
        });
    }
}

/// The colour `t` of the way from `from` to `to`, channel by channel — what a rasteriser does
/// between two corners it is handed, written once so the tolerance below measures the same
/// arithmetic.
fn mix_colour(from: Color, to: Color, t: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

/// A vertex's colour as the components the file states, in the space ISO 32000-2 §8.7.4.4 has
/// the gradient calculated in.
///
/// > If ColorSpace is a CIE-based colour space, all gradient fill calculations shall be
/// > performed in that space. Conversion to device colours shall occur only after all
/// > interpolation calculations have been performed.
///
/// and, of a `Separation` or `DeviceN` space,
///
/// > In that case, gradient fill calculations shall be performed in the designated Separation
/// > or DeviceN colour space before conversion to the alternate space. Thus, nonlinear tint
/// > transformation functions shall be accommodated for an optimal representation of the
/// > shading.
///
/// An `Indexed` space's values "shall be immediately converted to the base colour space", which
/// [`ColourSpace::entry_of`] is, and the base's rule then applies.
///
/// A display list carries device colours, so what leaves here is still a triangle with a colour
/// at each corner — but as many triangles as it takes for linear interpolation between those
/// corners to stay within §10.7.3's tolerance of the conversion of the interpolated components.
/// [`MeshReader::refine`] is the subdivision.
#[derive(Debug, Clone)]
struct Components(Vec<f32>);

impl Corner for Components {
    fn read(reader: &MeshReader<'_>, bits: &mut BitReader<'_>) -> Option<Self> {
        let mut values = Vec::with_capacity(reader.components);
        for index in 0..reader.components {
            let raw = bits.read(reader.component_bits)?;
            values.push(reader.decode_at(index.checked_add(2)?, raw, reader.component_bits));
        }
        Some(Self(reader.space.entry_of(&values)))
    }

    fn mix(&self, other: &Self, t: f32) -> Self {
        Self(
            self.0
                .iter()
                .zip(other.0.iter())
                .map(|(from, to)| from + (to - from) * t)
                .collect(),
        )
    }

    fn placeholder() -> Self {
        Self(Vec::new())
    }

    fn emit(
        reader: &MeshReader<'_>,
        vertices: [Vertex<Self>; 3],
        out: &mut Vec<Triangle>,
        refinement: &mut Refinement,
    ) {
        let colours = vertices
            .each_ref()
            .map(|vertex| reader.colour_of(&vertex.corner));
        reader.refine(vertices, colours, 0, out, refinement);
    }

    /// A patch, asked once whether its conversion is linear across it.
    ///
    /// **Measured before it was written** (`examples/open_one`, `personwithdog.pdf`, two
    /// `DeviceN` tensor meshes of some three hundred patches): converting every grid vertex and
    /// every triangle's midpoints through a type 4 tint transform took the page from 92 ms to
    /// 2.6 s and moved no pixel, because the transform is linear and the subdivision never
    /// fired. So the question is asked of the *patch* first, at nine of its grid vertices — the
    /// four corners, whose colours the file's own route would have converted anyway, the four
    /// edge midpoints and the centre. Where the five converted colours agree with the bilinear
    /// mix of the four corner colours to §10.7.3's tolerance, the conversion is linear across
    /// this patch to that tolerance and the grid's colours *are* that mix: nine conversions a
    /// patch against four, and the same triangles as before. Where they do not, every grid
    /// vertex is converted — which is the clause's requirement paid in full — and each cell is
    /// asked in turn, from the converted grid alone, whether a rasteriser's plane through its
    /// corners is within tolerance of the conversion: [`cell_error`] is the estimate, and a cell
    /// it cannot clear goes to [`MeshReader::refine`], which measures rather than estimates.
    fn emit_patch(
        reader: &MeshReader<'_>,
        points: &[Point],
        corners: &[Self],
        out: &mut Vec<Triangle>,
        refinement: &mut Refinement,
    ) {
        let stride = PATCH_STEPS.saturating_add(1);
        let at = |u: usize, v: usize| u.saturating_mul(stride).saturating_add(v);
        let convert = |index: usize| {
            corners
                .get(index)
                .map_or(Color::BLACK, |components| reader.colour_of(components))
        };
        // The four corners in `bilinear`'s order: (0,0), (0,1), (1,1), (1,0).
        let last = PATCH_STEPS;
        let corner_colours = [
            convert(at(0, 0)),
            convert(at(0, last)),
            convert(at(last, last)),
            convert(at(last, 0)),
        ];
        let mixed = |u_step: usize, v_step: usize| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "PATCH_STEPS is a small constant"
            )]
            let (u, v) = (
                u_step as f32 / PATCH_STEPS as f32,
                v_step as f32 / PATCH_STEPS as f32,
            );
            let top = mix_colour(corner_colours[0], corner_colours[1], v);
            let bottom = mix_colour(corner_colours[3], corner_colours[2], v);
            mix_colour(top, bottom, u)
        };
        let tolerance = reader.tolerance();
        let mid = PATCH_STEPS / 2;
        let linear = [(mid, 0), (mid, last), (0, mid), (last, mid), (mid, mid)]
            .into_iter()
            .all(|(u, v)| within(convert(at(u, v)), mixed(u, v), tolerance));

        let colours: Vec<Color> = if linear {
            (0..stride)
                .flat_map(|u| (0..stride).map(move |v| (u, v)))
                .map(|(u, v)| mixed(u, v))
                .collect()
        } else {
            (0..corners.len()).map(convert).collect()
        };

        for (u_step, v_step, a, b, c, d) in patch_cells() {
            let colour = |index: usize| colours.get(index).copied().unwrap_or(Color::BLACK);
            let (ca, cb, cc, cd) = (colour(a), colour(b), colour(c), colour(d));
            if linear || cell_error(&colours, u_step, v_step) <= tolerance {
                let point =
                    |index: usize| points.get(index).copied().unwrap_or(Point::new(0.0, 0.0));
                out.push(Triangle {
                    points: [point(a), point(b), point(c)],
                    corners: Corners::Colours([ca, cb, cc]),
                });
                out.push(Triangle {
                    points: [point(b), point(d), point(c)],
                    corners: Corners::Colours([cb, cd, cc]),
                });
                continue;
            }
            let vertex = |index: usize| Vertex {
                point: points.get(index).copied().unwrap_or(Point::new(0.0, 0.0)),
                corner: corners
                    .get(index)
                    .cloned()
                    .unwrap_or_else(Self::placeholder),
            };
            reader.refine(
                [vertex(a), vertex(b), vertex(c)],
                [ca, cb, cc],
                0,
                out,
                refinement,
            );
            reader.refine(
                [vertex(b), vertex(d), vertex(c)],
                [cb, cd, cc],
                0,
                out,
                refinement,
            );
        }
    }
}

/// An estimate of how far a rasteriser's linear interpolation across the cell at `(u, v)` of a
/// converted patch grid can stray from the conversion, from the grid alone.
///
/// Linear interpolation of a twice-differentiable function over a step `h` errs by at most
/// `h²·|f″|/8`, and `h²·f″` is what a central second difference of the grid measures — so along
/// each axis the bound is an eighth of the largest second difference at the cell's corners,
/// taken one cell in from the grid's edge where a corner has no neighbour on one side. The plane
/// through three corners also misses the bilinear term of a quadrilateral cell, which the mixed
/// difference `f(u+1,v+1) − f(u+1,v) − f(u,v+1) + f(u,v)` measures and which reaches a quarter of
/// itself at the cell's centre. Summed, and taken over every channel, because §10.7.3 asks for
/// "the maximum independent error". What the estimate cannot see is a conversion that turns
/// between two grid vertices and back, which is the sampled-function case §10.7.3's NOTE 1
/// concedes; what it costs is nothing, since every value in it was converted already.
fn cell_error(colours: &[Color], u: usize, v: usize) -> f32 {
    let stride = PATCH_STEPS.saturating_add(1);
    let colour = |u: usize, v: usize| {
        colours
            .get(u.saturating_mul(stride).saturating_add(v))
            .copied()
            .unwrap_or(Color::BLACK)
    };
    let channels = |colour: Color| [colour.r, colour.g, colour.b, colour.a];
    let second = |back: Color, centre: Color, forward: Color| {
        let (back, centre, forward) = (channels(back), channels(centre), channels(forward));
        (0..4)
            .map(|channel| (back[channel] - 2.0 * centre[channel] + forward[channel]).abs())
            .fold(0.0_f32, f32::max)
    };
    // A second difference centred on the cell's first corner reaches one step back and one
    // centred on its second reaches one step forward; at the grid's edge, where there is no such
    // neighbour, the difference one cell in is the nearest measurement there is.
    let (u_next, v_next) = (u.saturating_add(1), v.saturating_add(1));
    let along_u = second(
        colour(u.saturating_sub(1), v),
        colour(u, v),
        colour(u_next, v),
    )
    .max(second(
        colour(u, v),
        colour(u_next, v),
        colour(u_next.saturating_add(1).min(PATCH_STEPS), v),
    ));
    let along_v = second(
        colour(u, v.saturating_sub(1)),
        colour(u, v),
        colour(u, v_next),
    )
    .max(second(
        colour(u, v),
        colour(u, v_next),
        colour(u, v_next.saturating_add(1).min(PATCH_STEPS)),
    ));
    let mixed = {
        let (pa, pb, pc, pd) = (
            channels(colour(u, v)),
            channels(colour(u, v_next)),
            channels(colour(u_next, v)),
            channels(colour(u_next, v_next)),
        );
        (0..4)
            .map(|channel| (pd[channel] - pc[channel] - pb[channel] + pa[channel]).abs())
            .fold(0.0_f32, f32::max)
    };
    (along_u + along_v) / 8.0 + mixed / 4.0
}

impl Corner for f32 {
    fn read(reader: &MeshReader<'_>, bits: &mut BitReader<'_>) -> Option<Self> {
        let raw = bits.read(reader.component_bits)?;
        Some(reader.fraction_of_range(reader.decode_at(2, raw, reader.component_bits)))
    }

    fn mix(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }

    fn placeholder() -> Self {
        0.0
    }

    fn emit(
        _: &MeshReader<'_>,
        vertices: [Vertex<Self>; 3],
        out: &mut Vec<Triangle>,
        _: &mut Refinement,
    ) {
        let [a, b, c] = vertices;
        out.push(Triangle {
            points: [a.point, b.point, c.point],
            corners: Corners::Parameters([a.corner, b.corner, c.corner]),
        });
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
    /// The space §8.7.4.4 has the gradient calculated in — `space` itself, or an `Indexed`
    /// space's base — and the one a [`Components`] vertex is converted from. Read by nothing on
    /// the other two routes.
    interpolation: &'a ColourSpace,
    functions: &'a [Function],
    /// §10.7.3's resolution, §8.6.5.9's conversion and §10.5's transfer, which every colour a
    /// mesh produces needs.
    ///
    /// The transfer is read by [`MeshReader::ramp`] alone: a mesh that states colours at its
    /// vertices has them mapped by [`transferred_corners`], after the patch subdivision this
    /// reader does.
    colouring: Colouring<'a>,
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
        refinement: &mut Refinement,
    ) -> Option<(Vec<Triangle>, bool)> {
        match kind {
            4 => Some(self.free_form::<C>(bits, refinement)),
            5 => Some(self.lattice::<C>(bits, per_row, refinement)),
            6 | 7 => Some(self.patches::<C>(bits, kind == 7, refinement)),
            _ => None,
        }
    }

    /// §10.7.3's tolerance, as the fraction of a component's range it is stated in.
    ///
    /// [`Colouring::resolution`] is the sample count `Ramp::resolution_for` derived from the
    /// same tolerance — at least `1/t` samples for a tolerance of `t` — so the reciprocal is
    /// the tolerance as this device honours it, and a mesh and a ramp under one graphics state
    /// answer to one number.
    fn tolerance(&self) -> f32 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a sample count between 256 and 4096, exact in an f32"
        )]
        let resolution = self.colouring.resolution as f32;
        1.0 / resolution
    }

    /// The device colour a set of components in [`Self::interpolation`] becomes: §8.6.5.9's
    /// conversion, then §10.5's transfer.
    fn colour_of(&self, components: &Components) -> Color {
        transferred(
            self.colouring.into.paint(self.interpolation, &components.0),
            self.colouring.transfer,
        )
    }

    /// Emits `vertices` as one triangle where a rasteriser's linear interpolation between
    /// `colours` stays within §10.7.3's tolerance of §8.7.4.4's answer, and as four otherwise,
    /// each asked the same question.
    ///
    /// The question is asked at the three edge midpoints and the centroid: the clause's colour
    /// there is the conversion of the interpolated components, the rasteriser's is the same mix
    /// of the converted corners, and §10.7.3 says how the two are compared — "[t]he error shall
    /// be measured for each colour component, and the maximum independent error shall be used."
    /// Four points see a conversion that curves across the triangle; what they cannot see is
    /// one that steps between them, which the clause's NOTE 1 already concedes of a sampled
    /// function. A triangle whose corners state one colour is emitted unasked, since nothing
    /// between them can differ.
    ///
    /// The four children are emitted in a fixed order, and the parent's place in the mesh's
    /// order is the place all four take: §8.7.4.5.7's precedence between patches, and within
    /// one, is by emission order, and a subdivision keeps it.
    fn refine(
        &self,
        vertices: [Vertex<Components>; 3],
        colours: [Color; 3],
        depth: usize,
        out: &mut Vec<Triangle>,
        refinement: &mut Refinement,
    ) {
        let emit = |out: &mut Vec<Triangle>, vertices: &[Vertex<Components>; 3]| {
            out.push(Triangle {
                points: [vertices[0].point, vertices[1].point, vertices[2].point],
                corners: Corners::Colours(colours),
            });
        };
        let [a, b, c] = &vertices;
        if a.corner.0 == b.corner.0 && b.corner.0 == c.corner.0 {
            emit(out, &vertices);
            return;
        }

        let midpoints = [a.midpoint(b), b.midpoint(c), c.midpoint(a)];
        let midpoint_colours = midpoints
            .each_ref()
            .map(|vertex| self.colour_of(&vertex.corner));
        let centroid = Components(
            a.corner
                .0
                .iter()
                .zip(&b.corner.0)
                .zip(&c.corner.0)
                .map(|((x, y), z)| (x + y + z) / 3.0)
                .collect(),
        );
        let centroid_colour = self.colour_of(&centroid);
        let tolerance = self.tolerance();
        let along_edges = [(0, 1), (1, 2), (2, 0)].into_iter().all(|(from, to)| {
            within(
                midpoint_colours[from],
                mix_colour(colours[from], colours[to], 0.5),
                tolerance,
            )
        });
        // A third of the way from the midpoint of one edge to the opposite corner is the
        // centroid, and a rasteriser's colour there is the same mix of the corner colours.
        let across = within(
            centroid_colour,
            mix_colour(
                mix_colour(colours[0], colours[1], 0.5),
                colours[2],
                1.0 / 3.0,
            ),
            tolerance,
        );
        if along_edges && across {
            emit(out, &vertices);
            return;
        }
        if depth >= MAX_REFINEMENT_DEPTH || out.len() >= REFINED_TRIANGLES {
            refinement.coarse = true;
            emit(out, &vertices);
            return;
        }

        let [a, b, c] = vertices;
        let [ab, bc, ca] = midpoints;
        let [ab_colour, bc_colour, ca_colour] = midpoint_colours;
        let deeper = depth.saturating_add(1);
        self.refine(
            [a, ab.clone(), ca.clone()],
            [colours[0], ab_colour, ca_colour],
            deeper,
            out,
            refinement,
        );
        self.refine(
            [ab.clone(), b, bc.clone()],
            [ab_colour, colours[1], bc_colour],
            deeper,
            out,
            refinement,
        );
        self.refine(
            [ca.clone(), bc.clone(), c],
            [ca_colour, bc_colour, colours[2]],
            deeper,
            out,
            refinement,
        );
        self.refine(
            [ab, bc, ca],
            [ab_colour, bc_colour, ca_colour],
            deeper,
            out,
            refinement,
        );
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
    fn ramp(&self) -> Ramp {
        let (low, high) = self.parameter_range();
        let span = high - low;
        let breaks = crate::shading::breakpoints_over(self.functions, low, high);
        Ramp::sample_across_at(self.colouring.resolution, &breaks, |t| {
            self.colour_of_parameter(low + t * span)
        })
    }

    /// The colour the shading's functions give one parametric value.
    ///
    /// §10.5's transfer is applied here, which is inside the sampling: the ramp is a sampling of
    /// the composition rather than a composition applied to the samples, so [`Ramp`]'s own
    /// simplifier measures the colours a rasteriser will draw. `crate::shading::kind_of` has the
    /// argument, and it is the same one for every ramp in this tree.
    fn colour_of_parameter(&self, parameter: f32) -> Color {
        let mut components = Vec::new();
        for function in self.functions {
            components.extend(function.eval(&[parameter]));
        }
        transferred(
            self.colouring.into.paint(self.space, &components),
            self.colouring.transfer,
        )
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
    ///
    /// The second half of the answer is [`MAX_TRIANGLES`] having stopped the reading with a
    /// vertex still to come — see [`read`] for what is done with it.
    fn free_form<C: Corner>(
        &self,
        bits: &mut BitReader<'_>,
        refinement: &mut Refinement,
    ) -> (Vec<Triangle>, bool) {
        let mut triangles = Vec::new();
        let mut truncated = false;
        // The previous two triangles' vertices, in the specification's `va`, `vb`, `vc`.
        let mut previous: Option<[Vertex<C>; 3]> = None;

        while let Some((flag, vertex)) = self.read_vertex(bits, true) {
            // The bound is tested *after* a vertex has been read rather than before, so that
            // the flag says a vertex was dropped rather than that the count reached a number:
            // a stream whose last vertex lands exactly on the bound is complete (trap 11).
            if triangles.len() >= MAX_TRIANGLES {
                truncated = true;
                break;
            }
            let corners = match (flag, previous) {
                // A new triangle needs two more vertices, whose own flags are ignored.
                (0, _) => {
                    let Some((_, second)) = self.read_vertex(bits, true) else {
                        break;
                    };
                    let Some((_, third)) = self.read_vertex(bits, true) else {
                        break;
                    };
                    [vertex, second, third]
                }
                // Flag 1 keeps the previous triangle's `vb` and `vc`; flag 2 keeps `va`
                // and `vc`. Reversing these produces a mesh with folded triangles.
                (1, Some([_, b, c])) => [b, c, vertex],
                (2, Some([a, _, c])) => [a, c, vertex],
                // A continuation with nothing to continue is malformed.
                _ => break,
            };
            C::emit(self, corners.clone(), &mut triangles, refinement);
            previous = Some(corners);
        }
        (triangles, truncated)
    }

    /// Type 5: rows of a lattice, with triangles between consecutive rows.
    ///
    /// The second half of the answer is [`MAX_TRIANGLES`] having stopped the reading with a
    /// whole row still to come — see [`read`] for what is done with it.
    fn lattice<C: Corner>(
        &self,
        bits: &mut BitReader<'_>,
        per_row: usize,
        refinement: &mut Refinement,
    ) -> (Vec<Triangle>, bool) {
        let mut rows: Vec<Vec<Vertex<C>>> = Vec::new();
        let mut truncated = false;
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
            // Tested with a complete row in hand, for [`free_form`]'s reason: the flag then
            // says a row was dropped rather than that a count was reached.
            if rows.len().saturating_mul(per_row) > MAX_TRIANGLES {
                truncated = true;
                break;
            }
            rows.push(row);
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
                C::emit(
                    self,
                    [a.clone(), b.clone(), c.clone()],
                    &mut triangles,
                    refinement,
                );
                C::emit(
                    self,
                    [b.clone(), d.clone(), c.clone()],
                    &mut triangles,
                    refinement,
                );
            }
        }
        (triangles, truncated)
    }

    /// Types 6 and 7: Bézier patches, evaluated into triangles.
    ///
    /// Each patch's triangles are appended in the order the stream states its patches, which
    /// is what ISO 32000-2 §8.7.4.5.7's other overlap rule asks for — "[i]f one patch overlaps
    /// another, the patch that appears later in the data stream shall paint over the earlier
    /// one" — given that a mesh is painted triangle by triangle in this order.
    /// [`tessellate`] owns the rule for an overlap *within* one patch.
    ///
    /// The second half of the answer is [`MAX_TRIANGLES`] having stopped the reading with a
    /// patch still to come — see [`read`] for what is done with it.
    fn patches<C: Corner>(
        &self,
        bits: &mut BitReader<'_>,
        tensor: bool,
        refinement: &mut Refinement,
    ) -> (Vec<Triangle>, bool) {
        let boundary = 12usize;
        let total = if tensor { 16 } else { boundary };

        let mut triangles = Vec::new();
        let mut truncated = false;
        let mut previous: Option<([Point; 16], [C; 4])> = None;

        while let Some(flag) = bits.read(self.flag_bits) {
            // Tested with a patch's flag in hand, for [`free_form`]'s reason: the flag then
            // says a patch was dropped rather than that a count was reached.
            if triangles.len() >= MAX_TRIANGLES {
                truncated = true;
                break;
            }
            let flag = flag & 0b11;

            // A continuation reuses four points and two corners from the previous patch's
            // named edge, so only the rest is in the stream.
            let (mut points, mut corners, start, corner_start) = match (flag, previous.as_ref()) {
                (0, _) => (
                    [Point::new(0.0, 0.0); 16],
                    std::array::from_fn(|_| C::placeholder()),
                    0,
                    0,
                ),
                (_, Some((last, last_corners))) => {
                    let (edge, shared) = shared_edge(flag, last, last_corners);
                    let mut points = [Point::new(0.0, 0.0); 16];
                    for (slot, point) in points.iter_mut().zip(edge.iter()) {
                        *slot = *point;
                    }
                    let mut corners: [C; 4] = std::array::from_fn(|_| C::placeholder());
                    let [first, second] = shared;
                    corners[0] = first;
                    corners[1] = second;
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
            // A patch's data is *not* padded to a byte boundary, unlike a vertex's, and that
            // is a reading rather than an omission. §8.7.4.5.5's padding sentence is about a
            // vertex — "[e]ach set of vertex data shall occupy a whole number of bytes. If the
            // total number of bits required is not divisible by 8, the last data byte for each
            // vertex is padded at the end" — and §8.7.4.5.7 states that a patch is laid out
            // differently for exactly this reason: "[a]ll of a patch's control points shall be
            // given first, followed by the colour values for its corners. This differs from a
            // triangle mesh (shading types 4 and 5), in which the coordinates and colour of
            // each vertex are given together." A patch has no vertices, so the sentence it
            // cross-refers to has nothing in a patch to apply to, and the clause says nothing
            // else about alignment. No document in `doc/pdf.js/test/pdfs` or in the four
            // `doc/corpora/` submodules — 1249 files, holding nine distinct type 6 or 7
            // shadings between them — can tell the two readings apart: every one of the nine
            // states `/BitsPerFlag 8` with coordinate and component widths that are whole
            // bytes, so each patch's own total is a whole number of bytes either way. A file
            // with `/BitsPerFlag 2` would be the witness that decides it.

            let grid = control_grid(&points, tensor);
            tessellate(self, &grid, &corners, &mut triangles, refinement);
            previous = Some((points, corners));
        }
        (triangles, truncated)
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
        1 => (pick([3, 4, 5, 6]), [corners[1].clone(), corners[2].clone()]),
        2 => (pick([6, 7, 8, 9]), [corners[2].clone(), corners[3].clone()]),
        _ => (
            pick([9, 10, 11, 0]),
            [corners[3].clone(), corners[0].clone()],
        ),
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
///
/// # The order the triangles come out in is the clause's, not the loop's
///
/// A patch may fold over itself, and ISO 32000-2 §8.7.4.5.7 says which of the parameter
/// points landing on one device point wins:
///
/// > If more than one point ( u, v ) in parameter space is mapped to the same point in device
/// > space, the point selected shall be the one with the largest value of v . If multiple
/// > points have the same v , the one with the largest value of u shall be selected.
///
/// Every rasteriser here paints a mesh's triangles in the order this function returns them,
/// each overwriting what is under it, so *later in this vector* is *what the reader sees* —
/// which makes the emission order the whole of how that sentence is obeyed. The precedence
/// is therefore lexicographic in `(v, u)`, so `v` is the outer loop: the last cell written
/// over any point is the one with the largest `v`, and among equal `v` the largest `u`.
/// Nesting them the other way round answers with the largest `u` instead, which is the
/// clause's *tie-breaker* promoted over its rule (ADR 0778).
fn tessellate<C: Corner>(
    reader: &MeshReader<'_>,
    grid: &[[Point; 4]; 4],
    patch: &[C; 4],
    out: &mut Vec<Triangle>,
    refinement: &mut Refinement,
) {
    let mut points = Vec::with_capacity(
        PATCH_STEPS
            .saturating_add(1)
            .saturating_mul(PATCH_STEPS.saturating_add(1)),
    );
    let mut corners = Vec::with_capacity(points.capacity());

    for u_step in 0..=PATCH_STEPS {
        for v_step in 0..=PATCH_STEPS {
            #[expect(
                clippy::cast_precision_loss,
                reason = "PATCH_STEPS is a small constant"
            )]
            let (u, v) = (
                u_step as f32 / PATCH_STEPS as f32,
                v_step as f32 / PATCH_STEPS as f32,
            );
            points.push(surface(grid, u, v));
            // The corners are `c1` at (0,0), `c2` at (0,1), `c3` at (1,1), `c4` at (1,0),
            // matching the order the control points visit them.
            corners.push(bilinear(patch, u, v));
        }
    }

    out.reserve(PATCH_STEPS.saturating_mul(PATCH_STEPS).saturating_mul(2));
    C::emit_patch(reader, &points, &corners, out, refinement);
}

/// Whether `actual` and `interpolated` agree to §10.7.3's `tolerance` in every channel — "the
/// maximum independent error shall be used".
fn within(actual: Color, interpolated: Color, tolerance: f32) -> bool {
    (actual.r - interpolated.r).abs() <= tolerance
        && (actual.g - interpolated.g).abs() <= tolerance
        && (actual.b - interpolated.b).abs() <= tolerance
        && (actual.a - interpolated.a).abs() <= tolerance
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
    let top = corners[0].mix(&corners[1], v);
    let bottom = corners[3].mix(&corners[2], v);
    top.mix(&bottom, u)
}
