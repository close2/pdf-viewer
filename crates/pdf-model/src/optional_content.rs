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
//! **This section listed five entries and four of them arrived**, and it went on saying they had
//! not for as long as the panel has existed. It read: "[t]he states of groups are read from the
//! document and never changed. §8.11.4.5's automatic adjustment from usage application
//! dictionaries (`/AS`), the radio-button relationships of `/RBGroups`, `/Locked`, the
//! presentation `/Order` and the alternate `/Configs` all describe an interactive processor
//! offering the user a layer panel … When a layer panel exists, this is the module it attaches
//! to." The panel exists, this is the module it attached to, and the sentence stayed — which is
//! the "capability that arrived and announced nothing" shape `doc/todo/02` §1 names. `/AS` is
//! [`apply_event`], `/RBGroups` is [`OptionalContent::apply`]'s exclusion, `/Locked` is
//! [`OptionalContent::is_locked`] and `/Order` is [`presentation`].
//!
//! What is genuinely not here is Table 98's `/Configs` and the `/Name` and `/Creator` of a
//! configuration, all three of which exist so that a person may choose *between* configurations
//! (§8.11.4.3); nothing in this tree offers that choice, so the default configuration is the only
//! one read. None of the three affects the initial state, which is the state §8.11.4.5 says every
//! processor starts from.

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

/// What the output being produced is for: §8.11.4.4's three events, as a host states them.
///
/// Table 101's `/Event` "[s]hall be one of View, Print , or Export", and §8.11.4.5 says when each
/// one runs. The `View` dictionaries run when the document is opened and again whenever a factor
/// they depend on moves; the other two run over an operation that is under way:
///
/// > When a document is printed by an interactive PDF processor, usage application dictionaries
/// > with an event type Print shall be applied over the current states of optional content
/// > groups. These changes shall persist only for the duration of the print operation; then all
/// > groups shall revert to their prior states.
///
/// > Similarly, when a document is exported to a format that does not support optional content,
/// > usage application dictionaries with an event type Export shall be applied over the current
/// > states of optional content groups. Changes shall persist only for the duration of the export
/// > operation; then all groups shall revert to their prior states.
///
/// The same answer decides §8.9.5.4 step c)'s "the PDF is being printed", which is the only other
/// place in this crate where what the output is *for* decides a mark. One input, asked once.
///
/// **It is a host's or an operation's to state and never this crate's to infer**, which is
/// `CLAUDE.md` principle 3's rule and the one [`Audience`] arrives under:
/// [`crate::view::ViewState::set_purpose`] is the only channel, and [`Purpose::View`] is what
/// every caller that says nothing gets. ADR 1173.
///
/// **This is not Table 101's `/Category`**, and §8.11.4.5 NOTE 3 is why the two are separate
/// types: "[a]lthough the event types Print and Export have identically named counterparts that
/// are usage categories, the corresponding usage application dictionaries are permitted to
/// specify that other categories can be applied."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Purpose {
    /// A reader is looking at the page. Table 101's `View`.
    #[default]
    View,
    /// The page is being printed. Table 101's `Print`, and §8.9.5.4 step c)'s condition.
    Print,
    /// The document, or part of it, is being saved to a format that cannot carry optional
    /// content — Table 100's `Export`, whose own example of such a format is "a raster image
    /// format".
    Export,
}

impl Purpose {
    /// Reads Table 101's `/Event`, which the table makes required and admits three values of.
    ///
    /// `None` for anything else: an application dictionary naming a fourth event names no
    /// situation this or any other processor is ever in, so it applies to nothing.
    fn read(name: &[u8]) -> Option<Self> {
        match name {
            b"View" => Some(Self::View),
            b"Print" => Some(Self::Print),
            b"Export" => Some(Self::Export),
            _ => None,
        }
    }
}

