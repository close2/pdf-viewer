//! §11.4.7 and §11.6.6: the blending colour space a page or a group composites in, and where this tree
//! departs from the device's three components. The departure is read from the file here; what the
//! conversion into the parent costs is `into_parent`'s, and a press's is `press`'s (ADR 2080).

use std::sync::Arc;

use pdf_render::Command;
use pdf_syntax::{Dictionary, Document, Object};

use crate::colour::{ColourSpace, Compositing, Presses};
use crate::icc::Rendering;
use crate::page::Page;

use crate::content::Interpreter;
use crate::content::report::Unsupported;

use super::TransparencyGroup;
use super::commands::{any_command, command_composites};
use super::into_parent::parent_channels;
use super::press::{BeyondPress, stated_cmyk_group};

/// Names a colour space if compositing in it is not compositing on the device's components.
///
/// This tree composites on the three components of the device raster, so the one space that
/// asks for what already happens is `/DeviceRGB`. **`CalRGB` and a three-component profile
/// were let through here as well until ADR 0797**, as "a colorimetric difference this
/// renderer takes page-wide and records as a choice"; they are named now, because they are
/// drawn in their own components ([`Interpreter::group_own_space`], [`page_own_space`]) and
/// a group inside one introducing a different space has to be a change of space.
///
/// What is named is a space whose components are not the device's: `/DeviceGray`,
/// `/DeviceCMYK`, `Separation` and `DeviceN` blend a different number of components, the
/// CIE-based spaces blend their own components behind a conversion, and `Lab` blends three
/// the clause forbids. §11.3.4 is why that is a difference rather than a notation:
///
/// > The result of the computation thus depends on the colour space in which the colours are
/// > represented.
///
/// Honouring one means compositing in its own components and converting once at the end, which
/// is a second raster format rather than a colour conversion — ADR 0251 measures how far apart
/// the two orders of operation are, and it is up to 48 of 255 for `/DeviceCMYK`.
///
/// `None` for an absent entry as well as for an RGB one, which is why a caller deciding
/// §11.6.6's inheritance tests the entry's presence itself rather than reading it off this.
///
/// **A one-component space is named here and is not reported where it is drawn**: `DeviceGray`
/// (ADR 0790), and `CalGray` and a one-component profile (ADR 0792) — what
/// [`Interpreter::group_own_space`] and [`page_own_space`] draw. Each stays a
/// [`Departure`] so that a group *inside* it introducing a different space is still a change
/// of space.
fn space_departure(document: &Document, entry: &Object) -> Option<Departure> {
    let object = document.resolve(entry);
    if matches!(object, Object::Null) {
        return None;
    }
    // Named before it is parsed, because what a report has to say is what the file
    // asked for, and a space this crate cannot read has no other description.
    let name = match &object {
        Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
        _ => "an array-formed space".to_owned(),
    };
    match ColourSpace::parse(document, &object, &Dictionary::new()) {
        Some(ColourSpace::Rgb) => None,
        space => Some(Departure {
            name,
            components: space.as_ref().map_or(0, ColourSpace::components),
            identity: space.as_ref().and_then(space_identity),
        }),
    }
}

/// What tells one CIE-based space from another of the same name, or `None` for a space
/// that is not CIE-based.
///
/// Two array-formed `/CS` entries print alike in a report and are not alike: a `CalRGB` of
/// gamma 1 inside a `CalRGB` of gamma 2.2 is a change of space with a conversion between the
/// two at its `Do`, and until ADR 0797 the two compared equal by name and the inner one
/// inherited in silence. The hash is of the space's own parameters — Table 62's and 63's
/// entries, a profile's bytes, `Lab`'s white point and range — and is compared within one
/// interpretation, which is all `std::hash::DefaultHasher` promises.
fn space_identity(space: &ColourSpace) -> Option<u64> {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    match space {
        ColourSpace::Icc { profile } => profile.identity().hash(&mut hasher),
        ColourSpace::CalGray { .. } => space.grey_identity()?.hash(&mut hasher),
        ColourSpace::CalRgb { .. } => space.rgb_identity()?.hash(&mut hasher),
        ColourSpace::Lab { white, range, .. } => {
            white.map(f32::to_bits).hash(&mut hasher);
            range.map(f32::to_bits).hash(&mut hasher);
        }
        _ => return None,
    }
    Some(hasher.finish())
}

