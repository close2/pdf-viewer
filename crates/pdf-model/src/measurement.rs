//! ISO 32000-2 §12.9's measurement properties: what a drawing's units mean.
//!
//! A CAD drawing or a map is a picture of something real, and §12.9.1 says what is missing from
//! the picture:
//!
//! > Users of such documents often require information about the scale and units of measurement
//! > of the corresponding real-world objects and their relationship to units in PDF user space.
//!
//! A page carries an optional `/VP` array of **viewports**, each a rectangle with a `/Measure`
//! dictionary saying what a unit of user space is worth inside it — so a plan and its detail
//! inset can be at different scales on one page, and a viewer that measures between two points
//! knows which scale to use.
//!
//! # Nothing here is drawn, and that is the clause's own position
//!
//! §12.9 states no marks. A measure dictionary is *input to a user interface*: the clause says a
//! measure dictionary "shall provide information for formatting the resulting values into
//! textual form for presentation in a graphical user interface". So this module reads the data,
//! implements the one thing the clause states as an algorithm — §12.9.2's formatting — and hands
//! the strings to whoever has a person in front of them. [`Viewports::traced`] is that hand-off
//! and `viewer_core::Query::Measure` carries it; no pixel of a page changes either way
//! (ADR 1191).
//!
//! # The rule that would be got wrong by intuition
//!
//! Viewports may overlap, and §12.9.1 does not say "the smallest" or "the innermost":
//!
//! > The dictionaries in the VP array shall be in drawing order. Since viewports might overlap,
//! > to determine the viewport to use for any point on a page, the dictionaries in the array
//! > shall be examined, starting with the last one and iterating in reverse, and the first one
//! > whose BBox entry contains the point shall be chosen.
//!
//! Last one wins, because the array is in drawing order and the last drawn is the one on top.
//! [`Viewports::at`] is that sentence, and a reader that searched forwards would answer with the
//! background scale on every page that has an inset.
//!
//! There is a second sentence of the same kind: "[a]ny measurement that potentially involves
//! multiple viewports, such as one specifying the distance between two points, shall use the
//! information specified in the viewport of the *first* point" — which is why
//! [`Viewports::at`] takes one point.
//!
//! **This module used to leave that sentence to "callers with two points", and there were
//! none**, so the rule was stated here and executed by nobody — a `shall` delegated to a layer
//! that did not exist. [`Viewports::distance`] carries it out: it needs two points and a page
//! and no user interface at all, and Table 267's `/D` cell is what makes it more than a
//! hypotenuse — each axis is converted by its own array's first factor *before* the distance
//! function is applied. [`Viewports::traced`] is the same rule over a whole path a person traced,
//! because *such as* makes two points the clause's example of a measurement involving several
//! viewports rather than its definition.
//!
//! # §12.9.2's algorithm, and its own worked example
//!
//! A number format *array* is a sequence of units in descending granularity — miles, then feet,
//! then inches — and §12.9.2 turns a number into "1 mi 2,378 ft 7 5/8 in" by walking it,
//! carrying the fractional remainder from each unit into the next. [`format`] is those five
//! steps, and the clause's EXAMPLE is its test.
//!
//! # The corpus's one witness
//!
//! One document of the 974 states a `/VP`, and it is `GEO` rather than `RL` — `bug1146106.pdf`,
//! a map whose `/GCS` is a geographic system stated as 145 characters of Well Known Text, with
//! four `/GPTS`–`/LPTS` registration points. Two things about it are the *file* being wrong and
//! are asserted in `tests/measurement.rs` rather than accommodated: its `/Name` is UTF-16
//! **little**-endian, which §7.9.2.2 has no case for, and its `/BBox` is stated upper-left
//! first, which Table 265 forbids and which this reader keeps as written because the ordering
//! is what "shall determine the orientation of the measuring coordinate system".
//!
//! So §12.9.2's algorithm has no corpus witness at all and is tested against the clause's own
//! worked example (trap 8), while §12.10's dictionaries have exactly one.
//!
//! **A census over names will contradict that sentence, and the sentence is right.** Asking the
//! corpus "does any page state a `/VP`" answers *one*, which looks like a witness for this whole
//! module; the distinction that decides it is one paragraph up — that viewport's `/Measure` is
//! `GEO`, and §12.9.2's arithmetic is `RL`'s. The five-hundred-and-seventieth session re-ran every
//! absence claim in this tree and this is the one that survived a measurement taken at the wrong
//! granularity, which is why `examples/absence_audit` asks structures rather than names. ADR 0405.

use pdf_syntax::{Dictionary, Document, Object};

/// Most viewports read from one page.
///
/// A viewport is a region of a page a person measures in; a page stating more of them than this
/// is one making a reader work.
const MAX_VIEWPORTS: usize = 1024;

/// Most number format dictionaries walked in one array.
///
/// The clause's own example uses three — miles, feet, inches — and a unit system with more
/// than this many granularities does not exist.
const MAX_UNITS: usize = 32;

/// Most points read from one geospatial array.
///
/// §12.10's `/GPTS`, `/LPTS`, `/Bounds` and `/XPTS` are all lists a file states, and a neatline
/// tracing a coastline is genuinely long — this bounds the work without bounding any real map.
const MAX_POINTS: usize = 1 << 16;

/// A rectangular region of a page with its own measuring system. Table 265.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    /// Table 265's `/BBox`, "[a] rectangle in default user space coordinates specifying the
    /// location of the viewport on the page".
    ///
    /// **Not normalised here**, unlike every other rectangle in this crate. The clause requires
    /// the file to state it lower-left first and then makes the ordering load-bearing: "[t]his
    /// ordering shall determine the orientation of the measuring coordinate system (that is,
    /// the direction of the positive x and y axes) in this viewport, which may have a different
    /// rotation from the page". Sorting the corners would throw that away.
    pub bbox: [f32; 4],
    /// Table 265's `/Name`, "[a] descriptive text string or title of the viewport, intended for
    /// use in a user interface".
    pub name: Option<String>,
    /// Table 265's `/Measure`, the scale and units inside this rectangle.
    pub measure: Option<Measure>,
    /// Whether Table 265's `/PtData` is present.
    ///
    /// §12.10.5's point data is geospatial and is not read; the flag exists so that a caller
    /// can say a viewport carries data this program does not use rather than implying it has
    /// none.
    pub has_point_data: bool,
}

impl Viewport {
    /// Whether this viewport's rectangle contains a point in default user space.
    ///
    /// The comparison normalises the corners even though [`Self::bbox`] does not, because
    /// containment is a question about the *region* and the orientation only decides which way
    /// the measuring axes run.
    #[must_use]
    pub fn contains(&self, (x, y): (f32, f32)) -> bool {
        let [x0, y0, x1, y1] = self.bbox;
        x >= x0.min(x1) && x <= x0.max(x1) && y >= y0.min(y1) && y <= y0.max(y1)
    }

    /// Where a point in default user space falls in this viewport's **unit square**.
    ///
    /// §12.10 states two of its arrays in that square rather than on the page: `/LPTS` holds
    /// "points in a 2D unit square" and says which square — "[t]he unit square is mapped to the
    /// rectangular bounds of the `Viewport`, image `XObject`, or forms `XObject` that contains
    /// the measure dictionary" — and `/Bounds`'s numbers "are expressed relative to a unit
    /// square that describes the `BBox` associated with a `Viewport`". So this is the one leg of
    /// a georeference that needs nothing outside the file, and it is a division.
    ///
    /// **The corners are used as stated and not normalised**, which is [`Self::bbox`]'s own
    /// reason: Table 265 makes their order "determine the orientation of the measuring
    /// coordinate system (that is, the direction of the positive x and y axes) in this
    /// viewport", so a file stating them the other way round has said its axes run the other
    /// way, and a point outside the rectangle lands outside the square on the same side it
    /// lies on.
    ///
    /// `None` for a rectangle with no extent along an axis, which states no square to map into.
    #[must_use]
    pub fn unit_square(&self, (x, y): (f32, f32)) -> Option<[f64; 2]> {
        let [x0, y0, x1, y1] = self.bbox.map(f64::from);
        let (width, height) = (x1 - x0, y1 - y0);
        if width == 0.0 || height == 0.0 {
            return None;
        }
        Some([(f64::from(x) - x0) / width, (f64::from(y) - y0) / height])
    }
}

/// A page's viewports, in the order the `/VP` array holds them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Viewports {
    /// The viewports, "in drawing order" — so the last is the one drawn on top.
    pub viewports: Vec<Viewport>,
}

impl Viewports {
    /// Reads a page's `/VP`, which almost no document has.
    #[must_use]
    pub fn read(document: &Document, page: &Dictionary) -> Self {
        let array = document.get_key(page, "VP");
        let Some(array) = array.as_array() else {
            return Self::default();
        };
        Self {
            viewports: array
                .iter()
                .take(MAX_VIEWPORTS)
                .filter_map(|entry| viewport(document, &document.resolve(entry)))
                .collect(),
        }
    }