/// The magnification §8.11.4.4's `Zoom` category is answered at when no caller has stated one.
///
/// 1.0 is the size a page is drawn at when nothing states otherwise, which is what
/// `ViewState::magnification`'s `None` means — *nobody has said* — rather than a guess at what a
/// window is doing. Every gate in this tree is in that position, so the answer they get is the
/// answer they got before a magnification could reach this module at all.
const UNSTATED_MAGNIFICATION: f32 = 1.0;

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
    /// Every group `/OCProperties /OCGs` lists, with the state it stands at now.
    states: BTreeMap<ObjectId, bool>,
    /// The same groups with §8.11.4.5 b)'s state, before any usage application dictionary.
    ///
    /// Kept because the reapplication that clause requires has to start from somewhere that is
    /// not the answer to the last zoom: "[t]his state shall be the initial state used by all PDF
    /// processors", and the automatic adjustment is stated as running over it.
    initial: BTreeMap<ObjectId, bool>,
    /// Every usage application dictionary the configuration states, resolved once; see
    /// [`UsageApplication`].
    applications: Vec<UsageApplication>,
    /// The states as a `Print` or `Export` event leaves them, or `None` under [`Purpose::View`].
    ///
    /// §8.11.4.5 gives each of those two events a *duration* — "[t]hese changes shall persist
    /// only for the duration of the print operation; then all groups shall revert to their prior
    /// states" — so the event is an overlay over [`Self::states`] rather than a write into it,
    /// and the reverting is deleting the overlay. Nothing can drift, and no caller has to
    /// remember to undo anything. ADR 1173.
    under_event: Option<BTreeMap<ObjectId, bool>>,
    /// Groups a manual change has pinned, which the reapplication may not touch.
    ///
    /// §8.11.4.5: "[m]anual changes shall override the states that were set automatically. The
    /// states of these groups remain overridden and shall not be readjusted based on usage
    /// application dictionaries with event type View as long as the document is open."
    overridden: BTreeSet<ObjectId>,
    /// Groups the configuration's `/Intent` does not cover, which therefore have no effect
    /// on visibility (§8.11.2.3).
    disregarded: BTreeSet<ObjectId>,
    /// Usage categories §8.11.4.4 asks a *viewer* for and this one cannot answer.
    ///
    /// Reported rather than guessed; see [`apply_event`] and [`Audience`].
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

        let initial = states.clone();
        let applications = usage_applications(document, &configuration, &states);
        // The magnification a page is drawn at when nothing states one, no audience, and the
        // `View` event: a document being opened has had neither a zoom nor a host's answer yet,
        // and what it is being opened *for* arrives through `ViewState` afterwards too.
        let unresolved = apply_event(
            &applications,
            Purpose::View,
            &initial,
            &BTreeSet::new(),
            &mut states,
            UNSTATED_MAGNIFICATION,
            &Audience::NONE,
        );

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
        // An entry that is not an array of arrays states no collection. (Errata Collection 3 —
        // Issue #225, `/State` `Review` `Completed` — struck the row's ", each of which
        // represents a collection" in favour of a sentence of its own, and adds "None of the
        // inner array elements shall be an empty array."; an empty one is dropped below, which
        // excludes nothing either way.)
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
            initial,
            applications,
            under_event: None,
            overridden: BTreeSet::new(),
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
    ///
    /// **The change is made to the view-side states and never to a `Print` or `Export` event's
    /// overlay**, which is the right half of the pair: §8.11.4.5 gives that overlay a duration
    /// and says the groups "revert to their prior states" afterwards, so a change made while one
    /// stands is a change to what the person will see when it is gone. `ViewState` recomputes
    /// the overlay over the result, so the two stay in step. ADR 1173.
    pub fn apply(&mut self, changes: &[(ObjectId, Change)], preserve_radio_buttons: bool) {
        for (group, change) in changes {
            let Some(current) = self.states.get_mut(group) else {
                continue;
            };
            let now = change.applied_to(*current);
            *current = now;
            // §8.11.4.5: "[m]anual changes shall override the states that were set
            // automatically … and shall not be readjusted based on usage application
            // dictionaries with event type View as long as the document is open". A change
            // recorded here is one of the two the clause names in the sentence before that —
            // a person at a panel, or §12.6.4.13's action — so both pin the group.
            self.overridden.insert(*group);
            if now && preserve_radio_buttons {
                self.exclude_others(*group);
            }
        }
    }

    /// §8.11.4.5's reapplication, and the event whatever the output is *for* applies over it.
    ///
    /// > Whenever there is a change to a factor that the usage application dictionaries with
    /// > event type View depend on (such as zoom level), the corresponding dictionaries shall be
    /// > reapplied.
    ///
    /// The factors are the three a host holds and a document cannot: the magnification the page
    /// is being drawn at, [`Audience`]'s answers to the `User` and `Language` categories, and
    /// [`Purpose`] — what the output is for. `None` for the magnification is
    /// `ViewState::magnification`'s own *nobody has said*, and is answered at
    /// [`UNSTATED_MAGNIFICATION`].
    ///
    /// **Two applications, in the clause's own order and over different bases.** The `View`
    /// dictionaries run first, over §8.11.4.5 b)'s initial state, and write [`Self::states`].
    /// Then, where the purpose is not [`Purpose::View`], that event's dictionaries run "over the
    /// current states" and their result becomes [`Self::under_event`] — an overlay, because the
    /// clause gives them a duration rather than a destination. ADR 1173.
    ///
    /// Returns whether any group's state moved, so a caller can decide whether the page has to
    /// be interpreted again — and, because §8.11 decides what is *drawn*, whether the ink it
    /// already has is superseded rather than merely re-placed.
    ///
    /// **Costs one empty-vector test on a document that states no `/AS`**, which
    /// `examples/oc_usage_census` measures as all but 475 of the 65 720 crawl documents that
    /// open and all but six of the pdf.js corpus: the dictionaries are resolved once by
    /// [`Self::read`], so nothing here reads the document.
    pub fn reapply(
        &mut self,
        magnification: Option<f32>,
        audience: &Audience,
        purpose: Purpose,
    ) -> bool {
        if self.applications.is_empty() {
            return false;
        }
        let before = self.effective().clone();
        let magnification = magnification.unwrap_or(UNSTATED_MAGNIFICATION);
        self.unresolved = apply_event(
            &self.applications,
            Purpose::View,
            &self.initial,
            &self.overridden,
            &mut self.states,
            magnification,
            audience,
        );
        self.under_event = match purpose {
            Purpose::View => None,
            event => {
                // "applied over the current states of optional content groups", so the base is
                // what stands now rather than §8.11.4.5 b)'s initial state — and nothing is
                // skipped, because the sentence that pins a manual change names only the `View`
                // event: "shall not be readjusted based on usage application dictionaries with
                // event type View as long as the document is open".
                let base = self.states.clone();
                let mut under = base.clone();
                let also = apply_event(
                    &self.applications,
                    event,
                    &base,
                    &BTreeSet::new(),
                    &mut under,
                    magnification,
                    audience,
                );
                for name in also {
                    if !self.unresolved.contains(&name) {
                        self.unresolved.push(name);
                    }
                }
                Some(under)
            }
        };
        // Table 99's `/Order` carries identities and not states, so the panel tree stands
        // whatever moves here; what a caller reads a state through is [`Self::state`].
        *self.effective() != before
    }

    /// The states a caller reads: the event's overlay where one stands, else [`Self::states`].
    fn effective(&self) -> &BTreeMap<ObjectId, bool> {
        self.under_event.as_ref().unwrap_or(&self.states)
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
                // Switched off as a consequence of a manual change, so pinned for the same
                // sentence: a reapplication that turned one of these back on would put two
                // members of one collection on, which Table 99 says cannot happen.
                self.overridden.insert(other);
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
    /// Empty for every document that names none, which is all 974 in the corpus, and empty
    /// again wherever a host has answered: see [`Audience`] for the two questions and
    /// [`apply_event`] for why the clause's "otherwise OFF" is the answer to a comparison rather
    /// than to there being nobody to compare with.
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
        self.effective().get(&group).copied()
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

/// One of Table 101's usage application dictionaries, read once per document.
///
/// §8.11.4.5 states when each runs: the base state and the `/ON`/`/OFF` arrays give "the initial
/// state used by all PDF processors", and then an interactive processor "shall examine the AS
/// array for usage application dictionaries that have an Event of type View. For each one
/// found, the groups listed in its OCGs array shall be adjusted". The other two events run over
/// an operation — a print, or an export to a format that cannot carry optional content — and
/// [`Purpose`] is the input that says which operation is under way.
///
/// **Read once, applied many times, and that is what the next sentence of §8.11.4.5 costs**:
/// "[w]henever there is a change to a factor that the usage application dictionaries with event
/// type View depend on (such as zoom level), the corresponding dictionaries shall be reapplied".
/// A reapplication that re-read the document would put dictionary lookups on every step of a
/// zoom gesture; resolving Table 100's entries here instead makes [`apply_event`] a function of
/// two numbers and a list, and makes the 99.9% of documents that state no `/AS` at all cost one
/// empty-vector test per zoom. The census behind that share is `examples/oc_usage_census`.
///
/// All three events are resolved here rather than the `View` ones alone, because a print or an
/// export must not put dictionary lookups in front of the operation it belongs to either — and
/// the whole `/AS` array is already being walked. Of the pdf.js corpus's 963 documents that open,
/// six state an `/AS` at all, naming `View` six times, `Print` six and `Export` five.
#[derive(Debug, Clone, PartialEq, Eq)]
struct UsageApplication {
    /// Table 101's `/Event`: the situation this dictionary is for.
    event: Purpose,
    /// Table 101's `/Category`, in the order the array states them.
    categories: Vec<Category>,
    /// Table 101's `/OCGs`, restricted to groups the document declares, each with its
    /// `/Usage` dictionary.
    ///
    /// Restricted for [`OptionalContent::read`]'s reason: Table 98 requires `/OCGs` to list
    /// every group in the document, so an application naming something else names nothing.
    groups: Vec<(ObjectId, Usage)>,
}

/// One of Table 101's `/Category` names, against the six §8.11.4.4 states a rule for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    /// The state the group's `/View` `/ViewState` entry names.
    View,
    /// The state the group's `/Print` `/PrintState` entry names, or unchanged where it has none.
    Print,
    /// The state the group's `/Export` `/ExportState` entry names.
    Export,
    /// The magnification range, read against the one the page is being drawn at.
    Zoom,
    /// Who is reading, which is a question about this processor; see [`Audience`].
    User,
    /// Which language this application is in, likewise.
    Language,
    /// A name §8.11.4.4's list does not reach, which therefore recommends nothing.
    ///
    /// Two different things land here and neither yields a state. `PageElement` and
    /// `CreatorInfo` *are* Table 100 entries, and §8.11.4.4's "[t]he entries in the usage
    /// dictionary shall be used as follows" list states no rule for either — a pagination
    /// artifact and an authoring application's private data are descriptions rather than
    /// recommendations. A name that is no Table 100 entry at all corresponds to no usage entry,
    /// which Table 101 requires it to do.
    Unstated,
}

