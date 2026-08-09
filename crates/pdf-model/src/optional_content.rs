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

use crate::action::Change;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalContent {
    /// Every group `/OCProperties /OCGs` lists, with the state `/D` gives it.
    states: BTreeMap<ObjectId, bool>,
    /// Groups the configuration's `/Intent` does not cover, which therefore have no effect
    /// on visibility (§8.11.2.3).
    disregarded: BTreeSet<ObjectId>,
    /// Usage categories §8.11.4.4 asks a *viewer* for and this one cannot answer.
    ///
    /// Reported rather than guessed; see [`apply_auto_states`].
    unresolved: Vec<&'static str>,
    /// Set when the configuration's `/Intent` is an empty array.
    ///
    /// §8.11.2.3: "If the configuration's Intent is an empty array, no groups shall be used
    /// in determining visibility; therefore, all content shall be considered visible." An
    /// empty array is not the same as an absent entry, which means `View`.
    everything_visible: bool,
    /// Table 99's `/Order`: how a user interface presents the groups, as a tree.
    ///
    /// Empty for a document that states none, which §8.11.4.3 makes decisive for the default
    /// configuration: "[i]n the default configuration dictionary, the default value shall be an
    /// empty array", and "[a]ny groups not listed in this array shall not be presented in any
    /// user interface that uses the configuration". So an empty `/Order` is a document saying
    /// its layers are not a person's business, and is not the same as a missing panel.
    order: Vec<Presented>,
    /// Table 99's `/Locked`: groups a user interface may not change.
    locked: BTreeSet<ObjectId>,
    /// Table 99's `/ListMode`, which decides which of `/Order`'s groups are shown.
    list_mode: ListMode,
    /// Table 99's `/RBGroups`: collections in which at most one group may be on.
    ///
    /// Read but never applied when the document is opened, and that is the clause's own
    /// arrangement rather than an omission: §8.11.4.5 builds the initial state from
    /// `/BaseState` and the `/ON`/`/OFF` arrays and says nothing about radio buttons, so a
    /// configuration that states two members of one collection as on has stated exactly that.
    /// The collections govern *changes* — a user's, or §12.6.4.13's `/PreserveRB`.
    radio_buttons: Vec<Vec<ObjectId>>,
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

        let unresolved = apply_auto_states(document, &configuration, &mut states);

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

        // Table 99: an array of one or more arrays, each inner one a collection of optional
        // content groups "whose states shall be intended to follow a radio button paradigm".
        // An entry that is not an array of arrays states no collection. (The row read ", each
        // of which represents a collection" until Errata Collection 3 — Issue #225, `/State`
        // `Review` `Completed` — which also adds "None of the inner array elements shall be an
        // empty array."; an empty one is dropped below, which excludes nothing either way.)
        let radio_buttons = document
            .get_key(&configuration, "RBGroups")
            .as_array()
            .map(|outer| {
                outer
                    .iter()
                    .filter_map(|inner| {
                        let resolved = document.resolve(inner);
                        let members: Vec<ObjectId> =
                            resolved.as_array()?.iter().filter_map(reference).collect();
                        (!members.is_empty()).then_some(members)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let order = presentation(document, &configuration, &states, 0);
        let locked: BTreeSet<ObjectId> = listed(document, &configuration, "Locked")
            .into_iter()
            .collect();
        // "AllPages Display all groups in the Order array. VisiblePages Display only those
        // groups in the Order array that are referenced by one or more visible pages." The
        // clause names two values and no default; an absent entry is read as `AllPages`,
        // because a panel that hides a group nobody asked it to hide is the worse of the two
        // mistakes, and `VisiblePages` is a question about which pages are on screen that this
        // module cannot answer at all.
        let list_mode = match document
            .get_key(&configuration, "ListMode")
            .as_name()
            .map(Name::as_bytes)
        {
            Some(b"VisiblePages") => ListMode::VisiblePages,
            _ => ListMode::AllPages,
        };

        Some(Self {
            states,
            disregarded,
            unresolved,
            everything_visible,
            order,
            locked,
            list_mode,
            radio_buttons,
        })
    }

    /// Table 99's `/Order`, as the tree a layer panel shows.
    #[must_use]
    pub fn presentation(&self) -> &[Presented] {
        &self.order
    }

    /// Whether Table 99's `/Locked` forbids a *user interface* changing this group.
    ///
    /// §8.11.4.3, of a locked group:
    ///
    /// > The state of a locked group cannot be changed through the user interface of an
    /// > interactive PDF processor.
    ///
    /// A locked group is not a constant: the clause's own next sentence says a processor "may
    /// allow the states of optional content groups to be changed by means other than the user
    /// interface, such as ECMAScript or items in the AS entry", so §12.6.4.13's action is not
    /// bound by this and [`Self::apply`] does not consult it.
    #[must_use]
    pub fn is_locked(&self, group: ObjectId) -> bool {
        self.locked.contains(&group)
    }

    /// Table 99's `/ListMode`.
    #[must_use]
    pub fn list_mode(&self) -> ListMode {
        self.list_mode
    }

    /// A group's `/Name`, which Table 96 makes required and a panel displays.
    #[must_use]
    pub fn name(&self, document: &Document, group: ObjectId) -> Option<String> {
        match document.get_key(document.get(group).as_dict()?, "Name") {
            Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        }
    }

    /// Applies §12.6.4.13's state changes, in the order the action states them.
    ///
    /// The changes live here rather than in `action.rs` because both halves of the rule are
    /// this module's data: the states, and Table 99's `/RBGroups` collections that
    /// `/PreserveRB` preserves.
    ///
    /// Table 217 states the rule: a group set to ON during processing of the `/State` array —
    /// by `ON` or by `Toggle` — turns off every other group belonging to the same radio-button
    /// collection, and a group set to OFF has no effect on any other.
    ///
    /// A group the document never declared is not adjusted, for the same reason
    /// [`Self::read`] adjusts rather than adds: Table 98 requires `/OCGs` to list every group
    /// in the document, so a `/State` array naming something else names nothing.
    pub fn apply(&mut self, changes: &[(ObjectId, Change)], preserve_radio_buttons: bool) {
        for (group, change) in changes {
            let Some(current) = self.states.get_mut(group) else {
                continue;
            };
            let now = change.applied_to(*current);
            *current = now;
            if now && preserve_radio_buttons {
                self.exclude_others(*group);
            }
        }
    }

    /// Turns off every other member of each radio-button collection `group` belongs to.
    fn exclude_others(&mut self, group: ObjectId) {
        // Collected first because the collections and the states are both borrowed from
        // `self`; a document states a handful of collections of a handful of groups.
        let others: Vec<ObjectId> = self
            .radio_buttons
            .iter()
            .filter(|collection| collection.contains(&group))
            .flatten()
            .copied()
            .filter(|other| *other != group)
            .collect();
        for other in others {
            if let Some(state) = self.states.get_mut(&other) {
                *state = false;
            }
        }
    }

    /// Whether a group is on, for a caller that holds the group's identity.
    ///
    /// `None` for a group that takes no part in deciding visibility — see [`Self::state_of`],
    /// whose answer this is.
    #[must_use]
    pub fn state(&self, group: ObjectId) -> Option<bool> {
        self.state_of(group)
    }

    /// The usage categories §8.11.4.4 asked for and this processor could not answer.
    ///
    /// Empty for every document that names none, which is all 974 in the corpus; see
    /// [`apply_auto_states`] for what the two are and why leaving the state alone beats the
    /// clause's "otherwise OFF" when the question is about this machine rather than the file.
    #[must_use]
    pub fn unresolved_usage(&self) -> &[&'static str] {
        &self.unresolved
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

/// One entry of Table 99's `/Order`, as a layer panel would show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Presented {
    /// A group, "whose Name entry shall be displayed in the user interface".
    Group(ObjectId),
    /// A nested array, with the optional text string the clause allows as its first element.
    ///
    /// §8.11.4.3 distinguishes the two shapes and says what each means, which is why the label
    /// is an `Option` rather than a `String`: "[t]ext labels in nested arrays shall be used to
    /// present collections of related optional content groups, and not to communicate actual
    /// nesting of content inside multiple layers of groups", and "[t]o reflect actual nesting of
    /// groups in the content, such as for layers with sublayers, nested arrays of groups
    /// without a text label shall be used". A panel that drew both the same way would tell a
    /// person that a heading is a layer.
    Collection {
        /// The non-selectable label, where the array opens with one.
        label: Option<String>,
        /// What the collection holds.
        children: Vec<Presented>,
    },
}

/// Table 99's `/ListMode`: which of `/Order`'s groups a panel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMode {
    /// "Display all groups in the Order array."
    AllPages,
    /// "Display only those groups in the Order array that are referenced by one or more
    /// visible pages."
    ///
    /// Which pages are visible is a question about a window and this crate has none, so what it
    /// supplies is the other half: [`groups_referenced_by`] answers *which groups one page
    /// references*, and a host with a window intersects that with the pages it is showing. A
    /// caller with no window shows every group, which is `AllPages`.
    VisiblePages,
}

/// Deepest nesting of form `XObject`s walked while gathering a page's groups.
///
/// A form may name a form, and the chain is the document's to state. The bound is generous
/// against real files — a drawing three templates deep is unusual — and finite against a file
/// whose forms name each other, which the visited set already stops but which would otherwise
/// be bounded by nothing at all.
const MAX_FORM_DEPTH: usize = 8;

/// Every optional content group one page's content and annotations reference.
///
/// Table 99's `/ListMode` `VisiblePages` displays "only those groups in the Order array that are
/// referenced by one or more visible pages", and the clause does not say what *referenced by*
/// means. What is taken here is the three places §8.11 puts an `/OC`: the page's
/// `/Resources /Properties`, which is what a `BDC /OC` names (§8.11.3.2); an `XObject`'s own
/// `/OC` (§8.11.3.3); and an annotation's, Table 166's (§8.11.4.4). A membership dictionary
/// contributes every group its `/OCGs` or its `/VE` names, because content governed by one is
/// content whose visibility those groups decide.
///
/// **Resources are followed into nested forms**, since a group referenced by a template placed
/// on the page is referenced by the page. Bounded by [`MAX_FORM_DEPTH`] and by a visited set,
/// both because the nesting is a document's word.
///
/// This does *not* interpret the page: a `BDC /OC` naming a property this walk found is what
/// makes the group reachable, and whether the operator is executed is a different question the
/// panel is not asking. Over-listing a group whose `BDC` never runs is the direction that
/// costs a person nothing; under-listing one hides a switch the document asked to show.
#[must_use]
pub fn groups_referenced_by(document: &Document, page: &crate::page::Page) -> BTreeSet<ObjectId> {
    let mut found = BTreeSet::new();
    let mut visited = BTreeSet::new();
    gather_from_resources(document, &page.resources, &mut found, &mut visited, 0);
    let annotations = document.get_key(&page.dict, "Annots");
    if let Some(annotations) = annotations.as_array() {
        for annotation in annotations {
            let resolved = document.resolve(annotation);
            if let Some(dict) = resolved.as_dict() {
                let oc = dict.get("OC").cloned().unwrap_or(Object::Null);
                gather_from_oc(document, &oc, &mut found);
            }
        }
    }
    found
}

/// Adds the groups a resource dictionary reaches: its `/Properties`, and its `/XObject`s.
fn gather_from_resources(
    document: &Document,
    resources: &Dictionary,
    found: &mut BTreeSet<ObjectId>,
    visited: &mut BTreeSet<ObjectId>,
    depth: usize,
) {
    let properties = document.get_key(resources, "Properties");
    if let Some(properties) = properties.as_dict() {
        for (_, value) in properties.iter() {
            gather_from_oc(document, value, found);
        }
    }
    let xobjects = document.get_key(resources, "XObject");
    let Some(xobjects) = xobjects.as_dict() else {
        return;
    };
    for (_, value) in xobjects.iter() {
        if let Some(id) = reference(value)
            && !visited.insert(id)
        {
            continue;
        }
        let resolved = document.resolve(value);
        let Some(stream) = resolved.as_stream() else {
            continue;
        };
        let oc = stream.dict.get("OC").cloned().unwrap_or(Object::Null);
        gather_from_oc(document, &oc, found);
        if depth >= MAX_FORM_DEPTH {
            continue;
        }
        let nested = document.get_key(&stream.dict, "Resources");
        if let Some(nested) = nested.as_dict() {
            let nested = nested.clone();
            gather_from_resources(document, &nested, found, visited, depth.saturating_add(1));
        }
    }
}

/// Adds the groups one `/OC` value names: a group itself, or every group a membership
/// dictionary's `/OCGs` or `/VE` mentions.
fn gather_from_oc(document: &Document, oc: &Object, found: &mut BTreeSet<ObjectId>) {
    if let Some(id) = reference(oc) {
        let resolved = document.get(id);
        let is_membership = resolved.as_dict().is_some_and(|dict| {
            document
                .get_key(dict, "Type")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"OCMD")
        });
        if !is_membership {
            found.insert(id);
            return;
        }
    }
    let resolved = document.resolve(oc);
    let Some(dict) = resolved.as_dict() else {
        return;
    };
    for id in listed(document, dict, "OCGs") {
        found.insert(id);
    }
    if let Some(single) = dict.get("OCGs").and_then(reference) {
        found.insert(single);
    }
    let expression = document.get_key(dict, "VE");
    if let Some(array) = expression.as_array() {
        gather_from_expression(document, array, found, 0);
    }
}

/// Adds every group a `/VE` visibility expression names, at any nesting the bound allows.
fn gather_from_expression(
    document: &Document,
    expression: &[Object],
    found: &mut BTreeSet<ObjectId>,
    depth: usize,
) {
    if depth >= MAX_EXPRESSION_DEPTH {
        return;
    }
    for operand in expression {
        if let Object::Array(nested) = operand {
            gather_from_expression(document, nested, found, depth.saturating_add(1));
        } else if let Some(id) = reference(operand) {
            match document.get(id) {
                Object::Array(nested) => {
                    gather_from_expression(document, &nested, found, depth.saturating_add(1));
                }
                _ => {
                    found.insert(id);
                }
            }
        }
    }
}

/// Deepest nesting of `/Order` that is read.
///
/// The array is a document's, and §8.11.4.3 puts no bound on how deep the nesting goes. Real
/// documents nest two or three levels — a drawing's layers and their sublayers — and a panel
/// that recursed on a file's word would be a stack the file controls.
const MAX_ORDER_DEPTH: usize = 16;

/// Reads Table 99's `/Order` into the tree a panel shows.
fn presentation(
    document: &Document,
    configuration: &Dictionary,
    states: &BTreeMap<ObjectId, bool>,
    depth: usize,
) -> Vec<Presented> {
    let entry = document.get_key(configuration, "Order");
    let Some(items) = entry.as_array().map(<[Object]>::to_vec) else {
        return Vec::new();
    };
    order_items(document, &items, states, depth)
}

/// The elements of one `/Order` array, or of one of its nested arrays.
fn order_items(
    document: &Document,
    items: &[Object],
    states: &BTreeMap<ObjectId, bool>,
    depth: usize,
) -> Vec<Presented> {
    let mut out = Vec::new();
    for item in items {
        match item {
            // A group the properties dictionary never declared is not one of the document's
            // groups (§8.11.3.2), so presenting it would offer a switch that governs nothing.
            Object::Reference(id) if states.contains_key(id) => out.push(Presented::Group(*id)),
            Object::Reference(id) => {
                if let Object::Array(nested) = document.get(*id) {
                    out.extend(collection(document, &nested, states, depth));
                }
            }
            Object::Array(nested) => out.extend(collection(document, nested, states, depth)),
            // A text string that is not the first element of its array labels nothing; the
            // clause admits one only "as its first element".
            _ => {}
        }
    }
    out
}

/// One nested array of `/Order`, with the label the clause allows it to open with.
fn collection(
    document: &Document,
    nested: &[Object],
    states: &BTreeMap<ObjectId, bool>,
    depth: usize,
) -> Option<Presented> {
    if depth >= MAX_ORDER_DEPTH {
        return None;
    }
    let (label, rest) = match nested.split_first() {
        Some((Object::String(bytes), rest)) => (Some(pdf_syntax::text_string(bytes)), rest),
        _ => (None, nested),
    };
    Some(Presented::Collection {
        label,
        children: order_items(document, rest, states, depth.saturating_add(1)),
    })
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

/// §8.11.4.4's automatic state adjustment, for the `View` event.
///
/// §8.11.4.5 states when it runs: the base state and the `/ON`/`/OFF` arrays give "the initial
/// state used by all PDF processors", and then an interactive processor "shall examine the AS
/// array for usage application dictionaries that have an Event of type View. For each one
/// found, the groups listed in its OCGs array shall be adjusted". Only `View`: `Print` and
/// `Export` apply "for the duration of the print operation" and of the export, and this is
/// neither.
///
/// The rule per group is the clause's own, and it is an AND across two levels. §8.11.4.4:
///
/// > For each of the groups in OCGs , the entries in its usage dictionary … specified by
/// > Category shall be examined to yield a recommended state for the group. If all the
/// > entries yield a recommended state of ON , the group's state shall be set to ON ;
/// > otherwise, its state shall be set to OFF .
///
/// — and across dictionaries, "if a given optional content group appears in more than one OCGs
/// array, its state shall be ON only if all categories in all the usage application
/// dictionaries it appears in have a state of ON ".
///
/// # The three categories a page cannot answer, and what happens to them
///
/// `Zoom` asks whether "the current magnification level of the document is greater than or
/// equal to min and less than max". It is answered at **1.0**, the magnification at which a page
/// is its stated size, and that is a choice with a measurement behind it rather than an
/// architectural limit.
///
/// **The reason this comment used to give has expired.** It said "a display list has no
/// magnification: it is built once and rasterised at whatever scale the caller asks for … the
/// alternative is to thread a scale into `interpret` and rebuild the display list per zoom, which
/// is a viewer's design question rather than a clause's". The tree answered that question in the
/// two-hundred-and-seventeenth session: §12.5.3's `NoZoom` threads exactly such a scale through
/// `ViewState::magnification`, and `Interpretation::view_dependent` says which pages notice, so
/// 923 of the 974 corpus documents never re-interpret on a zoom (ADR 0168). §8.11.4.5 states the
/// obligation that would follow — "[w]henever there is a change to a factor that the usage
/// application dictionaries with event type View depend on (such as zoom level), the
/// corresponding dictionaries shall be reapplied".
///
/// **What holds it is that nothing asks.** `examples/oc_usage_census` reads every configuration's
/// `/AS` in all 974 corpus documents: 31 state `/OCProperties`, **six** state a usage application
/// dictionary at all, and the categories they name are `View` (6), `Print` (6) and `Export` (5)
/// — **no `Zoom`, no `User`, no `Language` anywhere**. So the magnification would decide nothing
/// on any document anyone has, and building the reapplication would be shipping a path nobody
/// takes. The measurement is what makes that a decision rather than an omission, and a document
/// that named `Zoom` would make it work to do (the three-hundred-and-twenty-fourth session).
///
/// `User` matches "the user's identification" and `Language` "the language and locale of the
/// application". Both would be answers about this machine rather than about the document, and
/// `pdf-font`'s `substitute` module is deliberately the only place in the tree that reads one.
/// A group whose state either would decide is therefore **left as the configuration set it and
/// named in a report**, rather than switched off on the clause's "otherwise OFF" — which would
/// hide content on the strength of a question nobody asked. No corpus document uses either.
fn apply_auto_states(
    document: &Document,
    configuration: &Dictionary,
    states: &mut BTreeMap<ObjectId, bool>,
) -> Vec<&'static str> {
    /// The magnification a page is drawn at when nothing states one; see above.
    const MAGNIFICATION: f32 = 1.0;

    let mut unresolved: Vec<&'static str> = Vec::new();
    let auto = document.get_key(configuration, "AS");
    let Some(applications) = auto.as_array() else {
        // "If no AS entry is present, states shall not be automatically adjusted based on
        // usage information."
        return unresolved;
    };

    for application in applications {
        let application = document.resolve(application);
        let Some(application) = application.as_dict() else {
            continue;
        };
        if document
            .get_key(application, "Event")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .as_deref()
            != Some(b"View")
        {
            continue;
        }
        let categories: Vec<Vec<u8>> = document
            .get_key(application, "Category")
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        document
                            .resolve(item)
                            .as_name()
                            .map(|n| n.as_bytes().to_vec())
                    })
                    .collect()
            })
            .unwrap_or_default();
        if categories.is_empty() {
            continue;
        }

        for group in listed(document, application, "OCGs") {
            // Adjusted, never added, for the same reason the `/ON` and `/OFF` arrays are.
            if !states.contains_key(&group) {
                continue;
            }
            let dictionary = document.get(group);
            let Some(dictionary) = dictionary.as_dict() else {
                continue;
            };
            let usage = document.get_key(dictionary, "Usage");
            let usage = usage.as_dict().cloned().unwrap_or_default();

            let mut recommended = Some(true);
            for category in &categories {
                match recommendation(document, &usage, category, MAGNIFICATION) {
                    // `On`, and `Unchanged` for a `Print` category with no `/PrintState`,
                    // both leave the running AND alone: the clause's test is "if all the
                    // entries yield a recommended state of ON", and neither yields OFF.
                    Recommendation::On | Recommendation::Unchanged => {}
                    Recommendation::Off => recommended = Some(false),
                    Recommendation::Unanswerable(name) => {
                        if !unresolved.contains(&name) {
                            unresolved.push(name);
                        }
                        recommended = None;
                    }
                }
            }
            if let Some(state) = recommended
                && let Some(entry) = states.get_mut(&group)
            {
                // The AND across dictionaries: a group already switched off by an earlier
                // usage application dictionary stays off.
                *entry = *entry && state;
            }
        }
    }

    unresolved
}

