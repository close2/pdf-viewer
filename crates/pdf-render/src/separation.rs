//! A page drawn as a press would print it, with a plane per spot ink (ISO 32000-2 §10.8.3).
//!
//! §10.8.3 lets a processor imaging on a device that makes no separations simulate one that does,
//! and states the simulation as four steps:
//!
//! > - a) Process the PDF as if separations were to be created for a simulated device that
//! >   supports subtractive process colourants and possibly spot colours. …
//! > - b) Convert each separation into "flat XYZ" (no gamma) and using a background matte of all
//! >   white.
//! > - c) Blend the resulting separations into a single result using a multiply blend (see
//! >   "Table 133 -Variables used in the basic compositing formula").
//! > - d) Convert the result to the actual device colour space and output it.
//!
//! Step a) is the interpreter's: `pdf-model` interprets the page once per plane of the simulated
//! device and hands the planes over. The process colourants are the pair a page composited in
//! four components already travels as ([`crate::blending`]); the spot colourants are
//! [`SpotSeparation::planes`], three colourants to a plane because a raster holds three channels
//! and §11.3.4 composites each component on its own. Steps b) to d) are [`resolve`], and every
//! conversion it applies arrives sampled on the list, so a backend never sees a colour space —
//! [`crate::Color`]'s argument, and the reason this module holds tables rather than formulas.
//!
//! # Where the matte goes, and what that does to a pixel's alpha
//!
//! Step b)'s matte is the white every separation is converted *against*, and a separation is a
//! plane of one colourant's tints. So the matte is composited under each plane **in that plane's
//! own components** — a tint of zero, no ink — before the plane is converted, which is what a press
//! does: the paper is under every ink. On a plane stored as §11.3.4's additive complement,
//! premultiplied by the page's one alpha (§11.7.3: "Only a single shape value and opacity value
//! shall be maintained at each point in the computed group results; they shall apply to both
//! process and spot colour components"), a stored value `v` at alpha `α` composited over a matte
//! of additive 1.0 is `v + (1 − α)`, so the tint the press prints there is `α − v`. The result
//! of step d) is then a colour on the matte, which is opaque: [`resolve`] writes an alpha of one
//! wherever the page painted, and leaves a pixel it did not paint for the medium, which is the
//! same white nominally (§11.4.7's `W`, [`crate::medium`]).
//!
//! # The multiply, and its unit
//!
//! Step c)'s blend is Table 136's `B(cb, cs) = cb × cs`, whose NOTE 3 fixes the unit: "multiplying
//! any colour with black produces black while multiplying with white leaves the original colour
//! unchanged". The matte's white is therefore the 1.0 of the product, and each separation enters
//! it as its flat XYZ divided by that white — which is what every table here already holds, so
//! the multiply below is a componentwise product and nothing else. `pdf_colour`'s
//! `ColourSpace::simulated_xyz` is the same arithmetic over the colourants one painting operation
//! states, and ADR 1229 is where it was argued; ADR 1317 is this module's.

use std::sync::Arc;

use crate::blending::{BlendingSpace, ColourCube};
use crate::display_list::DisplayList;

/// How many colourants one spot plane carries: a raster's three colour channels.
pub const COLOURANTS_PER_PLANE: usize = 3;

/// One spot colourant of the simulated device: its name, and step b)'s conversion of its tints.
#[derive(Debug, Clone, PartialEq)]
pub struct SpotColourant {
    /// The colourant's name, as the bytes the document wrote (§7.3.5), for a sentence naming it.
    name: Arc<[u8]>,
    /// At least two samples of step b)'s flat XYZ, each divided by the matte's white:
    /// `flat[i]` is the separation at a tint of `i ÷ (len − 1)`.
    flat: Arc<[[f32; 3]]>,
}

impl SpotColourant {
    /// A colourant from its name and its flat-XYZ samples, or `None` for fewer than two samples,
    /// which is not a curve.
    #[must_use]
    pub fn new(name: Arc<[u8]>, flat: Arc<[[f32; 3]]>) -> Option<Self> {
        (flat.len() >= 2).then_some(Self { name, flat })
    }