impl Category {
    /// Reads one element of Table 101's `/Category` array.
    fn read(name: &[u8]) -> Self {
        match name {
            b"View" => Self::View,
            b"Print" => Self::Print,
            b"Export" => Self::Export,
            b"Zoom" => Self::Zoom,
            b"User" => Self::User,
            b"Language" => Self::Language,
            _ => Self::Unstated,
        }
    }
}

/// Table 100's `/Zoom` range: "greater than or equal to min and less than max".
///
/// `Eq` is implemented rather than derived because the bounds are magnifications. Both come
/// from a number the document states or from Table 100's own defaults of 0 and infinity, and
/// §7.3.3's real object is digits, an optional sign and an optional point — so neither can be
/// NaN, equality here is reflexive, and the trait's contract holds.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ZoomRange {
    /// `min`: "the minimum recommended magnification factor at which the group shall be ON".
    low: f32,
    /// `max`: "the magnification factor below which the group shall be ON".
    high: f32,
}

impl Eq for ZoomRange {}

/// Table 100's `/User` `/Type`: how the `/Name` entry beside it shall be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UserType {
    /// `Ind`, an individual.
    Individual,
    /// `Ttl`, a title or position.
    Title,
    /// `Org`, an organisation.
    Organisation,
}

/// One group's `/Usage` dictionary, resolved for the categories §8.11.4.4 states a rule for.
///
/// Every field is `Option`-shaped around the same sentence of the clause's own example: of a
/// group whose usage dictionary states nothing for the category being applied, "Object 4 has
/// none; therefore, it is not affected by zoom level changes". An entry that is not there
/// yields no recommendation rather than an OFF.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Usage {
    /// `/View` `/ViewState`, `false` only where the name is `OFF`; see [`recommendation`].
    ///
    /// `None` where the entry is absent, which is the clause's own "Object 4 has none;
    /// therefore, it is not affected" — a category with nothing to read recommends nothing. Under
    /// an event that *assigns*, the difference between recommending nothing and recommending ON
    /// is the difference between leaving a layer alone and turning it on.
    view: Option<bool>,
    /// `/Export` `/ExportState`, on the same rule.
    export: Option<bool>,
    /// `/Print` `/PrintState`, `None` where the entry is absent, which §8.11.4.4 states outright
    /// for this one category: "the state of the optional content group shall be left unchanged".
    print: Option<bool>,
    /// `/Zoom`, where the group states one.
    zoom: Option<ZoomRange>,
    /// `/User`: how `/Name` shall be read, and the names to match.
    ///
    /// `None` where `/User` is absent, and also where its `/Type` is not one of the three
    /// Table 100 requires — a dictionary that does not say how its names are to be
    /// interpreted states nothing to match against, which is the same position as stating no
    /// `/User` at all.
    user: Option<(UserType, Vec<String>)>,
    /// `/Language`: `/Lang`, and whether `/Preferred` is `ON`.
    ///
    /// Table 100: `Lang` is required and `Preferred` defaults to `OFF`. An empty tag is
    /// §14.9.2.2's "the empty text string, to indicate that the language is unknown", which
    /// matches nothing.
    language: Option<(String, bool)>,
}

