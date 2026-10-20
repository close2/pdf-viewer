//! An `ICCBased` profile stream's `N` restated as the number of components the profile has
//! (`doc/adr/2099`).
//!
//! ISO 19005-4 section 6.2.4.2 holds the profile forming an `ICCBased` space's stream to ISO
//! 32000-2 §8.6.5.5, whose Table 66 states of `N`:
//!
//! > ( Required ) The number of colour components in the colour space described by the ICC
//! > profile data. This number shall match the number of components actually in the ICC profile.
//!
//! So where `N` is missing, or states a number other than the profile's, the number the entry
//! owes is already in the file: the profile's header names its data colour space, and §8.6.5.5's
//! table of ranges gives that space its count (`pdf_archive::icc_profile_component_count`, the
//! requirement's own test). Writing it states what the profile says.
//!
//! **And it is what this reader drew.** `pdf_colour` draws an `ICCBased` space through its
//! profile wherever the stream decodes whole and the profile parses, and reads no `N` there — so
//! a page draws the same either side of the rewrite. A profile it does not draw through is drawn
//! in Table 65's `Alternate`, or the device space `N` names, and restating `N` there could change
//! that space; such a profile is refused, as is one whose device class or data colour space the
//! base standard does not admit, which no entry can correct.

use std::collections::BTreeMap;

use pdf_archive::icc_profile_component_count;
use pdf_model::icc::Profile;
use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

use super::decision::Because;
use super::sites::MAX_ENTRY_DEPTH;

/// Why a profile the base standard does not admit is not corrected.
const NOT_ADMITTED: &str = "an ICCBased colour space in this file carries a profile whose device \
     class or data colour space ISO 32000-2 8.6.5.5 does not admit for one, or which carries no \
     profile header at all. This profile is what the page's colours are given in: replacing it \
     recolours everything drawn through the space, and no entry of the stream dictionary can \
     correct what the profile itself states (doc/adr/2099)";

/// Why a profile this reader does not draw through is not given an `N`.
const NOT_DRAWN_THROUGH: &str = "an ICCBased colour space in this file states an N that is \
     missing or is not the number of components its profile has, and this reader cannot draw \
     through that profile: it draws the space in Table 65's Alternate or in the device space N \
     names, so restating N could change the space the page was drawn in (doc/adr/2099)";

/// Why a failure this walk did not find is not answered.
const NOT_REACHED: &str = "the requirement failed, and this conversion's walk over the file's \
     objects found no ICCBased profile whose N it could restate - the failure is in an object of \
     a generation other than 0 or past the walk's depth bound - so a rewrite of what it found would \
     leave the failure in place (doc/adr/2099)";

/// The profile streams whose `N` is restated, and the count each states.
#[derive(Debug, Default)]
pub(super) struct ComponentCounts {
    /// The count, by profile stream.
    pub(super) at: BTreeMap<ObjectId, i64>,
}

/// Every `ICCBased` profile whose `N` disagrees with its profile, or why one cannot be restated.
pub(super) fn restated(document: &Document) -> Result<ComponentCounts, Because> {
    let mut walk = Walk {
        out: ComponentCounts::default(),
        refused: None,
    };
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId::new(number, 0));
        walk.visit(document, &object, 0);
    }
    if let Some(because) = walk.refused {
        return Err(because);
    }
    if walk.out.at.is_empty() {
        return Err(Because::NotBuiltYet(NOT_REACHED));
    }
    Ok(walk.out)
}

/// One pass over the file's objects.
struct Walk {
    /// What it found to restate.
    out: ComponentCounts,
    /// The first profile it could not answer for, which refuses the whole file.
    refused: Option<Because>,
}

impl Walk {
    /// One value, every array and dictionary under it descended.
    fn visit(&mut self, document: &Document, object: &Object, depth: usize) {
        if depth >= MAX_ENTRY_DEPTH {
            return;
        }
        let deeper = depth.saturating_add(1);
        match object {
            Object::Array(items) => {
                if let Some(at) = icc_profile_of(document, items) {
                    self.judge(document, at);
                }
                for item in items {
                    self.visit(document, item, deeper);
                }
            }
            Object::Dictionary(dict) => {
                for (_, value) in dict.iter() {
                    self.visit(document, value, deeper);
                }
            }
            Object::Stream(stream) => {
                for (_, value) in stream.dict.iter() {
                    self.visit(document, value, deeper);
                }
            }
            _ => {}
        }
    }

    /// One profile stream, by the requirement's test and by whether this reader draws through it.
    fn judge(&mut self, document: &Document, at: ObjectId) {
        let Object::Stream(stream) = document.get(at) else {
            return;
        };
        let stated = document.get_key(&stream.dict, "N").as_integer();
        // A stream that does not decode is judged by its `N` alone, as the requirement judges it.
        let Ok(decoded) = document.decoded_stream_data_reported(&stream) else {
            if !matches!(stated, Some(1 | 3 | 4)) {
                self.refuse(NOT_DRAWN_THROUGH);
            }
            return;
        };
        let Some(count) = icc_profile_component_count(&decoded.data) else {
            self.refuse(NOT_ADMITTED);
            return;
        };
        if stated == Some(count) {
            return;
        }
        if decoded.damage.is_some() || Profile::parse(&decoded.data).is_none() {
            self.refuse(NOT_DRAWN_THROUGH);
            return;
        }
        self.out.at.insert(at, count);
    }

    /// Records the first reason the file cannot be answered.
    fn refuse(&mut self, sentence: &'static str) {
        self.refused.get_or_insert(Because::NotBuiltYet(sentence));
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