    /// Whether the page states any viewport.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.viewports.is_empty()
    }

    /// The viewport that applies at a point: the **last** one whose `/BBox` contains it.
    ///
    /// See the module comment — the array is in drawing order and the clause searches it in
    /// reverse, so an inset drawn over a plan wins inside its own rectangle.
    #[must_use]
    pub fn at(&self, point: (f32, f32)) -> Option<&Viewport> {
        self.viewports
            .iter()
            .rev()
            .find(|viewport| viewport.contains(point))
    }

    /// The distance between two points on the page, in the units the document states.
    ///
    /// **§12.9.1's second selection sentence, carried out rather than delegated.** The clause
    /// says which viewport a two-point measurement belongs to:
    ///
    /// > Any measurement that potentially involves multiple viewports, such as one specifying
    /// > the distance between two points, shall use the information specified in the viewport of
    /// > the first point.
    ///
    /// This module said that sentence and left it to "callers with two points", of which this
    /// tree had none — so the rule was stated and executed by nobody. It needs no user interface:
    /// two points and a page are all of its inputs.
    ///
    /// **The arithmetic is Table 267's `/D` cell and not a `hypot` of the page.** That cell says
    /// the array's first element converts "from units represented by the first element in `X`",
    /// and states the order outright: "[t]he scale factors from `X`, `Y` (if present) and `CYX`
    /// (if `Y` is present) shall be used to convert from default user space to the appropriate
    /// units **before** applying the distance function". So each axis is scaled by its own array's
    /// first conversion, `/CYX` brings the y result into x's units where the two differ, the
    /// distance is taken there, and [`format`] walks `/D` over the result. Taking the distance
    /// first and scaling afterwards is a different number on every drawing whose axes differ, and
    /// is what the word *before* forbids.
    ///
    /// Neither `/O` nor the `/BBox` corner order takes part, and that is the clause's arithmetic
    /// rather than an omission: an origin is a translation and the corner order an orientation,
    /// and a distance between two points is invariant under both.
    ///
    /// **`None` has four causes and each is the clause's own.** No viewport's `/BBox` contains
    /// the first point, so the page states no units there; the viewport states no `/Measure`, so
    /// Table 265's optional entry is absent; its `/Subtype` is `GEO` or a name later than this
    /// standard, whose distance is §12.10's ellipsoid and not this arithmetic; or `/Y` is stated
    /// with no `/CYX`, which Table 267 makes a refusal rather than a gap — "if not specified,
    /// these calculations may not be performed (which would be the case in situations such as x
    /// representing time and y representing temperature)". An empty `/D` answers with the empty
    /// string, which is [`format`]'s documented answer to an array stating no units.
    #[must_use]
    pub fn distance(&self, from: (f32, f32), to: (f32, f32)) -> Option<String> {
        self.at(from)
            .and_then(|viewport| viewport.measure.as_ref())
            .and_then(|measure| measure.distance(from, to))
    }

    /// What a path a person traced across the page comes to, in the document's own units.
    ///
    /// **§12.9.1's second selection sentence decides the viewport and nothing else does**:
    ///
    /// > Any measurement that potentially involves multiple viewports, such as one specifying
    /// > the distance between two points, shall use the information specified in the viewport of
    /// > the first point.
    ///
    /// So the whole path is measured in the first point's viewport even where it leaves that
    /// rectangle, which is the clause choosing one answer over two rather than this reader
    /// choosing convenience. The points are in default user space, which is where Table 265
    /// states a `/BBox`.
    ///
    /// # Which quantity each number is, and why all of them come back together
    ///
    /// Table 267 gives a rectilinear measure a separate number format array for each quantity —
    /// `/D` for distance, `/A` for area, `/T` for angles, `/S` for slope — and says nothing
    /// about which of them a *gesture* is asking for. A file that states `/A` and no `/D` has
    /// said areas are what its drawing is measured in; one that states both has said a person
    /// may want either. So every quantity the points support and the file formats is answered,
    /// and which to show is the question a user interface is for. It is the same decision
    /// [`Measured`] already takes for an annotation's own geometry, and for the same sentence.
    ///
    /// The angle is taken at the **last vertex** of the path, between the legs that meet there,
    /// and the slope is the **last leg**'s: a person tracing a path is asking about the piece
    /// they have just drawn, and every earlier one was answered while they drew it.
    ///
    /// `None` where no viewport's `/BBox` contains the first point — the page states no units
    /// there — and for an empty path. A viewport with no `/Measure` answers a [`Traced`] whose
    /// quantities are all absent and whose name is the one Table 265 states, which is the file
    /// saying it has a region and no scale for it.
    #[must_use]
    pub fn traced(&self, points: &[[f32; 2]]) -> Option<Traced> {
        let first = *points.first()?;
        let viewport = self.at((first[0], first[1]))?;
        let measure = viewport.measure.as_ref();
        // The last three points and the last two, taken as windows rather than by index: a path
        // shorter than either has no such leg, and `windows` is the expression of that.
        let vertex = points.windows(3).next_back();
        let leg = points.windows(2).next_back();
        Some(Traced {
            viewport: viewport.name.clone(),
            ratio: measure
                .and_then(Measure::rectilinear)
                .map(|scale| scale.ratio.clone())
                .filter(|ratio| !ratio.is_empty()),
            length: stated(measure.and_then(|measure| measure.length(points))),
            area: stated(measure.and_then(|measure| measure.area(points))),
            angle: stated(
                measure
                    .zip(vertex)
                    .and_then(|(measure, at)| measure.angle(at[1], at[0], at[2])),
            ),
            slope: stated(
                measure
                    .zip(leg)
                    .and_then(|(measure, leg)| measure.slope(leg[0], leg[1])),
            ),
            geospatial: measure
                .and_then(|measure| match measure {
                    Measure::Geospatial(geospatial) => Some(geospatial),
                    Measure::Rectilinear(_) | Measure::Other(_) => None,
                })
                .map(|geospatial| Geographic::of(viewport, geospatial, points)),
        })
    }
}

/// A formatted quantity the file actually stated, or nothing.
///
/// **The empty string is not a measurement**, and the distinction is Table 267's: each of its
/// four number format arrays is *(Optional)*, so a measure dictionary stating no `/S` has said
/// nothing about how a slope should be displayed — while [`format`] answers an absent array with
/// the empty string, because that is what §12.9.2's algorithm produces when it has no unit to
/// walk. The two are different answers and only one of them belongs in a user interface: a window
/// showing a blank beside the word *slope* would be claiming the document has a gradient of
/// nothing.
fn stated(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

/// What a traced path comes to in a viewport's own measuring system.
///
/// Every field is either a string §12.9.2's algorithm produced or a string the file states for a
/// person to read; nothing here is a number, because §12.9 is explicit that a measure dictionary
/// "shall provide information for formatting the resulting values into textual form for
/// presentation in a graphical user interface". A host shows what it has room for.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Traced {
    /// Table 265's `/Name`, "[a] descriptive text string or title of the viewport, intended for
    /// use in a user interface" — which is this.
    pub viewport: Option<String>,
    /// Table 267's `/R`, "[a] text string expressing the scale ratio of the drawing", such as
    /// `1/4 in = 1 ft`. Stated by the producer for a person and passed through unparsed.
    pub ratio: Option<String>,
    /// The path's total length, formatted by `/D`.
    pub length: Option<String>,
    /// The area it encloses, formatted by `/A`. Absent for fewer than three points.
    pub area: Option<String>,
    /// The angle at its last vertex, formatted by `/T`. Absent for fewer than three points.
    pub angle: Option<String>,
    /// The slope of its last leg, formatted by `/S`. Absent for a single point and for a
    /// vertical leg.
    pub slope: Option<String>,
    /// What §12.10 states about the path, where the viewport's measure is a geospatial one.
    pub geospatial: Option<Geographic>,
}

impl Traced {
    /// Whether the viewport stated no quantity at all for this path.
    ///
    /// True for a viewport with no `/Measure`, and for one whose measuring system describes none
    /// of the quantities these points support — a `/Y` with no `/CYX` over two points, say,
    /// which Table 267 refuses a distance for by name.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length.is_none() && self.area.is_none() && self.angle.is_none() && self.slope.is_none()
    }
}

/// What §12.10 states about a traced path in a geospatial viewport.
///
/// # The half that is the file's and the half that is a registry's
///
/// Everything here is read out of the document. What is **not** here is a latitude, and the
/// reason is the clause rather than the work: §12.10 states the correspondence between the
/// object's unit square and the earth — `/GPTS` against `/LPTS`, point for point — and states no
/// function between the registration points. Turning an arbitrary position into a coordinate
/// means choosing one, and where `/GCS` is projected it means the EPSG registry and ISO 19162's
/// grammar as well, both of which §12.10.3 names as texts outside this standard. So a position
/// this reader cannot derive is absent rather than guessed, and what a host says instead is what
/// the file does state: which system the map is in, how many points register it, and whether the
/// place a person is pointing at is one the document's own neatline covers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Geographic {
    /// `/GCS`, the system the map's coordinates are in. Table 269 requires it.
    pub system: Option<CoordinateSystem>,
    /// `/DCS`, the system Table 269 says "shall be used for the display of position values".
    pub display_system: Option<CoordinateSystem>,
    /// `/PDU`'s three preferred display units: linear, area, angular, in that order.
    pub units: Option<[String; 3]>,
    /// How many `/GPTS`–`/LPTS` pairs register the object to the earth.
    pub registration: usize,
    /// Whether every point of the path is inside `/Bounds` — the region "for which geospatial
    /// transformations are valid".
    pub within_bounds: bool,
    /// Whether `/PCSM` is the transformation Table 269 would have applied here, which needs a
    /// projected `/GCS` beside the matrix ([`Geospatial::matrix_has_priority`]).
    pub matrix_applies: bool,
}

impl Geographic {
    /// Reads what §12.10 states about these points, in this viewport.
    fn of(viewport: &Viewport, geospatial: &Geospatial, points: &[[f32; 2]]) -> Self {
        Self {
            system: geospatial.coordinate_system.clone(),
            display_system: geospatial.display_system.clone(),
            units: geospatial.display_units.clone(),
            registration: geospatial.registration().len(),
            within_bounds: !points.is_empty()
                && points.iter().all(|point| {
                    viewport
                        .unit_square((point[0], point[1]))
                        .is_some_and(|local| geospatial.within_bounds(local))
                }),
            matrix_applies: geospatial.matrix_has_priority(),
        }
    }
}

/// A measuring coordinate system. Table 266.
#[derive(Debug, Clone, PartialEq)]
pub enum Measure {
    /// `RL`, "a rectilinear coordinate system … in which the x and y axes are perpendicular and
    /// have units that increment linearly (to the right and up, respectively)".
    ///
    /// Table 266's default: a measure dictionary with no `/Subtype` is this one.
    Rectilinear(Box<Rectilinear>),
    /// `GEO`, PDF 2.0's geospatial coordinate system (§12.10).
    Geospatial(Box<Geospatial>),
    /// A `/Subtype` this reader does not know.
    ///
    /// The clause invites them — "[o]ther subtypes may be used, providing the flexibility to
    /// measure using other types of coordinate systems" — so an unknown name is a document
    /// using a later standard rather than a malformed one.
    Other(String),
}

impl Measure {
    /// The distance between two points in default user space, in the units this measure states.
    ///
    /// **The arithmetic is Table 267's `/D` cell and not a `hypot` of the page.** That cell says
    /// the array's first element converts "from units represented by the first element in `X`",
    /// and states the order outright: "[t]he scale factors from `X`, `Y` (if present) and `CYX`
    /// (if `Y` is present) shall be used to convert from default user space to the appropriate
    /// units **before** applying the distance function". So each axis is scaled by its own
    /// array's first conversion, `/CYX` brings the y result into x's units where the two differ,
    /// the distance is taken there, and [`format`] walks `/D` over the result. Taking the
    /// distance first and scaling afterwards is a different number on every drawing whose axes
    /// differ, and is what the word *before* forbids.
    ///
    /// Neither `/O` nor the `/BBox` corner order takes part, and that is the clause's arithmetic
    /// rather than an omission: an origin is a translation and the corner order an orientation,
    /// and a distance between two points is invariant under both.
    ///
    /// `None` where [`Rectilinear::axes`] states no conversion, and where the subtype is `GEO` or
    /// a name later than this standard — §12.10's distance is an ellipsoid's, not this
    /// arithmetic. An empty `/D` answers with the empty string, which is [`format`]'s documented
    /// answer to an array stating no units.
    #[must_use]
    pub fn distance(&self, from: (f32, f32), to: (f32, f32)) -> Option<String> {
        let scale = self.rectilinear()?;
        let (along_x, along_y) = scale.axes()?;
        let (dx, dy) = (f64::from(to.0 - from.0), f64::from(to.1 - from.1));
        Some(format((dx * along_x).hypot(dy * along_y), &scale.distance))
    }

    /// The length of the path through these points, in the units this measure states.
    ///
    /// The same `/D` arithmetic as [`Self::distance`], applied to each leg and summed in the
    /// measuring system's own units rather than in user space: Table 267 puts the conversion
    /// *before* the distance function, so scaling once at the end would be a different number on
    /// any drawing whose axes differ. [`format`] is a display step and runs once, over the total.
    ///
    /// Fewer than two points state no path and answer `None` rather than a length of zero — a
    /// caller may not be told that a shape this could not read measures nothing.
    #[must_use]
    pub fn length(&self, points: &[[f32; 2]]) -> Option<String> {
        let scale = self.rectilinear()?;
        let (along_x, along_y) = scale.axes()?;
        if points.len() < 2 {
            return None;
        }
        let total: f64 = points
            .windows(2)
            .map(|leg| {
                let (dx, dy) = (
                    f64::from(leg[1][0] - leg[0][0]),
                    f64::from(leg[1][1] - leg[0][1]),
                );
                (dx * along_x).hypot(dy * along_y)
            })
            .sum();
        Some(format(total, &scale.distance))
    }