/// §8.11.4.4's answers about *this* processor, which a host supplies.
///
/// Two of Table 100's categories ask questions no document can answer and no renderer may
/// invent. §8.11.4.4, of `User`: "[t]he Name entry shall specify a name or names to match with
/// the user's identification"; and of `Language`: "[t]his category shall allow the selection of
/// content based on the language and locale of the application". Both are facts about the
/// machine and the person in front of it, so `CLAUDE.md` principle 3's rule applies — the
/// policy is asked once, in a place a host can supply — and the surfaces are the ones
/// ADR 1076 and ADR 1101 built for a trust anchor and a referenced file.
///
/// **[`Audience::NONE`] is the default and is what every caller that says nothing gets.** Under
/// it both categories are [`Recommendation::Unanswerable`], the configuration's own state
/// stands, and the page says which category it could not answer — which is the position this
/// program was in before a host could answer at all. ADR 1106.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Audience {
    /// Table 100's `/User`: who the reader is, under each of the three `/Type` values.
    pub reader: Reader,
    /// "the language and locale of the application", as §14.9.2.2's BCP 47 language tag.
    ///
    /// `None` is *nobody has said*, and is not the same as the empty string, which
    /// §14.9.2.2 gives a meaning of its own: "the empty text string, to indicate that the
    /// language is unknown".
    pub language: Option<String>,
}

