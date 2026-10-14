//! The map between an object's unit square and the earth that a registration determines, in both
//! directions (ADRs 1593, 1672 and 1918).
//!
//! Table 269 states registration points — `/GPTS` against `/LPTS`, pair for pair — and no function
//! between them. §12.10.4 permits "direct mapping between geographic coordinates and PDF object
//! coordinates" for small areas without saying what form it has; the one function Table 269 does
//! state between an object's coordinates and the earth is `/PCSM`, a matrix, and every other
//! mapping between two coordinate spaces in this standard is §8.3's affine one. So the map read
//! here is affine — each of two coordinates a linear function of the unit square's two coordinates
//! plus a constant — fitted by least squares to every pair, which is exact wherever the file's
//! points lie on one such map and the closest one where they do not. **How far the fit departs from
//! the file's own points is kept beside it**, in degrees of either axis, for a host to say rather
//! than to hide or to turn into a refusal by a threshold of this program's choosing. That choice is
//! ADR 1593's.
//!
//! **Which two coordinates depends on the system.** A geographic system's map is affine in its
//! degrees, which is ADR 1593's reading of §12.10.4's direct mapping. A projected system's is affine
//! in the system's plane, because the plane is what the projection made the page a picture of —
//! §12.10.4's system transforms "between geographic coordinates and a two-dimensional (projected)
//! coordinate system" — so a position between the points is the plane's, carried to the earth by
//! the system's inverse projection; registration points written as the base system's degrees are
//! carried into the plane by the forward projection first (ADR 1918).
//!
//! The inverse, [`AffineRegistration::local`], is the same map run backwards rather than a second
//! fit of the other direction: a second least-squares fit would disagree with the first wherever
//! the points lie on no affine map, and a position given to a viewport would then not come back to
//! the point a person reading it put down (ADR 1672).

use super::{GeographicPosition, ProjectedSystem, Refusal};

/// The fewest registration pairs that determine an affine map of the plane.
const AFFINE_PAIRS: usize = 3;

/// One registration pair: a position on the earth and the point of the unit square it is at.
type Pair = (GeographicPosition, [f64; 2]);

/// One registration pair of a projected system: the position on the earth, the same position in
/// the system's plane as an easting and a northing in its own unit, and the point of the unit
/// square it is at.
pub(super) type PlanePair = (GeographicPosition, [f64; 2], [f64; 2]);

/// Two coordinates a map is fitted to, and the point of the unit square they are at.
type Sample = ([f64; 2], [f64; 2]);

/// An affine map from the unit square to two coordinates, centred on the points it was fitted to.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Affine {
    /// The registration points' mean position in the unit square.
    centre: [f64; 2],
    /// Their mean value of each coordinate.
    centre_value: [f64; 2],
    /// Each coordinate's change per unit of `u` and of `v`.
    rows: [[f64; 2]; 2],
}

impl Affine {
    /// The least-squares affine map through `(value, local)` pairs; `None` where fewer than three
    /// are given or every one lies on one line, which determine no map of the plane.
    fn fit(pairs: &[Sample]) -> Option<Self> {
        if pairs.len() < AFFINE_PAIRS {
            return None;
        }
        // The normal equations of `value = a·u + b·v + c`, one shared matrix for both
        // coordinates, centred on the points' mean so that a map registered far from its origin
        // keeps its precision.
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of registration pairs, which a file states in single figures"
        )]
        let count = pairs.len() as f64;
        let mean = |pick: &dyn Fn(&Sample) -> f64| pairs.iter().map(pick).sum::<f64>() / count;
        let centre = [mean(&|pair| pair.1[0]), mean(&|pair| pair.1[1])];
        let (mut suu, mut suv, mut svv) = (0.0, 0.0, 0.0);
        for (_, [u, v]) in pairs {
            let (du, dv) = (u - centre[0], v - centre[1]);
            suu += du * du;
            suv += du * dv;
            svv += dv * dv;
        }
        let determinant = suu * svv - suv * suv;
        // Collinear points: the spread across the line they share is nothing, relative to the
        // spread along it, to the precision a double carries.
        if determinant.abs() <= f64::EPSILON * (suu * svv).max(f64::MIN_POSITIVE) {
            return None;
        }
        let axis = |which: usize| {
            let middle = pairs.iter().map(|pair| pair.0[which]).sum::<f64>() / count;
            let (mut su, mut sv) = (0.0, 0.0);
            for ([first, second], [u, v]) in pairs {
                let delta = [first, second][which] - middle;
                su += (u - centre[0]) * delta;
                sv += (v - centre[1]) * delta;
            }
            (
                middle,
                [
                    (su * svv - sv * suv) / determinant,
                    (sv * suu - su * suv) / determinant,
                ],
            )
        };
        let (first_middle, first) = axis(0);
        let (second_middle, second) = axis(1);
        Some(Self {
            centre,
            centre_value: [first_middle, second_middle],
            rows: [first, second],
        })
    }

    /// The two coordinates the map gives a point of the unit square.
    fn value(&self, local: [f64; 2]) -> [f64; 2] {
        let (du, dv) = (local[0] - self.centre[0], local[1] - self.centre[1]);
        let [first, second] = self.rows;
        [
            self.centre_value[0] + first[0] * du + first[1] * dv,
            self.centre_value[1] + second[0] * du + second[1] * dv,
        ]
    }

    /// The point of the unit square the map gives two coordinates: [`Self::value`] solved for its
    /// argument, or `None` where the map is singular.
    fn local(&self, value: [f64; 2]) -> Option<[f64; 2]> {
        let [[a, b], [c, d]] = self.rows;
        let determinant = a * d - b * c;
        let scale = (a.abs() + b.abs()) * (c.abs() + d.abs());
        if !determinant.is_finite() || determinant.abs() <= f64::EPSILON * scale {
            return None;
        }
        let first = value[0] - self.centre_value[0];
        let second = value[1] - self.centre_value[1];
        Some([
            self.centre[0] + (d * first - b * second) / determinant,
            self.centre[1] + (a * second - c * first) / determinant,
        ])
    }
}

