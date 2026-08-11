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
//! **§11.3.5.3's four non-separable modes are carried by the same pair, and the clause's own
//! split falls exactly where the two rasters already are.** Its auxiliary functions "operate on
//! colours that are assumed to have red, green, and blue components" — three, which is what
//! each raster holds — and the CMYK rule is two bullets:
//!
//! > The C , M and Y components shall be converted to their complementary R , G and B
//! > components by subtracting each from 1.0. The formulae in this subclause shall be applied
//! > to the RGB colour values. The results shall be complemented back to C , M and Y in the
//! > same way.
//!
//! > For the K component, the result shall be the K component of Cb for the Hue , Saturation ,
//! > and Color blend modes; it shall be the K component of Cs for the Luminosity blend mode.
//!
//! The first bullet is the chromatic raster's contents, so a backend's `Hue`, `Saturation`,
//! `Color` and `Luminosity` see the R, G and B the clause names, with nothing mapped around
//! them. **The second is not a second rule at all**: the black raster is neutral in all three
//! of its channels by construction, and on a neutral pair the clause's own four functions
//! return the backdrop for `Hue`, `Saturation` and `Color` and the source for `Luminosity` —
//! which is that bullet, term for term. `render-cpu`'s `blend` module states the derivation and
//! `the_clauses_own_functions_give_the_black_components_rule_on_a_neutral_pair` checks it over
//! 200 000 pairs. So a page in four components asks a backend for nothing it does not already
//! do for a page in three.
//!
//! The interpreter names the conversion rather than the space, for the reason
//! [`crate::Color`] gives: a backend never sees a colour space, and sixteen corners are a
//! table rather than one.

/// The conversion out of a four-component blending colour space, as a sampled grid.
///
/// A colour of four components becomes a device colour by multilinear interpolation over a
/// grid of `side` samples per axis — the construction ADRs 0009 and 0042 chose, and the one an
/// ICC lookup table uses over its own grid. The interpreter resolves the samples (which press
/// to assume, or the answers a document's own profile gives at them) and hands them over;
/// nothing below the display list needs to know which space they came from.
///
/// **`side` is 2 for the assumed process inks**, whose sixteen corners are the whole table and
/// for which the interpolation below is exactly the multilinear one this struct held until the
/// four-hundred-and-thirty-sixth session. A document that names its own press (§8.6.5.6's
/// `/DefaultCMYK`, §14.11.5's output intent) or states a four-component `ICCBased` blending
/// space (§11.7.2) supplies a finer grid instead, because a real press is not multilinear
/// between its corners. ADR 0272.
#[derive(Debug, Clone, PartialEq)]
pub struct BlendingSpace {
    /// How many samples the grid holds along each of the four axes; at least two.
    side: usize,
    /// `side⁴` device colours. The index runs `c` fastest and `k` slowest, so at `side` 2 it
    /// is the bits `c m y k` from the least significant — index 0 is no ink at all and the
    /// last is every ink at once.
    grid: std::sync::Arc<[[f32; 3]]>,
}

impl BlendingSpace {
    /// A blending space from a grid, or `None` if the grid is not one.
    ///
    /// The two conditions are the ones every reader below depends on and neither is checked
    /// again: at least two samples per axis, and exactly `side⁴` of them.
    #[must_use]
    pub fn new(side: usize, grid: std::sync::Arc<[[f32; 3]]>) -> Option<Self> {
        let wanted = side.checked_pow(4)?;
        (side >= 2 && grid.len() == wanted).then_some(Self { side, grid })
    }

    /// How many samples the grid holds along each axis.
    #[must_use]
    pub fn side(&self) -> usize {
        self.side
    }

    /// The samples themselves, in the index order [`BlendingSpace::new`] documents.
    #[must_use]
    pub fn grid(&self) -> &[[f32; 3]] {
        &self.grid
    }

