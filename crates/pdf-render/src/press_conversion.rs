//! A page group of one or three components carried onto §10.8.3's simulated press, so that the
//! press's own transfer functions meet it (ISO 32000-2 §10.5, §11.7.4.2).
//!
//! §10.8.3 a) processes the page "as if separations were to be created for a simulated device that
//! supports subtractive process colourants and possibly spot colours", and §10.5 says where that
//! device's functions operate:
//!
//! > Transfer functions shall always operate in the native colour space of the output device,
//! > regardless of the colour space in which colours were originally specified.
//!
//! A page whose group composites in one or three components does not composite in that space, and
//! §11.7.4.2 says when it reaches it: "the blending computations shall affect the process colour
//! ants only after the group's results have been converted to the device colour space". So the
//! page's composited result — the list's own components, before the curve or the cube that would
//! take it to the screen — is converted to the press's cyan, magenta, yellow and black at every
//! painted pixel ([`split`] through an [`InkCube`]), and from there it is exactly the pair a page
//! compositing in the press is drawn as: the press's four functions go on the planes
//! (`crate::resolve_press_transfers`), and the pair leaves through the press's own conversion
//! ([`PressConversion::space`], `crate::blending::resolve` or `crate::separation::resolve`).
//!
//! Every painted pixel takes the round trip, not only the ones a function changes, or the page
//! would change colour where a function is the identity: what such a page looks like on the
//! simulated press is the press's picture of it, whose gamut is the press's. The cube is sampled
//! by the interpreter from the conversion a colour in the group's space takes onto the press, so
//! a backend never sees a colour space ([`crate::Color`]'s argument). ADR 3857, which closes ADR
//! 3805's departure.

use rayon::iter::{IndexedParallelIterator as _, ParallelIterator as _};
use rayon::slice::ParallelSliceMut as _;

use crate::blending::BlendingSpace;

/// The conversion from a group's one or three components to a press's four inks, sampled on a
/// regular grid and interpolated multilinearly between the samples.
///
/// The inks are subtractive — 0 is no ink and 1 is full ink — in the order cyan, magenta, yellow,
/// black, which is Table 57's order of a CMYK device's functions.
///
/// The grid is evenly spaced in the components themselves, as the raster holds them, and not in
/// linear light: a press's conversion in is closer to linear in an encoded component than in
/// light, and a `CalRGB` page group with sRGB's primaries came back through a grid of side 17
/// within 3 levels of 255 on its neutral ramp this way and within 17 over its own conversion
/// out's input curves (ADR 3857).
#[derive(Debug, Clone, PartialEq)]
pub struct InkCube {
    /// How many of a pixel's channels are the group's components: 1, read from the first channel
    /// because a list compositing in one component paints it in all three
    /// (`crate::blending::resolve_grey`), or 3.
    components: usize,
    /// How many samples the grid holds along each axis; at least two.
    side: usize,
    /// `sideⁿ` inks for `n` [`InkCube::components`]. The first component runs fastest, so index 0
    /// is every component at 0 and the last is every component at 1.
    grid: std::sync::Arc<[[f32; 4]]>,
}

impl InkCube {
    /// A cube from its parts, or `None` where they are not one: a component count other than one
    /// or three, fewer than two samples per axis, or a grid other than `sideⁿ` samples.
    #[must_use]
    pub fn new(components: usize, side: usize, grid: std::sync::Arc<[[f32; 4]]>) -> Option<Self> {
        let exponent = u32::try_from(components).ok()?;
        let wanted = side.checked_pow(exponent)?;
        (matches!(components, 1 | 3) && side >= 2 && grid.len() == wanted).then_some(Self {
            components,
            side,
            grid,
        })
    }

    /// How many of a pixel's channels are the group's components: 1 or 3.
    #[must_use]
    pub fn components(&self) -> usize {
        self.components
    }

    /// How many samples the grid holds along each axis.
    #[must_use]
    pub fn side(&self) -> usize {
        self.side
    }

    /// The samples, in the index order [`InkCube::new`] documents.
    #[must_use]
    pub fn grid(&self) -> &[[f32; 4]] {
        &self.grid
    }