/// The largest distance, in degrees of either axis, between two positions.
fn apart(a: GeographicPosition, b: GeographicPosition) -> f64 {
    (a.latitude - b.latitude)
        .abs()
        .max((a.longitude - b.longitude).abs())
}

/// An affine map from an object's unit square to the earth, fitted to a registration: in degrees
/// for a geographic system, in the plane for a projected one.
#[derive(Debug, Clone, PartialEq)]
pub struct AffineRegistration {
    /// The fitted map: to latitude and longitude, or to easting and northing in `projection`'s
    /// own unit.
    map: Affine,
    /// The projected system whose plane the map is fitted in, or `None` where it is fitted in
    /// degrees.
    projection: Option<ProjectedSystem>,
    /// The largest distance, in degrees of either axis, between a registration point the file
    /// states and where the map puts it.
    departure: f64,
}

impl AffineRegistration {
    /// The least-squares affine map through a registration's pairs, in degrees — a geographic
    /// system's (ADR 1593).
    ///
    /// `None` where fewer than three pairs are stated or every pair lies on one line, which
    /// determine no map of the plane.
    #[must_use]
    pub fn fit(pairs: &[Pair]) -> Option<Self> {
        let values: Vec<Sample> = pairs
            .iter()
            .map(|(position, local)| ([position.latitude, position.longitude], *local))
            .collect();
        let map = Affine::fit(&values)?;
        let departure = pairs
            .iter()
            .map(|(position, at)| {
                let [latitude, longitude] = map.value(*at);
                apart(
                    GeographicPosition {
                        latitude,
                        longitude,
                    },
                    *position,
                )
            })
            .fold(0.0, f64::max);
        Some(Self {
            map,
            projection: None,
            departure,
        })
    }

    /// The least-squares affine map through a projected system's registration, in its plane
    /// (ADR 1918): each pair's easting and northing against its point of the unit square, with
    /// the departure measured on the earth, between each pair's position and the inverse
    /// projection of where the map puts its point.
    ///
    /// `Ok(None)` where the pairs determine no map of the plane, as [`Self::fit`].
    ///
    /// # Errors
    ///
    /// What the system's inverse projection refuses at a point the map puts a registration point.
    pub(super) fn fit_in_plane(
        projection: ProjectedSystem,
        pairs: &[PlanePair],
    ) -> Result<Option<Self>, Refusal> {
        let values: Vec<Sample> = pairs
            .iter()
            .map(|(_, grid, local)| (*grid, *local))
            .collect();
        let Some(map) = Affine::fit(&values) else {
            return Ok(None);
        };
        let mut departure = 0.0_f64;
        for (position, _, at) in pairs {
            let [easting, northing] = map.value(*at);
            departure = departure.max(apart(projection.geographic(easting, northing)?, *position));
        }
        Ok(Some(Self {
            map,
            projection: Some(projection),
            departure,
        }))
    }

    /// The position the map gives a point of the unit square.
    ///
    /// # Errors
    ///
    /// What a projected system's inverse projection refuses of the point the map gives in its
    /// plane; a map fitted in degrees refuses nothing.
    pub fn position(&self, local: [f64; 2]) -> Result<GeographicPosition, Refusal> {
        let [first, second] = self.map.value(local);
        match &self.projection {
            None => Ok(GeographicPosition {
                latitude: first,
                longitude: second,
            }),
            Some(projection) => projection.geographic(first, second),
        }
    }