/// What one of Table 100's categories recommends for a group.
enum Recommendation {
    On,
    Off,
    /// `Print` with no `/PrintState`: "the state … shall be left unchanged".
    Unchanged,
    /// A category this processor cannot answer; see [`apply_auto_states`].
    Unanswerable(&'static str),
}

/// §8.11.4.4's per-category rule, for one group's usage dictionary.
fn recommendation(
    document: &Document,
    usage: &Dictionary,
    category: &[u8],
    magnification: f32,
) -> Recommendation {
    let state = |key: &str, entry: &str| {
        let dict = document.get_key(usage, key);
        let dict = dict.as_dict().cloned().unwrap_or_default();
        document
            .get_key(&dict, entry)
            .as_name()
            .map(|name| name.as_bytes().to_vec())
    };
    // Only `OFF` recommends off. A name that is not `ON`, and an absent entry, both leave the
    // running AND alone: the clause's rule is "if all the entries yield a recommended state of
    // ON", and an entry the usage dictionary does not have yields none.
    let on_or_off = |value: Option<Vec<u8>>| {
        if value.as_deref() == Some(b"OFF") {
            Recommendation::Off
        } else {
            Recommendation::On
        }
    };

    match category {
        b"View" => on_or_off(state("View", "ViewState")),
        b"Export" => on_or_off(state("Export", "ExportState")),
        b"Print" => match state("Print", "PrintState") {
            None => Recommendation::Unchanged,
            value => on_or_off(value),
        },
        b"Zoom" => {
            let zoom = document.get_key(usage, "Zoom");
            let Some(zoom) = zoom.as_dict() else {
                return Recommendation::On;
            };
            let bound =
                |key: &str, default: f32| {
                    document.get_key(zoom, key).as_number().map_or(default, |value| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a magnification outside f32's range is not a magnification"
                    )]
                    {
                        value as f32
                    }
                })
                };
            // "greater than or equal to min and less than max", with Table 100's defaults of
            // 0 and infinity.
            let (low, high) = (bound("min", 0.0), bound("max", f32::INFINITY));
            if magnification >= low && magnification < high {
                Recommendation::On
            } else {
                Recommendation::Off
            }
        }
        b"User" => Recommendation::Unanswerable("User"),
        b"Language" => Recommendation::Unanswerable("Language"),
        // Table 101 requires each name to correspond to a Table 100 entry; one that does not
        // corresponds to no usage entry, so it recommends nothing.
        _ => Recommendation::On,
    }
}
