//! Which of a document's layers are visible: optional content, ISO 32000-2 §8.11.
//!
//! # Why this is not a small feature ranked by five corpus pages
//!
//! ISO 32000-2 §6.3.2.2 places three obligations on a processor that renders a page. One is
//! to render the page contents; one is to draw the appearance stream of every annotation
//! whose flags call for one; and one is to respect the optional content configuration. Two
//! of the three were built long before this module existed. Drawing a layer the document
//! says is off is not a missing feature — it is drawing something the file states is not
//! there, and `issue12007_reduced.pdf` draws a whole screenshot over a page every other
//! renderer leaves nearly blank.
//!
//! # What decides visibility
//!
//! Three things, and the middle one is what makes this more than reading a list of groups
//! that are off:
//!
//! - **The default configuration** (§8.11.4.3, §8.11.4.5). `/OCProperties /D` gives a
//!   `/BaseState` for every group in the document, then an `/ON` or `/OFF` array adjusts it.
//!   That state is the initial state *every* processor starts from.
//! - **Membership** (§8.11.2.2). Content usually points not at a group but at an optional
//!   content *membership* dictionary, which combines several groups under a policy —
//!   `AnyOn`, `AllOn`, `AnyOff`, `AllOff` — or under a visibility expression, a small
//!   boolean tree of `/And`, `/Or` and `/Not`. Content that is visible when a group is
//!   *off* is written this way, so reading `/OFF` alone gets such a page exactly backwards.
//! - **Intent** (§8.11.2.3). A group states what it is for, and a configuration states which
//!   intents it considers. A group the configuration does not consider has no effect on
//!   visibility at all — it is neither on nor off, it simply does not participate.
//!
//! # Where it is asked
//!
//! Two entry points, both of which real documents use (§8.11.3.1): a `BDC /OC` … `EMC` span
//! in a content stream, and an `/OC` entry on a form or image `XObject` or on an annotation.
//! `issue12007_reduced.pdf` hides its layers through the second, which is why implementing
//! only the first would have looked like a fix and changed nothing on the page that
//! motivated it.
//!
//! # What is deliberately not here
//!
//! The states of groups are read from the document and never changed. §8.11.4.5's automatic
//! adjustment from usage application dictionaries (`/AS`), the radio-button relationships of
//! `/RBGroups`, `/Locked`, the presentation `/Order` and the alternate `/Configs` all
//! describe an interactive processor offering the user a layer panel. None of them affects
//! the initial state, which is the state §8.11.4.5 says every processor starts from and the
//! only one a renderer with no user interface can be in. When a layer panel exists, this is
//! the module it attaches to.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

/// Deepest nesting of a visibility expression that will be evaluated.
///
/// `/VE` is a tree of arrays a document supplies, so it is untrusted input with a natural
/// recursion in it. Legitimate expressions are two or three levels deep; anything deeper is
/// a file built to make a reader recurse. Reaching the bound is *reported* rather than
/// treated as a visibility answer — see [`Visibility::TooDeep`].
const MAX_EXPRESSION_DEPTH: usize = 32;

/// Whether a piece of optional content is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Drawn, either because its groups are on or because nothing here applies to it.
    Visible,
    /// Not drawn: the document's default configuration hides it.
    Hidden,
    /// A `/VE` visibility expression nested deeper than [`MAX_EXPRESSION_DEPTH`].
    ///
    /// The content is drawn — of the two ways to be wrong, drawing something that should be
    /// hidden is the one a reader can see — and the interpreter reports it, because a bound
    /// reached in silence is the failure mode this project's own habit forbids.
    TooDeep,
}

/// The visibility policy of a membership dictionary. Table 97, `/P`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    /// Visible only if all of the groups are on.
    AllOn,
    /// Visible if any of the groups is on. The default.
    AnyOn,
    /// Visible if any of the groups is off.
    AnyOff,
    /// Visible only if all of the groups are off.
    AllOff,
}

impl Policy {
    /// Reads `/P`, which defaults to `AnyOn` when absent or unrecognised.
    fn read(document: &Document, dict: &Dictionary) -> Self {
        match document.get_key(dict, "P").as_name().map(Name::as_bytes) {
            Some(b"AllOn") => Self::AllOn,
            Some(b"AnyOff") => Self::AnyOff,
            Some(b"AllOff") => Self::AllOff,
            _ => Self::AnyOn,
        }
    }

