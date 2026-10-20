//! §11.7.2: a colour space's components converted into a four-component press.

use super::device_cmyk::channel;
use super::ink_search::{rgb_to_ink, xyz_to_ink};
use super::press::Press;
use super::{ColourSpace, MAX_DEPTH};
use crate::black_generation::BlackGeneration;
use crate::icc::Rendering;

impl ColourSpace {
    /// Converts a colour in this space to the four components of `DeviceCMYK`.
    ///
    /// §11.7.2 is what asks for this: "[i]f the colour space of a graphics object within the
    /// group is not equivalent to the group's blending colour space, then it shall be
    /// converted to the group's colour space, and all blending and compositing computations
    /// shall be done in that space".
    ///
    /// **Which conversion it is follows the branch this tree is already on**, and §11.7.5.3 is
    /// the clause that says so:
    ///
    /// > The rendering intent influences the conversion from a CIE-based colour space to a
    /// > target colour space, taking into account the target space's colour gamut (the range
    /// > of colours it can reproduce). Whereas in the opaque imaging model the target space
    /// > shall always be the native colour space of the output device, in the transparent
    /// > model it may instead be the group colour space of a transparency group into which an
    /// > object is being painted.
    ///
    /// So the conversion into a group's space is the *same* conversion as the one onto the
    /// device, with a different target — which puts it on §10.3's branch, where §10.4.2.1
    /// ranks it and where ADRs 0009 and 0042 put this tree's conversion out. [`rgb_to_ink`]
    /// is that conversion: the right inverse of [`super::device_cmyk::cmyk`], so an opaque mark painted into a
    /// page composited in ink comes back the colour the file states. ADR 0263.
    ///
    /// §11.7.5.3's other two bullets are §10.4.2's branch and are not in force here — they say
    /// *which* black-generation and undercolour-removal functions §10.4.2.4 uses, and a
    /// document that states its own keeps `pdf_model::content`'s report rather than being drawn
    /// with them ignored.
    ///
    /// A space that already resolves to `DeviceCMYK` is passed straight through, including a
    /// `Separation` or `DeviceN` whose alternate is one — §11.3.4 requires exactly that:
    /// spot colours "shall not be converted to a blending colour space (except in the case
    /// where they first revert to their alternate colour space)". §11.7.2 is what makes that
    /// right where the press is the document's rather than this tree's:
    ///
    /// > If an isolated transparency group or page has an ICCBased 'CMYK' colour space ,
    /// > DeviceCMYK shall be redefined within the transparency group to be the same as the
    /// > blending colour space and references to the process colourants Cyan , Magenta ,
    /// > Yellow and Black are defined to be references to the corresponding colourants in the
    /// > blending colour space, even where the actual or simulated output device is not CMYK.
    ///
    /// **A colour already in the press's own space is passed through too**, which is
    /// §8.6.5.7's implicit conversion, and that clause gives the reason it exists:
    ///
    /// > This avoids any unwanted computational error and in the case of 4 component colour
    /// > spaces avoids the conversion from 4 components to 3 and back to 4, a process that
    /// > loses critical colour information.
    ///
    /// [`crate::icc::Profile::identity`] is what recognises it.
    ///
    /// Everything else depends on which press it is. **A press sampled from a bi-directional
    /// profile converts in through the profile's own `B2A`** (ADR 0796) — §8.6.5.5's "from
    /// CIE" information, which that clause requires of a blending-space profile precisely
    /// because "the group colour space shall be used as both the destination for objects being
    /// painted within the group and the source for the group's results" — and a colour reaches
    /// it as an XYZ: a CIE-based colour's own ([`Self::cie_xyz_at`], §10.3.1's conversion
    /// between two CIE-based spaces with no screen in the way) and a device colour's through
    /// the sRGB this processor takes device colours to be (§10.3.2, [`super::cie::srgb_to_xyz_d50`]).
    /// **Every other press** — the assumed inks, and a profile without the table — takes this
    /// crate's one route to RGB and then [`rgb_to_ink`], so a `Lab` or a *different*
    /// `ICCBased` colour reaches ink through sRGB: colorimetric where sRGB holds it, and
    /// clipped where it does not, which is the same gamut question one space earlier.
    #[must_use]
    pub fn to_cmyk(&self, values: &[f32], rendering: Rendering, press: &Press) -> [f32; 4] {
        self.to_cmyk_at(values, 0, rendering, press, None)
    }

