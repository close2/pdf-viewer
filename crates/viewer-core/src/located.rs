//! §12.10's position: where on the earth a point of a geospatial viewport is (ADR 1593).
//!
//! `pdf_model::geospatial` carries a file's registration into degrees — a geographic `/GCS`'s
//! `/GPTS` as the file states them, a projected one's through its inverse projection, and `/PCSM`
//! where Table 269 gives it priority. What it leaves open, because §12.10 states no function
//! between registration points, is the position *between* them, which is the only position a
//! person pointing at a map ever asks for. This module is that step, and it is a choice.
//!
//! # The choice: the affine map the registration determines
//!
//! §12.10.4 says that "[f]or small areas, this distortion may be small enough to allow direct
//! mapping between geographic coordinates and PDF object coordinates without requiring the use of
//! a projected coordinate system". The standard states no form for that direct mapping; the one
//! function Table 269 does state between an object's coordinates and the earth is `/PCSM`, a
//! matrix, and every other mapping between two coordinate spaces in this standard is §8.3's affine
//! one. So the mapping read here is affine — latitude and longitude each a linear function of the
//! unit square's two coordinates plus a constant — fitted by least squares to every registration
//! pair, which is exact wherever the file's points lie on one such map (a north-up rectangle of
//! degrees does) and the closest one where they do not. **What the fit departs from the file's own
//! points by is said beside the answer** rather than hidden or turned into a refusal by a
//! threshold of this program's choosing: the file stated those points, and a person reading a
//! position is owed how far the reading is from them.

use pdf_model::geospatial::GeographicPosition;
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
        /// one: the position in that system, or the sentence saying why it cannot be reached.
        display: Option<Result<(f64, f64), String>>,
        /// The largest distance, in degrees of either axis, between a registration point the
        /// file states and where the affine map puts it; zero where `/PCSM` gave the position.
        departure: f64,
    },
    /// No position, and the sentence saying why — the file's system refused by name, a point
    /// outside the neatline, or registration points that determine no map.
    Refused(String),
}

/// The fewest registration pairs that determine an affine map of the plane.
const AFFINE_PAIRS: usize = 3;

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
    let (position, departure) = match matrix {
        Some(Ok(position)) => (position, 0.0),
        Some(Err(refusal)) => return Located::Refused(refusal.to_string()),
        None => match geospatial.registration_geographic() {
            Ok(pairs) => match fitted(&pairs, local) {
                Some(fit) => fit,
                None => {
                    return Located::Refused(format!(
                        "the file's {} registration point(s) do not span the map: an affine map \
                         needs three that are not on one line",
                        pairs.len()
                    ));
                }
            },
            Err(refusal) => return Located::Refused(refusal.to_string()),
        },
    };
    let display = geospatial.display_system.as_ref().map(|_| {
        geospatial
            .display_position(position)
            .map(|shown| (shown.latitude, shown.longitude))
            .map_err(|refusal| refusal.to_string())
    });
    Located::At {
        latitude: position.latitude,
        longitude: position.longitude,
        display,
        departure,
    }
}

