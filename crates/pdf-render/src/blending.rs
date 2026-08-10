//! Compositing a page in a subtractive blending colour space (ISO 32000-2 §11.4.7).
//!
//! §11.4.7 puts a colour space under the whole page:
//!
//! > All page-level compositing shall be done in the default blending colour space of the
//! > page, and the entire result shall then, if the colour spaces are not equivalent, be
//! > converted to the native colour space of the output device before being composited with
//! > the context-dependent backdrop.
//!
//! Where that space has four components and the device raster holds three, the two orders of
//! operation are the same picture only if the conversion out is affine, and this tree's is
//! not (ADR 0251). What makes the four components fit anyway is that §11.3.3's compositing is
//! **per component**: a rasteriser that composites three channels composites four if it is
//! run twice on the same geometry with a different three loaded, and §11.3.5.2's separable
//! blend functions are per component as well. So a page is drawn twice — once carrying cyan,
//! magenta and yellow, once carrying black — and [`resolve`] puts the two rasters back
//! together at the end, which is where §11.4.7 puts the conversion.
//!
//! Both rasters hold §11.3.4's **additive** form, so nothing here or in a backend has to
//! complement anything around a blend function:
//!
//! > When performing blending operations in subtractive colour spaces ( DeviceCMYK ,
//! > ICCBased 'CMYK', Separation , and DeviceN ), the colour component values shall be
//! > complemented (subtracted from 1.0) before the blend function is applied and the results
//! > of the function shall then be complemented back before being used.
//!
//! The interpreter names the conversion rather than the space, for the reason
//! [`crate::Color`] gives: a backend never sees a colour space, and sixteen corners are a
//! table rather than one.

/// The conversion out of a four-component blending colour space, as its cube's corners.
///
/// A `DeviceCMYK` colour becomes a device colour by multilinear interpolation between the
/// sixteen corners of the ink cube — the construction ADRs 0009 and 0042 chose, and the one
/// an ICC lookup table uses over its own grid. The interpreter resolves the corners (which
/// press to assume, or a profile's answer at them) and hands them over; nothing below the
/// display list needs to know which space they came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlendingSpace {
    /// The device colour at each corner, indexed by the bits `c m y k` from the least
    /// significant, so index 0 is no ink at all and index 15 is every ink at once.
    pub corners: [[f32; 3]; 16],
}

impl BlendingSpace {
    /// The device colour of one set of four components, each in `0.0..=1.0`.
    #[must_use]
    pub fn convert(&self, cyan: f32, magenta: f32, yellow: f32, black: f32) -> [f32; 3] {
        let axes = [
            [1.0 - cyan, cyan],
            [1.0 - magenta, magenta],
            [1.0 - yellow, yellow],
            [1.0 - black, black],
        ];
        let mut rgb = [0.0f32; 3];
        for (index, corner) in self.corners.iter().enumerate() {
            let weight = axes[0][index & 1]
                * axes[1][(index >> 1) & 1]
                * axes[2][(index >> 2) & 1]
                * axes[3][(index >> 3) & 1];
            if weight == 0.0 {
                continue;
            }
            for (channel, value) in rgb.iter_mut().zip(corner) {
                *channel += weight * value;
            }
        }
        rgb
    }
}

