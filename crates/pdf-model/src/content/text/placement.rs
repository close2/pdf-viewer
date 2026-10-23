//! §9.4.4's arithmetic: text space, the glyph's matrix and the advance after it.

use pdf_render::{Point, Transform};

use crate::content::Interpreter;

impl Interpreter<'_> {
    /// A step between two text positions, from the space the text matrix's translation is in
    /// back into text space with `Th` taken out: the space `space` and the font size measure.
    ///
    /// The translation is not part of a *step*, so only the matrix's linear part is undone. `Th`
    /// multiplies §9.4.4's `tx` alone, so it is divided out of the horizontal component alone,
    /// sign and all: a negative horizontal scaling mirrors the line and is undone with it. A
    /// matrix or a scaling that collapses text space has nothing to undo, and the step is read
    /// as it stands rather than divided by zero.
    pub(super) fn text_space_step(matrix: Transform, scale: f32, step: (f32, f32)) -> (f32, f32) {
        let linear = Transform::new(matrix.a, matrix.b, matrix.c, matrix.d, 0.0, 0.0);
        let Some(inverse) = linear.invert() else {
            return step;
        };
        let back = inverse.apply(Point::new(step.0, step.1));
        let x = if scale.abs() > f32::EPSILON {
            back.x / scale
        } else {
            back.x
        };
        (x, back.y)
    }

    /// Glyph space to text space: the font size, the horizontal scaling, and the rise.
    ///
    /// §9.2.4 adds one term in writing mode 1: "the glyph position shall be described by a
    /// position vector from the origin used for horizontal writing (origin 0) to the origin
    /// used for vertical writing (origin 1)". The outline is stated relative to origin 0 and
    /// the text position *is* origin 1, so the glyph moves back by `v`, which is zero for
    /// every font in writing mode 0.
    pub(super) fn glyph_to_text(size: f32, scale: f32, rise: f32, position: [f32; 2]) -> Transform {
        Transform::new(
            size * scale,
            0.0,
            0.0,
            size,
            -position[0] * size * scale,
            (-position[1]).mul_add(size, rise),
        )
    }

    /// §9.4.4's combined displacement, as the translation it applies to the text matrix.
    ///
    /// The clause computes `tx` in horizontal writing mode and `ty` in vertical, "the
    /// variable corresponding to the other writing mode shall be set to 0", and the two
    /// differ in one term: the horizontal scaling multiplies `tx` alone, because `Th` scales
    /// the *width* of a line rather than the advance along it. Character and word spacing are
    /// added to whichever component applies.
    pub(super) fn advance_step(
        displacement: f32,
        size: f32,
        spacing: f32,
        scale: f32,
        vertical: bool,
    ) -> Transform {
        if vertical {
            Transform::translate(0.0, displacement.mul_add(size, spacing))
        } else {
            Transform::translate(displacement.mul_add(size, spacing) * scale, 0.0)
        }
    }
}
