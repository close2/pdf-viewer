//! §10.8.3: the spot colourants a simulated device has a plane for, and the two conversions around the separation multiply.

use std::sync::{Arc, OnceLock};

use pdf_render::Color;
use pdf_syntax::Name;

use super::cie::{D50, degamma, gamma, linear_srgb_to_xyz_d50, xyz_d50_to_linear_srgb};
use super::compositing::{Compositing, Plane, process_plane};
use super::device_cmyk::channel;
use super::press::{Press, PressIdentity};
use super::rgb_route::{INWARD_INPUT_SAMPLES, RGB_OUTPUT_SAMPLES, corners};
use super::{ColourSpace, MAX_DEPTH, Tints};
use crate::black_generation::BlackGeneration;
use crate::icc::Rendering;

/// The names §8.6.6.4 and §8.6.6.5 reserve: "Cyan , Magenta , Yellow and Black … are reserved to
/// name the process colourants of a CMYK device", in the order of `DeviceCMYK`'s components.
pub const PROCESS_COLOURANTS: [&[u8]; 4] = [b"Cyan", b"Magenta", b"Yellow", b"Black"];

/// The spot colourants ISO 32000-2 §10.8.3's simulated device has a plane for, and the press
/// whose process colourants it has.
///
/// Step a) processes the page "as if separations were to be created for a simulated device that
/// supports subtractive process colourants and possibly spot colours" and leaves which ones to
/// the processor; `pdf_model::colourants` enumerates them before the first mark and this carries
/// the answer to every place a colour is resolved, beside the [`Press`] in
/// [`Compositing::Subtractive`]. Empty on every run no reader asked the simulation of, which is
/// every run a backend draws today: [`Compositing::paint`] then takes the path it took before
/// the type existed. ADR 1311.
///
/// A name the page states beyond the plane bound is held too, apart, so that a mark painting it
/// can be named by it — [`DeviceSpots::without_a_plane`] — rather than the page as a whole.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceSpots(pub(super) Option<Arc<SpotSet>>);

/// What a non-empty [`DeviceSpots`] holds.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct SpotSet {
    /// The colourants with a plane, in plane order.
    names: Vec<Name>,
    /// The colourants the page names past the bound, which revert as they are painted.
    beyond: Vec<Name>,
    /// The simulated device's own press, whose process colourants a `Separation` named `Cyan`
    /// may paint directly — §11.7.3 allows it only where "the group inherits the native colour
    /// space of the output device". `None` where the run composites in a space that is not that
    /// press's: a page or group whose `/CS` is one or three components, where the same clause
    /// has such a space "converted to its alternate colour space" (ADR 1329).
    device: Option<PressIdentity>,
}

impl DeviceSpots {
    /// The spot colourants `names` given planes on a device whose process colourants are
    /// `device`'s, with `beyond` the ones past the bound. No names at all is no spot plane.
    ///
    /// `device` is `None` where what is being composited is not the device's native space — a
    /// page whose group states a `/CS` of one or three components — so that no process name is
    /// available to a `Separation` or `DeviceN` anywhere on it (§11.7.3, ADR 1329).
    #[must_use]
    pub fn new(names: Vec<Name>, beyond: Vec<Name>, device: Option<&Press>) -> Self {
        if names.is_empty() && beyond.is_empty() {
            return Self(None);
        }
        Self(Some(Arc::new(SpotSet {
            names,
            beyond,
            device: device.map(|press| press.identity),
        })))
    }

