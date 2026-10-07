//! The map between an object's unit square and the earth that a registration determines, in both
//! directions (ADRs 1593 and 1672).
//!
//! Table 269 states registration points — `/GPTS` against `/LPTS`, pair for pair — and no function
//! between them. §12.10.4 permits "direct mapping between geographic coordinates and PDF object
//! coordinates" for small areas without saying what form it has; the one function Table 269 does
//! state between an object's coordinates and the earth is `/PCSM`, a matrix, and every other
//! mapping between two coordinate spaces in this standard is §8.3's affine one. So the map read
//! here is affine — latitude and longitude each a linear function of the unit square's two
//! coordinates plus a constant — fitted by least squares to every pair, which is exact wherever the
//! file's points lie on one such map (a north-up rectangle of degrees does) and the closest one
//! where they do not. **How far the fit departs from the file's own points is kept beside it**, for
//! a host to say rather than to hide or to turn into a refusal by a threshold of this program's
//! choosing. That choice is ADR 1593's.
//!
//! The inverse, [`AffineRegistration::local`], is the same map run backwards rather than a second
//! fit of the other direction: a second least-squares fit would disagree with the first wherever
//! the points lie on no affine map, and a position given to a viewport would then not come back to
//! the point a person reading it put down (ADR 1672).

use super::GeographicPosition;

/// The fewest registration pairs that determine an affine map of the plane.
const AFFINE_PAIRS: usize = 3;

/// One registration pair: a position on the earth and the point of the unit square it is at.
type Pair = (GeographicPosition, [f64; 2]);

/// An affine map from an object's unit square to degrees, fitted to a registration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AffineRegistration {
    /// The registration points' mean position in the unit square.
    centre: [f64; 2],
    /// Their mean latitude and longitude.
    centre_degrees: [f64; 2],
    /// Degrees of latitude per unit of `u` and of `v`.
    latitude: [f64; 2],
    /// Degrees of longitude per unit of `u` and of `v`.
    longitude: [f64; 2],
    /// The largest distance, in degrees of either axis, between a registration point the file
    /// states and where the map puts it.
    departure: f64,
}

impl AffineRegistration {
    /// The least-squares affine map through a registration's pairs.
    ///
    /// `None` where fewer than three pairs are stated or every pair lies on one line, which
    /// determine no map of the plane.
    #[must_use]
    pub fn fit(pairs: &[Pair]) -> Option<Self> {
        if pairs.len() < AFFINE_PAIRS {
            return None;
        }
        // The normal equations of `value = a·u + b·v + c`, one shared matrix for both axes,
        // centred on the points' mean so that a map registered far from its origin keeps its
        // precision.
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of registration pairs, which a file states in single figures"
        )]
        let count = pairs.len() as f64;
        let mean = |pick: &dyn Fn(&Pair) -> f64| pairs.iter().map(pick).sum::<f64>() / count;
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
        let axis = |pick: &dyn Fn(&GeographicPosition) -> f64| {
            let middle = pairs.iter().map(|pair| pick(&pair.0)).sum::<f64>() / count;
            let (mut su, mut sv) = (0.0, 0.0);
            for (position, [u, v]) in pairs {
                let delta = pick(position) - middle;
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
        let (middle_latitude, latitude) = axis(&|position| position.latitude);
        let (middle_longitude, longitude) = axis(&|position| position.longitude);
        let mut fit = Self {
            centre,
            centre_degrees: [middle_latitude, middle_longitude],
            latitude,
            longitude,
            departure: 0.0,
        };
        fit.departure = pairs
            .iter()
            .map(|(position, at)| {
                let mapped = fit.position(*at);
                (mapped.latitude - position.latitude)
                    .abs()
                    .max((mapped.longitude - position.longitude).abs())
            })
            .fold(0.0, f64::max);
        Some(fit)
    }

    /// The position the map gives a point of the unit square.
    #[must_use]
    pub fn position(&self, [u, v]: [f64; 2]) -> GeographicPosition {
        let (du, dv) = (u - self.centre[0], v - self.centre[1]);
        GeographicPosition {
            latitude: self.centre_degrees[0] + self.latitude[0] * du + self.latitude[1] * dv,
            longitude: self.centre_degrees[1] + self.longitude[0] * du + self.longitude[1] * dv,
        }
    }

    /// The point of the unit square the map gives a position: [`Self::position`] solved for its
    /// argument.
    ///
    /// `None` where the map is singular — every registration point on one parallel, say, so that
    /// no latitude but that one is anywhere on the map.
    #[must_use]
    pub fn local(&self, position: GeographicPosition) -> Option<[f64; 2]> {
        let [[a, b], [c, d]] = [self.latitude, self.longitude];
        let determinant = a * d - b * c;
        let scale = (a.abs() + b.abs()) * (c.abs() + d.abs());
        if !determinant.is_finite() || determinant.abs() <= f64::EPSILON * scale {
            return None;
        }
        let latitude = position.latitude - self.centre_degrees[0];
        let longitude = position.longitude - self.centre_degrees[1];
        Some([
            self.centre[0] + (d * latitude - b * longitude) / determinant,
            self.centre[1] + (a * longitude - c * latitude) / determinant,
        ])
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
        let middle = fit.position([0.5, 0.5]);
        assert!((middle.latitude - (-13.574_12)).abs() < 1e-9);
        assert!((middle.longitude - 171.193_325).abs() < 1e-9);
        for (position, local) in corners {
            let back = fit.local(position).expect("an invertible map");
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
            let back = fit.local(fit.position(local)).expect("an invertible map");
            assert!((back[0] - local[0]).abs() < 1e-12 && (back[1] - local[1]).abs() < 1e-12);
        }
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
            .is_none()
        );
    }
}
