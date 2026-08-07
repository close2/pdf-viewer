//! Position-dependent opacity, derived from a transparency group (ISO 32000-2 §11.5).
//!
//! A soft mask is the third source of shape and opacity §11.6.4.1 names, beside the
//! object's own and the graphics state's constants, and the only one that varies across the
//! page. §11.5.1:
//!
//! > Such an independent source, called a soft mask , defines values that may vary across
//! > different points on the page.
//!
//! It is defined by drawing a transparency group and taking either its alpha (§11.5.2) or
//! the luminosity of its colour over a chosen backdrop (§11.5.3). That is why this type
//! carries a command list rather than a raster: the group is evaluated at *device*
//! resolution, which the display list deliberately does not know, so a backend rasterises
//! it exactly as it rasterises the page.
//!
//! # One derivation, two backends
//!
//! [`SoftMask::values`] turns rendered pixels into mask values, and both backends call it.
//! That is the same rule as [`crate::Image::area_averaged`]: a decision the CPU oracle and
//! the GPU backend could make differently is a decision that belongs here, once.

use crate::display_list::Command;
use crate::paint::Color;

/// Identifies a soft mask within a [`crate::DisplayList`].
///
/// Referenced rather than carried for the same reason a [`crate::ClipId`] is: a mask set by
/// `gs` applies to every object painted until it is replaced, and its group may hold
/// thousands of commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SoftMaskId(u32);

impl SoftMaskId {
    /// Creates an identifier for the mask at `index`.
    ///
    /// Only [`crate::DisplayList::add_soft_mask`] should mint one; it is public so that a
    /// backend's tests can name a mask they registered themselves.
    #[must_use]
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the index this identifier refers to.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Which of §11.5's two derivations turns a group into mask values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoftMaskKind {
    /// §11.5.2: the group's alpha, its colour ignored. §11.6.5.1 says it of the key:
    ///
    /// > If the subtype is Alpha , the transparency group XObject G shall be evaluated to
    /// > compute a group alpha only. The colours of the constituent objects shall be ignored
    /// > and the colour compositing computations may be omitted.
    Alpha,
    /// §11.5.3: the luminosity of the group composited onto an opaque backdrop.
    ///
    /// > The second method of deriving a soft mask from a transparency group shall begin by
    /// > compositing the group with a fully opaque backdrop of a specified colour. The mask
    /// > value at any given point shall then be defined to be the luminosity of the
    /// > resulting colour.
    Luminosity {
        /// §11.6.5.1's `/BC`, resolved into whatever the group's elements are painted in.
        ///
        /// > Outside the transparency group's bounding box, the mask value shall be derived
        /// > by transforming the BC colour to luminosity and applying the transfer function
        /// > to the result.
        ///
        /// Ordinarily that is the backdrop's own colour, in the device's three components,
        /// which is what a group composited in `DeviceRGB` or in a CIE-based space needs.
        /// **Where the group's blending colour space is subtractive it is a grey**, because
        /// `pdf_model` paints such a group in §10.4.2.3's grey rather than in colour: a
        /// backdrop and the elements composited onto it have to be the same quantity or the
        /// compositing is not the clause's, and this is the field where they meet.
        ///
        /// The default is "the colour space's initial value, representing black", which is
        /// what makes the area outside a mask group's own marks mask everything away.
        backdrop: Color,
    },
}

/// §11.6.5.1's `/TR`, sampled onto the 256 values an eight-bit mask can hold.
///
/// > A function object (see 7.10, "Functions") specifying the transfer function that shall
/// > be used in deriving the mask values. The function shall accept one input, the computed
/// > group alpha or luminosity (depending on the value of the subtype S ), and shall return
/// > one output, the resulting mask value.
///
/// A table rather than the function itself for the same reason [`crate::Ramp`] is one: a
/// backend has no business evaluating a PDF function, and colour and opacity are resolved
/// upstream so the two backends cannot disagree about them. The sampling is *exact* rather
/// than an approximation, because a mask value here is one byte and the table holds every
/// byte the function can be asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    table: [u8; 256],
}

impl Transfer {
    /// Builds a table from a function already evaluated at every eight-bit input.
    ///
    /// `outputs[i]` is the function's value at `i / 255`, clamped to `0..=255` — §11.6.5.1
    /// requires exactly that clamp: "if it falls outside this range, it shall be forced to
    /// the nearest valid value".
    #[must_use]
    pub fn from_samples(table: [u8; 256]) -> Self {
        Self { table }
    }

    /// Returns the mask value for a computed alpha or luminosity.
    #[must_use]
    pub fn apply(&self, value: u8) -> u8 {
        // Indexing a 256-entry table by a `u8` cannot be out of bounds; `get` keeps that
        // fact from resting on the reader's memory rather than on the compiler's.
        self.table.get(value as usize).copied().unwrap_or(value)
    }

