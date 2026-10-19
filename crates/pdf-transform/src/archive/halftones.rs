//! The halftone entries ISO 19005 forbids: a halftone's `HalftoneName`, a halftone of a type other
//! than 1 or 5, and a graphics state's halftone phase or origin.
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
//!
//! # A halftone of another type: its graphics states set `Default`
//!
//! ISO 19005-2 section 6.2.5 and ISO 19005-4 section 6.2.5 admit halftone types 1 and 5 alone,
//! part 2's note saying why: a threshold screen produces different appearances at different
//! resolutions. A threshold array is a whole screen rather than a pointer to one, so no entry of
//! it can go and leave the rest standing; what can change is the graphics state that names it.
//! Its `HT` is written Table 57's `Default`, "denoting the halftone that was in effect at the start
//! of the page" — the device's own — and the threshold halftone, which nothing names any more,
//! leaves the file with every object only it reached. Nothing a screen shows moves, for §10.6.1's
//! reason above; a press screens those marks with its own halftone instead of the producer's,
//! which is [`super::Loss::Halftone`] (`doc/adr/2034`). A halftone named by a type 5 halftone's
//! colourant, or from anywhere but the top level of a graphics state's own object, is refused.
//!
//! # A graphics state's phase is stated away, and its origin is a press's loss
//!
//! Neither ISO 32000-1, which part 2 rests on, nor ISO 32000-2 defines `HTP` — part 2's own note
//! records that PDF 1.3 removed it — and ISO 32000-2's Annex I.3 says that an entry the
//! specification does not describe, added to a dictionary, does not change what a processor does.
//! So removing it moves nothing any processor of the base standard does, press or screen, and is a
//! statement (`doc/adr/2035`). `HTO` is ISO 32000-2's own (Table 57, PDF 2.0), which ISO 19005-4
//! section 6.2.5 forbids: a press places its screen at its own origin without it, and that is
//! [`super::Loss::Halftone`] again.

use std::collections::BTreeSet;

use pdf_archive::Outcome;
use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

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

/// The requirement [`defaults`] answers.
const TYPE_SITE: &str = "graphics/halftone-type-is-one-or-five";

/// The requirement [`phases`] answers, part 2's.
const PHASE_SITE: &str = "graphics/no-halftone-phase-in-a-graphics-state";

/// The requirement [`origins`] answers, part 4's.
const ORIGIN_SITE: &str = "graphics/no-halftone-origin-in-a-graphics-state";

/// Why a halftone of another type is not replaced by the device's own.
const TYPE_NOT_REACHED: &str = "a halftone in this file is of a type other than 1 or 5 where \
     this conversion cannot replace it: the validator's list was shortened, a type 5 halftone \
     names it as a colourant's screen, or a graphics state names it from inside another object \
     or from an entry other than HT. Replacing the halftones it can reach would leave the \
     requirement failing at the others, so none is replaced (doc/adr/2034)";

/// Why a graphics state's halftone phase or origin is not removed.
const KEY_NOT_REACHED: &str = "a graphics state in this file states a halftone phase or origin \
     where this conversion cannot remove it: the validator's list was shortened, or the graphics \
     state is written inside another object rather than as one of its own. Removing the entries \
     it can reach would leave the requirement failing at the others, so none is removed \
     (doc/adr/2034, doc/adr/2035)";

/// The most objects [`defaults`] walks for references to one of the halftones it replaces.
///
/// The same bound `pdf_archive`'s own walks use for nesting, applied to the depth of one object's
/// direct values; a file nested deeper than this is refused rather than half-searched.
const MOST_DEPTH: usize = 32;