impl Audience {
    /// Nobody, and no language.
    pub const NONE: Self = Self {
        reader: Reader::NONE,
        language: None,
    };

    /// Whether a host has answered either question.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.reader.is_empty() && self.language.is_none()
    }
}

/// Who a host says is reading, under Table 100's three `/User` `/Type` values.
///
/// Three lists rather than one, because Table 100 makes `/Type` decide what the names beside it
/// mean — "A name object that shall be either Ind (individual), Ttl (title or position), or Org
/// (organisation)" — so a document asking which organisation a reader belongs to is asking a
/// different question from one asking the reader's name, and a host that answered only the
/// first must not be read as having answered the second.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reader {
    /// `/Ind`: the individual's name or names.
    pub individual: Vec<String>,
    /// `/Ttl`: the title or position held.
    pub title: Vec<String>,
    /// `/Org`: the organisation or organisations.
    pub organisation: Vec<String>,
}

impl Reader {
    /// Nobody.
    pub const NONE: Self = Self {
        individual: Vec::new(),
        title: Vec::new(),
        organisation: Vec::new(),
    };

    /// Whether a host has said who is reading, under any of the three types.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.individual.is_empty() && self.title.is_empty() && self.organisation.is_empty()
    }

    /// What this reader is known as, under the type a group's `/User` dictionary states.
    fn known_as(&self, kind: UserType) -> &[String] {
        match kind {
            UserType::Individual => &self.individual,
            UserType::Title => &self.title,
            UserType::Organisation => &self.organisation,
        }
    }
}