    /// The point of the unit square the map gives a position: [`Self::position`] solved for its
    /// argument.
    ///
    /// `Ok(None)` where the map is singular — every registration point on one parallel, say, so
    /// that no latitude but that one is anywhere on the map.
    ///
    /// # Errors
    ///
    /// What a projected system's forward projection refuses of the position.
    pub fn local(&self, position: GeographicPosition) -> Result<Option<[f64; 2]>, Refusal> {
        let value = match &self.projection {
            None => [position.latitude, position.longitude],
            Some(projection) => projection.grid(position)?,
        };
        Ok(self.map.local(value))
    }

    /// The largest distance, in degrees of either axis, between a registration point the file
    /// states and where this map puts it; zero where the points lie on an affine map.
    #[must_use]
    pub fn departure(&self) -> f64 {
        self.departure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(latitude: f64, longitude: f64, local: [f64; 2]) -> (GeographicPosition, [f64; 2]) {
        (
            GeographicPosition {
                latitude,
                longitude,
            },
            local,
        )
    }

    #[test]
    fn a_rectangle_of_degrees_is_fitted_exactly_and_run_backwards() {
        // The pdf.js map's four corners, `bug1146106.pdf`: a north-up rectangle of degrees.
        let corners = [
            at(-17.714_38, 165.520_69, [0.0, 1.0]),
            at(-9.433_86, 165.520_69, [0.0, 0.0]),
            at(-9.433_86, 176.865_96, [1.0, 0.0]),
            at(-17.714_38, 176.865_96, [1.0, 1.0]),
        ];
        let fit = AffineRegistration::fit(&corners).expect("four corners span the square");
        assert!(fit.departure() < 1e-12);
        let middle = fit.position([0.5, 0.5]).expect("degrees refuse nothing");
        assert!((middle.latitude - (-13.574_12)).abs() < 1e-9);
        assert!((middle.longitude - 171.193_325).abs() < 1e-9);
        for (position, local) in corners {
            let back = fit
                .local(position)
                .expect("degrees refuse nothing")
                .expect("an invertible map");
            assert!((back[0] - local[0]).abs() < 1e-12 && (back[1] - local[1]).abs() < 1e-12);
        }
    }

    #[test]
    fn a_position_comes_back_to_the_point_that_read_it_where_the_points_lie_on_no_map() {
        // Four points that no affine map passes through: the fit departs from them, and the
        // inverse is still the fit's own, so a point read and given back is the same point.
        let pairs = [
            at(10.0, 20.0, [0.0, 0.0]),
            at(10.0, 21.0, [1.0, 0.0]),
            at(11.0, 21.1, [1.0, 1.0]),
            at(11.0, 20.0, [0.0, 1.0]),
        ];
        let fit = AffineRegistration::fit(&pairs).expect("a map");
        assert!(fit.departure() > 0.0);
        for local in [[0.25, 0.75], [0.5, 0.5], [0.9, 0.1]] {
            let position = fit.position(local).expect("degrees refuse nothing");
            let back = fit
                .local(position)
                .expect("degrees refuse nothing")
                .expect("an invertible map");
            assert!((back[0] - local[0]).abs() < 1e-12 && (back[1] - local[1]).abs() < 1e-12);
        }
    }

    #[test]
    fn a_registration_no_affine_map_passes_through_says_how_far_it_is() {
        // The least-squares map through these four puts each 0.05° of latitude from its point.
        let skewed = [
            at(0.0, 0.0, [0.0, 0.0]),
            at(0.0, 1.0, [1.0, 0.0]),
            at(1.0, 0.0, [0.0, 1.0]),
            at(1.2, 1.0, [1.0, 1.0]),
        ];
        let fit = AffineRegistration::fit(&skewed).expect("four points span");
        assert!((fit.departure() - 0.05).abs() < 1e-9, "{}", fit.departure());
    }

    #[test]
    fn points_on_one_line_or_on_one_parallel_determine_no_map() {
        let collinear = [
            at(10.0, 20.0, [0.0, 0.0]),
            at(10.5, 20.5, [0.5, 0.5]),
            at(11.0, 21.0, [1.0, 1.0]),
        ];
        assert!(AffineRegistration::fit(&collinear).is_none());
        assert!(AffineRegistration::fit(&collinear[..2]).is_none());
        // Spanning the square, but every point on one parallel: the map exists and has no
        // inverse, because no other latitude is anywhere on it.
        let one_parallel = [
            at(10.0, 20.0, [0.0, 0.0]),
            at(10.0, 21.0, [1.0, 0.0]),
            at(10.0, 20.0, [0.0, 1.0]),
        ];
        let fit = AffineRegistration::fit(&one_parallel).expect("a map");
        assert!(
            fit.local(GeographicPosition {
                latitude: 10.0,
                longitude: 20.5
            })
            .expect("degrees refuse nothing")
            .is_none()
        );
    }
}