/// How many components a `/CS` entry has where it names a device colour space, as written.
///
/// §11.7.2's rule for such an entry inside a CIE-based group reads the count and nothing
/// else, so a `/DefaultRGB` remapping is not consulted here: the rule is about the space the
/// file wrote, as [`space_departure`]'s name is.
fn device_components(document: &Document, entry: &Object) -> Option<usize> {
    match ColourSpace::parse(document, &document.resolve(entry), &Dictionary::new())? {
        ColourSpace::Gray => Some(1),
        ColourSpace::Rgb => Some(3),
        ColourSpace::Cmyk => Some(4),
        _ => None,
    }
}

/// A blending colour space that is not the device's three components, as the file names it.
///
/// The component count travels with the name because it decides *when* the departure is
/// visible. §11.6.6 has every painting operator "convert source colours … to the group colour
/// space before compositing", and for a space of three or four components that conversion
/// has an inverse over the colours a device can show, so an opaque `Normal` mark comes out
/// the colour it went in and only a mark that composites can tell the spaces apart. A
/// one-component space has no inverse: a red mark painted into it is a grey, whatever
/// composites over it, so the departure is every mark on the page and not only the ones
/// §11.3.3 blends. [`Interpreter::note_page_blending_space`] and
/// [`Interpreter::note_group_departures`] read the count for exactly that condition — on
/// the one-component spaces this tree does *not* draw, which since ADR 0792 are the ones
/// §11.3.4 does not list (a `Separation` or `Indexed` space as a group's `/CS`) and a
/// profile whose curve has no inverse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::content) struct Departure {
    /// What the file wrote — `/DeviceCMYK`, `/CalGray`, or "an array-formed space".
    pub(in crate::content) name: String,
    /// How many components the space has, or 0 where this crate could not parse it.
    pub(in crate::content) components: usize,
    /// What tells this space from another of the same name where it is CIE-based, and
    /// `None` where it is not — which is also the condition §11.7.2 puts on a device space
    /// inside it being the space itself: see [`group_blending`] and [`space_identity`].
    pub(in crate::content) identity: Option<u64>,
}

impl Departure {
    /// §10.8.3's simulated device's own four components, for a run that composites in its press
    /// on a page stating no blending space: a group inside naming `/DeviceCMYK` then inherits
    /// "the native colour space of the output device" (§11.7.3) rather than changing the space
    /// in force, which is what drawing it in that press already does (ADR 1329).
    pub(in crate::content) fn simulated_press() -> Self {
        Self {
            name: "/DeviceCMYK".to_owned(),
            components: 4,
            identity: None,
        }
    }

    /// Whether the space is CIE-based.
    fn calibrated(&self) -> bool {
        self.identity.is_some()
    }

    /// Whether every mark painted into this space changes colour, compositing or not.
    ///
    /// One component: the conversion in keeps a grey and nothing else, so there is no
    /// conversion out that could give a mark its colour back.
    pub(super) fn loses_chroma(&self) -> bool {
        self.components == 1
    }
}

