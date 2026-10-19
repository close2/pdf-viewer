//! The `HalftoneName` entries ISO 19005 forbids, removed from the halftones that state them.
//!
//! # What the name does, and what removing it costs
//!
//! ISO 19005-2 section 6.2.5 and ISO 19005-4 section 6.2.5 forbid a halftone to state the key.
//! ISO 32000-2 §10.6.5.1 says what it is for:
//!
//! > Halftone dictionaries have an optional entry, HalftoneName , that identifies the halftone by
//! > name. If this entry is present, all other entries, including HalftoneType , are optional. At
//! > rendering time, if the output device has a halftone with the specified name, that halftone
//! > may be used, overriding any other halftone parameters specified in the dictionary.
//!
//! and what a device does without it: "the halftone's parameters may be defined by the other
//! entries in the dictionary, if any". So the name is a pointer into a marking device's own table,
//! and removing it from a halftone that defines itself leaves every device lacking that halftone
//! where it was. **Nothing on a screen moves** — §10.6.1: "Halftoning is not required for such
//! devices", and this reader is one. What moves is a press that has a halftone of that name,
//! which screens with the dictionary's own parameters instead: that is
//! [`super::Loss::HalftoneName`], which the caller authorises (`doc/adr/1965`).
//!
//! # A halftone that is only a name is refused
//!
//! Without the name, a dictionary stating no `HalftoneType` defines no halftone at all, and the
//! clause's fallback — "the default halftone may be used" — is a graphics state entry rather than
//! anything this removal could write. So such a halftone is refused by name, as is one this
//! removal cannot reach: a findings list the validator shortened, or a halftone written inside
//! another object, the components of a type 5 halftone among them.

use std::collections::BTreeSet;

use pdf_archive::Outcome;
use pdf_syntax::Document;
use pdf_syntax::object::Object;

use super::decision::Because;
use super::sites::Sites;

/// The requirement this module answers, by the identifier the validator reports it under.
const SITE: &str = "graphics/no-halftone-name";

/// Why a halftone's name is not removed.
const HALFTONE_NOT_REACHED: &str = "a halftone in this file states a HalftoneName where this \
     conversion cannot remove it: the validator's list was shortened, the halftone is written \
     inside another object (a type 5 halftone's component among them), or it states no \
     HalftoneType, so that without the name it would define no halftone at all. Removing the \
     names it can reach would leave the requirement failing at the others, so none is removed \
     (doc/adr/1965)";

/// Every halftone whose `HalftoneName` goes.
///
/// The findings are the whole population — every halftone stating the key fails the requirement —
/// so a list the validator shortened is refused rather than answered for the part of it this can
/// see, and each finding must name the halftone itself: an object whose top-level dictionary, or
/// stream dictionary, states both the name and a `HalftoneType`.
pub(super) fn removal(document: &Document, input: &pdf_archive::Report) -> Result<Sites, Because> {
    let refused = Because::NotBuiltYet(HALFTONE_NOT_REACHED);
    let mut at = BTreeSet::new();
    for judgement in input.failures() {
        if judgement.id != SITE {
            continue;
        }
        let Outcome::Failed { places, total } = &judgement.outcome else {
            continue;
        };
        if places.len() != *total {
            return Err(refused);
        }
        for finding in places {
            let id = finding.place.object.ok_or(refused)?;
            let object = document.get(id);
            let halftone = match &object {
                Object::Dictionary(dict) => dict,
                Object::Stream(stream) => &stream.dict,
                _ => return Err(refused),
            };
            if halftone.get("HalftoneName").is_none() || halftone.get("HalftoneType").is_none() {
                return Err(refused);
            }
            at.insert(id);
        }
    }
    Ok(Sites { at })
}
