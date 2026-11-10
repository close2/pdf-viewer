//! A `/DefaultCMYK` naming the press profile the operator supplied, where the file's own output
//! intent is of another family (`doc/adr/2130`).
//!
//! # The licence
//!
//! ISO 19005-2 section 6.2.4.3 and ISO 19005-4 section 6.2.4.3 admit `DeviceCMYK` through a
//! PDF/A output intent holding a CMYK destination profile, or through a device-independent
//! `DefaultCMYK` set where the space is used; part 2 alone adds the `DeviceN`-based default its
//! NOTE 2 makes device independent. Section 6.2.3 of each lets a file state one destination
//! profile, so a file whose own intent is an RGB profile cannot license its `DeviceCMYK` through
//! the intent at all, and §11.6.6's Table 145 excludes a `DeviceN` space as a group's colour space
//! — so for a group blending in `DeviceCMYK` at part 2, and for every `DeviceCMYK` use at part 4,
//! the licence left is an `ICCBased` CMYK default.
//!
//! # Why the profile is the operator's
//!
//! An ICC CMYK profile states a press, and nothing in the file says which press its `DeviceCMYK`
//! was separated for; ISO 32000-2 §8.6.4.1 gives the device spaces no colorimetric meaning. This
//! reader drew those colours through its assumed inks (`pdf_colour::assumed_press`), which no ICC
//! profile states, so writing the shipped press profile would be a statement this reader's own
//! page contradicts (ADR 2114 section 4). The operator who supplies `--output-intent-profile`
//! with a CMYK profile has made the statement instead: that profile is the press the document
//! was made for. Where the file names no output intent the same profile becomes the intent; where
//! the file's own intent is of another family it becomes this default, and nothing else in the
//! file changes. So the decision is `Decision::Stated` and carries [`REINTERPRETS`], the report
//! names the profile as the operator's, and the file's own `xmpMM:History` records it.
//!
//! # And where the operator names none
//!
//! Under a profile that authorises [`Loss::PressAssumed`](super::decision::Loss::PressAssumed) —
//! `as-if-printed` does — a part 4 file with `DeviceCMYK` beside an RGB intent takes the CMYK
//! profile this program ships as the press instead of being refused. The profile is still a
//! statement the file's own bytes do not make, so it is a named loss rather than a
//! `Decision::Stated`, reported with the profile's own description and recorded in the file's
//! `xmpMM:History`. Part 2 is offered it only for a transparency group blending in `DeviceCMYK`:
//! its other licence, the `DeviceN` default, states no press at all (`doc/adr/2518`), and ISO
//! 32000-2 §11.6.6's Table 145 forbids that default as a group's colour space (`doc/adr/2795`).
//!
//! # And where nobody accepts an assumed press
//!
//! Every other profile at part 4 — `as-shown`, `no-data-loss`, `state-every-loss`, and a run with
//! none — takes the profile of this renderer's own assumed inks (`doc/adr/2700`) as the default.
//! With no CMYK intent this reader draws an uncharacterised `DeviceCMYK` value through those inks,
//! and the profile's `A2B` is that drawing sampled, so the default states what the window already
//! showed rather than a press the file never named: a `Decision::Stated` carrying
//! [`INKS_REINTERPRETS`], as the same profile is when it is the file's CMYK intent
//! (`doc/adr/2701`). It revisits `doc/adr/2114` section 4, whose refusal rested on no ICC profile
//! stating those inks (`doc/adr/2747`).
//!
//! # Where it goes
//!
//! Wherever the other two defaults go: every resource dictionary explicitly associated with a
//! content stream, through `super::rewrite`'s `DefaultSites`, and never over an entry the
//! producer wrote.

use super::prepare::{DestinationProfile, ProfileSource};
use pdf_archive::survey::{DeviceFamily, ProcessCmykOverprint, SpaceKind, Survey};
use pdf_model::icc::Identification;
use pdf_syntax::Document;
use pdf_syntax::object::{Name, Object, ObjectId};

use super::decision::Because;
use super::prepare::{Intent, Spare};

/// What a `/DefaultCMYK` is prepared to answer, which differs by part (`doc/adr/2795`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reach {
    /// ISO 19005-4: every `DeviceCMYK` use, and `zero_modes` is whether the caller authorised
    /// [`Loss::OverprintMode`](super::decision::Loss::OverprintMode).
    PartFour {
        /// Whether a mark's overprint mode 1 may be written 0 beside the default.
        zero_modes: bool,
    },
    /// ISO 19005-2: a stand-in for a supply answers only a group blending in `DeviceCMYK`.
    PartTwo,
}