/// What a `/CS` entry of one or three components asks a page or group to composite in, after
/// the default colour spaces' remapping — or `None` where it is not one this tree draws in a
/// space of its own.
///
/// §11.3.4 lists three one-component spaces a processor "shall" support as blending colour
/// spaces, and this answers for all three: `DeviceGray` is
/// [`crate::colour::Compositing::Grey`], whose component is the channel; `CalGray` and an
/// `ICCBased` 'GRAY' profile are [`crate::colour::Compositing::Calibrated`], whose component
/// leaves through a curve ([`crate::colour::GreyRoute`]). And two of its three-component
/// ones since ADR 0797: `CalRGB` and a bi-directional `ICCBased` 'RGB ' profile are
/// [`crate::colour::Compositing::Additive`], whose components leave through a cube
/// ([`crate::colour::RgbRoute`]) — `DeviceRGB` is the device's own and asks for nothing.
/// §11.6.6's Table 145 subjects a group's device colour space to remapping first —
///
/// > Device colour spaces shall be subject to remapping according to the DefaultGray ,
/// > DefaultRGB , and DefaultCMYK entries in the ColorSpace subdictionary of the current
/// > resource dictionary (see 8.6.5.6, "Default colour spaces").
///
/// — so a `/DeviceGray` beside a `/DefaultGray` is that default, and takes the default's
/// route. The parse is against `resources` for that reason, where [`space_departure`]'s is
/// not: that one names what the file *wrote*, and this one asks what it means here.
fn own_space_compositing(
    document: &Document,
    entry: &Object,
    resources: &Dictionary,
    presses: &Presses,
) -> Option<Compositing> {
    match ColourSpace::parse(document, entry, resources)? {
        ColourSpace::Gray => Some(Compositing::Grey),
        ColourSpace::Rgb => None,
        space => crate::colour::GreyRoute::of(&space)
            .map(Arc::new)
            .map(Compositing::Calibrated)
            .or_else(|| presses.rgb_route(&space).map(Compositing::Additive)),
    }
}

/// What §11.4.7's page group asks the page to composite in, where it is one component or
/// three CIE-based ones.
///
/// The counterpart of [`super::press::page_press`] for the spaces that need one interpretation rather than
/// two: a page whose group states `/DeviceGray`, `CalGray`, `CalRGB` or a one- or
/// three-component profile is interpreted once under the [`Compositing`] this returns, which
/// `interpreted` chooses on this answer. A page group with no `/S /Transparency` asks for
/// nothing, as [`page_blending_space`] reads it.
pub(in crate::content) fn page_own_space(
    document: &Document,
    page: &Page,
    presses: &Presses,
) -> Option<Compositing> {
    let attributes = document.get_key(&page.dict, "Group");
    let attributes = attributes.as_dict()?;
    if document
        .get_key(attributes, "S")
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"Transparency")
    {
        return None;
    }
    let entry = document.get_key(attributes, "CS");
    own_space_compositing(document, &entry, &page.resources, presses)
}

/// The blending colour space §11.4.7 gives a page, named where it is one this tree departs from.
///
/// The root of the inheritance §11.6.6 states, and §11.4.7 says where its space comes from:
///
/// > That initial colour space shall serve as the default blending colour space for each page,
/// > unless the page explicitly specifies an alternative default by means of its page
/// > dictionary containing a Group key that contains a CS key whose value represents a
/// > different colour space from the initial blending colour space.
///
/// The initial one "is inherited from the native colour space of the actual, assumed or
/// simulated output device", which for this processor is the device raster's three components.
/// So a page that states nothing composites in what this tree composites in, and a page that
/// states a `/Group /CS` is judged by [`space_departure`] — and that entry decides the whole
/// page, because §11.4.7 also says "\[a]ll page-level compositing shall be done in the default
/// blending colour space of the page".
pub(in crate::content) fn page_blending_space(
    document: &Document,
    page: &Page,
) -> Option<Departure> {
    let attributes = document.get_key(&page.dict, "Group");
    let attributes = attributes.as_dict()?;
    // §8.10.3 Table 94's `/S`, the same required entry a form's group carries.
    if document.get_key(attributes, "S").as_name()?.as_bytes() != b"Transparency" {
        return None;
    }
    space_departure(document, &document.get_key(attributes, "CS"))
}