    /// The area these points enclose, in the units this measure states.
    ///
    /// Table 267's `/A` cell states the same conversion order as `/D` does and squares the unit:
    /// "[t]he first element in the array shall specify the conversion to the largest area unit
    /// from units represented by the first element in `X`, squared", and again "[t]he scale
    /// factors from `X`, `Y` (if present) and `CYX` (if `Y` is present) shall be used to convert
    /// from default user space to the appropriate units before applying the area function". So
    /// each coordinate is scaled into the measuring system first and the enclosed area taken
    /// there.
    ///
    /// The enclosed area of a closed polygon is the shoelace sum, which is planar geometry rather
    /// than a reading of anything: the clause says what units the answer is in and nothing about
    /// how an area is computed, because there is one answer. Its magnitude is taken, so a shape
    /// whose vertices run clockwise measures the same as its mirror — the standard gives `/A` no
    /// sign, and a negative area is not a quantity a unit can be put on.
    ///
    /// Fewer than three points enclose nothing and answer `None` for [`Self::length`]'s reason.
    #[must_use]
    pub fn area(&self, points: &[[f32; 2]]) -> Option<String> {
        let scale = self.rectilinear()?;
        let (along_x, along_y) = scale.axes()?;
        if points.len() < 3 {
            return None;
        }
        let scaled: Vec<(f64, f64)> = points
            .iter()
            .map(|[x, y]| (f64::from(*x) * along_x, f64::from(*y) * along_y))
            .collect();
        // The shoelace sum walks every leg including the one back to the first vertex, which is
        // what closes the shape: `previous` starts at the last point so that leg is the first
        // term rather than a special case after the loop.
        let mut twice = 0.0;
        let mut previous = *scaled.last()?;
        for point in &scaled {
            twice += previous.0.mul_add(point.1, -(point.0 * previous.1));
            previous = *point;
        }
        Some(format(twice.abs() / 2.0, &scale.area))
    }

    /// The angle at `at` between the rays to `from` and to `to`, in the units this measure
    /// states.
    ///
    /// **The initial value is degrees, which §12.9.2 states outright**: step a) says the entry
    /// referencing a number format array "determines the meaning of the initial measurement
    /// value" and gives this very entry as its example — "the `T` entry specifies degrees". So
    /// the angle is taken in degrees and [`format`] applies Table 267's `/T`, whose first
    /// element "shall specify the conversion to the largest angle unit from degrees".
    ///
    /// **The angle is taken in the measuring system and not on the page**, which is the other
    /// half of that cell: "[t]he scale factor from `CYX` (if present) shall be used to convert
    /// from default user space to the appropriate units before applying the angle function".
    /// [`Rectilinear::axes`] is the pair of factors that conversion comes to — where `/Y` is
    /// absent both are `/X`'s and cancel, so the angle is the page's own; where `/Y` is present
    /// the ratio between them is what `/CYX` supplies, and a drawing whose axes carry different
    /// units has angles that are not the page's.
    ///
    /// `None` where [`Rectilinear::axes`] states no conversion — which includes Table 267's own
    /// refusal for a `/Y` with no `/CYX`, and that refusal covers this quantity by name: `/CYX`
    /// "shall be used for calculations (distance, area, and angle) where the units are be
    /// equivalent; if not specified, these calculations may not be performed". `None` also where
    /// either ray has no length, because two coincident points name no direction.
    ///
    /// The answer is the non-reflex angle, in `[0, 180]`. The standard gives `/T` no sign and no
    /// orientation, so a quantity a unit can be put on is what is measured; which of the two
    /// angles at a vertex a person meant is a question no clause answers.
    #[must_use]
    pub fn angle(&self, at: [f32; 2], from: [f32; 2], to: [f32; 2]) -> Option<String> {
        let scale = self.rectilinear()?;
        let (along_x, along_y) = scale.axes()?;
        let ray = |point: [f32; 2]| {
            (
                f64::from(point[0] - at[0]) * along_x,
                f64::from(point[1] - at[1]) * along_y,
            )
        };
        let (first, second) = (ray(from), ray(to));
        let (first_length, second_length) = (first.0.hypot(first.1), second.0.hypot(second.1));
        if first_length == 0.0 || second_length == 0.0 {
            return None;
        }
        // `atan2` of the cross and the dot products rather than `acos` of the normalised dot:
        // the two agree, and this one keeps its precision for the small angles where the dot
        // product is within a rounding of the product of the lengths.
        let cross = first.0.mul_add(second.1, -(first.1 * second.0));
        let dot = first.0.mul_add(second.0, first.1 * second.1);
        Some(format(cross.abs().atan2(dot).to_degrees(), &scale.angle))
    }

    /// The slope of the line from `from` to `to`, in the units this measure states.
    ///
    /// Table 267's `/S`: its first element "shall specify the conversion to the largest slope
    /// unit from units represented by the first element in `Y` divided by the first element in
    /// `X`". So the value [`format`] is given is a change along y in the y array's largest units
    /// over a change along x in the x array's largest units, and the label the file puts on it
    /// is a label on that ratio.
    ///
    /// **`/CYX` takes no part, and that is Table 267 saying so rather than an omission.** The
    /// `/S` cell names all three factors together, and the `/CYX` cell then separates them: that
    /// entry "shall be used for calculations (distance, area, and angle) where the units are be
    /// equivalent", and "[o]ther calculations (change in x , change in y , and slope) shall not
    /// require this value". A slope is a ratio *between* the two axes' own units and never a
    /// length in one of them, so the entry that makes the two commensurable has nothing to do —
    /// which is why [`Self::slope`] answers on a drawing where [`Self::distance`] refuses, and
    /// why a plot of temperature against time, the clause's own example of such a drawing, has a
    /// gradient and no distance.
    ///
    /// `None` where `/X` states no number format, and for a vertical line: a change of nothing
    /// along x has no ratio, and the standard states no slope for one. A caller is told that
    /// rather than handed an infinity with a unit on it.
    #[must_use]
    pub fn slope(&self, from: [f32; 2], to: [f32; 2]) -> Option<String> {
        let scale = self.rectilinear()?;
        let (along_x, along_y) = scale.gradient_axes()?;
        let (dx, dy) = (
            f64::from(to[0] - from[0]) * along_x,
            f64::from(to[1] - from[1]) * along_y,
        );
        if dx == 0.0 {
            return None;
        }
        Some(format(dy / dx, &scale.slope))
    }

    /// The rectilinear entries, where that is the subtype this measure states.
    ///
    /// `GEO` and any subtype later than this standard answer `None`: §12.10's coordinates are an
    /// ellipsoid's and Table 267's conversions do not describe them.
    fn rectilinear(&self) -> Option<&Rectilinear> {
        match self {
            Self::Rectilinear(scale) => Some(scale),
            Self::Geospatial(_) | Self::Other(_) => None,
        }
    }
}

/// A rectilinear measuring system. Table 267.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rectilinear {
    /// `/R`, "[a] text string expressing the scale ratio of the drawing", such as
    /// `1/4 in = 1 ft`. Stated for a person, and this reader does not parse it.
    pub ratio: String,
    /// `/X`, the number format array for change along the x axis.
    pub x: Vec<NumberFormat>,
    /// `/Y`, "[r]equired when the x and y scales have different units or conversion factors".
    pub y: Vec<NumberFormat>,
    /// `/D`, for measurement of distance in any direction.
    pub distance: Vec<NumberFormat>,
    /// `/A`, for measurement of area.
    pub area: Vec<NumberFormat>,
    /// `/T`, for measurement of angles.
    pub angle: Vec<NumberFormat>,
    /// `/S`, for measurement of the slope of a line.
    pub slope: Vec<NumberFormat>,
    /// `/O`, the origin of the measuring coordinate system in default user space.
    ///
    /// `None` takes the table's default, which is not a constant: "the first coordinate pair
    /// (lower-left corner) of the rectangle specified by the viewport's `BBox` entry" — a
    /// default that depends on another dictionary, which is why it is not resolved here.
    pub origin: Option<[f32; 2]>,
    /// `/CYX`, converting the largest y units to the largest x units.
    ///
    /// Meaningful only where `/Y` is present, and its absence is a *statement*: "if not
    /// specified, these calculations may not be performed (which would be the case in
    /// situations such as x representing time and y representing temperature)".
    pub cyx: Option<f64>,
}

impl Rectilinear {
    /// The two factors that carry a default-user-space coordinate into the measuring system.
    ///
    /// Table 267 gives `/X` "the scale factor for converting from default user space units to
    /// the largest units in the measuring coordinate system along that axis", and makes `/Y`
    /// "(Required when the x and y scales have different units or conversion factors)" — so an
    /// absent `/Y` is the file saying `/X` measures both axes, which is what `/X`'s own cell
    /// says: "for measurement of change along the x axis and, if `Y` is not present, along the y
    /// axis as well". Where `/Y` is present, the y factor carries `/CYX` with it, because that
    /// entry is what puts a y measurement into x's units — and its absence is Table 267 refusing
    /// the calculation rather than leaving a gap: "if not specified, these calculations may not
    /// be performed (which would be the case in situations such as x representing time and y
    /// representing temperature)".
    ///
    /// `None` where `/X` states no number format at all, so there is no conversion to apply.
    fn axes(&self) -> Option<(f64, f64)> {
        let along_x = self.x.first()?.conversion;
        match self.y.first() {
            None => Some((along_x, along_x)),
            Some(first) => Some((along_x, first.conversion * self.cyx?)),
        }
    }

    /// The two factors a **slope** is taken with, which are [`Self::axes`]'s without `/CYX`.
    ///
    /// Table 267 puts slope on the other side of its own division: `/CYX` "shall be used for
    /// calculations (distance, area, and angle) where the units are be equivalent", and
    /// "[o]ther calculations (change in x , change in y , and slope) shall not require this
    /// value". So a gradient is a `/Y` unit over an `/X` unit, and the entry that would make the
    /// two commensurable is the one thing that must not be applied to it — which also means a
    /// drawing stating `/Y` and no `/CYX` has a slope where it has no distance.
    ///
    /// `None` where `/X` states no number format, which is [`Self::axes`]'s condition too: with
    /// no conversion along x there is nothing to divide by.
    fn gradient_axes(&self) -> Option<(f64, f64)> {
        let along_x = self.x.first()?.conversion;
        Some((
            along_x,
            self.y.first().map_or(along_x, |first| first.conversion),
        ))
    }
}

/// One unit in a number format array. Table 268.
#[derive(Debug, Clone, PartialEq)]
pub struct NumberFormat {
    /// `/U`, "[a] text string specifying a label for displaying the units represented by this
    /// dictionary in a user interface".
    pub unit: String,
    /// `/C`, "[t]he conversion factor used to multiply a value in partial units of the previous
    /// number format array element to obtain a value in the units of this dictionary".
    pub conversion: f64,
    /// `/F`, how a fractional value is shown. Table 268's default is [`Fraction::Decimal`].
    pub fraction: Fraction,
    /// `/D`, "the precision or denominator of a fractional amount".
    ///
    /// Its default depends on `/F`: 100 for a decimal display, 16 for a fractional one, so it
    /// is resolved when the format is read rather than carried as an `Option`.
    pub denominator: u32,
    /// `/FD`: when true, a denominator may not be reduced nor low-order zeros truncated.
    pub keep_denominator: bool,
    /// `/RT`, "[t]ext that shall be used between orders of thousands". Default `,`.
    pub thousands: String,
    /// `/RD`, "[t]ext that shall be used as the decimal position". Default `.`.
    pub decimal_point: String,
    /// `/PS`, text concatenated to the left of the label. Default a single space.
    pub prefix_spacing: String,
    /// `/SS`, text concatenated after the label. Default a single space.
    pub suffix_spacing: String,
    /// `/O`, whether the label is a suffix (`S`, the default) or a prefix (`P`) of the value.
    pub label_before_value: bool,
}

/// Table 268's `/F`: whether and how a fractional value is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fraction {
    /// `D` — "[s]how as decimal to the precision specified by the D entry". The default.
    #[default]
    Decimal,
    /// `F` — "[s]how as a fraction with denominator specified by the D entry".
    Fractional,
    /// `R` — "[n]o fractional part; round to the nearest whole unit".
    Round,
    /// `T` — "[n]o fractional part; truncate to achieve whole units".
    Truncate,
}