/// Whose statement of the press a `/DefaultCMYK` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Press {
    /// The profile the operator supplied with `--output-intent-profile`.
    Supplied,
    /// The CMYK profile this program ships, taken where the operator named none and the caller
    /// authorised [`Loss::PressAssumed`](super::decision::Loss::PressAssumed).
    Assumed,
    /// The press the operator's configuration names once, taken where [`Self::Assumed`] would be
    /// and costing the same loss: nothing in this file names its press, and this renderer drew
    /// its colours through its own inks, so they move whoever chose the press (`doc/adr/2705`).
    Configured,
    /// The profile of this renderer's own assumed inks (`doc/adr/2700`), taken at part 4 where
    /// nothing was supplied and the caller did not authorise
    /// [`Loss::PressAssumed`](super::decision::Loss::PressAssumed): it states the colours this
    /// reader drew, so it is a statement and no loss (`doc/adr/2747`).
    Inks,
}

impl Press {
    /// Whether the press is assumed for this file rather than stated for it, which is
    /// [`Loss::PressAssumed`](super::decision::Loss::PressAssumed)'s condition.
    pub(super) const fn is_assumed(self) -> bool {
        matches!(self, Self::Assumed | Self::Configured)
    }
}

/// The `/DefaultCMYK` a conversion is in a position to write over the operator's profile.
#[derive(Debug)]
pub(super) struct SuppliedCmyk {
    /// Whose press it states.
    pub(super) press: Press,
    /// The colour space array, in the *source's* numbering.
    pub(super) space: ObjectId,
    /// The two objects this conversion adds for it: the profile stream and the array naming it.
    pub(super) written: [(ObjectId, Object); 2],
    /// The graphics states whose overprint mode 1 is written 0 beside the default, because the
    /// `DeviceCMYK` marks they govern become `ICCBased` ones under it: empty unless the caller
    /// authorised [`Loss::OverprintMode`](super::decision::Loss::OverprintMode) (`doc/adr/2749`).
    pub(super) modes: super::overprint::OverprintModes,
}

/// What writing the `/DefaultCMYK` asserts, which `doc/adr/0927` makes travel with the decision.
pub(super) const REINTERPRETS: &str = "every DeviceCMYK value in this file, and every \
     transparency group blending in DeviceCMYK, is now read through the CMYK profile you \
     supplied with --output-intent-profile, because this file's own destination profile is of \
     another colour family and ISO 19005 section 6.2.4.3's other licence for DeviceCMYK is a \
     device independent DefaultCMYK. That profile is your statement of the press this document \
     was made for: this renderer drew those colours through its own assumed inks, which no ICC \
     profile states, so they now look as that press prints them, on this reader and every other";

/// The `parameters` field of the action this conversion records in `xmpMM:History` when it
/// writes this default, saying whose statement the profile is.
///
/// The action is [`super::prepare::DEFAULT_CMYK_ACTION`]'s, for its reason: ISO 19005-2 section
/// 6.6.6 asks a recorded action for what was done, with what, and when.
pub(super) const PARAMETERS: &str = "an ICCBased DefaultCMYK was added naming the CMYK profile \
     the operator of this conversion supplied as the press this file's DeviceCMYK was made for; \
     the file's own output intent is of another colour family and was left as it was";

/// The `parameters` field of the action recorded when the press is the one this program ships,
/// naming it by the description its own `desc` tag states.
pub(super) const ASSUMED_PARAMETERS: &str = "an ICCBased DefaultCMYK was added over \
     GRACoL2006_Coated1v2.icc, the CMYK profile this program ships, as an assumed press (the loss \
     press-assumed): nothing in the file names the press its DeviceCMYK colours were made for \
     and no operator supplied one, so these colours are now read as that profile prints them, \
     which may not be the press the producer used; the file's own output intent is of another \
     colour family and was left as it was";

/// [`ASSUMED_PARAMETERS`] where the press is the one the operator's configuration names: the same
/// loss, and the press named as theirs (`doc/adr/2705`).
pub(super) const CONFIGURED_PARAMETERS: &str = "an ICCBased DefaultCMYK was added over the CMYK \
     press the operator of this conversion names in their configuration, as an assumed press (the \
     loss press-assumed): nothing in the file names the press its DeviceCMYK colours were made \
     for, so these colours are now read as the operator's press prints them, which may not be the \
     press the producer used; the file's own output intent is of another colour family and was \
     left as it was";