/// The blending colour space in force *inside* a group, given the one in force outside it.
///
/// §11.6.6 states the two cases, and the second is the one this tree used to ignore:
///
/// > For isolated groups, if a group colour space ( CS ) is specified in the group attributes
/// > dictionary, all painting operators shall convert source colours in a colour space (that
/// > are not equivalent to the group colour space) to the group colour space before compositing
/// > objects into the group.
///
/// > For non-isolated groups, or if no group colour space is specified, the group colour space
/// > shall be inherited from the parent group or page.
///
/// §11.7.2 says it a second time and gives the reason — "the use of an explicit colour space in
/// a non-isolated group would require converting colours from the backdrop's colour space to
/// that of the group in order to perform the compositing computations" — so a `/CS` on a
/// non-isolated group is not the space anything composites in, and reporting it as one is
/// reporting a departure that is not there.
///
/// And §11.7.2 gives a device space inside a CIE-based one no effect either:
///
/// > If the colour space of the transparency group is a device colour space, and some
/// > ancestor of the group has a CIE-based colour space with the same number of colourants,
/// > then the colour space of this group shall be the CIE-based space of the nearest such
/// > ancestor.
///
/// So an isolated `/DeviceRGB` group inside a `CalRGB` page, a `/DeviceGray` one inside a
/// `CalGray` page and a `/DeviceCMYK` one inside a profile's press all inherit — which is
/// what keeps the commonest group of all, a `/DeviceRGB` group on a page whose group is the
/// sRGB profile, from being a change of space at every `Do`.
pub(super) fn group_blending(
    document: &Document,
    group: &TransparencyGroup,
    inherited: Option<&Departure>,
) -> Option<Departure> {
    let entry = document.resolve(&group.colour_space);
    if !group.isolated || matches!(entry, Object::Null) {
        return inherited.cloned();
    }
    if let Some(inherited) = inherited
        && inherited.calibrated()
        && device_components(document, &entry) == Some(inherited.components)
    {
        return Some(inherited.clone());
    }
    space_departure(document, &entry)
}

/// The conversion out a group drawn in its own space carries to the backend, or `None`
/// where the raster already holds the device's answer.
///
/// A calibrated component leaves by its curve and three components by their cube, which
/// the backend applies before painting the group onto its parent
/// (`pdf_render::GroupBlending`); device grey leaves as it is, §10.4.2.2's grey level being
/// "equivalent to an RGB value with all three components the same".
pub(super) fn own_space_conversion(compositing: &Compositing) -> Option<pdf_render::GroupBlending> {
    match compositing {
        Compositing::Calibrated(route) => Some(pdf_render::GroupBlending::OneComponent {
            curve: route.curve().clone(),
        }),
        Compositing::Additive(route) => Some(pdf_render::GroupBlending::ThreeComponents {
            cube: route.cube().clone(),
        }),
        Compositing::Device
        | Compositing::Luminosity(_)
        | Compositing::Grey
        | Compositing::Subtractive(..) => None,
    }
}

/// Whether a group ran in a space of its own, and what it leaves by if it did.
pub(super) enum OwnSpaceRun {
    /// Drawn in its own space, leaving by this conversion — `None` where the raster already
    /// holds what the parent composites in.
    Drawn(Option<pdf_render::GroupBlending>),
    /// One of [`Interpreter::group_commands`]' conditions gave the space up.
    GivenUp,
}

/// Whether a group's change of blending colour space can change a pixel of its elements.
///
/// Something compositing in the group can, in any space (§11.3.4). A space of one component
/// is a departure on every mark and not only on one that composites
/// ([`Departure::loses_chroma`]), so a grey group inside a press records itself whatever it
/// holds: the device rerun then draws the group in its grey and reports the press, which is
/// louder than drawing the group in ink and saying nothing.
pub(super) fn change_is_visible(introduced: Option<&Departure>, commands: &[Command]) -> bool {
    introduced.is_some_and(Departure::loses_chroma) || any_command(commands, &command_composites)
}