/// The least-squares affine map from the unit square to degrees, evaluated at `local`, and the
/// largest departure of the file's own points from it.
///
/// `None` where fewer than three pairs are stated or every pair lies on one line, which determine
/// no map of the plane.
fn fitted(
    pairs: &[(GeographicPosition, [f64; 2])],
    local: [f64; 2],
) -> Option<(GeographicPosition, f64)> {
    if pairs.len() < AFFINE_PAIRS {
        return None;
    }
    // The normal equations of `value = a·u + b·v + c`, one shared matrix for both axes. Centred on
    // the points' mean so that a map registered far from its origin keeps its precision.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a count of registration pairs, which a file states in single figures"
    )]
    let count = pairs.len() as f64;
    let mean = |pick: fn(&(GeographicPosition, [f64; 2])) -> f64| {
        pairs.iter().map(pick).sum::<f64>() / count
    };
    let (mu, mv) = (mean(|pair| pair.1[0]), mean(|pair| pair.1[1]));
    let (mut suu, mut suv, mut svv) = (0.0, 0.0, 0.0);
    for (_, [u, v]) in pairs {
        let (du, dv) = (u - mu, v - mv);
        suu += du * du;
        suv += du * dv;
        svv += dv * dv;
    }
    let determinant = suu * svv - suv * suv;
    // Collinear points: the spread across the line they share is nothing, relative to the spread
    // along it, to the precision a double carries.
    if determinant.abs() <= f64::EPSILON * (suu * svv).max(f64::MIN_POSITIVE) {
        return None;
    }
    let axis = |pick: fn(&GeographicPosition) -> f64| {
        let centre = pairs.iter().map(|pair| pick(&pair.0)).sum::<f64>() / count;
        let (mut su, mut sv) = (0.0, 0.0);
        for (position, [u, v]) in pairs {
            let delta = pick(position) - centre;
            su += (u - mu) * delta;
            sv += (v - mv) * delta;
        }
        let a = (su * svv - sv * suv) / determinant;
        let b = (sv * suu - su * suv) / determinant;
        move |[u, v]: [f64; 2]| centre + a * (u - mu) + b * (v - mv)
    };
    let latitude = axis(|position| position.latitude);
    let longitude = axis(|position| position.longitude);
    let departure = pairs
        .iter()
        .map(|(position, at)| {
            (latitude(*at) - position.latitude)
                .abs()
                .max((longitude(*at) - position.longitude).abs())
        })
        .fold(0.0, f64::max);
    Some((
        GeographicPosition {
            latitude: latitude(local),
            longitude: longitude(local),
        },
        departure,
    ))
}

#[cfg(test)]
mod tests {
    use super::{GeographicPosition, fitted};

    fn at(latitude: f64, longitude: f64, local: [f64; 2]) -> (GeographicPosition, [f64; 2]) {
        (
            GeographicPosition {
                latitude,
                longitude,
            },
            local,
        )
    }

    /// The pdf.js map's four corners: degrees that are a north-up rectangle, so the fit is
    /// exact and the middle of the square is the middle of the rectangle.
    #[test]
    fn a_rectangle_of_degrees_is_fitted_exactly() {
        let corners = [
            at(-17.714_38, 165.520_69, [0.0, 1.0]),
            at(-9.433_86, 165.520_69, [0.0, 0.0]),
            at(-9.433_86, 176.865_96, [1.0, 0.0]),
            at(-17.714_38, 176.865_96, [1.0, 1.0]),
        ];
        let (middle, departure) = fitted(&corners, [0.5, 0.5]).expect("four corners span");
        assert!((middle.latitude - (-13.574_12)).abs() < 1e-9, "{middle:?}");
        assert!((middle.longitude - 171.193_325).abs() < 1e-9, "{middle:?}");
        assert!(departure < 1e-9, "{departure}");
    }

    /// Points the map cannot pass through all of are fitted, and the departure says by how much.
    #[test]
    fn a_registration_no_affine_map_passes_through_says_how_far_it_is() {
        let skewed = [
            at(0.0, 0.0, [0.0, 0.0]),
            at(0.0, 1.0, [1.0, 0.0]),
            at(1.0, 0.0, [0.0, 1.0]),
            at(1.2, 1.0, [1.0, 1.0]),
        ];
        let (_, departure) = fitted(&skewed, [0.5, 0.5]).expect("four points span");
        assert!((departure - 0.05).abs() < 1e-9, "{departure}");
    }

    /// Two points, or three on one line, determine no map of the plane.
    #[test]
    fn points_on_one_line_determine_no_map() {
        assert!(
            fitted(
                &[at(0.0, 0.0, [0.0, 0.0]), at(1.0, 1.0, [1.0, 1.0])],
                [0.5, 0.5]
            )
            .is_none()
        );
        let line = [
            at(0.0, 0.0, [0.0, 0.0]),
            at(1.0, 1.0, [0.5, 0.5]),
            at(2.0, 2.0, [1.0, 1.0]),
        ];
        assert!(fitted(&line, [0.5, 0.5]).is_none());
    }
}