/// What writing the default over the assumed inks asserts (`doc/adr/2747`).
pub(super) const INKS_REINTERPRETS: &str = "every DeviceCMYK value in this file, and every \
     transparency group blending in DeviceCMYK, is now read through an ICC profile of the \
     process inks this renderer assumes for colour no profile characterises, because this \
     file's own destination profile is of another colour family and ISO 19005 section 6.2.4.3's \
     other licence for DeviceCMYK is a device independent DefaultCMYK. This renderer already \
     drew those colours through those inks, so they look as they did here; the profile \
     characterises this renderer and not a physical press, and another reader that follows it \
     shows them as this one does";

/// The `parameters` field of the action recorded when the default is the assumed inks' profile.
pub(super) const INKS_PARAMETERS: &str = "an ICCBased DefaultCMYK was added over the profile of \
     the process inks this renderer assumes for DeviceCMYK colour no profile characterises, the \
     inks it drew this file's DeviceCMYK colours through; the profile characterises this \
     renderer and not a physical press, and the file's own output intent is of another colour \
     family and was left as it was";

/// Why the default over the assumed inks is not written where the packet will not carry its
/// recorded action.
pub(super) const INKS_NO_PLACE_TO_RECORD: &str = "the DefaultCMYK over the profile of this \
     renderer's assumed inks is written only where this file's own xmpMM:History records it - \
     this document's XMP packet will not take that entry, so the default is not written either";

/// Why the default over the operator's configured press is not written where the packet will not
/// carry its recorded action.
pub(super) const CONFIGURED_NO_PLACE_TO_RECORD: &str = "the DefaultCMYK over the press your \
     configuration names is written only where this file's own xmpMM:History records that the \
     press was assumed - this document's XMP packet will not take that entry, so the default is \
     not written either";

/// Why the default over the shipped press is not written where the packet will not carry its
/// recorded action: the loss is named in the file or it is not taken.
pub(super) const ASSUMED_NO_PLACE_TO_RECORD: &str = "the DefaultCMYK over the CMYK profile this \
     program ships is written only where this file's own xmpMM:History records that the press \
     was assumed - this document's XMP packet will not take that entry, so the default is not \
     written either";

/// The assumed press as the report states it: the shipped one, or the operator's configured one.
pub(super) fn assumed_profile(plan: &super::ArchivePlan) -> Option<DestinationProfile> {
    let (source, bytes) = plan
        .press
        .as_deref()
        .map_or((ProfileSource::Shipped, super::prepare::CMYK), |press| {
            (ProfileSource::Configured, press)
        });
    let stated = Identification::read(bytes)?;
    Some(DestinationProfile {
        source,
        space: stated.space_name(),
        describes: stated.description,
        copyright: stated.copyright,
    })
}

/// Why the default is not written where the packet will not carry its recorded action.
///
/// RFC 0007 section 5b.1's condition on a `supply`: the operator's statement is recorded as
/// theirs in the report and in the file's own provenance, and a file that cannot carry the second
/// does not get the statement.
pub(super) const NO_PLACE_TO_RECORD: &str = "the DefaultCMYK over the profile you supplied is \
     your statement about this file's colours, and it is written only where this file's own \
     xmpMM:History records it as yours — this document's XMP packet will not take that entry, so \
     the default is not written either";

/// Why no default is written where the profile in hand is a CMYK one.
const BY_THE_INTENT: &str = "the destination profile in hand is a CMYK one, which licenses \
     DeviceCMYK itself, so no DefaultCMYK is written";

/// Why no ICC `DefaultCMYK` is prepared at part 2 when nothing was supplied.
///
/// Part 4 always has a profile in hand (`assumed`), so this is part 2's, whose `DeviceCMYK` row
/// reads the `DeviceN` default's answer instead; the sentence says what the operator can do.
const NO_PRESS_SUPPLIED: &str = "this file draws in DeviceCMYK and its destination profile is of \
     another colour family. ISO 19005 section 6.2.3 lets a file state one destination profile, \
     so the licences left are a device independent DefaultCMYK (ISO 19005 section 6.2.4.3) - \
     an ICC CMYK profile, which states a press - and, at part 2, the DeviceN-based default. \
     Supply a press's profile with --output-intent-profile and it is written as the \
     DefaultCMYK, recorded as your statement";

/// Why a supplied profile of another family does not answer the clause.
const SUPPLIED_NOT_CMYK: &str = "this file draws in DeviceCMYK, or blends a transparency group in \
     it, beside a destination profile of another colour family, and the profile supplied with \
     --output-intent-profile is not a CMYK one either. ISO 32000-2 \u{a7}8.6.5.6 requires a \
     DefaultCMYK to have four components, so only a CMYK press profile answers it";