    /// The same colourants inside a group that does not inherit the device's native colour
    /// space, where no process name is available any longer.
    ///
    /// §11.7.3: "within a transparency group, this should be done only if the group inherits
    /// the native colour space of the output device … If any other colour space has been
    /// specified for the group, the Separation or DeviceN colour space shall be converted to its
    /// alternate colour space." A spot plane passes through such a group unconverted, so it is
    /// drawn in the plane's own compositing — and this is what keeps its answer about a
    /// `DeviceN` naming `Cyan` beside a spot colourant the process plane's answer (ADR 1329).
    #[expect(
        clippy::doc_markdown,
        reason = "the paragraph quotes §11.7.3 verbatim, and a quotation may not gain backticks"
    )]
    #[must_use]
    pub fn outside_native(&self) -> Self {
        match &self.0 {
            Some(set) if set.device.is_some() => Self(Some(Arc::new(SpotSet {
                names: set.names.clone(),
                beyond: set.beyond.clone(),
                device: None,
            }))),
            _ => self.clone(),
        }
    }

    /// Whether the device has no spot colourant at all, which is every run but a simulation's.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// The colourants that have a plane, in plane order.
    #[must_use]
    pub fn names(&self) -> &[Name] {
        self.0.as_ref().map_or(&[], |set| set.names.as_slice())
    }

    /// How many spot planes the colourants take: `ceil(S / 3)`.
    #[must_use]
    pub fn planes(&self) -> usize {
        self.names().len().div_ceil(Plane::COLOURANTS)
    }

    /// The colourants a mark in `space` paints that the page named past the bound, and which
    /// therefore revert where the others have a plane — each named, for the mark's report.
    #[must_use]
    pub fn without_a_plane(&self, space: &ColourSpace) -> Vec<Name> {
        let Some(set) = &self.0 else {
            return Vec::new();
        };
        if set.beyond.is_empty() {
            return Vec::new();
        }
        let mut named = Vec::new();
        let mut space = space;
        for _ in 0..=MAX_DEPTH {
            match space {
                ColourSpace::Indexed { base, .. } | ColourSpace::Pattern { base: Some(base) } => {
                    space = base;
                }
                ColourSpace::Separation { names, .. } | ColourSpace::Simulated { names, .. } => {
                    named.extend(
                        names
                            .iter()
                            .filter(|name| set.beyond.contains(name))
                            .cloned(),
                    );
                    break;
                }
                _ => break,
            }
        }
        named
    }

    /// §11.7.4.3's special overprinting blend mode for one mark on a device with spot planes:
    /// which of `plane`'s three channels keep the backdrop, or `None` where the mark leaves no
    /// component of any plane to it and the mode is the one in the graphics state.
    ///
    /// The clause's two bullets, with the spot components the first one names:
    ///
    /// > If the overprint mode is 1 (nonzero overprint mode) and the current colour space and
    /// > group colour space are both DeviceCMYK , then process colour components with nonzero
    /// > values shall replace the corresponding component values of the backdrop; components
    /// > with zero values leave the existing backdrop value unchanged. … For spot colour
    /// > components, the value shall always be 𝐶𝑏 .
    ///
    /// > In all other cases, the value of 𝐵(𝐶𝑏,𝐶𝑠) shall be 𝐶𝑠 for all colour components
    /// > specified in the current colour space, otherwise 𝐶𝑏 .
    ///
    /// `tints` is the mark's four components where its current colour space is `DeviceCMYK` or
    /// reverts to it (§11.7.4.3 NOTE 2), and `nonzero_mode` the overprint mode being 1. The
    /// answer's `None`-ness depends on the mark alone and never on `plane`, so every run of one
    /// page takes the same blend mode for the same mark — a spot plane's list and a process
    /// plane's are then one structure, which is what lets them be checked against each other.
    #[must_use]
    pub fn overprint(
        &self,
        plane: Plane,
        press: &Press,
        space: &ColourSpace,
        tints: Option<[f32; 4]>,
        nonzero_mode: bool,
    ) -> Option<[bool; 3]> {
        let native = self.0.as_ref()?.native(press);
        self.overprint_on(plane, native, space, tints, nonzero_mode)
    }

    /// [`DeviceSpots::overprint`] on the process components of a run that composites in one or
    /// three components rather than in a press: which of its three channels keep the backdrop.
    ///
    /// The same two bullets. The first is conditioned on "the current colour space and group
    /// colour space [being] both DeviceCMYK", which such a group's never is, so a component is
    /// kept exactly where the mark specifies no process component at all — a `Separation` or
    /// `DeviceN` painting spot colourants alone, Table 146's "Separation or DeviceN" row under
    /// either overprint mode — and every channel is the mark's otherwise, because a process
    /// colour converted into the group's space specifies every one of its components. The
    /// answer's `None`-ness is the spot planes' answer for the same mark, so every plane of the
    /// page is one structure (ADR 1329).
    #[expect(
        clippy::doc_markdown,
        reason = "the paragraph quotes §11.7.3 verbatim, and a quotation may not gain backticks"
    )]
    #[must_use]
    pub fn overprint_beside(&self, space: &ColourSpace) -> Option<[bool; 3]> {
        let chromatic = self.overprint_on(Plane::Chromatic, false, space, None, false)?;
        let black = self.overprint_on(Plane::Black, false, space, None, false)?;
        let unspecified = chromatic.iter().all(|kept| *kept) && black.iter().all(|kept| *kept);
        Some([unspecified; 3])
    }

    /// [`DeviceSpots::overprint`] with the question of whether the run composites in the device's
    /// native colour space already answered.
    fn overprint_on(
        &self,
        plane: Plane,
        native: bool,
        space: &ColourSpace,
        tints: Option<[f32; 4]>,
        nonzero_mode: bool,
    ) -> Option<[bool; 3]> {
        let set = self.0.as_ref()?;
        let mut space = space;
        for _ in 0..MAX_DEPTH {
            match space {
                ColourSpace::Indexed { base, .. } | ColourSpace::Pattern { base: Some(base) } => {
                    space = base;
                }
                _ => break,
            }
        }
        let reached = set.named(space, native);
        let process: [bool; 4] = match &reached {
            Named::All | Named::Nothing => return None,
            Named::Direct(names) => PROCESS_COLOURANTS
                .map(|colourant| names.iter().any(|name| name.as_bytes() == colourant)),
            Named::PerComponent { process, .. } => {
                std::array::from_fn(|component| match process {
                    // Table 71's process components, where the space names only some of a CMYK
                    // space's four: an unnamed one is not specified.
                    Some(ColourSpace::Separation {
                        tints: Tints::Process(from),
                        ..
                    }) => from.get(component).is_some_and(Option::is_some),
                    Some(_) => true,
                    None => false,
                })
            }
            Named::Process => match (nonzero_mode, tints) {
                (true, Some(tints)) => tints.map(|tint| tint != 0.0),
                _ => [true; 4],
            },
        };
        let spot = |colourant: &Name| match &reached {
            Named::Direct(names) => names.contains(colourant),
            Named::PerComponent { names, spots, .. } => names
                .iter()
                .zip(spots.iter())
                .any(|(name, spot)| spot.is_some() && name == colourant),
            Named::All | Named::Nothing | Named::Process => false,
        };
        let kept_anywhere =
            process.contains(&false) || set.names.iter().any(|colourant| !spot(colourant));
        if !kept_anywhere {
            return None;
        }
        Some(match plane {
            Plane::Chromatic => [!process[0], !process[1], !process[2]],
            Plane::Black => [!process[3]; 3],
            Plane::Spot(index) => std::array::from_fn(|offset| {
                index
                    .checked_mul(Plane::COLOURANTS)
                    .and_then(|first| first.checked_add(offset))
                    .and_then(|colourant| set.names.get(colourant))
                    .is_none_or(|colourant| !spot(colourant))
            }),
        })
    }

    /// §11.7.3's paint of one colour on `plane`, where the device carries spot colourants.
    ///
    /// > In effect, every object paints every existing colour component, both process and
    /// > spot. Where no value has been explicitly specified for a given component in a given
    /// > object, an additive value of 1.0 (or a subtractive tint value of 0.0) shall be assumed.
    ///
    /// So a colour reaches every plane, and what it reaches each with is decided by which
    /// colourants its space names — [`SpotSet::reach`].
    pub(super) fn paint(
        &self,
        plane: Plane,
        press: &Press,
        space: &ColourSpace,
        values: &[f32],
        rendering: Rendering,
        generation: Option<&BlackGeneration>,
    ) -> Color {
        let Some(set) = &self.0 else {
            return process_plane(
                plane,
                space.to_cmyk_under(values, rendering, press, generation),
            );
        };
        set.reach(plane, press, space, values, rendering, generation, 0)
    }

    /// §11.7.3's paint of one colour on the process components of a run that composites in one
    /// or three components — a `/DeviceRGB`, `/DeviceGray`, `CalGray`, `CalRGB` or `ICCBased`
    /// group or page — where the device carries spot colourants beside them.
    ///
    /// > A spot colour retains its own identity; it shall not be subject to conversion to or from
    /// > the colour space of the enclosing transparency group or page.
    ///
    /// So a colour whose space paints spot colourants alone paints the group's process components
    /// with "an additive value of 1.0", which is white in every one of the four spaces this is
    /// asked for, and its spot colourants reach their planes as they would anywhere; an
    /// `NChannel` space paints Table 71's process components converted into the group's space;
    /// and every other colour is `into`'s own paint. No process name is available here —
    /// "[i]f any other colour space has been specified for the group, the Separation or DeviceN
    /// colour space shall be converted to its alternate colour space" — so a space naming one
    /// reverts whole, as it does on the spot planes beside this one (ADR 1329).
    #[expect(
        clippy::doc_markdown,
        reason = "the paragraph quotes §11.7.3 verbatim, and a quotation may not gain backticks"
    )]
    pub(super) fn paint_process(
        &self,
        into: &Compositing,
        space: &ColourSpace,
        values: &[f32],
        rendering: Rendering,
        generation: Option<&BlackGeneration>,
    ) -> Color {
        let Some(set) = &self.0 else {
            return into.paint(space, values, rendering, generation);
        };
        let (stated, stated_values) = (space, values);
        let mut space = space;
        let mut values = std::borrow::Cow::Borrowed(values);
        for _ in 0..=MAX_DEPTH {
            match space {
                ColourSpace::Indexed { base, .. } => {
                    values = std::borrow::Cow::Owned(space.entry_of(&values));
                    space = base;
                }
                ColourSpace::Pattern { base: Some(base) } => space = base,
                _ => break,
            }
        }
        match set.named(space, false) {
            Named::Direct(_) | Named::PerComponent { process: None, .. } => Color {
                a: space.to_rgb_under(&values, rendering).a,
                ..Color::rgb(1.0, 1.0, 1.0)
            },
            Named::PerComponent {
                process: Some(process),
                ..
            } => Color {
                a: space.to_rgb_under(&values, rendering).a,
                ..into.paint(process, &values, rendering, generation)
            },
            Named::All | Named::Nothing | Named::Process => {
                into.paint(stated, stated_values, rendering, generation)
            }
        }
    }
}