/// Reads the usage application dictionaries of one configuration, resolving Table 100.
///
/// `states` is the initial state map, which decides which groups the document declares.
fn usage_applications(
    document: &Document,
    configuration: &Dictionary,
    states: &BTreeMap<ObjectId, bool>,
) -> Vec<UsageApplication> {
    let auto = document.get_key(configuration, "AS");
    let Some(applications) = auto.as_array() else {
        // "If no AS entry is present, states shall not be automatically adjusted based on
        // usage information."
        return Vec::new();
    };

    let mut read = Vec::new();
    for application in applications {
        let application = document.resolve(application);
        let Some(application) = application.as_dict() else {
            continue;
        };
        // Table 101 makes `/Event` required and admits three names; a dictionary stating any
        // other names no situation, so it is read for none of them.
        let Some(event) = document
            .get_key(application, "Event")
            .as_name()
            .and_then(|name| Purpose::read(name.as_bytes()))
        else {
            continue;
        };
        let categories: Vec<Category> = document
            .get_key(application, "Category")
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        document
                            .resolve(item)
                            .as_name()
                            .map(|name| Category::read(name.as_bytes()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if categories.is_empty() {
            continue;
        }
        let groups: Vec<(ObjectId, Usage)> = listed(document, application, "OCGs")
            .into_iter()
            // Adjusted, never added, for the same reason the `/ON` and `/OFF` arrays are.
            .filter(|group| states.contains_key(group))
            .filter_map(|group| {
                let dictionary = document.get(group);
                let dictionary = dictionary.as_dict()?;
                Some((group, usage_of(document, dictionary)))
            })
            .collect();
        read.push(UsageApplication {
            event,
            categories,
            groups,
        });
    }
    read
}

/// Resolves one group's Table 100 `/Usage` dictionary.
fn usage_of(document: &Document, group: &Dictionary) -> Usage {
    let usage = document.get_key(group, "Usage");
    let usage = usage.as_dict().cloned().unwrap_or_default();
    let state = |key: &str, entry: &str| {
        let dict = document.get_key(&usage, key);
        let dict = dict.as_dict().cloned().unwrap_or_default();
        document
            .get_key(&dict, entry)
            .as_name()
            .map(|name| name.as_bytes() != b"OFF")
    };

    let zoom = document.get_key(&usage, "Zoom");
    let zoom = zoom.as_dict().map(|zoom| {
        let bound = |key: &str, default: f32| {
            document
                .get_key(zoom, key)
                .as_number()
                .map_or(default, |value| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a magnification outside f32's range is not a magnification"
                    )]
                    {
                        value as f32
                    }
                })
        };
        // Table 100's defaults: "Default value: 0" and "Default value: infinity".
        ZoomRange {
            low: bound("min", 0.0),
            high: bound("max", f32::INFINITY),
        }
    });

    let user = document.get_key(&usage, "User");
    let user = user.as_dict().and_then(|user| {
        let kind = match document.get_key(user, "Type").as_name().map(Name::as_bytes) {
            Some(b"Ind") => UserType::Individual,
            Some(b"Ttl") => UserType::Title,
            Some(b"Org") => UserType::Organisation,
            _ => return None,
        };
        // "A text string or array of text strings representing the name(s) of the individual,
        // position or organisation."
        let names = match document.get_key(user, "Name") {
            Object::String(bytes) => vec![pdf_syntax::text_string(&bytes)],
            Object::Array(items) => items
                .iter()
                .map(|item| document.resolve(item))
                .filter_map(|item| match item {
                    Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        (!names.is_empty()).then_some((kind, names))
    });

    let language = document.get_key(&usage, "Language");
    let language = language.as_dict().and_then(|language| {
        let tag = match document.get_key(language, "Lang") {
            Object::String(bytes) => pdf_syntax::text_string(&bytes),
            // `Lang` is Table 100's one required entry of this dictionary; without it the
            // dictionary states no language to match.
            _ => return None,
        };
        let preferred = document
            .get_key(language, "Preferred")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"ON");
        Some((tag, preferred))
    });

    Usage {
        view: state("View", "ViewState"),
        export: state("Export", "ExportState"),
        print: state("Print", "PrintState"),
        zoom,
        user,
        language,
    }
}

/// §8.11.4.4's automatic state adjustment, for one of Table 101's three events.
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
/// # It sets, and a category that read nothing does not make it set ON
///
/// "[S]hall be set to ON" is an assignment over `base` rather than an AND with it, and the
/// clause's own example is what proves it: under `/BaseState /OFF` with `/ON [1 0 R]`, objects
/// 2 and 3 start off, and the example says the `View` dictionary "specifies that all optional
/// content groups have their states managed based on zoom level when viewing" — which a rule
/// that could only ever turn a group off would not do for either of them.
///
/// What makes the assignment safe is [`Recommendation::Unchanged`]: a group is assigned only
/// where some category actually read a value, so a `/Category` naming an entry the group does
/// not state leaves it exactly where `base` put it. The two halves are one decision — an
/// assignment without the three-valued category would switch on every group whose usage
/// dictionary is silent. ADR 1173.
///
/// # The base, the skip, and the two events that do not touch `initial`
///
/// `base` is where a named group starts from before its categories are read, and the clause
/// gives the events different ones. For `View` it is §8.11.4.5 b)'s initial state — after
/// `/BaseState` and the array opposite it — so that the answer is a function of the factors
/// rather than of the order the zoom steps arrived in, which is what the reapplication requires:
/// "[w]henever there is a change to a factor that the usage application dictionaries with event
/// type View depend on (such as zoom level), the corresponding dictionaries shall be reapplied".
/// For `Print` and `Export` it is the states as they stand, because those two are "applied over
/// the current states of optional content groups".
///
/// `skip` is the exception the same clause writes, and it is written for `View` alone: "[m]anual
/// changes shall override the states that were set automatically. The states of these groups
/// remain overridden and shall not be readjusted based on usage application dictionaries with
/// event type View as long as the document is open". [`OptionalContent::apply`] is where a group
/// joins that set, and a `Print` or `Export` event passes an empty one — the sentence names the
/// event it pins against, and a person who switched a layer on to look at it has not said what
/// should happen to it on paper.
///
/// # The two categories a host answers, and the one it need not
///
/// `Zoom` is answered at the magnification the page is being drawn at, which `ViewState` has
/// carried since §12.5.3's `NoZoom` needed it (ADR 0168), and at 1.0 — the size a page is drawn
/// at when nothing states otherwise — where no caller has said.
///
/// `User` and `Language` are questions about this processor, and [`Audience`] is where a host
/// answers them. With an answer, §8.11.4.4's own sentences decide; with none, the category is
/// [`Recommendation::Unanswerable`], the state stands and the page reports it. **That is the
/// clause's own division rather than a departure from it.** Of `User` it writes "[i]f there is
/// an exact match, the ON state shall be used; otherwise OFF shall be used" — and "otherwise" is
/// the second branch of a *comparison with the user's identification*, not a verdict on there
/// being nobody to compare with. So a host that says who is reading gets the OFF, and a machine
/// that has not been told anything is outside both branches: switching a group off there would
/// hide content on the strength of a question nobody asked.
fn apply_event(
    applications: &[UsageApplication],
    event: Purpose,
    base: &BTreeMap<ObjectId, bool>,
    skip: &BTreeSet<ObjectId>,
    states: &mut BTreeMap<ObjectId, bool>,
    magnification: f32,
    audience: &Audience,
) -> Vec<&'static str> {
    let mut unresolved: Vec<&'static str> = Vec::new();
    let applications: Vec<&UsageApplication> = applications
        .iter()
        .filter(|application| application.event == event)
        .collect();

    for application in &applications {
        for (group, _) in &application.groups {
            if !skip.contains(group)
                && let Some(was) = base.get(group)
                && let Some(entry) = states.get_mut(group)
            {
                *entry = *was;
            }
        }
    }

    // The AND across dictionaries is accumulated per group before anything is written, because
    // the clause states it over "all the usage application dictionaries it appears in" at once:
    // a group named twice gets one verdict, not two assignments.
    let mut verdicts: BTreeMap<ObjectId, Verdict> = BTreeMap::new();
    for application in &applications {
        let language = language_recommendations(application, audience.language.as_deref());
        for (group, usage) in &application.groups {
            if skip.contains(group) {
                continue;
            }
            let verdict = verdicts.entry(*group).or_default();
            for category in &application.categories {
                let answer = match (category, &language) {
                    (Category::Language, None) => Recommendation::Unanswerable("Language"),
                    (Category::Language, Some(on)) => {
                        // "All other groups shall receive an OFF recommendation."
                        if on.contains(group) {
                            Recommendation::On
                        } else {
                            Recommendation::Off
                        }
                    }
                    _ => recommendation(*category, usage, magnification, &audience.reader),
                };
                match answer {
                    Recommendation::On => verdict.stated = true,
                    Recommendation::Off => {
                        verdict.stated = true;
                        verdict.on = false;
                    }
                    // The category read nothing, so it yields no recommended state and takes no
                    // part in the AND: "if all the entries yield a recommended state of ON".
                    Recommendation::Unchanged => {}
                    Recommendation::Unanswerable(name) => {
                        if !unresolved.contains(&name) {
                            unresolved.push(name);
                        }
                        verdict.answerable = false;
                    }
                }
            }
        }
    }

    for (group, verdict) in verdicts {
        if verdict.answerable
            && verdict.stated
            && let Some(entry) = states.get_mut(&group)
        {
            *entry = verdict.on;
        }
    }

    unresolved
}

