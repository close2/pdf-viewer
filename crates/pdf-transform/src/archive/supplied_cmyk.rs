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
//! # Where it goes
//!
//! Wherever the other two defaults go: every resource dictionary explicitly associated with a
//! content stream, through `super::rewrite`'s `DefaultSites`, and never over an entry the
//! producer wrote.

use pdf_archive::survey::{DeviceFamily, SpaceKind, Survey};
use pdf_model::icc::Identification;
use pdf_syntax::Document;
use pdf_syntax::object::{Name, Object, ObjectId};

use super::decision::Because;
use super::prepare::{Intent, Spare};

/// The `/DefaultCMYK` a conversion is in a position to write over the operator's profile.
#[derive(Debug)]
pub(super) struct SuppliedCmyk {
    /// The colour space array, in the *source's* numbering.
    pub(super) space: ObjectId,
    /// The two objects this conversion adds for it: the profile stream and the array naming it.
    pub(super) written: [(ObjectId, Object); 2],
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

/// Why the default is not written where the packet will not carry its recorded action.
///
/// RFC 0007 section 5b.1's condition on a `supply`: the operator's statement is recorded as
/// theirs in the report and in the file's own provenance, and a file that cannot carry the second
/// does not get the statement.
pub(super) const NO_PLACE_TO_RECORD: &str = "the DefaultCMYK over the profile you supplied is      your statement about this file's colours, and it is written only where this file's own      xmpMM:History records it as yours — this document's XMP packet will not take that entry, so      the default is not written either";

/// Why no default is written where the profile in hand is a CMYK one.
const BY_THE_INTENT: &str = "the destination profile in hand is a CMYK one, which licenses \
     DeviceCMYK itself, so no DefaultCMYK is written";

/// Why a `DeviceCMYK` beside a profile of another family is refused when nothing was supplied.
///
/// The sentence the operator reads, so it says what they can do: the only licence this file has
/// left needs a press, and the press is theirs to name.
const NO_PRESS_SUPPLIED: &str = "this file draws in DeviceCMYK and its destination profile is of \
     another colour family. ISO 19005 section 6.2.3 lets a \
     file state one destination profile, so the licence left is a device independent \
     DefaultCMYK (ISO 19005 section 6.2.4.3) - an ICC CMYK profile, which states a press, and \
     nothing in this file says which press its colours were separated for. Supply that press's \
     profile with --output-intent-profile and it is written as the DefaultCMYK, recorded as \
     your statement";

/// Why a supplied profile of another family does not answer the clause.
const SUPPLIED_NOT_CMYK: &str = "this file draws in DeviceCMYK, or blends a transparency group in \
     it, beside a destination profile of another colour family, and the profile supplied with \
     --output-intent-profile is not a CMYK one either. ISO 32000-2 \u{a7}8.6.5.6 requires a \
     DefaultCMYK to have four components, so only a CMYK press profile answers it";

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

/// Prepares the `/DefaultCMYK`: the operator's profile as a stream, and the `ICCBased` array.
pub(super) fn prepare(
    document: &Document,
    intent: &Intent,
    supplied: Option<&[u8]>,
    survey: Option<&Survey>,
    spare: &mut Spare,
) -> Result<SuppliedCmyk, Because> {
    if intent.family == DeviceFamily::Cmyk {
        return Err(Because::NotBuiltYet(BY_THE_INTENT));
    }
    // The profile before the survey, so that a conversion nobody supplied a press for walks no
    // content stream to be told so: part 4's `DeviceCMYK` row, the one that reads this sentence as
    // its refusal, failed because the file draws in `DeviceCMYK`.
    let profile = supplied.ok_or(Because::NotBuiltYet(NO_PRESS_SUPPLIED))?;
    let cmyk = Identification::read(profile).is_some_and(|stated| stated.space == *b"CMYK");
    if !cmyk {
        return Err(Because::NotBuiltYet(SUPPLIED_NOT_CMYK));
    }
    if !survey.is_some_and(draws_in_cmyk) {
        return Err(Because::NotBuiltYet(NOTHING_IN_CMYK));
    }
    let missing = Because::NotBuiltYet(NO_SPARE_OBJECT);
    let stream = spare.take(document).ok_or(missing)?;
    let at = spare.take(document).ok_or(missing)?;
    let icc_based = Object::Array(vec![
        Object::Name(Name::new(&b"ICCBased"[..])),
        Object::Reference(stream),
    ]);
    Ok(SuppliedCmyk {
        space: at,
        written: [
            (
                stream,
                super::prepare::profile_stream(profile, DeviceFamily::Cmyk),
            ),
            (at, icc_based),
        ],
    })
}