/// Converts a pair of premultiplied RGBA8 rasters out of the blending colour space.
///
/// `chromatic` carries the additive complements of cyan, magenta and yellow in its three
/// channels and is overwritten with the device colour; `black` carries the complement of the
/// black component in each of its three, so any one of them may be read. Both are
/// premultiplied by the same alpha, because the two passes composited the same geometry
/// under the same shapes and opacities — §11.4.7's conversion comes *before* the page is
/// composited with the medium, which is why the alpha survives it untouched.
///
/// **The components are recovered by dividing the alpha out, and that costs at most one level
/// of 255.** A premultiplied channel resolves a component to `1 ÷ (255 α)`, the conversion is
/// a convex combination of colours in `0..=1` and so cannot magnify an error, and the
/// converted colour is multiplied by `α` again — so the error in the pixel this writes is
/// bounded by `α × 1 ÷ (255 α)`, one level, whatever the alpha was. That is the same price
/// ADR 0220 paid for a mask's channel, for the same reason.
///
/// A pixel nothing painted is left alone: with no alpha there is no colour to convert, and
/// [`crate::impose_on_medium`] is what puts the medium there.
pub fn resolve(chromatic: &mut [u8], black: &[u8], space: &BlendingSpace) {
    for (pixel, ink) in chromatic.chunks_exact_mut(4).zip(black.chunks_exact(4)) {
        let alpha = f32::from(pixel[3]);
        if pixel[3] == 0 {
            continue;
        }
        let component = |value: u8| 1.0 - (f32::from(value) / alpha).clamp(0.0, 1.0);
        let rgb = space.convert(
            component(pixel[0]),
            component(pixel[1]),
            component(pixel[2]),
            component(ink[0]),
        );
        for (channel, value) in pixel.iter_mut().zip(rgb) {
            // Back into premultiplied form, which is what the caller's raster holds and what
            // `impose_on_medium` composites in.
            let scaled = value.clamp(0.0, 1.0).mul_add(alpha, 0.5);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a value in 0..=1 scaled by an alpha in 0..=255 is in 0..=255"
            )]
            {
                *channel = scaled as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sixteen corners this tree's `DeviceCMYK` conversion assumes, as this module's
    /// table. Copied from `pdf_model::colour`'s `CMYK_CORNERS`, which cannot be reached from
    /// here — `pdf-render` is below `pdf-model` — and which the interpreter hands over at
    /// run time.
    fn process_inks() -> BlendingSpace {
        const CORNERS: [[u8; 3]; 16] = [
            [255, 255, 255],
            [0, 173, 239],
            [236, 0, 140],
            [46, 49, 146],
            [255, 242, 0],
            [0, 166, 80],
            [237, 28, 36],
            [54, 54, 57],
            [35, 31, 32],
            [0, 15, 36],
            [36, 0, 0],
            [0, 0, 2],
            [28, 26, 0],
            [0, 19, 0],
            [34, 0, 0],
            [0, 0, 0],
        ];
        let mut corners = [[0.0f32; 3]; 16];
        for (target, source) in corners.iter_mut().zip(CORNERS) {
            for (channel, value) in target.iter_mut().zip(source) {
                *channel = f32::from(value) / 255.0;
            }
        }
        BlendingSpace { corners }
    }

    /// Half of registration black over paper, which is ADR 0251's own fixture.
    ///
    /// §11.4.7 composites in the page's space and converts the result, so half the ink of
    /// registration black is `[0.5, 0.5, 0.5, 0.5]` in `DeviceCMYK` — the average of the
    /// sixteen corners, 76.0 of 255 in red. Compositing on the device instead averages
    /// registration black with paper, which is 127.5. The gap is 51.5 of 255 and it is what
    /// this module exists to remove.
    #[test]
    fn half_of_registration_black_over_paper_is_the_average_of_the_cube() {
        let space = process_inks();
        // Premultiplied: the ink is at full coverage over paper already composited in, so
        // every component is half way between paper's and registration black's.
        let mut chromatic = vec![128, 128, 128, 255];
        let black = vec![128, 128, 128, 255];
        resolve(&mut chromatic, &black, &space);
        let expected = space.convert(0.498, 0.498, 0.498, 0.498);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a converted component in 0..=1 scaled by 255"
        )]
        let level = (expected[0] * 255.0 + 0.5) as u8;
        assert_eq!(
            chromatic[3], 255,
            "the alpha is not the conversion's to move"
        );
        assert_eq!(
            chromatic[0], level,
            "the red at half of registration black is the cube's own average, not 128"
        );
        assert!(
            (76..=77).contains(&chromatic[0]),
            "ADR 0251's number: 76.0 of 255, against the 127.5 compositing on the device gives \
             — got {}",
            chromatic[0]
        );
    }

    /// An unpainted pixel keeps no colour, because there is none to convert.
    #[test]
    fn a_pixel_with_no_alpha_is_left_for_the_medium() {
        let space = process_inks();
        let mut chromatic = vec![0, 0, 0, 0];
        resolve(&mut chromatic, &[0, 0, 0, 0], &space);
        assert_eq!(chromatic, vec![0, 0, 0, 0]);
    }

    /// Paper converts to white and registration black to black, which is the table's own two
    /// ends and the check that the index order is `c m y k` rather than the reverse.
    #[test]
    fn the_two_ends_of_the_cube_survive_the_round_trip() {
        let space = process_inks();
        // No ink at all: both rasters hold the complement, which is 1.0.
        let mut paper = vec![255, 255, 255, 255];
        resolve(&mut paper, &[255, 255, 255, 255], &space);
        assert_eq!(paper, vec![255, 255, 255, 255]);

        // Every ink at once: both complements are zero.
        let mut registration = vec![0, 0, 0, 255];
        resolve(&mut registration, &[0, 0, 0, 255], &space);
        assert_eq!(registration, vec![0, 0, 0, 255]);

        // Cyan alone, which is the corner at index 1 and would be index 8 if the bits ran the
        // other way.
        let mut cyan = vec![0, 255, 255, 255];
        resolve(&mut cyan, &[255, 255, 255, 255], &space);
        assert_eq!(cyan, vec![0, 173, 239, 255]);
    }
}