/// What every category of every usage application dictionary of one event said about one group.
///
/// `stated` is whether any of them yielded a recommended state at all; where none did, the group
/// is left where `apply_event`'s base put it. `answerable` is whether every category this
/// processor was asked could be answered — see [`Audience`].
#[derive(Debug, Clone, Copy)]
struct Verdict {
    /// Whether any category yielded a recommended state.
    stated: bool,
    /// The AND of the states that were yielded: "If all the entries yield a recommended state of
    /// ON , the group's state shall be set to ON ; otherwise, its state shall be set to OFF".
    on: bool,
    /// Whether every category could be answered at all.
    answerable: bool,
}

impl Default for Verdict {
    fn default() -> Self {
        Self {
            stated: false,
            on: true,
            answerable: true,
        }
    }
}

/// §8.11.4.4's `Language` rule, which is stated over a whole `/OCGs` list at once.
///
/// > If an exact match to the language and locale is found among the Lang entries of the
/// > optional content groups in the usage application dict ionary's OCGs list, all groups that
/// > have exact matches shall receive an ON recommendation. If no exact match is found, but a
/// > partial match is found (that is, the language matches but not the locale), all partially
/// > matching groups that have Preferred entries with a value of ON shall receive an ON
/// > recommendation. All other groups shall receive an OFF recommendation.
///
/// The answer is therefore a property of the *list* and not of one group, which is why this is
/// computed per application rather than inside [`recommendation`]. `None` where the host has
/// said no language: there is nothing to find a match against, and the clause's three cases all
/// begin with one.
///
/// Comparison is ASCII case-insensitive on §14.9.2.2's authority — "all language tags shall be
/// treated as case-insensitive" — and a partial match is the primary language subtag alone,
/// which is what "the language matches but not the locale" names.
fn language_recommendations(
    application: &UsageApplication,
    language: Option<&str>,
) -> Option<BTreeSet<ObjectId>> {
    let wanted = language?;
    // §14.9.2.2: the empty text string "indicate[s] that the language is unknown", so a host
    // that supplied one has not named a language to match.
    if wanted.is_empty() {
        return None;
    }
    let primary = |tag: &str| tag.split('-').next().unwrap_or(tag).to_owned();
    let wanted_primary = primary(wanted);

    let tags: Vec<(ObjectId, &str, bool)> = application
        .groups
        .iter()
        .filter_map(|(group, usage)| {
            let (tag, preferred) = usage.language.as_ref()?;
            (!tag.is_empty()).then_some((*group, tag.as_str(), *preferred))
        })
        .collect();

    let exact: BTreeSet<ObjectId> = tags
        .iter()
        .filter(|(_, tag, _)| tag.eq_ignore_ascii_case(wanted))
        .map(|(group, _, _)| *group)
        .collect();
    if !exact.is_empty() {
        return Some(exact);
    }
    Some(
        tags.iter()
            .filter(|(_, tag, preferred)| {
                *preferred && primary(tag).eq_ignore_ascii_case(&wanted_primary)
            })
            .map(|(group, _, _)| *group)
            .collect(),
    )
}

