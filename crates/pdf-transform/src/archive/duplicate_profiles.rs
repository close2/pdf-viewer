//! An `ICCBased` space whose profile is the CMYK profile already in force, named `DeviceCMYK`
//! (`doc/adr/2098`).
//!
//! # What the requirement forbids
//!
//! ISO 19005-4 section 6.2.4.2's last requirement forbids using an `ICCBased` space whose profile
//! is a CMYK destination profile identical to the current PDF/A output intent's or to the current
//! transparency blending space's, and section 6.2.4.4 sends the same restriction to a
//! `Separation` or `DeviceN` space's alternate. Its NOTE 2 gives the reason: the output would
//! otherwise depend on whether a renderer makes ISO 32000-2 §8.6.5.7's implicit conversion, which
//! that clause's NOTE 3 says "cannot be specified in PDF".
//!
//! # Why `DeviceCMYK` states what the file already said
//!
//! The profile is identical to the one in force, so the colour values are the same colour under
//! either name wherever the substitution is read through that same profile. Two clauses say where
//! that is:
//!
//! - **The output intent.** ISO 19005-4 section 6.2.4.3 has a conforming processor render a
//!   `DeviceCMYK` colour that no default space remaps through the current PDF/A output intent's
//!   profile as its source — the very profile the array named.
//! - **The blending space.** ISO 32000-2 §11.7.2:
//!
//!   > When converting colours, if the colour space of any graphics object is a device colour
//!   > space, and the current group or an ancestor of the current group is defined with a
//!   > CIE-based colour space with the same number of colourants, then, for compositing purposes
//!   > only, the colour space of the graphics object shall be the CIE-based space of the nearest
//!   > such ancestor.
//!
//!   So inside a group blending in the profile, a `DeviceCMYK` mark composites as that profile's
//!   components — the very components the array named.
//!
//! What a device space carries beyond its colour is the rest of §8.6.5.7's list, and the sentence
//! that follows §11.7.2's names the one that matters:
//!
//! > Nevertheless for graphics objects that are specified in DeviceCMYK , the full semantics of
//! > DeviceCMYK shall still apply, including overprinting.
//!
//! §8.6.7 says of overprint mode that "it shall have an effect only when the overprint parameter
//! is true", and with the parameter false a painting operation in a process colour space erases
//! the colourants it does not specify whichever space named it. So where nothing in the file turns
//! overprinting on, the name changes nothing §8.6.7 defines, and the decision §8.6.5.7 leaves to
//! the processor has no mark it could decide.
//!
//! # Where it is refused
//!
//! The substitution is made for every array naming the profile, so it is a statement only where
//! it is one at **every** place the profile was selected — a use under another page's output intent
//! of a different profile would change colour there. Refused by name:
//!
//! - a use where the output intent then current, or the `ICCBased` blending space then in force,
//!   holds a different profile, or where neither holds this one;
//! - a use outside an `ICCBased` blending space on a page stating any transparency group colour
//!   space, since the survey records the blending profile only where it is an `ICCBased` one, and an
//!   image's selection carries none — a `DeviceRGB` or `CalRGB` group would convert a device
//!   colour in a way §11.7.2 leaves to the implementation;
//! - a graphics state anywhere in the file turning overprinting on, whatever it governs — over-
//!   counted rather than under, because the survey records overprinting for an `ICCBased` mark
//!   but not for a `Separation` whose alternate is one;
//! - the profile named by a `DefaultCMYK` entry, which remaps `DeviceCMYK` itself, and an array
//!   object a transparency group's `CS` names as well, which would move the blending space;
//! - a content walk a bound stopped, whose list of uses may not be all of them.
//!
//! # The population, and why it is not a second content walk
//!
//! The uses come from the one survey the validator's findings come from, read by
//! `pdf_archive::cmyk_profile_uses` — the validator's own reading, so the two cannot drift. The
//! arrays rewritten are read off the objects, every `[/ICCBased …]` naming one of these profiles,
//! as `super::icc` reads the arrays an unreadable profile's alternate replaces. A group's `CS` is
//! the one entry left alone: it is the blending space a use was compared against, not a use.

use std::collections::BTreeSet;

use pdf_archive::survey::Survey;
use pdf_archive::{Beside, cmyk_profile_uses};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::decision::Because;
use super::sites::MAX_ENTRY_DEPTH;