    /// The device colour of one set of four components, each in `0.0..=1.0`.
    #[must_use]
    pub fn convert(&self, cyan: f32, magenta: f32, yellow: f32, black: f32) -> [f32; 3] {
        let last = self.side.saturating_sub(1);
        // Each axis contributes a cell index and the fraction across that cell; at `side` 2
        // the cell is the whole axis and the fraction is the component itself, which is what
        // makes this the multilinear interpolation of sixteen corners in that case.
        let axis = |value: f32| -> (usize, [f32; 2]) {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a grid side and a cell index, both far below f32's exact range"
            )]
            let scaled = value.clamp(0.0, 1.0) * last as f32;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "`scaled` is in 0..=last, so its floor is a valid index"
            )]
            let cell = (scaled as usize).min(last.saturating_sub(1));
            #[expect(
                clippy::cast_precision_loss,
                reason = "a cell index below the grid side"
            )]
            let fraction = scaled - cell as f32;
            (cell, [1.0 - fraction, fraction])
        };
        let (cyan_cell, cyan_weights) = axis(cyan);
        let (magenta_cell, magenta_weights) = axis(magenta);
        let (yellow_cell, yellow_weights) = axis(yellow);
        let (black_cell, black_weights) = axis(black);

        let mut rgb = [0.0f32; 3];
        for corner in 0..16usize {
            let offsets = [
                corner & 1,
                (corner >> 1) & 1,
                (corner >> 2) & 1,
                (corner >> 3) & 1,
            ];
            let weight = cyan_weights[offsets[0]]
                * magenta_weights[offsets[1]]
                * yellow_weights[offsets[2]]
                * black_weights[offsets[3]];
            if weight == 0.0 {
                continue;
            }
            let index = black_cell
                .saturating_add(offsets[3])
                .saturating_mul(self.side)
                .saturating_add(yellow_cell)
                .saturating_add(offsets[2])
                .saturating_mul(self.side)
                .saturating_add(magenta_cell)
                .saturating_add(offsets[1])
                .saturating_mul(self.side)
                .saturating_add(cyan_cell)
                .saturating_add(offsets[0]);
            let Some(sample) = self.grid.get(index) else {
                continue;
            };
            for (channel, value) in rgb.iter_mut().zip(sample) {
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
        BlendingSpace::new(2, corners.into()).expect("sixteen samples is a side of two")
    }

    /// A grid of side three, sampled from the assumed inks, interpolates back to them.
    ///
    /// The check that generalising the sixteen corners to a grid did not change what the grid
    /// *means*: sampling the multilinear cube at every third of the way along each axis and
    /// then interpolating that finer grid reproduces the cube, because a multilinear function
    /// is reproduced exactly by multilinear interpolation of its own samples. A press whose
    /// profile is not multilinear is why the finer grid exists at all (ADR 0272).
    #[test]
    fn a_finer_grid_sampled_from_the_cube_is_the_cube() {
        let cube = process_inks();
        let side = 3usize;
        let mut grid = Vec::with_capacity(side.pow(4));
        for black in 0..side {
            for yellow in 0..side {
                for magenta in 0..side {
                    for cyan in 0..side {
                        #[expect(clippy::cast_precision_loss, reason = "a grid index below three")]
                        let at = |index: usize| index as f32 / 2.0;
                        grid.push(cube.convert(at(cyan), at(magenta), at(yellow), at(black)));
                    }
                }
            }
        }
        let finer = BlendingSpace::new(side, grid.into()).expect("81 samples is a side of three");
        for step in 0..=10 {
            #[expect(clippy::cast_precision_loss, reason = "a step below eleven")]
            let value = step as f32 / 10.0;
            let coarse = cube.convert(value, 1.0 - value, value * 0.5, 0.25);
            let fine = finer.convert(value, 1.0 - value, value * 0.5, 0.25);
            for (left, right) in coarse.iter().zip(fine) {
                assert!(
                    (left - right).abs() < 1e-5,
                    "a multilinear cube resampled onto a finer grid is the same function: \
                     {coarse:?} against {fine:?}"
                );
            }
        }
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
