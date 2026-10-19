//! The region and the unit square in the space the running stream draws in, ISO 32000-2
//! §8.3.4 and §12.5.6.23.
//!
//! Every concern that tests a mark against the region asks here for the region's boxes and for the
//! box an image or a form occupies under the current transform.

use pdf_render::geom::Point;

use super::{Walk, bbox_of_points};

impl Walk<'_> {
    /// The region boxes as double-precision numbers, which the cut's decisions are taken in.
    pub(super) fn region_bounds(&self) -> Vec<[f64; 4]> {
        self.regions
            .iter()
            .map(|region| {
                [
                    f64::from(region[0]),
                    f64::from(region[1]),
                    f64::from(region[2]),
                    f64::from(region[3]),
                ]
            })
            .collect()
    }

    /// The unit square mapped through the current transform — an image or form's placement.
    pub(super) fn unit_square(&self) -> [f32; 4] {
        self.transformed_box(&[0.0, 0.0, 1.0, 1.0])
    }

    fn transformed_box(&self, rect: &[f32]) -> [f32; 4] {
        let corners = [
            self.ctm.apply(Point::new(rect[0], rect[1])),
            self.ctm.apply(Point::new(rect[2], rect[1])),
            self.ctm.apply(Point::new(rect[2], rect[3])),
            self.ctm.apply(Point::new(rect[0], rect[3])),
        ];
        let flat: Vec<f32> = corners
            .iter()
            .flat_map(|point| [point.x, point.y])
            .collect();
        bbox_of_points(&flat)
    }
}