    /// Whether this table is the identity, which is what `/Identity` and an absent `/TR`
    /// mean.
    ///
    /// Asked by a backend that can express a mask natively but not an arbitrary curve.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.table
            .iter()
            .enumerate()
            .all(|(index, &value)| usize::from(value) == index)
    }
}

/// A transparency group evaluated for its opacity rather than its colour (§11.5).
#[derive(Debug, Clone, PartialEq)]
pub struct SoftMask {
    /// The mask group's elements, in painting order.
    ///
    /// Positioned by "the transformation matrix specified by the Matrix entry in the
    /// transparency group's form dictionary … with the current transformation matrix at the
    /// moment the soft mask is established in the graphics state with the gs operator"
    /// (§11.6.5.1) — resolved into each command's own transform, as everywhere else here.
    ///
    /// [`crate::ClipId`]s inside refer to the enclosing [`crate::DisplayList`], for the same
    /// reason a [`Command::Group`]'s do.
    pub commands: Vec<Command>,
    /// Which derivation produces the mask values.
    pub kind: SoftMaskKind,
    /// §11.6.5.1's `/TR`, or `None` for the identity it defaults to.
    pub transfer: Option<Transfer>,
}

impl SoftMask {
    /// Converts one rendered pixel's straight-alpha RGBA into a mask value.
    ///
    /// The pixels are the mask group drawn onto a *transparent* backdrop, which is what
    /// both §11.5.2 and §11.5.3 ask for: the first factors the backdrop out, and the second
    /// composites onto the chosen `/BC` here, in one place, rather than filling a buffer
    /// with it and letting a blend mode inside the group see it. §11.4.7's page group has
    /// the same shape and `crate::impose_on_medium` is its counterpart.
    #[must_use]
    pub fn value(&self, pixel: [u8; 4]) -> u8 {
        let derived = match self.kind {
            // §11.5.2: "The mask value at each point shall then be derived from the alpha of
            // the group." Outside the group's bounding box the alpha is 0, which is the
            // clause's "the result of applying the transfer function to the input value 0.0"
            // and needs no separate case.
            SoftMaskKind::Alpha => pixel[3],
            // §11.5.3, for a device colour space: "convert the colour to DeviceGray by
            // implementation-defined means and use the resulting gray value as the
            // luminosity, with no compensation for gamma or other colour calibration", for
            // which EXAMPLE 2 gives Y = 0.30 R + 0.59 G + 0.11 B — [`Color::grey_level`],
            // §10.4.2.2's own formula.
            //
            // That is the whole of the derivation *for a group composited in the device's
            // three components*. A group whose blending space is subtractive is painted in a
            // grey by `pdf_model` instead, and this arithmetic then reads that grey back
            // unchanged — the three coefficients sum to 1.0 — leaving what the grey *means*
            // to [`Transfer`], which is where the second half of §10.4.2.3 lives.
            SoftMaskKind::Luminosity { backdrop } => {
                let alpha = f32::from(pixel[3]) / 255.0;
                // Source-over onto an opaque backdrop, in straight alpha: the result is
                // opaque, so its colour is the ordinary interpolation between the two.
                let over = |channel: u8, backdrop: f32| {
                    (f32::from(channel) / 255.0).mul_add(alpha, backdrop * (1.0 - alpha))
                };
                let composited = Color::rgb(
                    over(pixel[0], backdrop.r),
                    over(pixel[1], backdrop.g),
                    over(pixel[2], backdrop.b),
                );
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clamped to 0..=255 on the line above the cast"
                )]
                {
                    (composited.grey_level() * 255.0).round().clamp(0.0, 255.0) as u8
                }
            }
        };
        self.transfer
            .as_ref()
            .map_or(derived, |transfer| transfer.apply(derived))
    }

    /// Converts a rendered raster of the mask group into one mask value per pixel.
    ///
    /// `pixels` is straight-alpha RGBA8 — [`crate::Raster`]'s documented format, which is
    /// what both backends hand back. A trailing partial pixel is ignored rather than
    /// reported: it cannot arise from a raster either backend produces, and refusing to
    /// build a mask is a blank page where dropping a byte is nothing.
    ///
    /// **§11.6.5.1's outside-the-bounding-box rule needs no case here, and that is worth
    /// saying rather than leaving to be noticed.** Both backends draw the mask group into a
    /// buffer the size of the *whole target*, so a pixel the group's `/BBox` does not reach is
    /// `[0, 0, 0, 0]` and [`Self::value`] gives it the transfer function applied to 0.0 for
    /// `/Alpha` and to the backdrop's luminosity for `/Luminosity` — which is what the clause
    /// asks for, arrived at by the same arithmetic as every other pixel rather than by a second
    /// derivation that could drift from the first. This module carried an `outside_value()`
    /// helper for that rule until the hundred-and-seventy-fifth session and **no backend ever
    /// called it**: it was a path nobody took, which `CLAUDE.md` forbids, and the tests that
    /// used it now state the same thing through `value([0, 0, 0, 0])`.
    #[must_use]
    pub fn values(&self, pixels: &[u8]) -> Vec<u8> {
        pixels
            .chunks_exact(4)
            .map(|pixel| self.value([pixel[0], pixel[1], pixel[2], pixel[3]]))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{SoftMask, SoftMaskKind, Transfer};
    use crate::paint::Color;

    fn mask(kind: SoftMaskKind, transfer: Option<Transfer>) -> SoftMask {
        SoftMask {
            commands: Vec::new(),
            kind,
            transfer,
        }
    }

    /// §11.5.2: the alpha is the mask value, whatever colour carried it.
    #[test]
    fn an_alpha_mask_reads_the_alpha_and_ignores_the_colour() {
        let mask = mask(SoftMaskKind::Alpha, None);
        assert_eq!(mask.value([255, 0, 0, 64]), 64);
        assert_eq!(mask.value([0, 255, 0, 64]), 64);
        assert_eq!(
            mask.value([0, 0, 0, 0]),
            0,
            "§11.6.5.1, outside the group's box"
        );
    }

    /// §11.5.3 EXAMPLE 2's coefficients, which are not any library's luminance.
    ///
    /// Pure green is the case that separates them: 0.59 here, 0.7152 under Rec. 709 and
    /// 0.7154 under the SVG formula Vello's luminance mask uses. A mask built from green
    /// artwork is 21% more transparent under either of those, which is why this test states
    /// a colour rather than a grey.
    #[test]
    fn a_luminosity_mask_uses_the_clauses_own_coefficients() {
        let mask = mask(
            SoftMaskKind::Luminosity {
                backdrop: Color::BLACK,
            },
            None,
        );
        assert_eq!(mask.value([0, 255, 0, 255]), 150, "0.59 x 255 = 150.45");
        assert_eq!(mask.value([255, 0, 0, 255]), 77, "0.30 x 255 = 76.5");
        assert_eq!(mask.value([0, 0, 255, 255]), 28, "0.11 x 255 = 28.05");
        assert_eq!(mask.value([255, 255, 255, 255]), 255);
    }

    /// §11.6.5.1: outside the group's bounding box a luminosity mask takes the backdrop's.
    #[test]
    fn a_luminosity_mask_outside_the_group_is_the_backdrops_luminosity() {
        let white = mask(
            SoftMaskKind::Luminosity {
                backdrop: Color::WHITE,
            },
            None,
        );
        assert_eq!(white.value([0, 0, 0, 0]), 255);
        assert_eq!(
            white.value([0, 0, 0, 128]),
            127,
            "black at 128/255 coverage leaves 127/255 of the backdrop's luminosity"
        );

        let black = mask(
            SoftMaskKind::Luminosity {
                backdrop: Color::BLACK,
            },
            None,
        );
        assert_eq!(
            black.value([0, 0, 0, 0]),
            0,
            "§11.5.3 NOTE 2's usual choice"
        );
    }

    /// The transfer function is applied after the derivation, to both subtypes.
    #[test]
    fn the_transfer_function_maps_the_derived_value() {
        let mut table = [0_u8; 256];
        for (index, entry) in table.iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "255 - index is in 0..=255 for an index of a 256-entry table"
            )]
            {
                *entry = (255 - index) as u8;
            }
        }
        let inverted = Transfer::from_samples(table);
        assert!(!inverted.is_identity());

        let mask = mask(SoftMaskKind::Alpha, Some(inverted));
        assert_eq!(mask.value([0, 0, 0, 0]), 255, "an inverting /TR unmasks");
        assert_eq!(mask.value([0, 0, 0, 255]), 0);
    }

    /// The identity is recognised as such, since a backend may express only that.
    #[test]
    fn an_identity_table_is_identified() {
        let mut table = [0_u8; 256];
        for (index, entry) in table.iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "index of a 256-entry table is in 0..=255"
            )]
            {
                *entry = index as u8;
            }
        }
        assert!(Transfer::from_samples(table).is_identity());
    }

    /// A whole raster is converted pixel by pixel, in order.
    #[test]
    fn a_raster_converts_pixelwise() {
        let mask = mask(SoftMaskKind::Alpha, None);
        let pixels = [0, 0, 0, 10, 0, 0, 0, 200];
        assert_eq!(mask.values(&pixels), vec![10, 200]);
    }
}