/// §12.9.2's algorithm: a value and a number format array become a string.
///
/// The clause's five steps, in order. Each element converts the *fractional remainder* left by
/// the one before it — step e), "[m]ultiply its C entry by the fractional result from the
/// previous step" — so the array walks from the coarsest unit to the finest and stops as soon as
/// nothing is left over (step c) or the units run out (step d).
///
/// The result of the clause's own EXAMPLE is `1 mi 2,378 ft 7 5/8 in`, which is this function's
/// test. Two things about that string are decisions rather than arithmetic, and both are
/// documented on [`unit`]: the spacing between elements comes from `/PS` and `/SS`, and the
/// trailing one is trimmed.
///
/// An empty array produces an empty string: an array is "one or more number format
/// dictionaries", so a file stating none has said nothing about how to display anything.
#[must_use]
pub fn format(value: f64, formats: &[NumberFormat]) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let mut remaining = value.abs();
    let mut out = String::from(sign);
    for (index, format) in formats.iter().take(MAX_UNITS).enumerate() {
        let scaled = remaining * format.conversion;
        let last = index.saturating_add(1) >= formats.len().min(MAX_UNITS);
        let whole = scaled.trunc();
        let fractional = scaled - whole;

        // Step c): "[i]f the result contains no non-zero fractional portion", the label goes on
        // and the formatting is complete — whatever the array still holds.
        if fractional == 0.0 {
            out.push_str(&unit(&integer(whole, format), format));
            return out.trim_end().to_owned();
        }
        if last {
            // Step d): the last dictionary decides how the leftover is shown.
            out.push_str(&unit(&fractional_text(scaled, format), format));
            return out.trim_end().to_owned();
        }
        // Step e): carry the fraction into the next, finer unit.
        out.push_str(&unit(&integer(whole, format), format));
        remaining = fractional;
    }
    out.trim_end().to_owned()
}

/// A whole number with `/RT` between orders of thousands.
fn integer(value: f64, format: &NumberFormat) -> String {
    let digits = format!("{:.0}", value.abs().trunc());
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        let from_end = digits.len().saturating_sub(index);
        if index > 0 && from_end % 3 == 0 {
            out.push_str(&format.thousands);
        }
        out.push(digit);
    }
    out
}

/// The last element's value, shown as `/F` asks for.
fn fractional_text(value: f64, format: &NumberFormat) -> String {
    let whole = value.trunc();
    let fractional = value - whole;
    match format.fraction {
        Fraction::Round => integer(value.round(), format),
        Fraction::Truncate => integer(whole, format),
        Fraction::Fractional => {
            // "[T]he denominator of a fractional display. The fraction may be reduced unless the
            // value of FD is true."
            let denominator = format.denominator.max(1);
            let numerator = (fractional * f64::from(denominator)).round();
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a rounded product of a value in 0.0..1.0 with a u32 denominator"
            )]
            let (mut numerator, mut denominator) = (numerator as u32, denominator);
            if numerator == 0 {
                return integer(whole, format);
            }
            if numerator >= denominator {
                // The fraction rounded up to a whole unit, which is a carry rather than a
                // fraction of `denominator/denominator`.
                return integer(whole + 1.0, format);
            }
            if !format.keep_denominator {
                let divisor = gcd(numerator, denominator).max(1);
                numerator = numerator.checked_div(divisor).unwrap_or(numerator);
                denominator = denominator.checked_div(divisor).unwrap_or(denominator);
            }
            format!("{} {numerator}/{denominator}", integer(whole, format))
        }
        Fraction::Decimal => {
            // "[T]he precision of a decimal display; it shall be a multiple of 10", so /D 100
            // is two places. Low-order zeros are truncated "unless FD is true".
            let places = usize::try_from(format.denominator.max(1).ilog10()).unwrap_or(2);
            let text = format!("{:.*}", places, value.abs().fract());
            let digits = text.split_once('.').map_or(String::new(), |(_, rest)| {
                if format.keep_denominator {
                    rest.to_owned()
                } else {
                    rest.trim_end_matches('0').to_owned()
                }
            });
            if digits.is_empty() {
                integer(whole, format)
            } else {
                format!("{}{}{digits}", integer(whole, format), format.decimal_point)
            }
        }
    }
}

/// Greatest common divisor, for reducing a fraction.
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a.checked_rem(b).unwrap_or(0));
    }
    a
}

/// A formatted number with its label attached, as `/O`, `/PS` and `/SS` ask.
///
/// §12.9.1's Table 268, of `/O`:
///
/// > The characters specified by PS and SS shall be concatenated before considering this entry.
///
/// So the label carries its own spacing on both sides and `/O` decides which side of the value
/// the whole of it goes. The defaults are a single space each, which is what makes the clause's
/// EXAMPLE concatenate to `1 mi 2,378 ft 7 5/8 in` — every element ends with a space, and
/// [`format`] trims the last one. **Trimming is a documented choice**: the clause states the
/// concatenation and says nothing about what to do with the space its own example does not
/// show.
fn unit(value: &str, format: &NumberFormat) -> String {
    let label = format!(
        "{}{}{}",
        format.prefix_spacing, format.unit, format.suffix_spacing
    );
    if format.label_before_value {
        format!("{label}{value}")
    } else {
        format!("{value}{label}")
    }
}

/// §12.10.2's geospatial measure dictionary. Table 269.
///
/// The clause's own summary of what it is for: it "contains a description of the earth-based
/// coordinate system associated with the PDF object, and corresponding arrays of points in that
/// coordinate system and the local object coordinate system".
///
/// # Read as data, and the boundary is stated rather than assumed
///
/// Everything Table 269 holds is read. What is *not* here is the **projection**: turning a
/// point in a projected coordinate system into a latitude means evaluating the algorithm named
/// in a WKT string or looked up by an EPSG code, which is a geodesy library and a database —
/// ISO 19162 and the EPSG registry, both outside this standard, and both named by §12.10.3 as
/// external references. A reader that guessed at it would produce coordinates that look right
/// and are somewhere else.
///
/// **The registry is the second leg of the journey and not the whole of it**, which is worth
/// stating because this comment said otherwise for a long time. Two things are usable without
/// any of it:
///
/// - the registration — [`Self::registration`] pairs the `/GPTS` geographic points with the
///   `/LPTS` positions in the object's unit square, which is the correspondence the file states
///   directly;
/// - the first leg — [`Self::projected_position`] carries a position in the object's own
///   coordinates into the projected system by `/PCSM`, which is a matrix multiplication and
///   needs nothing outside the file. Table 269 gives that matrix priority over `/GPTS` where it
///   is present, so on a document that states one the arithmetic this module cannot do is
///   *projected to geographic* alone.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Geospatial {
    /// `/Bounds`, the polygon "for which geospatial transformations are valid" — a *neatline* on
    /// a map — as points in the object's unit square.
    ///
    /// The table's default is the whole unit square, `[0.0 0.0 0.0 1.0 1.0 1.0 1.0 0.0]`, and it
    /// is applied here rather than left to a caller, because the entry's absence means the
    /// transformations are valid everywhere rather than nowhere. NOTE 1 says the polygon "need
    /// not be explicitly closed by repeating the first point values as a final point", so
    /// nothing here closes it either.
    pub bounds: Vec<[f64; 2]>,
    /// `/GCS`, the coordinate system the points are in. Required by Table 269.
    pub coordinate_system: Option<CoordinateSystem>,
    /// `/DCS`, "a projected or geographic coordinate system that shall be used for the display
    /// of position values, such as latitude and longitude".
    ///
    /// A document may be authored in one system and *displayed* in another, which the clause
    /// illustrates with a map drawn on a 1927 datum showing WGS84 values "corresponding to
    /// values reported by a GPS device".
    pub display_system: Option<CoordinateSystem>,
    /// `/PDU`, "[p]referred [d]isplay [u]nits": a linear, an area and an angular unit, in that
    /// order.
    pub display_units: Option<[String; 3]>,
    /// `/GPTS`, points in geographic space, "as degrees of latitude and longitude, respectively"
    /// — or as eastings and northings where `/GCS` is a projected system.
    ///
    /// **Pairs rather than triples, and the condition for the other form is out of this
    /// reader's reach.** Table 269 requires the array "to hold 3D point coordinates as triples
    /// rather than pairwise" for the `Geospatial3D` requirement type, "where the third value of
    /// each tripe is an elevation value" — but only "when Geospatial feature information is
    /// present … in a 3D annotation", and Errata Collection 3's Issue #284 widens that to a
    /// `RichMedia` annotation, whose Table 333 states no `/GEO` entry until the same erratum
    /// adds one. A geospatial measure dictionary reaches this module through §12.9's `/VP`
    /// alone, so the dictionary those triples belong to sits inside an annotation of clause 13,
    /// which `CLAUDE.md`'s exclusion list excludes.
    pub geographic_points: Vec<[f64; 2]>,
    /// `/LPTS`, the same points in the object's own unit square, which Table 269 says "is
    /// mapped to the rectangular bounds of the `Viewport`, image `XObject`, or forms `XObject`
    /// that contains the measure dictionary".
    ///
    /// The published table makes it *(Optional)*; Errata Collection 3's Issue #533 strikes that
    /// word and writes *Required* in its place, which is the entry's own description agreeing —
    /// the array is what makes `/GPTS` a registration rather than a list of coordinates. The
    /// triples form on [`Self::geographic_points`] is stated here too and is out of reach for
    /// the same reason.
    pub local_points: Vec<[f64; 2]>,
    /// `/PCSM`, a twelve-element matrix "defining the transformation from `XObject` position
    /// coordinates to" the projected coordinate system.
    ///
    /// The table states its own precedence twice over: it "should be ignored" when `/GCS` is
    /// geographic, and where it is present "it has priority over GPTS , and GPTS values may be
    /// ignored". [`Geospatial::matrix_has_priority`] is which of the two the clause would
    /// prefer and [`Geospatial::projected_position`] is the transformation itself.
    ///
    /// **The layout of the twelve numbers is Errata Collection 3's**, not the published
    /// table's, which says only how many there are. Issue #534 strikes `projected coordinate
    /// system.` and writes *the projected coordinate system. This array represents a 4x4 affine
    /// transformation matrix in row order. The `XObject` position coordinates are represented as
    /// a 1x4 matrix, [ x y z 1 ], where the z value is non-zero only in the context of a
    /// `Geospatial3D`-enabled annotation.  `PCSM` only applies when `GCS` is a projected
    /// coordinate system.* Issue #358 strikes `real` from `of real numbers` in the same sentence.
    pub projected_matrix: Option<[f64; 12]>,
}

impl Geospatial {
    /// The `/GPTS` and `/LPTS` points paired, which is the registration the file states.
    ///
    /// Table 269 requires the two to be the same length — `/LPTS` "shall contain the same number
    /// of number pairs as the GPTS array" — so a file where they differ has contradicted itself
    /// and the pairing stops at the shorter, which is exactly the part it did state.
    #[must_use]
    pub fn registration(&self) -> Vec<([f64; 2], [f64; 2])> {
        self.geographic_points
            .iter()
            .zip(self.local_points.iter())
            .map(|(geographic, local)| (*geographic, *local))
            .collect()
    }

    /// Whether `/PCSM` is the transformation to use, by Table 269's two sentences about it.
    ///
    /// True only where a matrix is present *and* `/GCS` is projected: a geographic `/GCS` makes
    /// the matrix one the clause says "should be ignored".
    #[must_use]
    pub fn matrix_has_priority(&self) -> bool {
        self.projected_matrix.is_some()
            && self
                .coordinate_system
                .as_ref()
                .is_some_and(|system| system.projected)
    }