/// Why a profile identical to one in force at one use is not collapsed where it differs at another.
const ANOTHER_PROFILE_IN_FORCE: &str = "an ICCBased CMYK space here carries the profile of the \
     output intent or blending space in force where it is used, and the same profile is used \
     somewhere the output intent or the ICCBased blending space then in force holds a different \
     profile, or neither holds this one. DeviceCMYK takes its colour from the output intent's \
     profile (ISO 19005-4 section 6.2.4.3) or from the blending space's (ISO 32000-2 11.7.2), so \
     naming it there would move that colour, and naming it only at the other uses would mean \
     siting each one from the content (doc/adr/2098)";

/// Why a use in a page with a group space this conversion cannot read is not collapsed.
const UNREAD_GROUP_SPACE: &str = "an ICCBased CMYK space here duplicates the output intent's \
     profile, and it is used on a page that states a transparency group colour space other than \
     the profile itself - or is used by an image, whose blending space the content walk does not \
     record. Inside a group in another space, DeviceCMYK is converted in a way ISO 32000-2 11.7.2 \
     leaves to the implementation, so naming it there may move the colour (doc/adr/2098)";

/// Why a file that turns overprinting on is not collapsed.
const OVERPRINTING_ON: &str = "an ICCBased CMYK space here duplicates the profile in force, and a \
     graphics state in this file turns overprinting on. ISO 32000-2 11.7.2 keeps the full semantics \
     of DeviceCMYK for a DeviceCMYK mark, overprinting included, and 8.6.7's overprint mode acts \
     on DeviceCMYK marks where overprinting is on - so naming DeviceCMYK could settle what \
     8.6.5.7 leaves to the processor, which is the dependence ISO 19005-4 section 6.2.4.2 \
     forbids. The state is counted wherever it is, used or not (doc/adr/2098)";

/// Why a profile remapping `DeviceCMYK`, or shared with a group's space, is not collapsed.
const DEFAULT_OR_GROUP_SPACE: &str = "an ICCBased CMYK space here duplicates the profile in force, \
     and an array naming that profile is a DefaultCMYK entry, which remaps DeviceCMYK itself, or \
     is an object a transparency group's CS names as well, which this rewrite would turn into the \
     blending space's own change. Neither is a use this rewrite can restate (doc/adr/2098)";

/// Why a failure this conversion did not find is not answered.
const NOT_REACHED: &str = "the requirement failed, and this conversion's own content walk either \
     stopped at one of its bounds or found no ICCBased array naming a profile identical to the \
     one in force where it is used, so a rewrite of what it found would leave the failure in \
     place (doc/adr/2098)";

/// The profiles whose `ICCBased` arrays become `DeviceCMYK`.
#[derive(Debug, Default)]
pub(super) struct Duplicates {
    /// The profile streams, by object.
    pub(super) profiles: BTreeSet<ObjectId>,
}

/// The profiles to collapse, from the one content walk, or why none is.
pub(super) fn collapse(document: &Document, survey: &Survey) -> Result<Duplicates, Because> {
    if survey.stopped_short() {
        return Err(Because::NotBuiltYet(NOT_REACHED));
    }
    let uses = cmyk_profile_uses(document, survey.icc_selections(), |_| true);
    let mut out = Duplicates::default();
    for used in &uses {
        if used.intent == Beside::Same || used.blending == Beside::Same {
            let id = used
                .selection
                .profile
                .id
                .ok_or(Because::NotBuiltYet(NOT_REACHED))?;
            out.profiles.insert(id);
        }
    }
    if out.profiles.is_empty() {
        return Err(Because::NotBuiltYet(NOT_REACHED));
    }
    let grouped: BTreeSet<usize> = survey
        .group_spaces()
        .iter()
        .map(|group| group.page)
        .collect();
    for used in &uses {
        if !used
            .selection
            .profile
            .id
            .is_some_and(|id| out.profiles.contains(&id))
        {
            continue;
        }
        let differs = used.intent == Beside::Different || used.blending == Beside::Different;
        let neither = used.intent != Beside::Same && used.blending != Beside::Same;
        if differs || neither {
            return Err(Because::NotBuiltYet(ANOTHER_PROFILE_IN_FORCE));
        }
        if used.blending == Beside::Absent && grouped.contains(&used.selection.page) {
            return Err(Because::NotBuiltYet(UNREAD_GROUP_SPACE));
        }
    }
    let mut walk = Walk {
        profiles: &out.profiles,
        arrays: 0,
        overprinting: false,
        shared: false,
    };
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        walk.visit(document, &document.get(id), None, 0);
    }
    if walk.overprinting {
        return Err(Because::NotBuiltYet(OVERPRINTING_ON));
    }
    if walk.shared {
        return Err(Because::NotBuiltYet(DEFAULT_OR_GROUP_SPACE));
    }
    if walk.arrays == 0 {
        return Err(Because::NotBuiltYet(NOT_REACHED));
    }
    Ok(out)
}