    /// The colourant's name.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// The flat-XYZ samples, evenly spaced over the tint's `0.0..=1.0`.
    #[must_use]
    pub fn flat(&self) -> &[[f32; 3]] {
        &self.flat
    }

    /// Step b)'s flat XYZ of one tint in `0.0..=1.0`, relative to the matte's white.
    #[must_use]
    pub fn flat_at(&self, tint: f32) -> [f32; 3] {
        let last = self.flat.len().saturating_sub(1);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a sample count far below f32's exact range"
        )]
        let scaled = tint.clamp(0.0, 1.0) * last as f32;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "`scaled` is in 0..=last, so its floor is a valid index"
        )]
        let cell = (scaled as usize).min(last.saturating_sub(1));
        #[expect(
            clippy::cast_precision_loss,
            reason = "a cell index below the sample count"
        )]
        let fraction = scaled - cell as f32;
        match (self.flat.get(cell), self.flat.get(cell.saturating_add(1))) {
            (Some(low), Some(high)) => {
                std::array::from_fn(|axis| low[axis].mul_add(1.0 - fraction, high[axis] * fraction))
            }
            // Unreachable by construction — `new` requires two samples — and the matte's own
            // white, which leaves the product alone, is the honest answer if it ever were.
            _ => [1.0; 3],
        }
    }
}

/// The spot half of §10.8.3's simulated device: the spot planes, each colourant's step b), and
/// the two conversions steps b) and d) apply around the multiply.
///
/// Carried by [`DisplayList::set_separated`] beside the process pair — [`DisplayList::blending`]
/// and [`DisplayList::black`] — which is the process separation, and only on a page that names a
/// spot colourant under a reader's request for the simulation. See the module documentation for
/// what a backend does with it.
#[derive(Debug, Clone, PartialEq)]
pub struct SpotSeparation {
    /// The spot colourants in plane order: colourant `3 × n + c` is channel `c` of plane `n`.
    colourants: Vec<SpotColourant>,
    /// The spot planes, each the whole page drawn in three colourants' additive complements.
    planes: Vec<DisplayList>,
    /// Step b) for the process separation: a device colour — what the process pair's
    /// [`BlendingSpace`] gives — to its flat XYZ relative to the matte's white.
    process_to_flat: ColourCube,
    /// Step d): the multiplied flat XYZ, relative to the matte's white, to a device colour.
    flat_to_device: ColourCube,
}

impl SpotSeparation {
    /// A separation from its parts, or `None` where they are not one: no colourant, or a plane
    /// count other than [`COLOURANTS_PER_PLANE`] colourants to a plane.
    #[must_use]
    pub fn new(
        colourants: Vec<SpotColourant>,
        planes: Vec<DisplayList>,
        process_to_flat: ColourCube,
        flat_to_device: ColourCube,
    ) -> Option<Self> {
        let wanted = colourants.len().div_ceil(COLOURANTS_PER_PLANE);
        (!colourants.is_empty() && planes.len() == wanted).then_some(Self {
            colourants,
            planes,
            process_to_flat,
            flat_to_device,
        })
    }

    /// The spot colourants, in plane order.
    #[must_use]
    pub fn colourants(&self) -> &[SpotColourant] {
        &self.colourants
    }

    /// The spot planes, in order.
    #[must_use]
    pub fn planes(&self) -> &[DisplayList] {
        &self.planes
    }

    /// Step b)'s conversion of the process separation, from a device colour.
    #[must_use]
    pub fn process_to_flat(&self) -> &ColourCube {
        &self.process_to_flat
    }

    /// Step d)'s conversion of the multiplied result to a device colour.
    #[must_use]
    pub fn flat_to_device(&self) -> &ColourCube {
        &self.flat_to_device
    }
}