    /// [`Self::to_cmyk`] under §10.4.2.4's black generation, where the graphics state states one.
    ///
    /// **This is the one place §10.4.2's branch converts a colour this tree paints**, and it is
    /// there because §11.7.5.3 puts it there rather than because the branch was reconsidered:
    ///
    /// > When painting an elementary object with a DeviceRGB colour directly into a transparency
    /// > group whose colour space is DeviceCMYK , the functions used shall be the current
    /// > black-generation and undercolour-removal functions in effect in the graphics state at
    /// > the time of the painting operation.
    ///
    /// §10.4.2.4 is the only algorithm in the standard that invokes those two functions, and its
    /// title is the conversion this sentence is about. What a stated function replaces is a step
    /// the route above already performs: [`rgb_to_ink`]'s search chooses a black amount and
    /// solves the other three for the colour, which is a black generation and an undercolour
    /// removal, and §10.4.2.4 requires every device to have a pair — "[e]ach device shall be
    /// configured with default values that are appropriate for that device". So the file's pair
    /// substitutes for this device's, over the colours the sentence names and no others.
    ///
    /// Everything else stays on §10.3's branch, which is what §10.4.2.1's ranking recommends and
    /// what nothing here overrides: a `CalRGB`, a `Lab`, an `ICCBased` or a `DeviceGray` colour
    /// is not the "DeviceRGB colour" the sentence conditions on, and a press that converts in
    /// through its own `B2A` is the document's own measured transform rather than a default this
    /// pair could stand in for. ADR 1207, which supersedes ADR 1069.
    #[expect(
        clippy::doc_markdown,
        reason = "\"DeviceRGB colour\" is quoted from §11.7.5.3, and a quotation may not gain \
                  backticks"
    )]
    #[must_use]
    pub fn to_cmyk_under(
        &self,
        values: &[f32],
        rendering: Rendering,
        press: &Press,
        generation: Option<&BlackGeneration>,
    ) -> [f32; 4] {
        self.to_cmyk_at(values, 0, rendering, press, generation)
    }

    /// [`Self::to_cmyk`], carrying the recursion depth a nested space costs.
    fn to_cmyk_at(
        &self,
        values: &[f32],
        depth: usize,
        rendering: Rendering,
        press: &Press,
        generation: Option<&BlackGeneration>,
    ) -> [f32; 4] {
        if depth > MAX_DEPTH {
            return [0.0, 0.0, 0.0, 1.0];
        }
        let at = |index: usize| channel(values.get(index).copied().unwrap_or(0.0));
        match self {
            Self::Cmyk => [at(0), at(1), at(2), at(3)],
            Self::Icc { profile }
                if press.is_of_profile(profile.identity()) && profile.channels() == 4 =>
            {
                [at(0), at(1), at(2), at(3)]
            }
            Self::Indexed { base, .. } => base.to_cmyk_at(
                &self.entry_of(values),
                depth.saturating_add(1),
                rendering,
                press,
                generation,
            ),
            Self::Separation {
                alternate, tints, ..
            } => alternate.to_cmyk_at(
                &tints.eval(values),
                depth.saturating_add(1),
                rendering,
                press,
                generation,
            ),
            Self::Pattern { base } => base.as_ref().map_or([0.0, 0.0, 0.0, 1.0], |base| {
                base.to_cmyk_at(
                    values,
                    depth.saturating_add(1),
                    rendering,
                    press,
                    generation,
                )
            }),
            // §11.7.5.3's own condition, and it is the leaf rather than the operand that has
            // to meet it: an `Indexed` entry, a `Separation`'s alternate or a `/Default` space
            // arrives here as the `DeviceRGB` colour the clause names, and a `CalRGB` or an
            // `ICCBased` one never does. A press that converts in through its own `B2A` is not
            // a default this pair stands in for, so it keeps the profile's table.
            Self::Rgb if !press.converts_in_by_profile() => generation.map_or_else(
                || rgb_to_ink(press, self.to_rgb_at(values, depth, rendering)),
                |generation| generation.separate(at(0), at(1), at(2)),
            ),
            // A CIE-based colour goes into a profile's press from its own XYZ (§10.3.1), and
            // every other colour, and every press without a `B2A`, through this crate's one
            // RGB route — `xyz_to_ink` and `rgb_to_ink` say which is which.
            _ => match self.cie_xyz_at(values, depth, rendering) {
                Some(xyz) if press.converts_in_by_profile() => xyz_to_ink(press, xyz),
                _ => rgb_to_ink(press, self.to_rgb_at(values, depth, rendering)),
            },
        }
    }
}