/// Why no `DefaultCMYK` is written for a file whose `DeviceCMYK` marks overprint under a mode 1
/// no rewrite of this conversion sets to 0, or whose content walk a bound stopped before it could
/// say that none do.
///
/// A `DefaultCMYK` over an ICC profile makes every `DeviceCMYK` colour an `ICCBased` CMYK one
/// (ISO 32000-2 §8.6.5.6), and ISO 19005-2 and ISO 19005-4 section 6.2.4.2 forbid overprint mode 1
/// for an `ICCBased` CMYK mark with overprinting on. Writing the default would leave the file
/// failing that clause, for the operator's press as for the assumed one, so neither is written
/// and the refusal is this one rather than the output's own verdict (`doc/adr/2518`,
/// `doc/adr/2541`). Where the caller authorised the loss and every state can be reached, the
/// states are written 0 instead (`doc/adr/2749`).
const OVERPRINT_MODE_ONE: &str = "this file paints in DeviceCMYK with overprinting on and overprint \
     mode 1, or its content could not be read to the end to say that it does not, and a \
     DefaultCMYK over an ICC profile makes those marks ICCBased CMYK marks, for which ISO 19005 \
     section 6.2.4.2 forbids overprint mode 1. No DefaultCMYK is written for it, whatever press \
     is named: the mode is set by a graphics state this conversion does not set to 0. No \
     ICCBased CMYK mark of the file's own asks for it, and setting it to 0 beside the default, \
     the loss overprint-mode, is either not authorised or not possible - the state is written \
     inside a resource dictionary, or the mark is blended in four components, where this \
     reader's window would move (doc/adr/2749). A file left with the mode would not conform";

/// Why no default is written for a document whose content selects no `DeviceCMYK`.
const NOTHING_IN_CMYK: &str = "nothing this document draws selects DeviceCMYK, so a DefaultCMYK \
     would answer nothing; what failed is another family's, which the output intent answers or \
     the output's own verdict refuses";

/// Why the objects could not be numbered.
const NO_SPARE_OBJECT: &str = "no unused object number was found for the DefaultCMYK's profile \
     stream and colour space array";

/// Whether the content this survey walked selects `DeviceCMYK` with no `DefaultCMYK` in force,
/// as a colour or as a transparency group's `CS`.
///
/// The stricter of `pdf_archive::survey::DefaultSpace`'s two readings, as
/// [`super::default_rgb::draws_in_rgb`] reads it for the same reason.
pub(super) fn draws_in_cmyk(survey: &Survey) -> bool {
    survey
        .device_colours()
        .iter()
        .any(|colour| colour.family == DeviceFamily::Cmyk && colour.default.explicit.is_none())
        || survey.group_spaces().iter().any(|group| {
            group.kind == SpaceKind::Device(DeviceFamily::Cmyk) && group.default.explicit.is_none()
        })
}

/// Whether a transparency group this survey walked blends in `DeviceCMYK` with no `DefaultCMYK`
/// in force: the one use part 2's `DeviceN` default cannot answer, because ISO 32000-2 §11.6.6's
/// Table 145 excludes `DeviceN` from a group's colour space, and remaps a device one through the
/// defaults of the current resource dictionary, so a `DeviceN` default there would make the group
/// blend in a space the table forbids.
pub(super) fn blends_in_cmyk(survey: &Survey) -> bool {
    survey.group_spaces().iter().any(|group| {
        group.kind == SpaceKind::Device(DeviceFamily::Cmyk) && group.default.explicit.is_none()
    })
}

/// Whether a `DeviceCMYK` mark painted under a non-zero overprint mode leaves that mode only
/// through the graphics state the file's own `ICCBased` CMYK marks already ask
/// `overprint::zeroing` to set to 0.
///
/// That function zeroes every state in [`Survey::icc_overprint_mode_sources`] and refuses inside
/// a group blending in four components, so a mark governed by one of those states and blended
/// in three is one a written default leaves under mode 0; any other keeps mode 1 once the
/// default makes it an `ICCBased` mark.
fn zeroed_with_the_icc_marks(survey: &Survey, mark: &ProcessCmykOverprint) -> bool {
    !mark.in_four_components
        && mark
            .mode_set_by
            .is_some_and(|id| survey.icc_overprint_mode_sources().contains(&Some(id)))
}