impl Interpreter<'_> {
    /// What this group is drawn in where its `/CS` is one component or three CIE-based ones:
    /// §11.6.6's own-space case, in each of its shapes but the press's.
    ///
    /// The counterpart of [`Interpreter::group_press`] for a space of one or three components
    /// rather than four, and it needs fewer of that function's conditions because it needs no
    /// pair: the group's content runs once, its result is the space's components in the
    /// channels, and the conversion out is either what the raster already holds —
    /// `DeviceGray` under [`Compositing::Grey`], §10.4.2.2's grey level "equivalent to an RGB
    /// value with all three components the same" — or a curve or a cube the group carries out
    /// to its backend ([`Compositing::Calibrated`], [`Compositing::Additive`],
    /// `pdf_render::GroupBlending`), so the group composites onto its parent as any other
    /// group does. §11.4.6's rewrite therefore edits one list here rather than a pair's two,
    /// and Table 57's black generation does not enter — §11.7.5.3 puts it inside §10.4.2.4's
    /// conversion into `DeviceCMYK`, which is on no route into grey.
    ///
    /// Two of [`Interpreter::group_press`]'s conditions stay, for the reasons given there: the
    /// group must be isolated, because §11.6.6 gives a non-isolated group's `/CS` no effect;
    /// and no uncoloured cell may be supplying the colour from outside. Which parents it may
    /// be asked under is [`Interpreter::group_compositing`]'s question. A grey group inside a
    /// grey page is not this function's: it inherits, and [`group_blending`] says so.
    fn group_own_space(
        &self,
        group: &TransparencyGroup,
        resources: &Dictionary,
    ) -> Option<Compositing> {
        if !group.isolated || self.uncoloured {
            return None;
        }
        own_space_compositing(self.document, &group.colour_space, resources, self.presses)
    }

    /// What an isolated group's own `/CS` asks its elements to composite in, where this run
    /// can honour it — or `None`, which is "what the parent composites in" (§11.6.6, §11.7.2).
    ///
    /// Three answers, in the order the group's entry is read: a press
    /// ([`Interpreter::group_press`]), one component or three CIE-based ones
    /// ([`Interpreter::group_own_space`]), and — only inside a parent that composites in a
    /// space of its own — the device's own three components, for a `/DeviceRGB` group that
    /// §11.7.2's same-count rule did not fold into its parent.
    ///
    /// # Which parents
    ///
    /// Where the parent composites on the device, every one of the three is a construction
    /// this tree draws and the group's result leaves by its own conversion out. Where the
    /// parent composites in a space of its own, the group's result owes a second conversion
    /// at its `Do` — §11.6.6: "\[i]f colour conversion needs to take place in order to
    /// composite the group into its parent" — and [`parent_channels`] is that conversion, so
    /// the group is drawn in its own space exactly where `changed` says the space differs
    /// ([`group_blending`]) and the parent is one that function has a route into. A group
    /// restating its parent's space, or naming the parent's own press under another spelling,
    /// inherits: there is nothing to convert between.
    pub(in crate::content) fn group_compositing(
        &mut self,
        group: &TransparencyGroup,
        resources: &Dictionary,
        rendering: Rendering,
        changed: bool,
    ) -> Option<Compositing> {
        let nested = self.compositing != Compositing::Device;
        // `None` for the black generation: this asks only whether a route into the parent
        // exists, and every arm answers `Some` or `None` on the parent's variant alone.
        if nested
            && !(changed && parent_channels(&self.compositing, rendering, None, [0.0; 3]).is_some())
        {
            return None;
        }
        if let Some(press) = self.group_press(group, resources, rendering) {
            if let Compositing::Subtractive(_, parent, _) = &self.compositing
                && parent.identity() == press.identity()
            {
                return None;
            }
            // The device's spot colourants go into the group with it: §11.7.3's first bullet
            // has the group "maintain a separate colour value for each spot colour component,
            // independently of the group's colour space" — the run's, so that a press inside a
            // group of one or three components still carries them (ADRs 1311, 1329).
            return Some(Compositing::Subtractive(
                crate::colour::Plane::Chromatic,
                press,
                self.spots_beside.clone(),
            ));
        }
        if let Some(own) = self.group_own_space(group, resources) {
            return Some(own);
        }
        // §11.7.2 folds a device space of the parent's own count into the parent
        // (`group_blending`), so a `/DeviceRGB` entry that still changed the space is inside
        // a parent of another count, or of the same count but a device one: the device's
        // three components are the group's own here.
        (nested
            && group.isolated
            && !self.uncoloured
            && matches!(
                ColourSpace::parse(self.document, &group.colour_space, resources),
                Some(ColourSpace::Rgb)
            ))
        .then_some(Compositing::Device)
    }

    /// Puts §11.7.2's redefinition of `DeviceCMYK` in force for `group`'s content, and answers the
    /// one in force outside it, for the caller to put back after.
    ///
    /// [`Interpreter::group_commands`] holds it across every run of the group's content — the
    /// press's, the black half's and a rerun in the parent's space alike, since the redefinition
    /// is the colour's meaning and not the raster's. An isolated group stating an `ICCBased` CMYK
    /// space ([`stated_cmyk_group`]) redefines `DeviceCMYK`, and any other group keeps the one
    /// outside. ADR 2012.
    pub(super) fn enter_cmyk_group(
        &mut self,
        group: &TransparencyGroup,
        resources: &Dictionary,
    ) -> Option<ColourSpace> {
        let stated = if group.isolated {
            let document = self.document;
            stated_cmyk_group(document, &group.colour_space, resources, |entry| {
                self.blending_space_of(entry)
            })
        } else {
            None
        };
        match stated {
            Some(space) => self.cmyk_group.replace(space),
            None => self.cmyk_group.clone(),
        }
    }

    /// Reports §11.4.7's page group where its blending space is not the device's.
    ///
    /// > All page-level compositing shall be done in the default blending colour space of the
    /// > page, and the entire result shall then, if the colour spaces are not equivalent, be
    /// > converted to the native colour space of the output device before being composited
    /// > with the context-dependent backdrop.
    ///
    /// A page whose blending space has four components is drawn in those two orders' *agreeing*
    /// form since ADR 0262: the page is interpreted twice, once per half of the four, and
    /// `pdf_render::blending` converts the pair where the clause puts the conversion. Whose
    /// four they are is the document's since ADR 0272 — [`super::press::page_press`] is that reading. This
    /// fires for what is left — a page whose space is not four components this tree can sample,
    /// or one where the four would not answer the question — and only on the pass that
    /// composites on the device, since the subtractive passes are the ones that do not depart.
    ///
    /// Conditioned on something compositing, for [`Interpreter::note_group_departures`]'
    /// reason: an opaque `Normal` paint carries its colour through whatever space it is
    /// carried through, so a page of them is the same page in either.
    pub(in crate::content) fn note_page_blending_space(&mut self) {
        if self.compositing != Compositing::Device {
            return;
        }
        let Some(departure) = self.blending.clone() else {
            return;
        };
        // The compositing condition exempts a space whose conversion has an inverse, and a
        // one-component space has none — see [`Departure::loses_chroma`]. Applying the
        // exemption to every space would be trap 11's shape: a condition derived for four
        // components inherited by one (ADR 0790).
        if !departure.loses_chroma() && !any_command(self.list.commands(), &command_composites) {
            return;
        }
        let because = self.blending_undrawable().unwrap_or_else(|| {
            if departure.loses_chroma() {
                BeyondPress::stated(
                    "its one component is neither device grey's channel nor one this tree can \
                     composite through a curve with an inverse — §11.3.4 lists DeviceGray, \
                     CalGray and ICCBased 'GRAY' (§8.6.5.2, §8.6.5.5) — so §11.6.6 has no \
                     conversion into it and §11.3.4 none back out",
                )
            } else if departure.components == 3 {
                BeyondPress::stated(
                    "its three components are not ones this tree can composite through a cube \
                     with a way in — §11.3.4 lists CalRGB and a bi-directional ICCBased 'RGB ' \
                     (§8.6.5.3, §8.6.5.5) and forbids Lab — so §11.6.6 has no conversion into \
                     it and §11.3.4 none back out",
                )
            } else {
                BeyondPress::stated(
                    "its components are not four this tree can sample into a press, so §11.3.4 \
                     has no per-component formula to apply and no conversion back out",
                )
            }
        });
        self.note(Unsupported::TransparencyGroup {
            detail: format!(
                "the page group's blending colour space {} (§11.4.7): {}",
                departure.name, because.why
            ),
        });
    }

    /// Why this page cannot be drawn in the blending space it states, or `None` if it can.
    ///
    /// Two conditions, each of which is a *different* clause asking for something the pair of
    /// rasters does not carry, and each named rather than folded into the other. Both want a
    /// **second colour space** — one the document names, one a group introduces.
    ///
    /// **§11.3.5.3's rule for the black component under Table 135's four modes is not a
    /// condition, because it is not a second colour space at all**: it is drawn rather than
    /// reported (ADR 0277), and nothing was written for it — the clause splits
    /// a subtractive space's four components along the same line the two rasters already are,
    /// its chromatic bullet is what [`crate::colour::Plane::Chromatic`] holds, and the rule it
    /// gives the black component is what its own four functions return on the neutral colour
    /// [`crate::colour::Plane::Black`] holds. `render-cpu`'s `blend` module has the derivation.
    /// **The order among them is not a reading rule any more, and ADR 0417 is why.** ADR 0416
    /// had to put a file-stated reason in front of the one [`crate::colour::MAX_PRESSES`]
    /// supplied, because that one was a fact about the process and reporting it in place of
    /// what the page says about itself made a verdict that moved between runs. Every reason
    /// here is the document's now, so the order is back to the plain one — the space itself
    /// first, then what a group inside did to it — and either of them is the same answer on
    /// every run.
    ///
    /// **§11.7.5.3's black generation is not a condition either, since it is not a second colour
    /// space** (ADR 1069). Table 57's `/BG` and `/UCR` are §10.4.2.4's parameters, and §10.4.2.1
    /// puts §10.4.2's whole branch below §10.3's, where this tree's conversion into a press is — so
    /// the functions have no step of it to act on, and falling back to the device did not evaluate
    /// them either. It only cost the page the space §11.4.7 requires.
    /// [`Interpreter::note_black_generation_departure`] is the departure that is left.
    pub(in crate::content) fn blending_undrawable(&self) -> Option<BeyondPress> {
        if let Some(beyond) = self.blending_beyond {
            return Some(beyond);
        }
        if self.blending_changed {
            return Some(BeyondPress::stated(
                "a group inside it composites in a different space (§11.6.6), which needs a \
                 conversion between the two at its Do",
            ));
        }
        self.blending_beyond
    }

    /// Says where a stated black generation reached a conversion that did not use it.
    ///
    /// ISO 32000-2 §11.7.5.3 states the pair's three moments and this tree carries out the
    /// first two. The first: a `DeviceRGB` colour painted into a group whose colour space is
    /// `DeviceCMYK` is separated by §10.4.2.4 with the file's own functions
    /// (`ColourSpace::to_cmyk_under`, ADR 1207). The second: a group this tree composites on
    /// the device's three components painted into a four-component parent converts through
    /// [`parent_channels`], which takes the pair in effect at the `Do` (ADR 1242).
    ///
    /// One departure is left, and the condition names it rather than the statement alone: **a
    /// press whose own profile states the conversion in.** §8.6.5.5 requires a blending space's
    /// profile to carry "from CIE" information and this tree uses it (ADR 0796), so the black
    /// generation there is the document's own measured transform rather than the device default
    /// §10.4.2.4's last paragraph asks for — which is what a stated pair substitutes for.
    /// Replacing a `B2A` table with §10.4.2.4's arithmetic would convert into a profile's space
    /// by a formula that never reads the profile.
    ///
    /// The third bullet is neither: it conditions on "the native colour space of the output
    /// device" being `DeviceCMYK`, and this device's is sRGB (ADR 0009).
    ///
    /// Raised once per interpretation, the statement being monotone for the page: the parameters
    /// apply wherever a conversion happens rather than only where a `gs` set them.
    pub(in crate::content) fn note_black_generation_departure(&mut self) {
        if !self.black_generation_stated {
            return;
        }
        if !matches!(
            &self.compositing,
            Compositing::Subtractive(_, press, _) if press.converts_in_by_profile()
        ) {
            return;
        }
        self.note(Unsupported::BlackGeneration {
            detail: "an /ExtGState or a pattern states Table 57's /BG, /BG2, /UCR or /UCR2 as a \
                     function of its own, and this page composites in a press sampled from the \
                     document's own profile, whose B2A table is the conversion in (§8.6.5.5), so \
                     the stated pair reaches no step of it (§11.7.5.3). A DeviceRGB colour \
                     painted into a DeviceCMYK group, and a group painted into a DeviceCMYK \
                     parent, are separated with the stated functions and are not affected"
                .to_owned(),
        });
    }
}
