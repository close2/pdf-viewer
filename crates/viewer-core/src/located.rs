//! §12.10's position: where on the earth a point of a geospatial viewport is (ADR 1593).
//!
//! `pdf_model::geospatial` carries a file's registration to the earth — a geographic `/GCS`'s
//! `/GPTS` as the file states them, a projected one's through its projection, and `/PCSM` where
//! Table 269 gives it priority. What §12.10 leaves open, because it states no function between
//! registration points, is the position *between* them, which is the only position a person
//! pointing at a map ever asks for. This module asks for that position and says it, and the
//! function it is read through is a choice.
//!
//! # The choice: the affine map the registration determines
//!
//! §12.10.4 says that "\[f]or small areas, this distortion may be small enough to allow direct
//! mapping between geographic coordinates and PDF object coordinates without requiring the use of
//! a projected coordinate system". The standard states no form for that direct mapping; the one
//! function Table 269 does state between an object's coordinates and the earth is `/PCSM`, a
//! matrix, and every other mapping between two coordinate spaces in this standard is §8.3's affine
//! one. So the mapping read here is `Geospatial::registration_map`'s affine one, fitted by least
//! squares to every registration pair — in degrees for a geographic system, in the plane for a
//! projected one (ADR 1918) — which is exact wherever the file's points lie on one such map (a
//! north-up rectangle of degrees does) and the closest one where they do not. **What the fit
//! departs from the file's own points by is said beside the answer** rather than hidden or turned
//! into a refusal by a threshold of this program's choosing: the file stated those points, and a
//! person reading a position is owed how far the reading is from them.

use pdf_model::geospatial::Displayed;
use pdf_model::measurement::{Geospatial, Measure, Viewports};

/// Where on the earth one point of a geospatial viewport is, or why no position is given.
#[derive(Debug, Clone, PartialEq)]
pub enum Located {
    /// A latitude and a longitude of the file's own geographic system.
    At {
        /// Degrees north, negative south.
        latitude: f64,
        /// Degrees east of the system's prime meridian, negative west.
        longitude: f64,
        /// Table 269's `/DCS`, "used for the display of position values", where the file names
        /// one: the position in that system — a latitude and a longitude for a geographic one, an
        /// easting and a northing in its own unit for a projected one (ADR 1678) — or the
        /// sentence saying why it cannot be reached.
        display: Option<Result<Displayed, String>>,
        /// The largest distance, in degrees of either axis, between a registration point the
        /// file states and where the affine map puts it; zero where `/PCSM` gave the position.
        departure: f64,
    },
    /// No position, and the sentence saying why — the file's system refused by name, a point
    /// outside the neatline, or registration points that determine no map.
    Refused(String),
}

/// The position of a point in default user space, where the viewport containing it is
/// geospatial; `None` where it is not, because then there is nothing of §12.10's to say.
pub(crate) fn locate(viewports: &Viewports, point: (f32, f32)) -> Option<Located> {
    let viewport = viewports.at(point)?;
    let Some(Measure::Geospatial(geospatial)) = &viewport.measure else {
        return None;
    };
    let Some(local) = viewport.unit_square(point) else {
        return Some(Located::Refused(
            "the viewport's /BBox has no extent, so it states no unit square to locate a point in"
                .to_owned(),
        ));
    };
    // Table 269's `/Bounds` describes "the bounds of an area for which geospatial transformations
    // are valid": outside it, the file has disclaimed every position.
    if !geospatial.within_bounds(local) {
        return Some(Located::Refused(
            "this point is outside the map's neatline, where the file's registration does not \
             apply (Table 269's /Bounds)"
                .to_owned(),
        ));
    }
    Some(position(geospatial, point, local))
}

/// The position itself: `/PCSM` where Table 269 gives it priority, the affine fit otherwise.
fn position(geospatial: &Geospatial, point: (f32, f32), local: [f64; 2]) -> Located {
    // `/PCSM` is "the transformation from XObject position coordinates", and a page's viewport is
    // positioned in default user space, so that is the position it is handed.
    let matrix = geospatial.geographic_position([f64::from(point.0), f64::from(point.1), 0.0]);
    let found = match matrix {
        Some(found) => found.map(|position| (position, 0.0)),
        None => geospatial
            .registration_map()
            .and_then(|map| Ok((map.position(local)?, map.departure()))),
    };
    let (position, departure) = match found {
        Ok(found) => found,
        Err(refusal) => return Located::Refused(refusal.to_string()),
    };
    // `display` rather than `display_position`, which keeps its degrees-only contract: a
    // projected `/DCS` is a system whose position values are an easting and a northing, and the
    // table says that system "shall be used for the display" of them (ADRs 1672, 1678).
    let display = geospatial.display_system.as_ref().map(|_| {
        geospatial
            .display(position)
            .map_err(|refusal| refusal.to_string())
    });
    Located::At {
        latitude: position.latitude,
        longitude: position.longitude,
        display,
        departure,
    }
}