/// Prepares the `/DefaultCMYK`: the operator's profile as a stream, and the `ICCBased` array.
///
/// `assumed` is what stands in for a supply where the target is part 4, or part 2 where a group's
/// `CS` failed, and whose statement it is: the operator's configured press or the profile this program ships where the caller
/// authorised [`Loss::PressAssumed`](super::decision::Loss::PressAssumed), and the assumed inks'
/// profile otherwise. It is read only when the operator supplied none: a supply outranks it,
/// including one that is not a CMYK profile.
///
/// `reach` is what the default is written to answer: at part 4 every `DeviceCMYK` use, with
/// `zero_modes` saying whether the caller authorised [`Loss::OverprintMode`](super::decision::Loss::OverprintMode),
/// where the default is the only licence for `DeviceCMYK` left: a `DeviceCMYK` mark overprinting
/// under mode 1 then has its graphics state's mode written 0 beside the default rather than
/// keeping the default from being written (`doc/adr/2749`). At part 2 the mode is never zeroed,
/// because its `DeviceN` default leaves the marks `DeviceCMYK` and costs nothing, and a stand-in
/// for a supply answers only a transparency group blending in `DeviceCMYK` ([`blends_in_cmyk`]);
/// a profile the operator supplied answers every use as at part 4 (`doc/adr/2795`).
pub(super) fn prepare(
    document: &Document,
    intent: &Intent,
    supplied: Option<&[u8]>,
    assumed: Option<(Press, &[u8])>,
    survey: Option<&Survey>,
    reach: Reach,
    spare: &mut Spare,
) -> Result<SuppliedCmyk, Because> {
    if intent.family == DeviceFamily::Cmyk {
        return Err(Because::NotBuiltYet(BY_THE_INTENT));
    }
    // The profile before the survey, so that a conversion nobody supplied a press for walks no
    // content stream to be told so: part 4's `DeviceCMYK` row, the one that reads this sentence as
    // its refusal, failed because the file draws in `DeviceCMYK`.
    let (press, profile) = match (supplied, assumed) {
        (Some(profile), _) => (Press::Supplied, profile),
        (None, Some(assumed)) => assumed,
        (None, None) => return Err(Because::NotBuiltYet(NO_PRESS_SUPPLIED)),
    };
    let cmyk = Identification::read(profile).is_some_and(|stated| stated.space == *b"CMYK");
    if !cmyk {
        return Err(Because::NotBuiltYet(SUPPLIED_NOT_CMYK));
    }
    // A stand-in at part 2 answers only a group, because a mark there has the `DeviceN` default.
    let (zero_modes, groups_only) = match reach {
        Reach::PartFour { zero_modes } => (zero_modes, false),
        Reach::PartTwo => (false, true),
    };
    let needed = if groups_only && supplied.is_none() {
        blends_in_cmyk
    } else {
        draws_in_cmyk
    };
    if !survey.is_some_and(needed) {
        return Err(Because::NotBuiltYet(NOTHING_IN_CMYK));
    }
    // A `DefaultCMYK` turns every `DeviceCMYK` mark under the mode into an `ICCBased` one, which
    // section 6.2.4.2 forbids the mode for. The default is written where each such mark's mode is
    // set by a graphics state `overprint::zeroing` sets to 0 for the file's own `ICCBased` marks,
    // or, where the caller authorised the loss `overprint-mode`, by one `overprint::under_a_default`
    // sets to 0 beside it; never past a walk a bound stopped before it could say there is no
    // other (`doc/adr/2518`, `doc/adr/2541`, `doc/adr/2749`).
    let modes = match survey {
        Some(s) if s.stopped_short() => return Err(Because::NotBuiltYet(OVERPRINT_MODE_ONE)),
        Some(s)
            if !s
                .process_cmyk_overprints()
                .iter()
                .all(|mark| zeroed_with_the_icc_marks(s, mark)) =>
        {
            zero_modes
                .then(|| super::overprint::under_a_default(document, s))
                .flatten()
                .ok_or(Because::NotBuiltYet(OVERPRINT_MODE_ONE))?
        }
        _ => super::overprint::OverprintModes::default(),
    };
    let missing = Because::NotBuiltYet(NO_SPARE_OBJECT);
    let stream = spare.take(document).ok_or(missing)?;
    let at = spare.take(document).ok_or(missing)?;
    let icc_based = Object::Array(vec![
        Object::Name(Name::new(&b"ICCBased"[..])),
        Object::Reference(stream),
    ]);
    Ok(SuppliedCmyk {
        press,
        space: at,
        written: [
            (
                stream,
                super::prepare::profile_stream(profile, DeviceFamily::Cmyk),
            ),
            (at, icc_based),
        ],
        modes,
    })
}