    /// A position in the object's own coordinates, carried into the projected coordinate system
    /// by `/PCSM` — the one leg of a georeference this program can do without a registry.
    ///
    /// `None` where [`Self::matrix_has_priority`] is false, which is the clause's own answer
    /// rather than a failure: with no matrix there is nothing to apply, and with a geographic
    /// `/GCS` Table 269 says the matrix "should be ignored".
    ///
    /// # Where the arithmetic comes from
    ///
    /// The published Table 269 says only that `/PCSM` is a twelve-element matrix, which is not
    /// enough to multiply by. Errata Collection 3's Issue #534 says the rest — a 4x4 affine
    /// matrix in row order, applied to the position written as the row vector *[ x y z 1 ]* —
    /// and twelve numbers for a 4×4 matrix is §8.3.4's own convention one dimension up. There a
    /// point is "expressed in vector form as [ x y 1]", the matrix is 3-by-3, and:
    ///
    /// > Because a transformation matrix has only six elements that can be changed, in most
    /// > cases in PDF it shall be specified as the six-element array [a b c d e f].
    ///
    /// The six are the three rows' first two columns, the elided third column being 0, 0, 1, and
    /// the multiplication §8.3.4 then writes out takes `a` and `c` and `e` — one element per
    /// row, a stride of two. Twelve numbers for four rows of a 4×4 matrix is the same elision
    /// with the last column 0, 0, 0, 1, so the stride here is three.
    ///
    /// The `z` of the argument "is non-zero only in the context of a Geospatial3D-enabled
    /// annotation" — Issue #534 again — so a caller measuring on a page passes `0.0` for it.
    #[must_use]
    pub fn projected_position(&self, [x, y, z]: [f64; 3]) -> Option<[f64; 3]> {
        if !self.matrix_has_priority() {
            return None;
        }
        // One row per line, each name saying which input axis it scales and which output
        // component it lands in; the fourth row is the translation and the elided last column
        // is 0, 0, 0, 1.
        let &[xx, xy, xz, yx, yy, yz, zx, zy, zz, tx, ty, tz] = self.projected_matrix.as_ref()?;
        Some([
            x * xx + y * yx + z * zx + tx,
            x * xy + y * yy + z * zy + ty,
            x * xz + y * yz + z * zz + tz,
        ])
    }

    /// Whether a point of the object's unit square lies inside `/Bounds`.
    ///
    /// Table 269 gives that entry one job: its points "describe the bounds of an area for which
    /// geospatial transformations are valid", and "[f]or maps, this bounding polygon is known as
    /// a neatline". So this is the question a measuring tool asks before it says anything about
    /// where a point is on the earth — outside the neatline the file has said its own
    /// registration does not apply, and an answer taken there would be a coordinate the document
    /// disclaims.
    ///
    /// The absent entry is the whole unit square, which [`Self::bounds`] has already applied, so
    /// a file stating no neatline answers `true` everywhere inside its viewport.
    ///
    /// The test is the even-odd crossing count, which is planar geometry rather than a reading
    /// of anything: the clause names a polygon and says nothing about how a polygon contains a
    /// point, because there is one answer for a simple one. NOTE 1 says the polygon "need not be
    /// explicitly closed by repeating the first point values as a final point", so the leg back
    /// to the first point is walked here rather than expected in the array.
    ///
    /// Fewer than three points enclose nothing and answer `false`.
    #[must_use]
    pub fn within_bounds(&self, [u, v]: [f64; 2]) -> bool {
        if self.bounds.len() < 3 {
            return false;
        }
        let mut inside = false;
        let Some(&last) = self.bounds.last() else {
            return false;
        };
        let mut previous = last;
        for point in &self.bounds {
            let [x0, y0] = previous;
            let [x1, y1] = *point;
            // A ray cast along +u: an edge counts when it straddles this point's v, and the
            // crossing is to the right of it. The half-open comparison on v is what keeps a
            // vertex from being counted twice.
            if (y1 > v) != (y0 > v) && u < (x0 - x1) * (v - y1) / (y0 - y1) + x1 {
                inside = !inside;
            }
            previous = *point;
        }
        inside
    }
}

/// §12.10.3's geographic and §12.10.4's projected coordinate systems. Tables 270 and 271.
///
/// One type for two tables because the two hold the same two entries and differ in what they
/// *mean*: a GEOGCS "specifies an ellipsoidal object in geographic coordinates: angular units of
/// latitude and longitude", and a PROJCS "specifies the algorithms and associated parameters
/// used to transform points between geographic coordinates and a two-dimensional (projected)
/// coordinate system". Which one a dictionary is, is its `/Type`.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateSystem {
    /// Whether this is a `PROJCS` (`true`) or a `GEOGCS` (`false`).
    pub projected: bool,
    /// `/EPSG`, "[a]n EPSG reference code specifying the … coordinate system".
    pub epsg: Option<i64>,
    /// `/WKT`, "[a] string of Well Known Text describing the … coordinate system".
    ///
    /// The format is ISO 19162's, and this reader keeps the string. Both tables say the two
    /// entries exclude each other — `/EPSG` "[s]hall not be present if WKT is present" — while
    /// §12.10.3's closing sentence requires one of them: "[e]ither an EPSG code or a WKT string
    /// shall be present". [`CoordinateSystem::is_stated`] is that requirement.
    pub wkt: Option<String>,
}

impl CoordinateSystem {
    /// Whether the dictionary states a system at all, which §12.10.3 requires of it.
    #[must_use]
    pub fn is_stated(&self) -> bool {
        self.epsg.is_some() || self.wkt.is_some()
    }
}

/// §12.10.5's point data: extra values attached to points in the object's 2D space. Table 272.
///
/// > The names LAT , LON , and ALT are predefined, and shall be used to associate altitude
/// > information with latitude and longitude positions.
///
/// `/Names` are "in effect, column headers for the array of XPTS values", so this is a table:
/// one name per column, one tuple per point. The values are kept as objects because only the
/// three predefined names have a stated type — "each member in the interior arrays is of a type
/// defined by the corresponding name in the Names array", and a name this standard does not
/// define carries a type it does not define either.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PointData {
    /// `/Names`, the column headers.
    pub names: Vec<String>,
    /// `/XPTS`, one row per point, "a collection of tuples without any guaranteed ordering or
    /// relationship from point to point".
    pub points: Vec<Vec<Object>>,
}

/// Reads Table 269's entries.
fn geospatial(document: &Document, dict: &Dictionary) -> Geospatial {
    let system = |key: &str| {
        let value = document.get_key(dict, key);
        let dict = value.as_dict()?;
        Some(CoordinateSystem {
            // Table 270 and Table 271 both make `/Type` required, so the name is the whole
            // distinction; anything that is not `PROJCS` is read as the geographic form, which
            // is the one whose points are degrees.
            projected: document
                .get_key(dict, "Type")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"PROJCS"),
            epsg: document.get_key(dict, "EPSG").as_integer(),
            wkt: match document.get_key(dict, "WKT") {
                Object::String(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
                _ => None,
            },
        })
    };
    let bounds = pairs(document, dict, "Bounds");
    Geospatial {
        bounds: if bounds.is_empty() {
            // Table 269's stated default: "a rectangle describing the full unit square".
            vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]
        } else {
            bounds
        },
        coordinate_system: system("GCS"),
        display_system: system("DCS"),
        display_units: display_units(document, dict),
        geographic_points: pairs(document, dict, "GPTS"),
        local_points: pairs(document, dict, "LPTS"),
        projected_matrix: matrix(document, dict),
    }
}

/// Table 269's `/PDU`: a linear, an area and an angular unit.
fn display_units(document: &Document, dict: &Dictionary) -> Option<[String; 3]> {
    let value = document.get_key(dict, "PDU");
    let array = value.as_array()?;
    let [linear, area, angular, ..] = array else {
        return None;
    };
    let name = |object: &Object| {
        document
            .resolve(object)
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
    };
    Some([name(linear)?, name(area)?, name(angular)?])
}

/// An array of numbers "taken pairwise", which is how Table 269 states three of its entries.
///
/// Pairwise unconditionally, because the one condition under which two of them are triples puts
/// the dictionary somewhere this module never looks — see [`Geospatial::geographic_points`].
fn pairs(document: &Document, dict: &Dictionary, key: &str) -> Vec<[f64; 2]> {
    let value = document.get_key(dict, key);
    let Some(array) = value.as_array() else {
        return Vec::new();
    };
    array
        .chunks_exact(2)
        .take(MAX_POINTS)
        .filter_map(|pair| {
            Some([
                document.resolve(pair.first()?).as_number()?,
                document.resolve(pair.get(1)?).as_number()?,
            ])
        })
        .collect()
}

/// Table 269's `/PCSM`, a twelve-element matrix.
///
/// Shorter than twelve is refused rather than padded: the missing elements of a transformation
/// are not zeroes, and a matrix read from an array that does not state one would georeference a
/// page onto somewhere else. [`Geospatial::projected_position`] has the layout.
fn matrix(document: &Document, dict: &Dictionary) -> Option<[f64; 12]> {
    let value = document.get_key(dict, "PCSM");
    let array = value.as_array()?;
    if array.len() < 12 {
        return None;
    }
    let mut out = [0.0f64; 12];
    for (slot, entry) in out.iter_mut().zip(array.iter()) {
        *slot = document.resolve(entry).as_number()?;
    }
    Some(out)
}

/// Reads Table 272's point data, from a dictionary or an array of them.
///
/// §12.10.5 states two spellings — the value of a `/PtData` entry "is a point data dictionary or
/// an array of point data dictionaries" — and both answer with a list.
#[must_use]
pub fn point_data(document: &Document, viewport: &Dictionary) -> Vec<PointData> {
    let value = document.get_key(viewport, "PtData");
    let dictionaries: Vec<Dictionary> = match &value {
        Object::Dictionary(dict) => vec![dict.clone()],
        Object::Array(items) => items
            .iter()
            .take(MAX_POINTS)
            .filter_map(|entry| document.resolve(entry).as_dict().cloned())
            .collect(),
        _ => Vec::new(),
    };
    dictionaries
        .iter()
        .map(|dict| PointData {
            names: document
                .get_key(dict, "Names")
                .as_array()
                .unwrap_or_default()
                .iter()
                .filter_map(|entry| {
                    document
                        .resolve(entry)
                        .as_name()
                        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                })
                .collect(),
            points: document
                .get_key(dict, "XPTS")
                .as_array()
                .unwrap_or_default()
                .iter()
                .take(MAX_POINTS)
                .filter_map(|entry| {
                    Some(
                        document
                            .resolve(entry)
                            .as_array()?
                            .iter()
                            .map(|value| document.resolve(value))
                            .collect(),
                    )
                })
                .collect(),
        })
        .collect()
}

/// Reads one Table 265 viewport.
fn viewport(document: &Document, entry: &Object) -> Option<Viewport> {
    let dict = entry.as_dict()?;
    let bbox = rectangle(document, dict)?;
    let measure = document.get_key(dict, "Measure");
    Some(Viewport {
        bbox,
        name: match document.get_key(dict, "Name") {
            Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        },
        measure: measure
            .as_dict()
            .map(|dict| measure_dictionary(document, dict)),
        has_point_data: document.get_key(dict, "PtData").as_dict().is_some(),
    })
}

/// Table 266's `/Subtype`, with the rectilinear entries where it names one.
fn measure_dictionary(document: &Document, dict: &Dictionary) -> Measure {
    let subtype = document.get_key(dict, "Subtype");
    let subtype = subtype.as_name().map(|name| name.as_bytes().to_vec());
    match subtype.as_deref() {
        // "Default value: RL", so an absent subtype is rectilinear.
        None | Some(b"RL") => Measure::Rectilinear(Box::new(rectilinear(document, dict))),
        Some(b"GEO") => Measure::Geospatial(Box::new(geospatial(document, dict))),
        Some(other) => Measure::Other(String::from_utf8_lossy(other).into_owned()),
    }
}

/// Table 267's entries.
fn rectilinear(document: &Document, dict: &Dictionary) -> Rectilinear {
    let array = |key: &str| {
        let value = document.get_key(dict, key);
        let Some(items) = value.as_array() else {
            return Vec::new();
        };
        items
            .iter()
            .take(MAX_UNITS)
            .filter_map(|entry| number_format(document, document.resolve(entry).as_dict()?))
            .collect()
    };
    Rectilinear {
        ratio: match document.get_key(dict, "R") {
            Object::String(bytes) => pdf_syntax::text_string(&bytes),
            _ => String::new(),
        },
        x: array("X"),
        y: array("Y"),
        distance: array("D"),
        area: array("A"),
        angle: array("T"),
        slope: array("S"),
        origin: origin(document, dict),
        cyx: document.get_key(dict, "CYX").as_number(),
    }
}