#[expect(
    clippy::doc_markdown,
    reason = "the paragraphs quote §8.6.6.4, §8.6.6.5 and §11.7.3 verbatim, and a quotation may \
              not gain backticks"
)]
impl SpotSet {
    /// Whether a space naming `names` paints its colourants directly on this device.
    ///
    /// §8.6.6.4: "At the moment the colour space is set to a Separation space, the PDF reader
    /// shall determine whether the device has an available colourant corresponding to the name
    /// of the requested space. If so, the PDF processor shall ignore the alternateSpace and
    /// tintTransform parameters; subsequent painting operations within the space shall apply the
    /// designated colourant directly, according to the tint values supplied." §8.6.6.5 asks it of
    /// every name, and "[r]eversion shall occur only if at least one colour component (other
    /// than None ) is specified and is not available on the device".
    ///
    /// A spot colourant is available where it has a plane. A process name is available where the
    /// run composites in the device's own press, and only there — §11.7.3: "within a transparency
    /// group, this should be done only if the group inherits the native colour space of the
    /// output device … If any other colour space has been specified for the group, the
    /// Separation or DeviceN colour space shall be converted to its alternate colour space."
    fn available(&self, names: &[Name], native: bool) -> bool {
        names.iter().all(|name| {
            let bytes = name.as_bytes();
            bytes == b"None"
                || self.names.contains(name)
                || (native && PROCESS_COLOURANTS.contains(&bytes))
        })
    }