/// What one of Table 100's categories recommends for a group.
enum Recommendation {
    On,
    Off,
    /// The category read nothing, so it recommends nothing and the group is left where it is.
    ///
    /// §8.11.4.4 writes it outright for one category — `Print` with no `/PrintState`: "the
    /// state … shall be left unchanged" — and its own `Zoom` example writes the general rule:
    /// "Object 4 has none; therefore, it is not affected by zoom level changes". A `/Category`
    /// name that is no Table 100 entry, and one whose entry the group does not state, both land
    /// here.
    Unchanged,
    /// A category this processor cannot answer; see [`apply_event`].
    Unanswerable(&'static str),
}

/// §8.11.4.4's per-category rule, for one group's resolved usage dictionary.
///
/// [`Category::Language`] is not answered here: its rule is stated over the whole `/OCGs` list,
/// and [`language_recommendations`] is where that lives.
fn recommendation(
    category: Category,
    usage: &Usage,
    magnification: f32,
    reader: &Reader,
) -> Recommendation {
    // Only `OFF` recommends off. A name that is not `ON`, and an absent entry, both leave the
    // running AND alone: the clause's rule is "if all the entries yield a recommended state of
    // ON", and an entry the usage dictionary does not have yields none.
    let on_or_off = |on: bool| {
        if on {
            Recommendation::On
        } else {
            Recommendation::Off
        }
    };

    match category {
        Category::View => usage.view.map_or(Recommendation::Unchanged, on_or_off),
        Category::Export => usage.export.map_or(Recommendation::Unchanged, on_or_off),
        Category::Print => usage.print.map_or(Recommendation::Unchanged, on_or_off),
        Category::Zoom => usage.zoom.map_or(Recommendation::Unchanged, |range| {
            // "greater than or equal to min and less than max".
            on_or_off(magnification >= range.low && magnification < range.high)
        }),
        Category::User => match &usage.user {
            None => Recommendation::Unchanged,
            Some((kind, names)) => match reader.known_as(*kind) {
                // Nobody has said who is reading, under the type this group asks about.
                [] => Recommendation::Unanswerable("User"),
                held => on_or_off(names.iter().any(|name| held.contains(name))),
            },
        },
        // Answered by the caller, and reached only if one forgot to.
        Category::Language => Recommendation::Unanswerable("Language"),
        Category::Unstated => Recommendation::Unchanged,
    }
}