/// Table 267's `/O`, an origin in default user space.
fn origin(document: &Document, dict: &Dictionary) -> Option<[f32; 2]> {
    let value = document.get_key(dict, "O");
    let array = value.as_array()?;
    let [x, y, ..] = array else {
        return None;
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a page coordinate in f32, as every coordinate in this crate is"
    )]
    Some([
        document.resolve(x).as_number()? as f32,
        document.resolve(y).as_number()? as f32,
    ])
}

/// Table 268's entries, with the defaults the table states.
fn number_format(document: &Document, dict: &Dictionary) -> Option<NumberFormat> {
    let unit = match document.get_key(dict, "U") {
        Object::String(bytes) => pdf_syntax::text_string(&bytes),
        // `/U` and `/C` are both required; a dictionary without them has not stated a unit, and
        // taking part in the walk would multiply by a conversion factor nobody wrote.
        _ => return None,
    };
    let conversion = document.get_key(dict, "C").as_number()?;
    let fraction = match document
        .get_key(dict, "F")
        .as_name()
        .map(|name| name.as_bytes().to_vec())
    {
        Some(name) => match name.as_slice() {
            b"F" => Fraction::Fractional,
            b"R" => Fraction::Round,
            b"T" => Fraction::Truncate,
            // `D` and anything Table 268 does not list: the table's default value is `D`, and a
            // name outside the four is a file stating nothing this standard defines.
            _ => Fraction::Decimal,
        },
        None => Fraction::Decimal,
    };
    let text = |key: &str, default: &str| match document.get_key(dict, key) {
        // "An empty string indicates that no text shall be added" — which is a *stated* value
        // and not the default, so an absent entry and an empty one differ here.
        Object::String(bytes) => pdf_syntax::text_string(&bytes),
        _ => default.to_owned(),
    };
    Some(NumberFormat {
        unit,
        conversion,
        fraction,
        denominator: document
            .get_key(dict, "D")
            .as_integer()
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
            .unwrap_or(match fraction {
                Fraction::Fractional => 16,
                _ => 100,
            }),
        keep_denominator: matches!(document.get_key(dict, "FD"), Object::Boolean(true)),
        thousands: text("RT", ","),
        decimal_point: text("RD", "."),
        prefix_spacing: text("PS", " "),
        suffix_spacing: text("SS", " "),
        label_before_value: document
            .get_key(dict, "O")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"P"),
    })
}

/// A rectangle as the file states it, without normalising the corners.
fn rectangle(document: &Document, dict: &Dictionary) -> Option<[f32; 4]> {
    let value = document.get_key(dict, "BBox");
    let array = value.as_array()?;
    if array.len() < 4 {
        return None;
    }
    let mut out = [0.0f32; 4];
    for (slot, entry) in out.iter_mut().zip(array.iter()) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a page coordinate in f32, as every rectangle in this crate is"
        )]
        {
            *slot = document.resolve(entry).as_number()? as f32;
        }
    }
    Some(out)
}

/// An annotation's own `/Measure`, where it states one.
///
/// # Two clauses, one entry
///
/// §12.5.6.9's Table 181 and §12.5.6.7's Table 178 each give their subtype a `/Measure` with the
/// same words but for different shapes. The second of the two reads:
///
/// > A measure dictionary (see "Table 266 - Entries in a measure dictionary") that shall specify
/// > the scale and units that apply to the line annotation.
///
/// — and the first says the same of a polygon or a polyline, the scale and units that apply to
/// the annotation. So an annotation may carry its own measuring system, and where it does, that
/// system is the one that applies to it:
/// §12.9.1's rule about which *viewport* to use decides a measurement between two points on a
/// page, and this entry is not a viewport at all.
///
/// # Why this may be read even when the annotation has an appearance stream
///
/// §12.5.2 lists the keys a reader "shall ignore" while rendering a stored appearance —
/// `/C`, `/IC`, `/Border`, `/BS`, `/BE`, `/CA`, `/ca`, `/H`, `/DA`, `/Q`, `/DS`, `/LE`, `/LL`,
/// `/MK`, `/LLE` and `/Sy`, as Errata Collection 3 rewrites it — and neither `/IT` nor
/// `/Measure` is on it. Neither states a mark, which is why: this is input to a user interface,
/// exactly as §12.9.1 says a measure dictionary is.
#[must_use]
pub fn annotation_measure(document: &Document, annotation: &Dictionary) -> Option<Measure> {
    let measure = document.get_key(annotation, "Measure");
    Some(measure_dictionary(document, measure.as_dict()?))
}

/// What an annotation's geometry comes to in the units its own `/Measure` states.
///
/// Both quantities are given rather than one, because the standard names both and does not say
/// which a subtype is for. Table 181's `/IT` has `PolyLineDimension` and `PolygonDimension`, and
/// Table 267 has a number format array for each of the two quantities a dimension could be — `/D`
/// "for measurement of distance in any direction" and `/A` "for measurement of area" — so
/// choosing between them for a closed shape would be this reader deciding something the clause
/// leaves to whoever displays it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Measured {
    /// The length of the annotation's path, formatted by Table 267's `/D`.
    ///
    /// A polygon's includes the leg back to its first vertex, which §12.5.6.9 states: a polyline
    /// is a polygon "except that the first and last vertex are not implicitly connected".
    pub length: Option<String>,
    /// The area a closed annotation encloses, formatted by Table 267's `/A`.
    ///
    /// `None` for a line and a polyline, which enclose nothing.
    pub area: Option<String>,
}

impl Measured {
    /// Whether the annotation measured to nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length.is_none() && self.area.is_none()
    }
}

