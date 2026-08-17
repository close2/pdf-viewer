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
//! textual form for presentation in a graphical user interface". So this module reads the data
//! and implements the one thing the clause states as an algorithm — §12.9.2's formatting — and
//! `viewer-ui`, which has no measuring tool, calls neither.
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
//! [`Viewports::at`] takes one point and callers with two are expected to ask about the first.
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
/// Everything Table 269 holds is read. What is *not* here is the transformation: turning a point
/// on a page into a latitude means evaluating the projection named in a WKT string or looked up
/// by an EPSG code, which is a geodesy library and a database — ISO 19162 and the EPSG registry,
/// both outside this standard, and both named by §12.10.3 as external references. A reader that
/// guessed at it would produce coordinates that look right and are somewhere else.
///
/// What *is* usable without any of that is the registration: [`Self::registration`] pairs the
/// `/GPTS` geographic points with the `/LPTS` positions in the object's unit square, which is
/// the correspondence the file states directly.
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
    pub geographic_points: Vec<[f64; 2]>,
    /// `/LPTS`, the same points in the object's own unit square, which Table 269 says "is
    /// mapped to the rectangular bounds of the `Viewport`, image `XObject`, or forms `XObject`
    /// that contains the measure dictionary".
    pub local_points: Vec<[f64; 2]>,
    /// `/PCSM`, a twelve-element matrix "defining the transformation from `XObject` position
    /// coordinates to projected coordinate system".
    ///
    /// The table states its own precedence twice over: it "should be ignored" when `/GCS` is
    /// geographic, and where it is present "it has priority over GPTS , and GPTS values may be
    /// ignored". Both sentences are about a consumer, and this reader is not one — it carries
    /// the matrix and the points and says which the clause would prefer.
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