    /// The inks of one pixel's components, each in `0.0..=1.0`; a one-component cube reads the
    /// first.
    #[must_use]
    pub fn convert(&self, components: [f32; 3]) -> [f32; 4] {
        let last = self.side.saturating_sub(1);
        // The cell below `value` on one axis and how far into it `value` lies.
        let axis = |value: f32| -> (usize, f32) {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a grid side far below f32's exact range"
            )]
            let scaled = value.clamp(0.0, 1.0) * last as f32;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a non-negative value below the grid side, floored"
            )]
            let cell = (scaled.floor() as usize).min(last.saturating_sub(1));
            #[expect(
                clippy::cast_precision_loss,
                reason = "a cell index below the grid side"
            )]
            let fraction = scaled - cell as f32;
            (cell, fraction)
        };
        let sample = |index: usize| self.grid.get(index).copied().unwrap_or([0.0; 4]);
        let mut inks = [0.0f32; 4];
        if self.components == 1 {
            let (cell, fraction) = axis(components[0]);
            let (low, high) = (sample(cell), sample(cell.saturating_add(1)));
            for ((ink, low), high) in inks.iter_mut().zip(low).zip(high) {
                *ink = (high - low).mul_add(fraction, low);
            }
            return inks;
        }
        let cells = components.map(axis);
        let side = self.side;
        // The eight corners of the cell, each weighted by the product of its three fractions.
        for corner in 0..8usize {
            let mut index = 0usize;
            let mut weight = 1.0f32;
            let mut stride = 1usize;
            for (bit, &(cell, fraction)) in cells.iter().enumerate() {
                let upper = corner >> bit & 1 == 1;
                index = index.saturating_add(
                    cell.saturating_add(usize::from(upper))
                        .saturating_mul(stride),
                );
                weight *= if upper { fraction } else { 1.0 - fraction };
                stride = stride.saturating_mul(side);
            }
            for (ink, value) in inks.iter_mut().zip(sample(index)) {
                *ink = value.mul_add(weight, *ink);
            }
        }
        inks
    }
}

/// A page group of one or three components on §10.8.3's simulated press: the cube onto the press's
/// inks, and the press's own conversion out of them.
///
/// Carried by [`crate::DisplayList::set_press_conversion`] on a page whose group is not a press,
/// under a reader's request for the simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct PressConversion {
    /// The group's components to the press's four inks.
    inks: InkCube,
    /// The press's four inks to a device colour — the same table a page compositing in that press
    /// carries in [`crate::DisplayList::blending`].
    space: BlendingSpace,
}

impl PressConversion {
    /// The conversion onto a press through `inks` and out of it through `space`.
    #[must_use]
    pub fn new(inks: InkCube, space: BlendingSpace) -> Self {
        Self { inks, space }
    }

    /// The group's components to the press's four inks.
    #[must_use]
    pub fn inks(&self) -> &InkCube {
        &self.inks
    }

    /// The press's four inks to a device colour.
    #[must_use]
    pub fn space(&self) -> &BlendingSpace {
        &self.space
    }
}

/// The press's pair from a page composited in its group's components: `data`, premultiplied RGBA8
/// holding those components, is overwritten with the additive complements of cyan, magenta and
/// yellow, and the black plane — the complement of black, in all three channels — is returned
/// beside it, both premultiplied by the pixel's own alpha.
///
/// That is the shape `crate::blending::resolve` and `crate::separation::resolve` read and the one
/// `crate::resolve_press_transfers` puts the functions on: §11.7.3 keeps "a single shape value and
/// opacity value" at each point for every component, so both planes carry the page's one alpha.
/// A pixel nothing painted is left alone in both. The error is a level of 255, as in
/// `crate::blending::resolve`: a premultiplied channel resolves a component to `1 ÷ (255 α)`.
///
/// Divided into blocks over the pool where the raster is a page's, for `crate::resolve_cube`'s
/// reason: each pixel's answer is a function of its own four bytes.
#[must_use]
pub fn split(data: &mut [u8], inks: &InkCube) -> Vec<u8> {
    let mut black = vec![0u8; data.len()];
    if data.len() >= SPLIT_PARALLEL_FLOOR {
        data.par_chunks_mut(SPLIT_BLOCK)
            .zip(black.par_chunks_mut(SPLIT_BLOCK))
            .for_each(|(chromatic, black)| split_in(chromatic, black, inks));
    } else {
        split_in(data, &mut black, inks);
    }
    black
}

/// The bytes of raster below which [`split`] stays on the calling thread: four blocks.
const SPLIT_PARALLEL_FLOOR: usize = 4 * SPLIT_BLOCK;

/// The bytes of raster one piece of a divided [`split`] converts: sixteen thousand pixels.
const SPLIT_BLOCK: usize = 4 * 16_384;

