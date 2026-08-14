//! The quadrilaterals this boundary hands over, as one owned handle.
//!
//! **`doc/ui-boundary.md`'s rule is that interactive chrome crosses as geometry rather than as
//! pixels** — a selection highlight, a field's selection, a search hit — "so that a native host can
//! draw them in macOS's selection colour, KDE's accent or the Windows highlight brush". Four of
//! `viewer_core`'s answers are the same shape, `[x0, y0, … x3, y3]` in device pixels of the
//! viewport, and a C caller wants one way to read all four rather than four.
//!
//! Owned, for [`crate::events`]'s reason: `Answer::Selected` borrows the viewer, and a caller
//! holding that borrow while it calls back in is the aliasing hazard nothing on this side would
//! notice. A drag asks for these on every frame, so the allocation is on a path a person drives —
//! it is one `Vec` of eight floats per run of a line, which is what the Rust host already copies
//! into its own scene.

use crate::status::Status;

/// A list of quadrilaterals, in device pixels of the viewport.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Quads {
    /// One entry per shape, in the order the viewer answered with.
    shapes: Vec<[f32; 8]>,
}

impl Quads {
    /// Takes what one of `viewer_core`'s shape answers held.
    #[must_use]
    pub fn new(shapes: Vec<[f32; 8]>) -> Self {
        Self { shapes }
    }

    /// How many shapes there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// Whether there are none, which is what an empty selection answers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// One shape's eight numbers.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such shape.
    pub fn get(&self, index: usize) -> Result<[f32; 8], Status> {
        self.shapes.get(index).copied().ok_or(Status::OutOfRange)
    }
}

#[cfg(test)]
mod tests {
    use super::Quads;
    use crate::status::Status;

    /// An index past the end is a refusal rather than eight zeroes, which would be a shape.
    #[test]
    fn a_shape_that_is_not_there_is_refused_rather_than_answered_with_zeroes() {
        let quads = Quads::new(vec![[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]]);
        assert_eq!(quads.len(), 1);
        assert_eq!(quads.get(0), Ok([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]));
        assert_eq!(quads.get(1), Err(Status::OutOfRange));
        assert!(Quads::default().is_empty());
    }
}