    /// Whether a run compositing in `press` composites in the device's native colour space,
    /// which is where a process name is available (§11.7.3).
    fn native(&self, press: &Press) -> bool {
        self.device == Some(press.identity)
    }

    /// What a space's colour paints on this device, by which colourants the space names.
    ///
    /// Asked of a space that is not `Indexed` or a pattern space: those two name their colourants
    /// through their base, and [`SpotSet::reach`] and [`DeviceSpots::overprint`] look through
    /// them first (§8.6.6.3, §8.7.3.3, and §11.7.4.3 NOTE 2 for the overprint question).
    fn named<'s>(&self, space: &'s ColourSpace, native: bool) -> Named<'s> {
        match space {
            ColourSpace::AllColourants => Named::All,
            ColourSpace::NoColourant { .. } => Named::Nothing,
            ColourSpace::Separation { names, .. } if self.available(names, native) => {
                Named::Direct(names)
            }
            ColourSpace::Simulated {
                names,
                process,
                spots,
            } if names
                .iter()
                .zip(spots)
                .all(|(name, spot)| spot.is_none() || self.names.contains(name)) =>
            {
                Named::PerComponent {
                    names,
                    spots,
                    process: process.as_deref(),
                }
            }
            _ => Named::Process,
        }
    }

    /// One colour on `plane`, by what its space names ([`SpotSet::named`]).
    ///
    /// - **An `Indexed` or pattern space** is the colour its base space gives, which is where the
    ///   colourants are named (§8.6.6.3, §8.7.3.3).
    /// - **`All`** paints its tint on every colourant the device has — §8.6.6.4's "all colourants
    ///   available on an output device, including those for the standard process colourants" —
    ///   so every process component and every spot colourant with a plane.
    /// - **A `Separation` or `DeviceN` whose colourants are all available** paints each named
    ///   colourant's tint on the plane that carries it and no ink everywhere else, `None`
    ///   components discarded: "When a DeviceN colour space is painting the named device
    ///   colourants directly, colour components corresponding to None colourants shall be
    ///   discarded."
    /// - **An `NChannel` space under the simulation** is evaluated per component, §8.6.6.5's "only
    ///   the ones not present on the output device shall use the alternate colour space of that
    ///   component": where every spot component has a plane, its process components go through
    ///   Table 71's process space onto the process planes and its spot components onto theirs.
    /// - **Every other colour** — a process colour, and a space that reverts — is its four
    ///   process components and no spot ink, §11.7.3's "the process colour components shall be
    ///   painted as specified and the spot colour components shall be painted with an additive
    ///   value of 1.0".
    #[expect(
        clippy::too_many_arguments,
        reason = "`Compositing::paint`'s four inputs, the run's plane and press, and the depth a \
                  nested space costs"
    )]
    fn reach(
        &self,
        plane: Plane,
        press: &Press,
        space: &ColourSpace,
        values: &[f32],
        rendering: Rendering,
        generation: Option<&BlackGeneration>,
        depth: usize,
    ) -> Color {
        let deeper = depth.saturating_add(1);
        match space {
            _ if depth > MAX_DEPTH => return process_plane(plane, [0.0; 4]),
            ColourSpace::Indexed { base, .. } => {
                return self.reach(
                    plane,
                    press,
                    base,
                    &space.entry_of(values),
                    rendering,
                    generation,
                    deeper,
                );
            }
            ColourSpace::Pattern { base: Some(base) } => {
                return self.reach(plane, press, base, values, rendering, generation, deeper);
            }
            _ => {}
        }
        let tint = |index: usize| channel(values.get(index).copied().unwrap_or(0.0));
        match self.named(space, self.native(press)) {
            Named::All => {
                let all = channel(values.first().copied().unwrap_or(1.0));
                self.on_plane(plane, [all; 4], |_| all)
            }
            Named::Nothing => process_plane(plane, [0.0; 4]),
            Named::Direct(names) => {
                let tint_of = |colourant: &[u8]| {
                    names
                        .iter()
                        .position(|name| name.as_bytes() == colourant)
                        .map_or(0.0, tint)
                };
                self.on_plane(plane, PROCESS_COLOURANTS.map(tint_of), |colourant| {
                    tint_of(colourant.as_bytes())
                })
            }
            Named::PerComponent {
                names,
                spots,
                process,
            } => {
                let inks = match (plane, process) {
                    (Plane::Chromatic | Plane::Black, Some(process)) => {
                        process.to_cmyk_under(values, rendering, press, generation)
                    }
                    _ => [0.0; 4],
                };
                self.on_plane(plane, inks, |colourant| {
                    names
                        .iter()
                        .zip(spots)
                        .position(|(name, spot)| spot.is_some() && name == colourant)
                        .map_or(0.0, tint)
                })
            }
            Named::Process => match plane {
                Plane::Chromatic | Plane::Black => process_plane(
                    plane,
                    space.to_cmyk_under(values, rendering, press, generation),
                ),
                Plane::Spot(_) => Color::rgb(1.0, 1.0, 1.0),
            },
        }
    }

    /// `process` on a process plane, or each spot colourant's tint on its spot plane, as §11.3.4's
    /// additive complement; a channel no colourant fills is white.
    fn on_plane(&self, plane: Plane, process: [f32; 4], spot: impl Fn(&Name) -> f32) -> Color {
        match plane {
            Plane::Chromatic | Plane::Black => process_plane(plane, process),
            Plane::Spot(index) => {
                let channel = |offset: usize| {
                    index
                        .checked_mul(Plane::COLOURANTS)
                        .and_then(|first| first.checked_add(offset))
                        .and_then(|colourant| self.names.get(colourant))
                        .map_or(1.0, |name| 1.0 - spot(name))
                };
                Color::rgb(channel(0), channel(1), channel(2))
            }
        }
    }
}

