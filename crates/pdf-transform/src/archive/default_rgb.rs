//! A `/DefaultRGB` naming the ICC sRGB profile, where the profile in hand is of another family
//! (`doc/adr/2114`).
//!
//! # The licence
//!
//! ISO 19005-2 section 6.2.4.3 and ISO 19005-4 section 6.2.4.3 admit `DeviceRGB` in two ways: a
//! device-independent `DefaultRGB` set where the space is used, or a PDF/A output intent holding
//! an RGB destination profile. Section 6.2.3 of each lets a file state one destination profile,
//! so a file whose own intent is a press's CMYK profile — or whose only unlicensed device colour
//! the shipped CMYK profile answered — cannot license its `DeviceRGB` through the intent at all.
//! The first licence is the one left, and it is the same for both parts.
//!
//! The same licence reaches a transparency group's `CS`: ISO 19005-2 section 6.2.10 and ISO
//! 19005-4 section 6.2.9 hold that entry to section 6.2.4, and ISO 32000-2 §11.6.6's Table 145
//! makes a group's device colour space subject to remapping by the `DefaultGray`, `DefaultRGB`
//! and `DefaultCMYK` entries of the current resource dictionary's `ColorSpace` subdictionary. It reaches an `Indexed` or a `Pattern` base and a `Separation` or
//! `DeviceN` alternate the same way, since §8.6.5.6 remaps a device space wherever one is the
//! space of the operation being performed.
//!
//! # Why sRGB states what this reader drew
//!
//! Nothing in the file says what its `DeviceRGB` meant, and ISO 32000-2 §8.6.4.1 gives the device
//! spaces no colorimetric meaning; sRGB is this program's documented convention for that silence
//! (`doc/pdf-a-conversion-limits.md` section 4.1), the one the shipped output intent already
//! states. On this reader it is also what the page showed: `pdf_colour` lets an output intent
//! stand in only for the family with its own number of components, so a CMYK intent left
//! `DeviceRGB` drawn as the screen's sRGB, and the shipped sRGB profile read as an `ICCBased`
//! source is the identity to one level of 255 (`pdf_colour::icc`'s
//! `the_shipped_srgb_profile_is_the_identity_through_the_profile_route`). What changes is what the
//! file asserts; a reader that had shown the same values through some other device sees them as
//! sRGB now, which is why the decision is `Decision::Stated` and carries [`REINTERPRETS`].
//!
//! # Where it goes
//!
//! Wherever the `/DefaultCMYK` would go: every resource dictionary explicitly associated with a
//! content stream, through the same structural walk (`super::rewrite`'s `DefaultSites`), and never
//! over an entry the producer wrote.

use pdf_archive::survey::{DeviceFamily, SpaceKind, Survey};
use pdf_syntax::Document;
use pdf_syntax::object::{Name, Object, ObjectId};

use super::decision::Because;
use super::prepare::{Intent, Spare};

/// The `/DefaultRGB` a conversion is in a position to write, and the objects it needs.
#[derive(Debug)]
pub(super) struct DefaultRgb {
    /// The colour space array, in the *source's* numbering.
    pub(super) space: ObjectId,
    /// The two objects this conversion adds for it: the profile stream and the array naming it.
    pub(super) written: [(ObjectId, Object); 2],
}

/// What writing the `/DefaultRGB` asserts, which `doc/adr/0927` makes travel with the decision.
pub(super) const REINTERPRETS: &str = "every DeviceRGB value in this file, and every \
     transparency group blending in DeviceRGB, is now read through the ICC sRGB profile, because \
     this file's destination profile is of another colour family and ISO 19005 section 6.2.4.3's \
     other licence for DeviceRGB is a device independent DefaultRGB. This renderer already showed \
     those colours as sRGB, to within one level of 255; a reader that had shown them through some \
     other device shows them as sRGB now";

/// Why no `/DefaultRGB` is written where the profile in hand is an RGB one.
const BY_THE_INTENT: &str = "the destination profile in hand is an RGB one, which licenses \
     DeviceRGB itself, so no DefaultRGB is written";

/// Why no `/DefaultRGB` is written for a document whose content selects no `DeviceRGB`.
const NOTHING_IN_RGB: &str = "nothing this document draws selects DeviceRGB, so a DefaultRGB \
     would answer nothing; what failed is another family's, which the output intent answers or \
     the output's own verdict refuses";

/// Why the objects could not be numbered.
const NO_SPARE_OBJECT: &str = "no unused object number was found for the DefaultRGB's profile \
     stream and colour space array";

/// Whether the content this survey walked selects `DeviceRGB` with no `DefaultRGB` in force, as
/// a colour or as a transparency group's `CS`.
///
/// In force under the stricter of the two readings `pdf_archive::survey::DefaultSpace` keeps,
/// the explicitly associated dictionary's, which is `None` wherever the other is.
///
/// **Read off the survey rather than off the failed rows**, because six of the rows a
/// `/DefaultRGB` can answer — the group, the base and the alternate rows of both parts — state no
/// family: which family failed is the document's, and the survey is where the validator found it.
pub(super) fn draws_in_rgb(survey: &Survey) -> bool {
    survey
        .device_colours()
        .iter()
        .any(|colour| colour.family == DeviceFamily::Rgb && colour.default.explicit.is_none())
        || survey.group_spaces().iter().any(|group| {
            group.kind == SpaceKind::Device(DeviceFamily::Rgb) && group.default.explicit.is_none()
        })
}

/// Prepares the `/DefaultRGB`: the shipped sRGB profile as a stream, and the `ICCBased` array.
pub(super) fn prepare(
    document: &Document,
    intent: &Intent,
    survey: Option<&Survey>,
    spare: &mut Spare,
) -> Result<DefaultRgb, Because> {
    if intent.family == DeviceFamily::Rgb {
        return Err(Because::NotBuiltYet(BY_THE_INTENT));
    }
    if !survey.is_some_and(draws_in_rgb) {
        return Err(Because::NotBuiltYet(NOTHING_IN_RGB));
    }
    let missing = Because::NotBuiltYet(NO_SPARE_OBJECT);
    let profile = spare.take(document).ok_or(missing)?;
    let at = spare.take(document).ok_or(missing)?;
    let icc_based = Object::Array(vec![
        Object::Name(Name::new(&b"ICCBased"[..])),
        Object::Reference(profile),
    ]);
    Ok(DefaultRgb {
        space: at,
        written: [
            (
                profile,
                super::prepare::profile_stream(super::prepare::SRGB, DeviceFamily::Rgb),
            ),
            (at, icc_based),
        ],
    })
}