/// Steps b) to d) over a separated page's rasters, overwriting `chromatic` with the result.
///
/// `chromatic` and `black` are the process pair [`crate::blending::resolve`] resolves, and `spots`
/// the spot planes in [`SpotSeparation::planes`]' order, every raster premultiplied RGBA8 of one
/// size. Per pixel the page painted:
///
/// - **step b)**, each separation composited over the white matte in its own components — the tint
///   `α − v` of the module documentation — and converted to flat XYZ relative to the matte's
///   white: the process colourants together through `space` and
///   [`SpotSeparation::process_to_flat`], because they share one alternate and combine inside it
///   (ADR 1229 section 2), and each spot colourant through its own [`SpotColourant::flat_at`];
/// - **step c)**, the componentwise product of those ratios;
/// - **step d)**, the product through [`SpotSeparation::flat_to_device`], written opaque.
///
/// A pixel nothing painted is left alone for the medium. A raster shorter than `chromatic` is read
/// as unpainted where it ends, which the interpreter's geometry check makes unreachable.
pub fn resolve(
    chromatic: &mut [u8],
    black: &[u8],
    spots: &[&[u8]],
    space: &BlendingSpace,
    separation: &SpotSeparation,
) {
    for (index, pixel) in chromatic.chunks_exact_mut(4).enumerate() {
        if pixel[3] == 0 {
            continue;
        }
        let at = index.saturating_mul(4);
        // The tint a press prints at this pixel for one channel of one plane: the stored
        // additive complement `v`, premultiplied by `α`, composited over the matte's 1.0.
        let tint = |plane: &[u8], channel: usize| -> f32 {
            let alpha = plane.get(at.saturating_add(3)).copied().unwrap_or(0);
            let value = plane.get(at.saturating_add(channel)).copied().unwrap_or(0);
            f32::from(alpha.saturating_sub(value)) / 255.0
        };
        let process = [
            f32::from(pixel[3].saturating_sub(pixel[0])) / 255.0,
            f32::from(pixel[3].saturating_sub(pixel[1])) / 255.0,
            f32::from(pixel[3].saturating_sub(pixel[2])) / 255.0,
            tint(black, 0),
        ];
        let device = space.convert(process[0], process[1], process[2], process[3]);
        let mut product = separation.process_to_flat.convert(device);
        for (colourant, spot) in separation.colourants.iter().enumerate() {
            let plane = spots
                .get(colourant / COLOURANTS_PER_PLANE)
                .copied()
                .unwrap_or_default();
            let flat = spot.flat_at(tint(plane, colourant % COLOURANTS_PER_PLANE));
            for (value, factor) in product.iter_mut().zip(flat) {
                *value *= factor;
            }
        }
        let result = separation.flat_to_device.convert(product);
        for (channel, value) in pixel.iter_mut().zip(result) {
            let scaled = value.clamp(0.0, 1.0).mul_add(255.0, 0.5);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a value in 0..=1 scaled by 255 is in 0..=255"
            )]
            {
                *channel = scaled as u8;
            }
        }
        pixel[3] = u8::MAX;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cube that hands its three components back unchanged: identity curves either side of an
    /// identity grid.
    fn identity() -> ColourCube {
        let corners: Vec<[f32; 3]> = (0..8usize)
            .map(|corner| {
                std::array::from_fn(|axis| if corner >> axis & 1 == 1 { 1.0 } else { 0.0 })
            })
            .collect();
        ColourCube::new(
            Arc::from(vec![[0.0; 3], [1.0; 3]]),
            2,
            Arc::from(corners),
            Arc::from(vec![0.0, 1.0]),
        )
        .expect("two samples a curve and eight corners")
    }

    /// A press whose device colour is the additive complement of the chromatic components times
    /// that of black: the identity of the pair, so only the multiply is under test.
    fn press() -> BlendingSpace {
        let corners: Vec<[f32; 3]> = (0..16usize)
            .map(|corner| {
                let black = if corner >> 3 & 1 == 1 { 0.0 } else { 1.0 };
                std::array::from_fn(|axis| if corner >> axis & 1 == 1 { 0.0 } else { black })
            })
            .collect();
        BlendingSpace::new(2, Arc::from(corners)).expect("sixteen corners")
    }

    fn separation(flat: [[f32; 3]; 2]) -> SpotSeparation {
        let colourant = SpotColourant::new(Arc::from(b"Spot".as_slice()), Arc::from(flat.to_vec()))
            .expect("two samples");
        SpotSeparation::new(
            vec![colourant],
            vec![DisplayList::new(crate::Size::new(1.0, 1.0))],
            identity(),
            identity(),
        )
        .expect("one colourant, one plane")
    }

    /// Table 136's NOTE 3, "multiplying with white leaves the original colour unchanged": a spot
    /// colourant at no tint is the matte's white, and the process colour comes through alone.
    #[test]
    fn a_spot_at_no_tint_leaves_the_process_colour() {
        let separated = separation([[1.0; 3], [0.5, 0.25, 0.0]]);
        // Cyan at full tint, opaque: additive complement 0 in the first channel.
        let mut chromatic = [0, 255, 255, 255];
        let black = [255, 255, 255, 255];
        let spot = [255, 255, 255, 255];
        resolve(&mut chromatic, &black, &[&spot], &press(), &separated);
        assert_eq!(chromatic, [0, 255, 255, 255]);
    }

    /// The multiply itself: the spot at full tint scales each process component by its ratio.
    #[test]
    fn a_full_spot_multiplies_the_process_colour() {
        let separated = separation([[1.0; 3], [0.5, 0.25, 0.0]]);
        let mut chromatic = [255, 255, 255, 255];
        let black = [255, 255, 255, 255];
        let spot = [0, 255, 255, 255];
        resolve(&mut chromatic, &black, &[&spot], &press(), &separated);
        // 1.0 × 0.5, 1.0 × 0.25 and 1.0 × 0.0, to eight bits.
        assert_eq!(chromatic, [128, 64, 0, 255]);
    }

    /// The matte under a half-covered pixel: a full tint at half alpha prints half a tint, and the
    /// result is opaque on the matte.
    #[test]
    fn a_half_covered_pixel_prints_on_the_matte() {
        let separated = separation([[1.0; 3], [0.0, 1.0, 1.0]]);
        // Premultiplied at α = 128: nothing on the process planes is 128 in every channel.
        let mut chromatic = [128, 128, 128, 128];
        let black = [128, 128, 128, 128];
        let spot = [0, 128, 128, 128];
        resolve(&mut chromatic, &black, &[&spot], &press(), &separated);
        // The tint is 128 ÷ 255, so the ratio is 1 − 128/255 and the channel is 127.
        assert_eq!(chromatic, [127, 255, 255, 255]);
    }

    /// A pixel the page did not paint is the medium's.
    #[test]
    fn an_unpainted_pixel_is_left_alone() {
        let separated = separation([[1.0; 3], [0.0; 3]]);
        let mut chromatic = [0, 0, 0, 0];
        resolve(&mut chromatic, &[0; 4], &[&[0; 4]], &press(), &separated);
        assert_eq!(chromatic, [0, 0, 0, 0]);
    }

    /// A plane count that does not carry the colourants three to a plane is not a separation.
    #[test]
    fn the_planes_carry_three_colourants_each() {
        let colourant =
            SpotColourant::new(Arc::from(b"A".as_slice()), Arc::from(vec![[1.0; 3]; 2]))
                .expect("two samples");
        let four = vec![colourant; 4];
        let one_plane = vec![DisplayList::new(crate::Size::new(1.0, 1.0))];
        assert!(
            SpotSeparation::new(four.clone(), one_plane.clone(), identity(), identity()).is_none()
        );
        let two_planes = vec![one_plane[0].clone(), one_plane[0].clone()];
        assert!(SpotSeparation::new(four, two_planes, identity(), identity()).is_some());
    }
}