/// What one pass over the file's objects found.
struct Walk<'a> {
    /// The profiles being collapsed.
    profiles: &'a BTreeSet<ObjectId>,
    /// How many arrays naming one of them the rewrite will reach.
    arrays: usize,
    /// Whether a dictionary states `OP` or `op` true.
    overprinting: bool,
    /// Whether an array naming one of them is a `DefaultCMYK` value or an object a group's `CS`
    /// names.
    shared: bool,
}

impl Walk<'_> {
    /// One value, `key` being the dictionary entry it is the value of.
    fn visit(&mut self, document: &Document, object: &Object, key: Option<&[u8]>, depth: usize) {
        if depth >= MAX_ENTRY_DEPTH {
            return;
        }
        let deeper = depth.saturating_add(1);
        match (key, object) {
            (Some(b"DefaultCMYK"), value) if self.names_one(document, &document.resolve(value)) => {
                self.shared = true;
            }
            (Some(b"CS"), Object::Reference(_)) => {
                if self.names_one(document, &document.resolve(object)) {
                    self.shared = true;
                }
            }
            // A group's own space, which the rewrite leaves as the producer wrote it.
            (Some(b"CS"), _) => {}
            (_, Object::Array(items)) => {
                if self.names_one(document, object) {
                    self.arrays = self.arrays.saturating_add(1);
                }
                for item in items {
                    self.visit(document, item, None, deeper);
                }
            }
            (_, Object::Dictionary(dict)) => self.visit_dictionary(document, dict, deeper),
            (_, Object::Stream(stream)) => self.visit_dictionary(document, &stream.dict, deeper),
            _ => {}
        }
    }

    /// Every entry of one dictionary, after Table 57's two overprint entries are read.
    fn visit_dictionary(&mut self, document: &Document, dict: &Dictionary, depth: usize) {
        for key in ["OP", "op"] {
            if matches!(document.get_key(dict, key), Object::Boolean(true)) {
                self.overprinting = true;
            }
        }
        for (name, value) in dict.iter() {
            self.visit(document, value, Some(name.as_bytes()), depth);
        }
    }

    /// Whether a value is an `[/ICCBased stream]` array naming one of the profiles.
    fn names_one(&self, document: &Document, value: &Object) -> bool {
        match value {
            Object::Array(items) => {
                icc_profile_of(document, items).is_some_and(|id| self.profiles.contains(&id))
            }
            _ => false,
        }
    }
}

/// The profile stream an `[/ICCBased stream]` array names by reference, where the array is one.
fn icc_profile_of(document: &Document, items: &[Object]) -> Option<ObjectId> {
    let family = document.resolve(items.first()?);
    if family.as_name()?.as_bytes() != b"ICCBased" {
        return None;
    }
    items.get(1)?.as_reference()
}

/// The value with every `ICCBased` array naming one of `profiles` written `DeviceCMYK`.
///
/// Returns how many arrays were replaced. `value` itself is replaced where it is such an array,
/// which is the shape of a colour space written as an object of its own.
pub(super) fn write_device_cmyk(
    document: &Document,
    value: &mut Object,
    profiles: &BTreeSet<ObjectId>,
    depth: usize,
) -> usize {
    if depth >= MAX_ENTRY_DEPTH {
        return 0;
    }
    let deeper = depth.saturating_add(1);
    match value {
        Object::Array(items) => {
            if icc_profile_of(document, items).is_some_and(|id| profiles.contains(&id)) {
                *value = Object::Name(Name::new(&b"DeviceCMYK"[..]));
                return 1;
            }
            items
                .iter_mut()
                .map(|item| write_device_cmyk(document, item, profiles, deeper))
                .sum()
        }
        Object::Dictionary(dict) => write_device_cmyk_in(document, dict, profiles, deeper),
        _ => 0,
    }
}

/// [`write_device_cmyk`] over each entry of a dictionary but a group's `CS`, the entries it
/// changed written back.
pub(super) fn write_device_cmyk_in(
    document: &Document,
    dict: &mut Dictionary,
    profiles: &BTreeSet<ObjectId>,
    depth: usize,
) -> usize {
    let mut written: usize = 0;
    let entries: Vec<(Name, Object)> = dict
        .iter()
        .filter(|(name, _)| name.as_bytes() != b"CS")
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    for (name, mut value) in entries {
        let here = write_device_cmyk(document, &mut value, profiles, depth);
        if here > 0 {
            dict.insert(name, value);
            written = written.saturating_add(here);
        }
    }
    written
}