    /// Applies the policy to the states of the groups that participate.
    fn holds(self, states: &[bool]) -> bool {
        match self {
            Self::AllOn => states.iter().all(|on| *on),
            Self::AnyOn => states.iter().any(|on| *on),
            Self::AnyOff => states.iter().any(|on| !*on),
            Self::AllOff => states.iter().all(|on| !*on),
        }
    }
}

/// A document's optional content groups and the state the default configuration gives them.
///
/// Built once per document. Reading it costs one dictionary and two arrays, which is why it
/// happens on the page-one path without a measurable cost — but it is still built lazily by
/// the interpreter rather than at open time, because a document without `/OCProperties` must
/// not pay for a lookup it will never use.
#[derive(Debug, Clone)]
pub struct OptionalContent {
    /// Every group `/OCProperties /OCGs` lists, with the state `/D` gives it.
    states: BTreeMap<ObjectId, bool>,
    /// Groups the configuration's `/Intent` does not cover, which therefore have no effect
    /// on visibility (§8.11.2.3).
    disregarded: BTreeSet<ObjectId>,
    /// Set when the configuration's `/Intent` is an empty array.
    ///
    /// §8.11.2.3: "If the configuration's Intent is an empty array, no groups shall be used
    /// in determining visibility; therefore, all content shall be considered visible." An
    /// empty array is not the same as an absent entry, which means `View`.
    everything_visible: bool,
}

impl OptionalContent {
    /// Reads the document's default optional content configuration.
    ///
    /// `None` when the catalog has no `/OCProperties`, which §8.11.4.2 makes decisive: the
    /// dictionary "shall be present if the PDF file contains any optional content; if it is
    /// missing, a PDF processor shall ignore any optional content structures in the
    /// document". So a stray `/OC` in a file without it is not optional content at all, and
    /// nothing here has to guess.
    #[must_use]
    pub fn read(document: &Document) -> Option<Self> {
        let catalog = document.catalog().ok()?;
        let properties = document.get_key(&catalog, "OCProperties");
        let properties = properties.as_dict()?;

        let groups: Vec<ObjectId> = document
            .get_key(properties, "OCGs")
            .as_array()
            .map(|array| array.iter().filter_map(reference).collect())
            .unwrap_or_default();

        let configuration = document.get_key(properties, "D");
        let configuration = configuration.as_dict().cloned().unwrap_or_default();

        // §8.11.4.5 a) and b): the base state reaches every group, and then the array
        // opposite to it adjusts the groups it names. Table 99 states the more general rule
        // — that both arrays are processed — and the two agree on every file that follows
        // its own requirement that a group in `/ON` "shall not also be included in `/OFF`".
        let base = document
            .get_key(&configuration, "BaseState")
            .as_name()
            .map(Name::as_bytes)
            .map_or(BaseState::On, |name| match name {
                b"OFF" => BaseState::Off,
                b"Unchanged" => BaseState::Unchanged,
                _ => BaseState::On,
            });
        let mut states: BTreeMap<ObjectId, bool> = groups
            .iter()
            .map(|group| (*group, base != BaseState::Off))
            .collect();
        for (key, state) in base.arrays_to_apply() {
            for group in listed(document, &configuration, key) {
                // Adjusted, never added. Table 98 requires `/OCGs` to list *every* group in
                // the document, and §8.11.3.2 makes membership of that array the test for
                // whether content is optional content at all — so a group named only by
                // `/OFF` is not one of the document's groups and governs nothing.
                if let Some(entry) = states.get_mut(&group) {
                    *entry = *state;
                }
            }
        }

        let (intents, everything_visible) = intents_of(document, &configuration, b"View");
        let disregarded = states
            .keys()
            .copied()
            .filter(|group| {
                let dictionary = document.get(*group);
                let Some(dictionary) = dictionary.as_dict() else {
                    return true;
                };
                let (own, empty) = intents_of(document, dictionary, b"View");
                // An empty `/Intent` on a *group* is not given a meaning by §8.11.2.3; only
                // the configuration's empty array is. A group naming no intent it shares
                // with the configuration is the case the clause does describe, and an empty
                // array names none.
                empty || !own.iter().any(|intent| covers(&intents, intent))
            })
            .collect();

        Some(Self {
            states,
            disregarded,
            everything_visible,
        })
    }