/// [`split`] over one piece of a raster, on the calling thread.
///
/// A pixel equal to the one before it takes that pixel's answer, as `crate::resolve_cube`'s
/// pieces do: a page is mostly runs of one colour, and the answer is a function of the four bytes.
fn split_in(chromatic: &mut [u8], black: &mut [u8], inks: &InkCube) {
    let mut previous: Option<([u8; 4], [u8; 4], [u8; 4])> = None;
    let pixels = chromatic.as_chunks_mut::<4>().0.iter_mut();
    for (pixel, ink) in pixels.zip(black.as_chunks_mut::<4>().0.iter_mut()) {
        if pixel[3] == 0 {
            continue;
        }
        let before = *pixel;
        if let Some((converted, chromatic, key)) = previous
            && converted == before
        {
            *pixel = chromatic;
            *ink = key;
            continue;
        }
        let alpha = f32::from(pixel[3]);
        let components = [
            f32::from(pixel[0]) / alpha,
            f32::from(pixel[1]) / alpha,
            f32::from(pixel[2]) / alpha,
        ];
        let [cyan, magenta, yellow, key] = inks.convert(components);
        // The additive complement of an ink, premultiplied: what a press's plane stores.
        let stored = |value: f32| -> u8 {
            let scaled = (1.0 - value.clamp(0.0, 1.0)).mul_add(alpha, 0.5);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a value in 0..=1 scaled by an alpha in 0..=255 is in 0..=255"
            )]
            {
                scaled as u8
            }
        };
        let complement = stored(key);
        *ink = [complement, complement, complement, pixel[3]];
        pixel[0] = stored(cyan);
        pixel[1] = stored(magenta);
        pixel[2] = stored(yellow);
        previous = Some((before, *pixel, *ink));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cube whose inks are the components' complements in cyan, magenta and yellow and no black:
    /// the multilinear interpolation of a multilinear function is that function.
    fn complements() -> InkCube {
        let mut grid = Vec::new();
        for blue in [0.0, 1.0] {
            for green in [0.0, 1.0] {
                for red in [0.0f32, 1.0] {
                    grid.push([1.0 - red, 1.0 - green, 1.0 - blue, 0.0]);
                }
            }
        }
        InkCube::new(3, 2, grid.into()).expect("eight samples is a cube of side two")
    }

    #[test]
    fn a_cube_interpolates_between_its_samples() {
        let inks = complements().convert([0.25, 0.5, 1.0]);
        for (got, want) in inks.iter().zip([0.75, 0.5, 0.0, 0.0]) {
            assert!((got - want).abs() < 1e-6, "{inks:?}");
        }
    }

    #[test]
    fn a_one_component_cube_reads_the_first_channel() {
        let cube = InkCube::new(1, 2, vec![[0.0, 0.0, 0.0, 1.0], [0.0; 4]].into())
            .expect("two samples is a curve");
        let inks = cube.convert([0.25, 0.9, 0.9]);
        assert!((inks[3] - 0.75).abs() < 1e-6, "{inks:?}");
    }

    #[test]
    fn a_cube_of_the_wrong_shape_is_none() {
        assert!(InkCube::new(2, 2, vec![[0.0; 4]; 4].into()).is_none());
        assert!(InkCube::new(3, 2, vec![[0.0; 4]; 7].into()).is_none());
        assert!(InkCube::new(1, 1, vec![[0.0; 4]].into()).is_none());
    }

    /// The pair keeps the page's alpha on both planes, stores complements premultiplied, and leaves
    /// an unpainted pixel alone.
    #[test]
    fn the_split_is_the_presss_pair() {
        let mut data = vec![255, 0, 0, 255, 0, 0, 0, 0, 64, 64, 64, 128];
        let black = split(&mut data, &complements());
        assert_eq!(&data[..4], &[255, 0, 0, 255], "red is magenta and yellow");
        assert_eq!(&black[..4], &[255, 255, 255, 255], "and no black");
        assert_eq!(&data[4..8], &[0, 0, 0, 0], "an unpainted pixel");
        assert_eq!(&black[4..8], &[0, 0, 0, 0]);
        // A half-covered mid grey: components 0.5, so inks 0.5, so complements 0.5 at alpha 128.
        assert_eq!(&data[8..], &[64, 64, 64, 128]);
        assert_eq!(&black[8..], &[128, 128, 128, 128]);
    }
}