/// Every graphics state whose `HT` names a halftone of a type other than 1 or 5, to be written
/// `Default`.
///
/// The findings are the whole population, so a shortened list is refused. Each names either the
/// halftone's own object — whose every reference must then be the top-level `HT` of an object
/// that is a graphics state, the rewrite's one reach — or a graphics state whose `HT` is that
/// halftone written directly.
pub(super) fn defaults(document: &Document, input: &pdf_archive::Report) -> Result<Sites, Because> {
    let refused = Because::NotBuiltYet(TYPE_NOT_REACHED);
    let mut halftones = BTreeSet::new();
    let mut states = BTreeSet::new();
    for id in complete_findings(input, TYPE_SITE).ok_or(refused)? {
        let object = document.get(id);
        let dict = match &object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => return Err(refused),
        };
        if matches!(
            dict.get("HT"),
            Some(Object::Dictionary(_) | Object::Stream(_))
        ) {
            states.insert(id);
        } else if dict.get("HalftoneType").and_then(Object::as_integer) == Some(5) {
            // The finding is a colourant's screen written inside this type 5 halftone.
            return Err(refused);
        } else {
            halftones.insert(id);
        }
    }
    for number in document.xref().object_numbers() {
        let holder = ObjectId::new(number, 0);
        let object = document.get(holder);
        let dict = match &object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        for (key, value) in dict.iter() {
            let by_ht = key.as_bytes() == b"HT"
                && value
                    .as_reference()
                    .is_some_and(|named| halftones.contains(&named));
            if by_ht {
                states.insert(holder);
            } else if names_any(value, &halftones, 0).ok_or(refused)? {
                return Err(refused);
            }
        }
    }
    Ok(Sites { at: states })
}

/// Every graphics state stating `HTP`, whose entry goes (`doc/adr/2035`).
pub(super) fn phases(document: &Document, input: &pdf_archive::Report) -> Result<Sites, Because> {
    states_stating(document, input, PHASE_SITE, "HTP")
}

/// Every graphics state stating `HTO`, whose entry goes (`doc/adr/2034`).
pub(super) fn origins(document: &Document, input: &pdf_archive::Report) -> Result<Sites, Because> {
    states_stating(document, input, ORIGIN_SITE, "HTO")
}

/// The graphics states one forbidden key's findings name, each stating the key at the top level of
/// its own object.
fn states_stating(
    document: &Document,
    input: &pdf_archive::Report,
    site: &str,
    key: &str,
) -> Result<Sites, Because> {
    let refused = Because::NotBuiltYet(KEY_NOT_REACHED);
    let mut at = BTreeSet::new();
    for id in complete_findings(input, site).ok_or(refused)? {
        let states = match &document.get(id) {
            Object::Dictionary(dict) => dict.get(key).is_some(),
            _ => false,
        };
        if !states {
            return Err(refused);
        }
        at.insert(id);
    }
    Ok(Sites { at })
}

/// The objects one requirement's findings name, or nothing where the list was shortened or a
/// finding names no object.
fn complete_findings(input: &pdf_archive::Report, site: &str) -> Option<BTreeSet<ObjectId>> {
    let mut named = BTreeSet::new();
    for judgement in input.failures() {
        if judgement.id != site {
            continue;
        }
        let Outcome::Failed { places, total } = &judgement.outcome else {
            continue;
        };
        if places.len() != *total {
            return None;
        }
        for finding in places {
            named.insert(finding.place.object?);
        }
    }
    Some(named)
}

/// Whether a value refers to one of `halftones` anywhere inside it, or nothing where it nests past
/// [`MOST_DEPTH`].
fn names_any(value: &Object, halftones: &BTreeSet<ObjectId>, depth: usize) -> Option<bool> {
    if depth > MOST_DEPTH {
        return None;
    }
    let deeper = depth.saturating_add(1);
    match value {
        Object::Reference(id) => Some(halftones.contains(id)),
        Object::Array(items) => {
            for item in items {
                if names_any(item, halftones, deeper)? {
                    return Some(true);
                }
            }
            Some(false)
        }
        Object::Dictionary(dict) => {
            for (_, item) in dict.iter() {
                if names_any(item, halftones, deeper)? {
                    return Some(true);
                }
            }
            Some(false)
        }
        _ => Some(false),
    }
}