/// Which colourants one colour paints on a device with spot planes: [`SpotSet::named`]'s
/// answer, which the paint and the overprint question both read so that they cannot disagree
/// about a space (trap 6).
enum Named<'s> {
    /// §8.6.6.4's `All`: every colourant the device has.
    All,
    /// §8.6.6.4's `None`, or a `DeviceN` of nothing else: no colourant at all.
    Nothing,
    /// A `Separation` or `DeviceN` painting the colourants it names directly.
    Direct(&'s [Name]),
    /// An `NChannel` space under the simulation, every spot component with a plane: Table 71's
    /// process components through `process`, the spot components by name.
    PerComponent {
        /// The space's `names`.
        names: &'s [Name],
        /// Which components are spot ones — `Some` — as [`ColourSpace::Simulated`] holds them.
        spots: &'s [Option<ColourSpace>],
        /// The process space the process components are painted through, where there are any.
        process: Option<&'s ColourSpace>,
    },
    /// Four process components and no spot ink: a process colour, or a space that reverts.
    Process,
}

/// ISO 32000-2 §10.8.3's two conversions around step c)'s multiply, sampled for a backend:
/// step b) of the process separation, from the device colour its pair resolves to, and step d),
/// from the multiplied result to the device.
///
/// Both are relative to the matte's white — step b)'s "background matte of all white", which is
/// the 1.0 of Table 136's multiply — so the first ends in `XYZ ÷ D50` and the second begins
/// there. Each is this crate's one conversion between the device and XYZ — [`super::cie::srgb_to_xyz_d50`]
/// and [`super::cie::xyz_d50_to_srgb`] — split where [`pdf_render::ColourCube`] carries a conversion: the
/// device's decoding on the input curves, the linear map in a grid of side two, which
/// interpolates a linear map exactly, and the encoding on the output curve. The process
/// separation goes through the device colour rather than straight from its four components
/// because that is the route [`ColourSpace::flat_ratio`] gives a `DeviceCMYK` separation, so a
/// page separated into planes and one painting operation simulated alone take one conversion
/// between them (trap 6). Built once, on first use.
#[must_use]
pub fn separation_conversions() -> &'static (pdf_render::ColourCube, pdf_render::ColourCube) {
    static CONVERSIONS: OnceLock<(pdf_render::ColourCube, pdf_render::ColourCube)> =
        OnceLock::new();
    CONVERSIONS.get_or_init(|| {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an index below the sample count"
        )]
        let at = |index: usize, count: usize| index as f32 / count.saturating_sub(1) as f32;
        let identity_in: Arc<[[f32; 3]]> = Arc::from(vec![[0.0; 3], [1.0; 3]]);
        let identity_out: Arc<[f32]> = Arc::from(vec![0.0, 1.0]);
        let decode: Vec<[f32; 3]> = (0..INWARD_INPUT_SAMPLES)
            .map(|index| [degamma(at(index, INWARD_INPUT_SAMPLES)); 3])
            .collect();
        let to_flat = corners(|linear| {
            let xyz = linear_srgb_to_xyz_d50(linear);
            std::array::from_fn(|axis| xyz[axis] / D50[axis])
        });
        let to_device = corners(|ratio| {
            xyz_d50_to_linear_srgb(std::array::from_fn(|axis| ratio[axis] * D50[axis]))
        });
        let encode: Vec<f32> = (0..RGB_OUTPUT_SAMPLES)
            .map(|index| gamma(at(index, RGB_OUTPUT_SAMPLES)))
            .collect();
        let process_to_flat = pdf_render::ColourCube::new(
            Arc::from(decode),
            2,
            Arc::from(to_flat),
            Arc::clone(&identity_out),
        );
        let flat_to_device =
            pdf_render::ColourCube::new(identity_in, 2, Arc::from(to_device), Arc::from(encode));
        match (process_to_flat, flat_to_device) {
            (Some(process_to_flat), Some(flat_to_device)) => (process_to_flat, flat_to_device),
            _ => unreachable!("two curves of at least two samples and a grid of eight corners"),
        }
    })
}