/// Measures a line, polyline or polygon annotation against its own `/Measure`.
///
/// # What each subtype's geometry is, and where each comes from
///
/// - §12.5.6.7's `Line` is Table 178's `/L`, "[a]n array of four numbers … specifying the
///   starting and ending coordinates of the line in default user space". **`/LL` does not change
///   the length**, and that is worth stating because it changes what the two points *are*: with
///   leader lines present they "represent the endpoints of the leader lines rather than the
///   endpoints of the line itself", and the line proper is that segment translated perpendicular
///   to itself by `/LL`. A translation moves both ends by the same vector, so the distance
///   between them is the one this measures either way.
/// - §12.5.6.9's `PolyLine` and `Polygon` are Table 181's `/Vertices`, "the alternating
///   horizontal and vertical coordinates … of each vertex, in default user space", closed for a
///   polygon and open for a polyline.
///
/// # What is not measured, and why that is the clause rather than a gap
///
/// A `/Path` — PDF 2.0's replacement for `/Vertices`, "an array of n arrays, each supplying the
/// operands for a path building operator (m, l or c)" — states curves, and the length of a cubic
/// Bézier is not a quantity Table 267's conversions describe. Reported as no measurement rather
/// than measured over its control points, which would answer with a number for a shape that is
/// not that shape.
///
/// `None` for an annotation of any other subtype, for one stating no `/Measure`, and for one
/// whose measure is `GEO` — §12.10's distances are an ellipsoid's, not Table 267's.
#[must_use]
pub fn annotation_measurement(document: &Document, annotation: &Dictionary) -> Option<Measured> {
    let measure = annotation_measure(document, annotation)?;
    let subtype = document.get_key(annotation, "Subtype");
    let subtype = subtype.as_name()?.as_bytes().to_vec();
    if !document
        .get_key(annotation, "Path")
        .as_array()
        .is_none_or(<[Object]>::is_empty)
    {
        return None;
    }
    match subtype.as_slice() {
        b"Line" => {
            let ends = crate::appearance::points(document, annotation, "L")?;
            Some(Measured {
                length: measure.length(ends.get(..2)?),
                area: None,
            })
        }
        b"PolyLine" => {
            let vertices = crate::appearance::points(document, annotation, "Vertices")?;
            Some(Measured {
                length: measure.length(&vertices),
                area: None,
            })
        }
        b"Polygon" => {
            let vertices = crate::appearance::points(document, annotation, "Vertices")?;
            let mut closed = vertices.clone();
            if let Some(first) = vertices.first() {
                closed.push(*first);
            }
            Some(Measured {
                length: measure.length(&closed),
                area: measure.area(&vertices),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Fraction, Measure, NumberFormat, Viewports, format};
    use pdf_syntax::Document;

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// §12.9.2's own EXAMPLE, read from the clause's own measure dictionary.
    ///
    /// > Given a sample distance in scaled units of 1.4505 miles, the formatted text produced by
    /// > applying the number format array would be "1 mi 2,378 ft 7 5/8 in".
    ///
    /// Every part of the algorithm is load-bearing in that one string: the carry from miles to
    /// feet (0.4505 × 5280), the `/RT` comma in 2,378, the second carry into inches
    /// (0.64 × 12), and `/F /F /D 8` rounding 0.68 of an inch to the nearest eighth.
    #[test]
    fn the_clauses_own_example_formats_the_string_it_states() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 612 792] /Name (Plan) /Measure 5 0 R >>",
            "<< /Type /Measure /Subtype /RL /R (1in = 0.1 mi) \
             /X [<< /U (mi) /C .00139 /D 100000 >>] \
             /D [<< /U (mi) /C 1 >> << /U (ft) /C 5280 >> << /U (in) /C 12 /F /F /D 8 >>] \
             /A [<< /U (acres) /C 640 >>] >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);
        let [viewport] = viewports.viewports.as_slice() else {
            panic!("one viewport, got {viewports:?}");
        };
        assert_eq!(viewport.name.as_deref(), Some("Plan"));
        let Some(Measure::Rectilinear(measure)) = &viewport.measure else {
            panic!("a rectilinear measure, got {:?}", viewport.measure);
        };
        assert_eq!(measure.ratio, "1in = 0.1 mi");
        assert_eq!(measure.distance.len(), 3);
        assert_eq!(measure.area.len(), 1);

        assert_eq!(format(1.4505, &measure.distance), "1 mi 2,378 ft 7 5/8 in");
    }

    /// Step c) stops the walk: a value with no fractional part uses one unit and no more.
    ///
    /// §12.9.2 step c):
    ///
    /// > If the result contains no non-zero fractional portion, concatenate the label specified
    /// > by the U entry in the order specified by O … The formatting is then complete.
    ///
    /// So a distance of exactly two miles is "2 mi" even though feet and inches are available,
    /// which is the difference between §12.9.2's algorithm and dividing the value by every unit
    /// in turn.
    #[test]
    fn a_value_with_no_fraction_stops_at_the_first_unit() {
        let miles = NumberFormat {
            unit: "mi".to_owned(),
            conversion: 1.0,
            fraction: Fraction::Decimal,
            denominator: 100,
            keep_denominator: false,
            thousands: ",".to_owned(),
            decimal_point: ".".to_owned(),
            prefix_spacing: " ".to_owned(),
            suffix_spacing: " ".to_owned(),
            label_before_value: false,
        };
        let feet = NumberFormat {
            unit: "ft".to_owned(),
            conversion: 5280.0,
            ..miles.clone()
        };
        assert_eq!(format(2.0, &[miles.clone(), feet.clone()]), "2 mi");
        assert_eq!(format(0.5, &[miles.clone(), feet]), "0 mi 2,640 ft");

        // Table 268's `/O P` puts the label in front, with `/PS` and `/SS` still around it.
        let dollars = NumberFormat {
            unit: "$".to_owned(),
            prefix_spacing: String::new(),
            suffix_spacing: String::new(),
            label_before_value: true,
            ..miles
        };
        assert_eq!(format(3.0, &[dollars]), "$3");
    }

    /// The four values of `/F`, on the same number.
    ///
    /// `D` shows a decimal to `/D`'s precision with low-order zeros truncated unless `/FD`;
    /// `F` a fraction over `/D`, reduced unless `/FD`; `R` rounds and `T` truncates. 7.5 of an
    /// inch is 7.5, 7 1/2, 8 and 7 — four different strings from one value, which is what makes
    /// this entry worth reading rather than assuming.
    #[test]
    fn each_value_of_f_shows_the_fraction_its_own_way() {
        let base = NumberFormat {
            unit: "in".to_owned(),
            conversion: 1.0,
            fraction: Fraction::Decimal,
            denominator: 100,
            keep_denominator: false,
            thousands: ",".to_owned(),
            decimal_point: ".".to_owned(),
            prefix_spacing: " ".to_owned(),
            suffix_spacing: " ".to_owned(),
            label_before_value: false,
        };
        let with = |fraction, denominator, keep| NumberFormat {
            fraction,
            denominator,
            keep_denominator: keep,
            ..base.clone()
        };
        assert_eq!(
            format(7.5, &[with(Fraction::Decimal, 100, false)]),
            "7.5 in"
        );
        assert_eq!(
            format(7.5, &[with(Fraction::Decimal, 100, true)]),
            "7.50 in",
            "/FD true keeps the low-order zero"
        );
        assert_eq!(
            format(7.5, &[with(Fraction::Fractional, 16, false)]),
            "7 1/2 in",
            "8/16 reduces unless /FD says not to"
        );
        assert_eq!(
            format(7.5, &[with(Fraction::Fractional, 16, true)]),
            "7 8/16 in"
        );
        assert_eq!(format(7.5, &[with(Fraction::Round, 100, false)]), "8 in");
        assert_eq!(format(7.5, &[with(Fraction::Truncate, 100, false)]), "7 in");
    }

    /// Overlapping viewports: the **last** one containing the point wins.
    ///
    /// §12.9.1:
    ///
    /// > the dictionaries in the array shall be examined, starting with the last one and
    /// > iterating in reverse, and the first one whose BBox entry contains the point shall be
    /// > chosen.
    ///
    /// The two viewports here are a plan and a detail inset drawn over it, and a reader
    /// searching forwards would answer with the plan's scale inside the inset.
    #[test]
    fn the_last_viewport_containing_a_point_is_the_one_that_applies() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R 5 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 600 700] /Name (Plan) >>",
            "<< /Type /Viewport /BBox [100 100 200 200] /Name (Inset) /PtData << /Type /PtData >> >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);
        assert_eq!(
            viewports.at((150.0, 150.0)).and_then(|v| v.name.as_deref()),
            Some("Inset")
        );
        assert_eq!(
            viewports.at((400.0, 400.0)).and_then(|v| v.name.as_deref()),
            Some("Plan")
        );
        assert_eq!(
            viewports.at((605.0, 750.0)),
            None,
            "outside both rectangles"
        );
        assert!(
            viewports.viewports[1].has_point_data,
            "a /PtData is recorded even though §12.10 is not read"
        );
    }

    /// §12.9.1's two-point rule: the *first* point's viewport, and its axes scaled before the
    /// distance is taken.
    ///
    /// Two sentences meet in one measurement and the fixture separates them. §12.9.1 says which
    /// viewport applies — "[a]ny measurement that potentially involves multiple viewports … shall
    /// use the information specified in the viewport of the first point" — so the measurement
    /// below starts inside an inset and ends on the plan outside it, and the inset's units are
    /// what the answer is in; measured the other way round it is the plan's.
    ///
    /// Table 267's `/D` says in which *order* — "[t]he scale factors from `X`, `Y` (if present)
    /// and `CYX` (if `Y` is present) shall be used to convert from default user space to the
    /// appropriate units before applying the distance function". The inset's axes are deliberately
    /// unequal: `/X` converts one user unit to 3, `/Y` to 2, and `/CYX` makes one y unit 2 x
    /// units, so the leg (3, 4) becomes (9, 16) and the distance is `hypot(9, 16)` = 18.3576…
    /// A reader taking `hypot(3, 4) = 5` first and scaling afterwards cannot produce that number
    /// under any single factor, which is what makes the assertion about the order rather than
    /// about the arithmetic.
    ///
    /// Trap 8: `doc/pdf.js` and `doc/corpora` hold one viewport between them and it is `GEO`, so
    /// every number here is the clause's own and none of it is a corpus reading.
    #[test]
    fn a_distance_is_measured_in_the_first_points_viewport_with_its_axes_scaled_first() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R 5 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 600 700] /Name (Plan) /Measure << /Type /Measure \
             /R (1 = 1) /X [<< /Type /NumberFormat /U (pl) /C 1 >>] \
             /D [<< /Type /NumberFormat /U (pl) /C 1 >>] >> >>",
            "<< /Type /Viewport /BBox [100 100 200 200] /Name (Inset) /Measure << /Type /Measure \
             /R (1 = 3) /X [<< /Type /NumberFormat /U (xu) /C 3 >>] \
             /Y [<< /Type /NumberFormat /U (yu) /C 2 >>] /CYX 2 \
             /D [<< /Type /NumberFormat /U (u) /C 1 >>] >> >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);
        assert_eq!(
            viewports
                .distance((150.0, 150.0), (153.0, 154.0))
                .as_deref(),
            Some("18.36 u"),
            "the inset holds the first point, and its axes are scaled before the hypotenuse"
        );
        assert_eq!(
            viewports
                .distance((400.0, 400.0), (403.0, 404.0))
                .as_deref(),
            Some("5 pl"),
            "the plan's own scale is 1:1 on both axes, so the same leg is a plain hypotenuse"
        );
        assert_eq!(
            viewports
                .distance((153.0, 154.0), (150.0, 150.0))
                .as_deref(),
            Some("18.36 u"),
            "and the answer is the first point's viewport whichever end is further in"
        );
        assert_eq!(
            viewports.distance((605.0, 750.0), (150.0, 150.0)),
            None,
            "no viewport contains the first point, so the page states no units there"
        );
    }

    /// A `/Y` with no `/CYX` is Table 267 refusing the calculation, not a gap to fill.
    ///
    /// ISO 32000-2 §12.9.1, Table 267's `/CYX` cell:
    ///
    /// > if not specified, these calculations may not be performed (which would be the case in
    /// > situations such as x representing time and y representing temperature)
    ///
    /// So a drawing whose two axes are not the same kind of quantity has no distance at all, and
    /// answering with one would be this reader inventing a metric for a plot of temperature
    /// against time. The pair is the fixture above, which states the `/CYX` this one omits.
    #[test]
    fn two_axes_with_no_conversion_between_them_have_no_distance() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 600 700] /Measure << /Type /Measure /R (t/T) \
             /X [<< /Type /NumberFormat /U (s) /C 1 >>] \
             /Y [<< /Type /NumberFormat /U (K) /C 1 >>] \
             /D [<< /Type /NumberFormat /U (?) /C 1 >>] >> >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);
        assert_eq!(viewports.distance((10.0, 10.0), (20.0, 20.0)), None);
    }

    /// A `GEO` subtype is recorded as itself rather than read as a rectilinear system.
    ///
    /// Table 266 makes `RL` the default, so a measure dictionary with no `/Subtype` is
    /// rectilinear — and a `GEO` one carries §12.10's earth model, which this reader names and
    /// does not interpret.
    #[test]
    fn a_geospatial_measure_carries_the_earth_model_and_not_a_scale() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R 5 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 10 10] /Measure 6 0 R \
             /PtData << /Type /PtData /Subtype /Cloud /Names [/LAT /LON /ALT] \
             /XPTS [[52.5 13.4 34] [48.9 2.4 35]] >> >>",
            "<< /Type /Viewport /BBox [0 0 10 10] /Measure << /Type /Measure /R (1:1) >> >>",
            "<< /Type /Measure /Subtype /GEO /GCS << /Type /GEOGCS /EPSG 4326 >> \
             /DCS << /Type /PROJCS /WKT (PROJCS[\"x\"]) >> /PDU [/M /SQM /DEG] \
             /GPTS [52.5 13.4 48.9 2.4] /LPTS [0 0 1 1] >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);
        let Some(Measure::Geospatial(geospatial)) = &viewports.viewports[0].measure else {
            panic!(
                "a geospatial measure, got {:?}",
                viewports.viewports[0].measure
            );
        };
        let system = geospatial.coordinate_system.as_ref().expect("a /GCS");
        assert!(!system.projected, "/Type /GEOGCS is the geographic form");
        assert_eq!(system.epsg, Some(4326));
        assert!(
            system.is_stated(),
            "§12.10.3 requires an EPSG code or a WKT"
        );
        assert!(
            geospatial
                .display_system
                .as_ref()
                .is_some_and(|dcs| dcs.projected),
            "the /DCS is a projected system, which is what makes it a second system"
        );
        assert_eq!(
            geospatial.display_units,
            Some(["M".to_owned(), "SQM".to_owned(), "DEG".to_owned()])
        );
        assert_eq!(
            geospatial.registration(),
            vec![([52.5, 13.4], [0.0, 0.0]), ([48.9, 2.4], [1.0, 1.0])],
            "/GPTS and /LPTS are the same points in two coordinate systems"
        );
        assert_eq!(
            geospatial.bounds,
            vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            "an absent /Bounds is Table 269's whole unit square, not an empty polygon"
        );
        assert!(
            !geospatial.matrix_has_priority(),
            "no /PCSM, and a geographic /GCS would make one ignorable anyway"
        );

        // §12.10.5's point data, a table of three columns and two rows.
        let data = crate::measurement::point_data(&doc, &viewports_dict(&doc));
        let [data] = data.as_slice() else {
            panic!("one point data dictionary, got {data:?}");
        };
        assert_eq!(data.names, ["LAT", "LON", "ALT"]);
        assert_eq!(data.points.len(), 2);
        assert_eq!(data.points[0].len(), data.names.len());

        let Some(Measure::Rectilinear(measure)) = &viewports.viewports[1].measure else {
            panic!("an absent /Subtype is Table 266's default, RL");
        };
        assert_eq!(measure.ratio, "1:1");
    }

    /// `/PCSM` carries a position into the projected system, and a geographic `/GCS` disarms it.
    ///
    /// Table 269 states the matrix's *size* and Errata Collection 3's Issue #534 states its
    /// *shape*: a 4x4 affine matrix in row order applied to the row vector `[ x y z 1 ]`. The
    /// matrix below is that shape with every element distinct, so an implementation that read
    /// the twelve numbers column-first, or that dropped the translation row, answers differently
    /// on every component.
    ///
    /// It is a hand-built document because no corpus document states a `/PCSM` at all — the one
    /// witness `tests/measurement.rs` walks is a geographic system with four registration points
    /// and no matrix, which is trap 8's situation stated rather than worked around.
    #[test]
    fn a_projected_coordinate_system_matrix_is_read_in_row_order() {
        // Four rows of three, every element distinct and the 3×3 block deliberately
        // asymmetric: a reader that transposed it would agree on a diagonal matrix.
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R 5 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 10 10] /Measure << /Type /Measure /Subtype /GEO \
             /GCS << /Type /PROJCS /EPSG 32633 >> /GPTS [0 0] /LPTS [0 0] \
             /PCSM [1 2 3  4 5 6  7 8 9  10 20 30] >> >>",
            "<< /Type /Viewport /BBox [0 0 10 10] /Measure << /Type /Measure /Subtype /GEO \
             /GCS << /Type /GEOGCS /EPSG 4326 >> /GPTS [0 0] /LPTS [0 0] \
             /PCSM [1 2 3  4 5 6  7 8 9  10 20 30] >> >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);

        let Some(Measure::Geospatial(projected)) = &viewports.viewports[0].measure else {
            panic!("a geospatial measure");
        };
        assert!(
            projected.matrix_has_priority(),
            "a matrix beside a PROJCS is the one Table 269 prefers"
        );
        assert_eq!(
            projected.projected_position([1.0, 1.0, 1.0]),
            Some([22.0, 35.0, 48.0]),
            "each output component sums one column of the three rows, plus the fourth row"
        );
        assert_eq!(
            projected.projected_position([1.0, 0.0, 0.0]),
            Some([11.0, 22.0, 33.0]),
            "the x axis is the first row, which a transposed reading would take as a column"
        );
        assert_eq!(
            projected.projected_position([0.0, 0.0, 0.0]),
            Some([10.0, 20.0, 30.0]),
            "the origin lands on the translation row alone"
        );

        let Some(Measure::Geospatial(geographic)) = &viewports.viewports[1].measure else {
            panic!("a geospatial measure");
        };
        assert_eq!(
            geographic.projected_matrix, projected.projected_matrix,
            "the same twelve numbers, so what differs is the /GCS and nothing else"
        );
        assert_eq!(
            geographic.projected_position([1.0, 1.0, 1.0]),
            None,
            "Table 269: a geographic /GCS makes the matrix one that should be ignored"
        );
    }

    /// Table 267's `/S` against its `/CYX`: a slope is answered where a distance is refused.
    ///
    /// The `/CYX` cell states both halves. It "shall be used for calculations (distance, area,
    /// and angle) where the units are be equivalent; if not specified, these calculations may
    /// not be performed (which would be the case in situations such as x representing time and y
    /// representing temperature)" — and then "[o]ther calculations (change in x , change in y ,
    /// and slope) shall not require this value". So the clause's own example of a drawing with
    /// no `/CYX`, a plot of temperature against time, has a gradient and no length, and a reader
    /// that took `/CYX` for every quantity would refuse the one measurement such a plot is for.
    #[test]
    fn a_slope_is_measured_where_the_axes_have_no_conversion_between_them() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 612 792] /Name (Plot) /Measure 5 0 R >>",
            "<< /Type /Measure /Subtype /RL /R (1 in = 1 h) \
             /X [<< /U (h) /C 2 >>] /Y [<< /U (C) /C 3 >>] \
             /D [<< /U (h) /C 1 >>] \
             /S [<< /U (C/h) /C 1 /D 1000 >>] >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);
        let measure = viewports.viewports[0]
            .measure
            .as_ref()
            .expect("a measure dictionary");

        assert_eq!(
            measure.distance((0.0, 0.0), (10.0, 10.0)),
            None,
            "Table 267: with /Y and no /CYX these calculations may not be performed"
        );
        // Ten user-space units along each axis is 20 hours and 30 degrees, so the gradient is
        // 1.5 degrees per hour — the y array's largest unit over the x array's largest, which is
        // what `/S`'s own cell says its first element converts from.
        assert_eq!(
            measure.slope([0.0, 0.0], [10.0, 10.0]),
            Some("1.5 C/h".to_owned()),
            "a slope is a ratio between the axes' units and never a length in one of them"
        );
        assert_eq!(
            measure.slope([0.0, 0.0], [0.0, 10.0]),
            None,
            "a vertical leg has no ratio, and the standard states no slope for one"
        );
    }

    /// Table 267's `/T`, whose first element converts "to the largest angle unit from degrees".
    ///
    /// §12.9.2 step a) is what fixes the initial value — it gives this very entry as its example,
    /// "the `T` entry specifies degrees" — so the conversion in the array is applied to a number
    /// of degrees and a reader working in radians would be out by a factor of 57.
    #[test]
    fn an_angle_is_taken_in_degrees_and_converted_by_the_arrays_first_element() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 612 792] /Name (Plan) /Measure 5 0 R >>",
            "<< /Type /Measure /Subtype /RL /X [<< /U (m) /C 1 >>] \
             /T [<< /U (min) /C 60 /D 100 >>] >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);
        let measure = viewports.viewports[0]
            .measure
            .as_ref()
            .expect("a measure dictionary");

        // A right angle is 90 degrees, which this file's array turns into 5400 arcminutes. A
        // reader that measured in radians would answer with a hundredth of that.
        assert_eq!(
            measure.angle([0.0, 0.0], [1.0, 0.0], [0.0, 1.0]),
            Some("5,400 min".to_owned())
        );
        // The non-reflex angle either way round: `/T` carries no sign, so the two rays name one
        // quantity and not two.
        assert_eq!(
            measure.angle([0.0, 0.0], [0.0, 1.0], [1.0, 0.0]),
            Some("5,400 min".to_owned())
        );
        assert_eq!(
            measure.angle([0.0, 0.0], [1.0, 0.0], [-1.0, 0.0]),
            Some("10,800 min".to_owned()),
            "a straight line is two right angles and not none"
        );
        assert_eq!(
            measure.angle([0.0, 0.0], [0.0, 0.0], [1.0, 0.0]),
            None,
            "two coincident points name no direction"
        );
    }

    /// §12.10.2's `/Bounds`, which says where a geospatial reading applies at all.
    ///
    /// The entry's points "describe the bounds of an area for which geospatial transformations
    /// are valid", and its absence is the whole unit square rather than nothing — Table 269
    /// states that default outright, "[0.0 0.0 0.0 1.0 1.0 1.0 1.0 0.0]".
    #[test]
    fn a_neatline_says_where_a_geospatial_reading_applies() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R 6 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 100 100] /Name (Map) /Measure 5 0 R >>",
            "<< /Type /Measure /Subtype /GEO /Bounds [0 0 0 0.5 0.5 0.5 0.5 0] \
             /GCS << /Type /GEOGCS /EPSG 4326 >> /PDU [/M /SQM /DEG] \
             /GPTS [51.0 0.0 51.0 1.0 52.0 1.0 52.0 0.0] \
             /LPTS [0 0 0 1 1 1 1 0] >>",
            "<< /Type /Viewport /BBox [0 0 100 100] /Name (Whole) /Measure 7 0 R >>",
            "<< /Type /Measure /Subtype /GEO /GCS << /Type /GEOGCS /EPSG 4326 >> \
             /GPTS [51.0 0.0] /LPTS [0 0] >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);
        let Some(Measure::Geospatial(quarter)) = &viewports.viewports[0].measure else {
            panic!("a geospatial measure");
        };
        assert!(quarter.within_bounds([0.25, 0.25]));
        assert!(!quarter.within_bounds([0.75, 0.75]));
        let Some(Measure::Geospatial(whole)) = &viewports.viewports[1].measure else {
            panic!("a geospatial measure");
        };
        assert!(
            whole.within_bounds([0.75, 0.75]),
            "Table 269's default /Bounds is the whole unit square, so no neatline is not no area"
        );

        // A viewport's `/BBox` is what the unit square is mapped onto, which is the one leg of a
        // georeference the file states without any registry.
        assert_eq!(
            viewports.viewports[0].unit_square((25.0, 75.0)),
            Some([0.25, 0.75])
        );
    }

    /// §12.9.1: a path is measured in the viewport of its **first** point.
    ///
    /// > Any measurement that potentially involves multiple viewports, such as one specifying
    /// > the distance between two points, shall use the information specified in the viewport of
    /// > the first point.
    ///
    /// The page below states two viewports at two scales, laid out so that a path can start in
    /// either and end in the other, and the two answers differ by the ratio of the scales. A
    /// reader taking the last point's viewport, or the smaller one, gets the other number.
    #[test]
    fn a_traced_path_is_measured_in_the_viewport_of_its_first_point() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R 6 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 100 100] /Name (Plan) /Measure 5 0 R >>",
            "<< /Type /Measure /Subtype /RL /R (1 = 1 m) /X [<< /U (m) /C 1 >>] \
             /D [<< /U (m) /C 1 /D 100 >>] /A [<< /U (sqm) /C 1 /D 100 >>] \
             /T [<< /U (deg) /C 1 /D 100 >>] /S [<< /U (m/m) /C 1 /D 100 >>] >>",
            "<< /Type /Viewport /BBox [100 0 200 100] /Name (Inset) /Measure 7 0 R >>",
            "<< /Type /Measure /Subtype /RL /X [<< /U (m) /C 10 >>] \
             /D [<< /U (m) /C 1 /D 100 >>] >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);

        let from_plan = viewports
            .traced(&[[10.0, 0.0], [110.0, 0.0]])
            .expect("the first point is in the plan");
        assert_eq!(from_plan.viewport.as_deref(), Some("Plan"));
        assert_eq!(from_plan.ratio.as_deref(), Some("1 = 1 m"));
        assert_eq!(from_plan.length.as_deref(), Some("100 m"));

        let from_inset = viewports
            .traced(&[[110.0, 0.0], [10.0, 0.0]])
            .expect("the first point is in the inset");
        assert_eq!(from_inset.viewport.as_deref(), Some("Inset"));
        assert_eq!(
            from_inset.length.as_deref(),
            Some("1,000 m"),
            "the same hundred units, measured at the scale the first point's viewport states"
        );

        // Three points: the area they enclose, the angle where the legs meet and the last leg's
        // slope, each from its own array of Table 267's four.
        let corner = viewports
            .traced(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]])
            .expect("a path in the plan");
        assert_eq!(corner.area.as_deref(), Some("50 sqm"));
        assert_eq!(corner.angle.as_deref(), Some("90 deg"));
        assert_eq!(corner.slope.as_deref(), None, "the last leg is vertical");
        assert!(!corner.is_empty());

        assert_eq!(
            viewports.traced(&[[400.0, 400.0]]),
            None,
            "no viewport's /BBox contains the point, so the page states no units there"
        );
        assert_eq!(viewports.traced(&[]), None);
    }

    /// §12.10 reported rather than guessed: what a geospatial viewport states about a path.
    ///
    /// The row this fixture is about is the one thing a host can honestly show for a map: the
    /// system by name, how many points register it, and whether the place being pointed at is
    /// inside the document's own neatline. A latitude is absent because §12.10 states the
    /// correspondence at the registration points and no function between them.
    #[test]
    fn a_geospatial_viewport_reports_its_system_and_says_no_coordinate() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /VP [4 0 R] >>",
            "<< /Type /Viewport /BBox [0 0 100 100] /Name (Map) /Measure 5 0 R >>",
            "<< /Type /Measure /Subtype /GEO \
             /GCS << /Type /PROJCS /EPSG 32631 >> /DCS << /Type /GEOGCS /EPSG 4326 >> \
             /PDU [/M /SQM /DEG] \
             /GPTS [51.0 0.0 51.0 1.0 52.0 1.0 52.0 0.0] /LPTS [0 0 0 1 1 1 1 0] \
             /PCSM [1 0 0 0 1 0 0 0 1 0 0 0] >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let viewports = Viewports::read(&doc, &page.dict);
        let traced = viewports
            .traced(&[[10.0, 10.0], [90.0, 90.0]])
            .expect("a path in the map");

        assert_eq!(traced.length, None, "§12.10's distances are an ellipsoid's");
        assert!(traced.is_empty());
        let geospatial = traced.geospatial.expect("a geospatial reading");
        let system = geospatial.system.expect("Table 269 requires a /GCS");
        assert!(system.projected, "Table 271's PROJCS");
        assert_eq!(system.epsg, Some(32631));
        assert_eq!(
            geospatial.display_system.and_then(|display| display.epsg),
            Some(4326),
            "Table 269's /DCS is the system position values are displayed in"
        );
        assert_eq!(
            geospatial.units,
            Some(["M".to_owned(), "SQM".to_owned(), "DEG".to_owned()])
        );
        assert_eq!(geospatial.registration, 4);
        assert!(geospatial.within_bounds, "no /Bounds is the whole square");
        assert!(
            geospatial.matrix_applies,
            "Table 269: /PCSM has priority over GPTS where /GCS is projected"
        );
    }

    /// The first viewport's own dictionary, for the point-data reader above.
    fn viewports_dict(document: &Document) -> pdf_syntax::Dictionary {
        document
            .get(pdf_syntax::ObjectId {
                number: 4,
                generation: 0,
            })
            .as_dict()
            .cloned()
            .expect("the first viewport")
    }
}
