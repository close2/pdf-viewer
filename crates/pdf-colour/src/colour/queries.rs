//! What a parsed space states: its components, ranges, identity, initial colour and ink.

use pdf_syntax::{Dictionary, Document, Object};

use super::cie::nearest;
use super::device_cmyk::channel;
use super::grey_route::GreyIdentity;
use super::rgb_route::RgbIdentity;
use super::{ColourSpace, MAX_DEPTH, Reading};

impl ColourSpace {
    /// Table 88's default `/Decode` pair for one component of an image in this space.
    ///
    /// §8.9.5.2's table gives every family the full range of its components — which is
    /// [`Self::component_range`] — with one exception the table's own NOTE 2 names: an
    /// `Indexed` space's default is `[0 2^n − 1]`, so that "component values that index a
    /// colour table are passed through unchanged" rather than being scaled to 0.0..=1.0.
    #[must_use]
    pub fn default_decode(&self, component: usize, bits: u32) -> (f32, f32) {
        match self {
            Self::Indexed { .. } => {
                let max = (1u32 << bits.min(16)).saturating_sub(1);
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "at most 65535, which f32 represents exactly"
                )]
                let high = max as f32;
                (0.0, high)
            }
            _ => self.component_range(component),
        }
    }

    /// The range one component of a colour in this space may take.
    ///
    /// Every family's components run from 0.0 to 1.0 except three. `Lab`'s lightness is a
    /// percentage and its two chromatic axes take their bounds from the space's own `/Range`
    /// (§8.6.5.4, Table 65). An `Indexed` space's one component is an index, which §8.6.6.3
    /// bounds by `hival`: "if the value is greater than hival, it shall be clipped". An
    /// `ICCBased` space's are the profile's, which §8.6.5.5's prose under Table 67 and its
    /// Table 68 state and Table 65's `/Range` restates — the unit interval for the three
    /// device data colour spaces and something else for `'Lab '`. Table 88 names the same
    /// thing from the other side, as an image's default `/Decode`: "\[s]ame as the value of
    /// Range in the ICC profile of the image's colour space".
    ///
    /// Two callers, both about a *range* rather than a value: `Self::parse_indexed` scales
    /// a colour table's bytes onto the base space's components, and `pdf_model::image` clamps
    /// what a `/Decode` array produces.
    #[must_use]
    pub fn component_range(&self, component: usize) -> (f32, f32) {
        match self {
            Self::Indexed { high, .. } => {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "an index beyond f32's exact integers cannot address a table \
                              this crate built"
                )]
                let top = *high as f32;
                (0.0, top)
            }
            Self::Icc { profile } => profile.component_range(component),
            Self::Lab { range, .. } => match component {
                0 => (0.0, 100.0),
                other => {
                    let at = other.saturating_sub(1).saturating_mul(2);
                    (
                        range.get(at).copied().unwrap_or(-100.0),
                        range.get(at.saturating_add(1)).copied().unwrap_or(100.0),
                    )
                }
            },
            _ => (0.0, 1.0),
        }
    }

    /// The colour `cs` or `CS` starts a selection of `object` at, where the selection resolved
    /// to `self` — [`Self::initial_colour`], except where a device family was selected and a
    /// default or an output intent stands in for it.
    ///
    /// §8.6.8 states the initial colour per family — "\[i]n a DeviceCMYK colour space, the
    /// initial colour shall be [0.0 0.0 0.0 1.0]" — and §8.6.5.6 says what becomes of a device
    /// family's colour values when a default replaces the space:
    ///
    /// > Colour values in the original device colour space shall be passed unchanged to the
    /// > default colour space, which shall have the same number of components as the original
    /// > space.
    ///
    /// So a `/DeviceCMYK cs` under a four-component `/DefaultCMYK` starts at the family's
    /// black, passed unchanged, rather than at the zeros an `ICCBased` space's own initial
    /// colour would be — no ink, which is the paper. §14.11.5's output intent does not replace
    /// the space at all: it describes the device the document's `DeviceCMYK` was prepared for,
    /// and the family's initial colour is the one that description is asked about. The same
    /// clause's "\[i]f a colour value lies outside the range of the default colour space, it
    /// shall be adjusted to the nearest valid value" holds the values to the space that
    /// receives them. A substitute whose component count differs from the family's is not one
    /// §8.6.5.6 admits, and keeps its own initial colour. ADR 1755.
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes §8.6.8 and §8.6.5.6 verbatim, and a quotation is not marked up"
    )]
    #[must_use]
    pub fn initial_colour_of(
        &self,
        document: &Document,
        object: &Object,
        resources: &Dictionary,
    ) -> Vec<f32> {
        match Self::selected_device_family(document, object, resources, 0) {
            Some(family) if family.components() == self.components() => family
                .initial_colour()
                .into_iter()
                .enumerate()
                .map(|(component, value)| nearest(value, self.component_range(component)))
                .collect(),
            _ => self.initial_colour(),
        }
    }

    pub(super) fn named_default(
        document: &Document,
        key: &str,
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<Self> {
        if depth > MAX_DEPTH {
            return None;
        }
        let table = document.get_key(resources, "ColorSpace");
        let entry = table.as_dict()?.get(key)?;
        Self::parse_at(document, entry, resources, reading, depth.saturating_add(1))
    }

    /// The identity of this space's profile, where the space is an [`Self::Icc`].
    ///
    /// What a cache keyed on a space *stated as an output intent* compares — `shading::Cache`
    /// keys a parsed `/ColorSpace` reference on it — because an intent is a profile by Table
    /// 401's definition and [`crate::icc::Profile::identity`] is what tells two apart. `None`
    /// for every other space.
    #[must_use]
    pub fn profile_identity(&self) -> Option<u128> {
        match self {
            Self::Icc { profile } => Some(profile.identity()),
            _ => None,
        }
    }

    /// Which of §11.3.4's two calibrated one-component spaces this is, if either.
    ///
    /// `CalGray` by its gamma and white point, a one-component `ICCBased` space by its
    /// profile; `None` for everything else, `DeviceGray` included, whose component is the
    /// channel and needs no route.
    #[must_use]
    pub fn grey_identity(&self) -> Option<GreyIdentity> {
        match self {
            Self::CalGray { white, gamma, .. } => Some(GreyIdentity::CalGray {
                gamma: gamma.to_bits(),
                white: [white[0].to_bits(), white[1].to_bits(), white[2].to_bits()],
            }),
            Self::Icc { profile } if profile.channels() == 1 => {
                Some(GreyIdentity::Profile(profile.identity()))
            }
            _ => None,
        }
    }

    /// Which three-component CIE-based space this is, for [`super::rgb_route::RgbRoute`], or `None` for a
    /// space that is not one §11.3.4 admits as a blending colour space of three components.
    ///
    /// `Lab` is not one: the clause says it "shall not be used as blending colour spaces
    /// because the compositing computations in such spaces do not give meaningful results
    /// when applied separately to each component".
    #[must_use]
    pub fn rgb_identity(&self) -> Option<RgbIdentity> {
        match self {
            Self::CalRgb {
                white,
                black: _,
                gamma,
                matrix,
            } => Some(RgbIdentity::CalRgb {
                gamma: gamma.map(f32::to_bits),
                white: white.map(f32::to_bits),
                matrix: matrix.map(f32::to_bits),
            }),
            Self::Icc { profile } if profile.channels() == 3 => {
                Some(RgbIdentity::Profile(profile.identity()))
            }
            _ => None,
        }
    }

    /// How many numbers a colour in this space takes.
    #[must_use]
    pub fn components(&self) -> usize {
        match self {
            Self::Gray | Self::Indexed { .. } | Self::CalGray { .. } | Self::AllColourants => 1,
            Self::Icc { profile } => profile.channels(),
            // A pattern is named, not given as components; where an uncoloured one takes
            // a colour, that colour belongs to the underlying space.
            Self::Pattern { base } => base.as_ref().map_or(1, |base| base.components()),
            Self::Rgb | Self::Lab { .. } | Self::CalRgb { .. } => 3,
            Self::Cmyk => 4,
            Self::Separation { inputs, .. } | Self::NoColourant { inputs } => *inputs,
            Self::Simulated { spots, .. } => spots.len(),
        }
    }

    /// The colour a space starts in, when `cs` or `CS` selects it.
    ///
    /// ISO 32000-2 §8.6.8, of the `CS` operator: it "shall also set the current stroking
    /// colour to its initial value, which depends on the colour space", and then gives one
    /// per family. They are not all black, which is what makes this worth a function rather
    /// than a constant: a `Separation` starts at *full* ink, and an `Indexed` space starts at
    /// whatever its table's entry 0 happens to be.
    ///
    /// Returned as components in this space rather than as a colour, so that the caller
    /// converts them by the same route as any other colour — there is exactly one of those.
    #[must_use]
    pub fn initial_colour(&self) -> Vec<f32> {
        match self {
            // "In a DeviceGray, DeviceRGB, CalGray, or CalRGB colour space, the initial
            // colour shall have all components equal to 0.0."
            Self::Gray | Self::Rgb | Self::CalGray { .. } | Self::CalRgb { .. } => {
                vec![0.0; self.components()]
            }
            // "In a DeviceCMYK colour space, the initial colour shall be [0.0 0.0 0.0 1.0]."
            Self::Cmyk => vec![0.0, 0.0, 0.0, 1.0],
            // "In a Lab or ICCBased colour space, the initial colour shall have all
            // components equal to 0.0 unless that falls outside the intervals specified by
            // the space's Range entry, in which case the nearest valid value shall be
            // substituted." The range is `component_range`'s: a `Lab` space's `/Range` for
            // `a` and `b`, and an `ICCBased` space's profile range with Table 65's `/Range`
            // taken over it (ADR 1098). `nearest` takes a range stated backwards as the
            // conversion does rather than panicking on it, as `f32::clamp` would.
            Self::Lab { .. } | Self::Icc { .. } => (0..self.components())
                .map(|component| nearest(0.0, self.component_range(component)))
                .collect(),
            // "In an Indexed colour space, the initial colour value shall be 0."
            Self::Indexed { .. } => vec![0.0],
            // "In a Separation or DeviceN colour space, the initial tint value shall be 1.0
            // for all colourants." `/All` and `/None` are Separation spaces too, so they
            // start at full ink like any other — which for `/All` is black.
            Self::Separation { inputs, .. } | Self::NoColourant { inputs } => vec![1.0; *inputs],
            // The same sentence reaches §10.8.3's reading of the same space: the initial tint
            // is the space's, not the route's.
            Self::Simulated { spots, .. } => vec![1.0; spots.len()],
            Self::AllColourants => vec![1.0],
            // A pattern has no colour of its own until `scn` names one.
            Self::Pattern { .. } => Vec::new(),
        }
    }

    /// The colourant §10.4.2.3 weighs, before the clause's own `min`.
    ///
    /// ISO 32000-2 §10.4.2.3 states the conversion from `DeviceCMYK` to a grey level:
    ///
    /// > To obtain the equivalent gray level for a given CMYK value, the contributions of all
    /// > components shall be taken into account:
    ///
    /// and the formula it then prints is
    /// `gray = 1.0 − min(1.0, 0.3 × cyan + 0.59 × magenta + 0.11 × yellow + black)` — set out
    /// here rather than quoted because the standard sets its formulas in mathematical italics
    /// that no transcription survives. §11.5.3's EXAMPLE 2 prints the same formula for a `/Luminosity` soft mask. This
    /// returns the sum *inside* that `min`, which is what makes it worth a function of its
    /// own: the sum is **linear in the components** and the `min` is not, so a compositing
    /// computation that has to happen before the clamp can happen on this one number instead
    /// of on four (see [`Self::luminosity`] and `pdf_render::SoftMaskKind`). It can exceed
    /// 1.0 — registration black, all four components at 1.0, weighs 2.0.
    ///
    /// **Only `DeviceCMYK` needs an arm of its own here, and that is a result rather than an
    /// omission.** For every other device space §10.4.2 states a conversion to grey directly:
    /// §10.4.2.2 sends a grey to `red = green = blue = grey` and an RGB colour to
    /// `0.3 × red + 0.59 × green + 0.11 × blue`, which is `1 −` this tree's single RGB
    /// conversion, so no other space is taken through `DeviceCMYK` on the way. That route would
    /// agree with the direct one only under a condition: §10.4.2.4 sends an RGB colour to
    /// `c = 1 − red`, `m = 1 − green`, `y = 1 − blue`, `k = min(c, m, y)`, takes `UCR(k)` from
    /// each of the three and makes the black `BG(k)`, so that route's sum is
    /// `0.3c + 0.59m + 0.11y + BG(k) − UCR(k)` before §10.4.2.4's clamps — because §10.4.2.3's
    /// three weights sum to 1.0, the `k` terms cancel exactly where undercolour removal takes
    /// away what black generation adds and no clamp bites, and the round trip is off by
    /// `BG(k) − UCR(k)` everywhere else. §11.5.3 asks for the grey of the colour, not of a
    /// separation of it, so the direct conversion is the one taken (ADR 0217); a file's own
    /// functions reach only the separation §11.7.5.3 names (ADR 1207).
    #[must_use]
    pub fn ink(&self, values: &[f32]) -> f32 {
        self.ink_at(values, 0)
    }

    /// [`Self::ink`], carrying the recursion depth a nested space costs.
    fn ink_at(&self, values: &[f32], depth: usize) -> f32 {
        if depth > MAX_DEPTH {
            return 1.0;
        }
        let at = |index: usize| channel(values.get(index).copied().unwrap_or(0.0));
        match self {
            Self::Cmyk => 0.3_f32.mul_add(
                at(0),
                0.59_f32.mul_add(at(1), 0.11_f32.mul_add(at(2), at(3))),
            ),
            // §11.6.5.1 makes this the *only* reading available inside a mask group: "[i]f
            // the group XObject's content stream specifies a Separation or DeviceN colour
            // space that uses spot colour components, the alternate colour space shall be
            // substituted", which is what evaluating the tint transform does.
            Self::Separation {
                alternate, tints, ..
            } => alternate.ink_at(&tints.eval(values), depth.saturating_add(1)),
            Self::Indexed { base, .. } => {
                base.ink_at(&self.entry_of(values), depth.saturating_add(1))
            }
            Self::Pattern { base } => base
                .as_ref()
                .map_or(1.0, |base| base.ink_at(values, depth.saturating_add(1))),
            // Every other space is one this tree resolves to RGB, and §10.4.2.2's grey of
            // that RGB is the luminosity — so its ink is one minus it. That covers the
            // CIE-based branch of §11.5.3 as well, where the clause asks for the `Y` of the
            // colour in CIE 1931 XYZ and this tree answers with the grey of the sRGB it
            // converts everything to: the same page-wide choice `pdf_model::soft_mask` records
            // rather than a second view of it.
            _ => 1.0 - self.to_rgb(values).grey_level(),
        }
    }

    /// The mask value §11.5.3 derives from a colour in this space.
    ///
    /// > The colour C shall then be converted to luminosity in one of the following ways,
    /// > depending on the group's colour space
    ///
    /// For the device branch the clause says to "convert the colour to `DeviceGray` by
    /// implementation-defined means and use the resulting gray value as the luminosity", and
    /// §10.4.2 states those means for every device space — so this is [`Self::ink`] under
    /// §10.4.2.3's `min`, and nothing here is implementation-defined after all.
    #[must_use]
    pub fn luminosity(&self, values: &[f32]) -> f32 {
        1.0 - self.ink(values).min(1.0)
    }

    /// The space ISO 32000-2 §8.7.4.4 has a shading's gradient calculated in, where the clause
    /// names one.
    ///
    /// The clause gives each family its rule. A device space's values may be converted "at any
    /// time (before or after any interpolation on the colour values in the shading)", so it names
    /// none and this answers `None`. Of a CIE-based space: "all gradient fill calculations shall
    /// be performed in that space. Conversion to device colours shall occur only after all
    /// interpolation calculations have been performed." Of a `Separation` or `DeviceN` space:
    ///
    /// > In that case, gradient fill calculations shall be performed in the designated
    /// > Separation or DeviceN colour space before conversion to the alternate space.
    ///
    /// An `Indexed` space's values "shall be immediately converted to the base colour space" —
    /// [`Self::entry_of`] — after which "gradient fill calculations shall be performed as stated
    /// above", so this answers for the base. A `Pattern` space is not one a shading may state
    /// (Table 77).
    #[must_use]
    pub(crate) fn interpolates_in(&self) -> Option<&Self> {
        match self {
            Self::Gray | Self::Rgb | Self::Cmyk | Self::Pattern { .. } => None,
            Self::Indexed { base, .. } => base.interpolates_in(),
            Self::Lab { .. }
            | Self::CalGray { .. }
            | Self::CalRgb { .. }
            | Self::Icc { .. }
            | Self::Separation { .. }
            // The clause names the *space*, not the route out of it: a `DeviceN` read under
            // §10.8.3 is still the designated `DeviceN` colour space its tints interpolate in.
            | Self::Simulated { .. }
            | Self::AllColourants
            | Self::NoColourant { .. } => Some(self),
        }
    }

    /// The `Indexed` table entry `values` selects, in the base space's components.
    ///
    /// Shared by `Self::to_rgb_at` and `Self::ink_at` so that an index is rounded and
    /// clamped once: two readings of §8.6.6.3's table would be two chances to round it
    /// differently. Any other space's values are its own components already, and come back as
    /// they went in — which is what lets [`crate::mesh`] call this for every vertex it reads.
    ///
    /// Public for one reader outside this crate: ISO 32000-2 §11.6.5.2's `/Matte` is undone
    /// on an `Indexed` image's *table entries* rather than on its indices — Table 144, "the
    /// colour values in the colour table (not the index values themselves) shall be
    /// pre-blended" — so `pdf_model::image` needs the entry and [`Self::indexed_base`]'s
    /// space to invert in.
    #[must_use]
    pub fn entry_of(&self, values: &[f32]) -> Vec<f32> {
        let Self::Indexed { base, lookup, high } = self else {
            return values.to_vec();
        };
        let components = base.components();
        let raw = values.first().copied().unwrap_or(0.0);
        let index = if raw.is_nan() || raw <= 0.0 {
            0
        } else {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "guarded finite and positive; a float-to-integer cast saturates \
                          rather than wrapping, and the result is clamped to `high` below"
            )]
            let rounded = raw.round() as usize;
            rounded
        };
        let start = index.min(*high).saturating_mul(components);
        (0..components)
            .map(|offset| {
                lookup
                    .get(start.saturating_add(offset))
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect()
    }

    /// The space an `Indexed` space's table entries are stated in, or `None` for any other.
    ///
    /// §8.6.6.3's `base`, which is the space a table entry's components belong to and
    /// therefore the space Table 144's `/Matte` is counted and inverted in.
    #[must_use]
    pub fn indexed_base(&self) -> Option<&Self> {
        match self {
            Self::Indexed { base, .. } => Some(base),
            _ => None,
        }
    }
}