    /// Whether content governed by `oc` is drawn.
    ///
    /// `oc` is the object a `BDC /OC` names through the page's `/Properties`, or the `/OC`
    /// entry of an `XObject` or annotation: either an optional content group or a membership
    /// dictionary.
    #[must_use]
    pub fn visibility(&self, document: &Document, oc: &Object) -> Visibility {
        if self.everything_visible {
            return Visibility::Visible;
        }
        let resolved = document.resolve(oc);
        let Some(dictionary) = resolved.as_dict() else {
            return Visibility::Visible;
        };

        if document
            .get_key(dictionary, "Type")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"OCMD")
        {
            return self.membership(document, dictionary);
        }

        // §8.11.3.2: content is optional content "only if the tag is OC and the dictionary
        // operand is a valid optional content group that is included in the OCGs array of
        // the optional content properties dictionary … or a valid optional content
        // membership dictionary". A group nobody declared governs nothing.
        match reference(oc).and_then(|group| self.state_of(group)) {
            Some(true) | None => Visibility::Visible,
            Some(false) => Visibility::Hidden,
        }
    }

    /// The state of a group, or `None` if it does not take part in deciding visibility.
    ///
    /// A group is out of the reckoning either because the properties dictionary never listed
    /// it, or because the configuration's `/Intent` does not cover it (§8.11.2.3: "If there
    /// is no match, the group shall have no effect on visibility").
    fn state_of(&self, group: ObjectId) -> Option<bool> {
        if self.disregarded.contains(&group) {
            return None;
        }
        self.states.get(&group).copied()
    }

    /// Evaluates an optional content membership dictionary. §8.11.2.2, Table 97.
    fn membership(&self, document: &Document, dictionary: &Dictionary) -> Visibility {
        // "If the VE key is present it shall be used in preference to the OCGs and P keys."
        let expression = document.get_key(dictionary, "VE");
        if let Some(array) = expression.as_array() {
            return match self.evaluate(document, array, 0) {
                Some(true) => Visibility::Visible,
                Some(false) => Visibility::Hidden,
                None => Visibility::TooDeep,
            };
        }

        // Table 97 allows `/OCGs` to be "a dictionary or array of dictionaries", and a
        // group's identity is its reference — so the entry is read *unresolved*, and
        // resolved only far enough to tell the two shapes apart. Reading it resolved instead
        // turns the single-group form into a dictionary with no reference left on it, which
        // is how `issue12007_reduced.pdf` drew a whole hidden screenshot with this module
        // already in place: every one of its layers is `<< /Type /OCMD /OCGs 38 0 R >>`.
        let written = dictionary.get("OCGs").cloned().unwrap_or(Object::Null);
        let listed: Vec<Object> = match document.resolve(&written) {
            Object::Array(array) => array,
            Object::Null => Vec::new(),
            _ => vec![written],
        };
        let states: Vec<bool> = listed
            .iter()
            .filter_map(Object::as_reference)
            .filter_map(|group| self.state_of(group))
            .collect();

        // Table 97: if `/OCGs` "is not present, is an empty array, or contains references
        // only to null or deleted objects, the P entry shall have no effect on the
        // visibility of any content".
        if states.is_empty() {
            return Visibility::Visible;
        }
        if Policy::read(document, dictionary).holds(&states) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        }
    }

    /// Evaluates a visibility expression. §8.11.2.2.
    ///
    /// > Its first element shall be a name representing a boolean operator ( And , Or , or
    /// > Not ).
    ///
    /// `None` means the expression nested past [`MAX_EXPRESSION_DEPTH`]. A malformed
    /// expression — an unknown operator, a `/Not` with two operands — evaluates to visible
    /// rather than to an error: it is a defect in the file, and the clause gives no
    /// alternative reading to report against.
    fn evaluate(&self, document: &Document, expression: &[Object], depth: usize) -> Option<bool> {
        if depth > MAX_EXPRESSION_DEPTH {
            return None;
        }
        let operator = expression.first().and_then(Object::as_name)?;
        let operands = expression.get(1..).unwrap_or_default();
        let mut values = Vec::with_capacity(operands.len());
        for operand in operands {
            values.push(self.operand(document, operand, depth)?);
        }
        match operator.as_bytes() {
            // "If the first element is Not , it shall have only one subsequent element."
            b"Not" => values.first().map(|value| !*value),
            b"And" => Some(values.iter().all(|value| *value)),
            b"Or" => Some(values.iter().any(|value| *value)),
            _ => Some(true),
        }
    }

    /// One operand of a visibility expression: a nested expression, or a group.
    ///
    /// §8.11.2.2: "In evaluating a visibility expression, the ON state of an optional
    /// content group shall be equated to the boolean value true ; OFF shall be equated to
    /// false ." A group that takes no part — undeclared, or outside the configuration's
    /// intent — is `true`, which is the same thing as it having no effect on the result.
    fn operand(&self, document: &Document, operand: &Object, depth: usize) -> Option<bool> {
        if let Object::Array(nested) = operand {
            return self.evaluate(document, nested, depth.saturating_add(1));
        }
        if let Object::Reference(id) = operand
            && let Object::Array(nested) = document.get(*id)
        {
            return self.evaluate(document, &nested, depth.saturating_add(1));
        }
        Some(
            reference(operand)
                .and_then(|group| self.state_of(group))
                .unwrap_or(true),
        )
    }
}

/// `/BaseState`, and which of `/ON` and `/OFF` it leaves to be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaseState {
    /// Every group on, the default. Only `/OFF` adjusts anything.
    On,
    /// Every group off. Only `/ON` adjusts anything.
    Off,
    /// States left as they were — which, for a document being opened, is the group default.
    Unchanged,
}

impl BaseState {
    /// The arrays §8.11.4.5 b) leaves to be processed, with the state each sets.
    fn arrays_to_apply(self) -> &'static [(&'static str, bool)] {
        match self {
            Self::On => &[("OFF", false)],
            Self::Off => &[("ON", true)],
            // Nothing has changed the states yet, so both arrays still have work to do.
            Self::Unchanged => &[("ON", true), ("OFF", false)],
        }
    }
}

/// The object identifiers an array-valued key lists.
fn listed(document: &Document, dictionary: &Dictionary, key: &str) -> Vec<ObjectId> {
    document
        .get_key(dictionary, key)
        .as_array()
        .map(|array| array.iter().filter_map(reference).collect())
        .unwrap_or_default()
}

/// An object's identity, which for an optional content group is the only identity it has.
///
/// §8.11.2.2 notes that "a group shall be an indirect object". A directly-written dictionary
/// therefore cannot be one of the groups `/OCProperties` lists, and has no effect.
fn reference(object: &Object) -> Option<ObjectId> {
    match object {
        Object::Reference(id) => Some(*id),
        _ => None,
    }
}

/// Reads an `/Intent`, which is a name or an array of names. §8.11.2.3.
///
/// Returns the intents and whether the entry was written as an *empty* array, which the
/// clause gives a meaning of its own to for a configuration. `All` is expanded here by
/// matching everything: it "is used to indicate the set of all intents".
fn intents_of(
    document: &Document,
    dictionary: &Dictionary,
    default: &'static [u8],
) -> (BTreeSet<Vec<u8>>, bool) {
    let entry = document.get_key(dictionary, "Intent");
    let names: Vec<Vec<u8>> = match &entry {
        Object::Name(name) => vec![name.as_bytes().to_vec()],
        Object::Array(array) => array
            .iter()
            .map(|item| document.resolve(item))
            .filter_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
            .collect(),
        _ => vec![default.to_vec()],
    };
    let empty = matches!(&entry, Object::Array(array) if array.is_empty());
    (names.into_iter().collect(), empty)
}

/// Whether a configuration's intents cover one of a group's.
///
/// §8.11.4.3 Table 99: `All` "is used to indicate the set of all intents", so a
/// configuration naming it covers every group.
fn covers(configuration: &BTreeSet<Vec<u8>>, intent: &[u8]) -> bool {
    configuration
        .iter()
        .any(|held| held.as_slice() == intent || held.as_slice() == b"All")
}
