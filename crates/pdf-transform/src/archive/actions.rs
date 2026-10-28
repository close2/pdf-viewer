//! ISO 19005-2 sections 6.4.1, 6.5.1 and 6.5.2, ISO 19005-4 sections 6.4.1, 6.6.1 and 6.6.3:
//! the actions a target does not admit, taken out of the file.
//!
//! # One walk, three rewrites
//!
//! The three acts the parts ask for are different enough to be counted apart and alike enough to
//! be computed once, because they all read ISO 32000-2 §12.6's action trees:
//!
//! | rewrite | what goes |
//! |---|---|
//! | [`Rewrite::ForbiddenActionRemoved`] | an action whose type the part forbids, out of the tree it sits in |
//! | [`Rewrite::AdditionalActionsRemoved`] | an `/AA` entry, or the keys of one the part does not admit |
//! | [`Rewrite::WidgetActionEntryRemoved`] | the `/A` of a widget annotation or field dictionary |
//!
//! # What a removal may not leave behind
//!
//! §12.6.2's Table 196 gives an action a `/Next` —
//!
//! > The next action or sequence of actions that shall be performed after the action represented
//! > by this dictionary.
//!
//! — so an action removed from the middle of a chain would take the permitted actions behind it
//! with it. ADR 0947's second rule forbids that: nothing is changed that no failed requirement
//! asked for, and a `GoTo` behind a `Launch` is failing nothing. So a removed action's `/Next`
//! **takes its place**, which leaves the surviving actions in the order §12.6.2's NOTE 1 states
//! them:
//!
//! > Actions within each Next array are executed in order, each followed in turn by any actions
//! > specified in its Next entry, and so on recursively.
//!
//! Promoting a subtree therefore preserves that order exactly, and the one position that cannot
//! hold what comes back — an entry Table 196 types as a single action dictionary, with two
//! survivors to put in it — takes the first and hands the rest to *its* `/Next`, which is the
//! same sentence read the other way.
//!
//! # The fence
//!
//! Removing a dictionary entry invents no mark: every operator of every content stream crosses
//! this rewrite byte for byte, and what changes is what a reader *does* when a user clicks, which
//! is not something the page shows. `CLAUDE.md`'s fourth amendment makes provenance the test, and
//! nothing here writes a mark of any provenance at all (ADR 1175).

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::{
    ActionHolder, AdditionalActions, action_admitted, action_entry_admitted, action_sites,
    additional_actions_admitted, annotation_trigger,
};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use pdf_archive::Target;

use super::decision::{Because, Decision};
use super::report::Decided;
use super::rewrite::Rewrite;
use crate::json::Value;

/// How far a `/Next` chain is followed before it is treated as unbounded.
///
/// The validator's own bound, for its reason: a chain this long describes no document anybody
/// wrote, and following one in a hostile file is the shape of an exhaustion attack.
const MAX_DEPTH: usize = 32;

/// Why an action nothing can name is not removed.
///
/// §12.5.2's Table 166 and §12.7.4.1's Table 228 describe dictionaries a file normally writes as
/// objects, and files exist that write one directly into its parent. This rewrite acts on objects,
/// so there is nothing for it to replace — the same limit `prepare_forbidden_annotations` states.
const REMOVAL_FROM_A_DIRECT_HOLDER: &str = "an action a target does not admit is held by a \
     dictionary written directly into its parent rather than as an object of its own, and the \
     rewrite that removes one acts on objects. Nothing here can reach it";

/// Why a chain too long to follow stops the conversion.
const CHAIN_TOO_LONG: &str = "an action chain in this document is longer than the thirty-two \
     links this converter follows, so what survives the removal cannot be worked out without \
     following it further. ISO 32000-2 \u{a7}12.6.2's NOTE 1 recommends a processor guard against \
     self-referential actions, and this is that guard";

/// Why an action tree that comes back to itself stops the conversion.
const SELF_REFERENTIAL: &str = "an action in this document reaches itself through its own Next \
     chain, so there is no order in which the survivors of a removal would be performed. ISO \
     32000-2 \u{a7}12.6.2's NOTE 1 says self-referential actions ought not be executed more than \
     once, and a converter cannot decide which once";

/// Why one action reached from two chains that need different tails stops the conversion.
const SHARED_ACTION_TWO_TAILS: &str = "one action dictionary is performed from two places whose \
     removals leave different actions to follow it, so its Next entry would have to hold two \
     different sequences at once. Splitting the object would write an action dictionary this \
     document does not contain";

/// One action, or one action-holding entry, that this conversion took out.
///
/// **A list rather than a count**, for the reason [`super::prepare::RemovedAnnotation`] is one: an
/// action that is gone leaves nothing in the output to notice, and an operator who authorised
/// *buttons stop doing things* is owed the list of buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedAction {
    /// The zero-based page the holder was on, where it was on one.
    pub page: Option<usize>,
    /// The kind of dictionary the entry was written in.
    pub holder: &'static str,
    /// The entry it was reached through — `OpenAction`, `A`, `AA /U`, `Names /JavaScript`.
    pub entry: String,
    /// What went: the action's `/S`, or the words for an entry removed whole.
    pub what: String,
    /// Which of the three acts removed it, so that each can be counted apart.
    pub by: Rewrite,
}

impl RemovedAction {
    /// One removed action as JSON.
    pub(super) fn to_json(&self) -> Value {
        let mut fields = vec![
            ("holder".to_owned(), Value::text(self.holder.to_owned())),
            ("entry".to_owned(), Value::text(self.entry.clone())),
            ("removed".to_owned(), Value::text(self.what.clone())),
            ("rewrite".to_owned(), Value::text(self.by.word().to_owned())),
        ];
        if let Some(page) = self.page {
            fields.insert(0, ("page".to_owned(), Value::count(page)));
        }
        Value::Object(fields)
    }

    /// The holder's name, as the report prints it.
    const fn name_of(holder: ActionHolder) -> &'static str {
        match holder {
            ActionHolder::Catalog => "the document catalog",
            ActionHolder::Page => "a page",
            ActionHolder::Widget => "a widget annotation",
            ActionHolder::Field => "a form field",
            ActionHolder::Annotation => "an annotation",
            ActionHolder::OutlineItem => "an outline item",
        }
    }
}

/// What one object restates once the removals are worked out.
#[derive(Debug, Clone)]
pub(super) enum Edit {
    /// One entry restated, or removed where the value is `None`.
    Entry {
        /// The key whose value changes, which is always one this file names.
        key: &'static str,
        /// The new value, or `None` where the key goes.
        value: Option<Object>,
        /// Which rewrite asked for it, so the rewriter applies only what a decision wanted.
        by: Rewrite,
    },
    /// The whole dictionary restated, which is what a filtered `/AA` object becomes.
    ///
    /// An `/AA` whose keys are being filtered is an object whose *remaining* entries are the
    /// producer's and whose removed ones may be spelled with bytes no key literal here names,
    /// so what crosses is the dictionary rather than a list of keys to take out.
    Whole {
        /// The dictionary the object is to hold.
        dict: Dictionary,
        /// Which rewrite asked for it.
        by: Rewrite,
    },
}

impl Edit {
    /// Which rewrite asked for this edit.
    pub(super) const fn by(&self) -> Rewrite {
        match self {
            Self::Entry { by, .. } | Self::Whole { by, .. } => *by,
        }
    }
}

/// Every edit the action clauses ask of this document, worked out once.
#[derive(Debug, Default)]
pub(super) struct Removals {
    /// The entries each object restates, in the order they are applied.
    pub(super) edits: BTreeMap<ObjectId, Vec<Edit>>,
    /// Every action and every entry that went, for the report.
    pub(super) removed: Vec<RemovedAction>,
    /// What only the annotations a subtype row removes perform: those holders, and the action
    /// objects their trees reach that no kept holder's tree reaches too.
    gone: BTreeSet<ObjectId>,
}

impl Removals {
    /// How many places one of the three rewrites took something out of.
    pub(super) fn places(&self, rewrite: Rewrite) -> usize {
        self.removed.iter().filter(|row| row.by == rewrite).count()
    }

    /// Whether every place a requirement failed at goes with an annotation a subtype row removes.
    ///
    /// The validator names each failed action by its own object and an entry written into its
    /// holder by the holder's, and its walk visits a shared action once, under whichever holder
    /// came first — so a place is the removal's only where no kept holder's tree reaches it. A
    /// set cut short, a place naming no object, or nothing named at all is the action rows'
    /// own case, answered as before.
    pub(super) fn answered_by_removal(&self, named: &pdf_archive::Named) -> bool {
        !named.cut_short()
            && !named.elsewhere()
            && !named.objects().is_empty()
            && named.objects().is_subset(&self.gone)
    }
}

/// The requirement identifiers the three rewrites answer, so that a rule departed from or met
/// asks for nothing.
///
/// A rule whose requirement is not among the document's failures is **not in force here**: a
/// conversion that removed an `/AA` no requirement had reported would be changing what nothing
/// asked to change, which is ADR 0947's second rule.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per rule, the shape `Authorisations` takes for the same reason: a rule \
              added to the table is then a compile error everywhere it has to be answered rather \
              than a word nobody matched. The lint's remedy \u{2014} a set \u{2014} is what that \
              declines"
)]
struct InForce {
    /// Whether any of the six action-type rows failed.
    types: bool,
    /// Whether ISO 19005-2 section 6.5.2's row failed.
    additional_forbidden: bool,
    /// Whether ISO 19005-4 section 6.6.3's row failed.
    additional_restricted: bool,
    /// Whether both parts' section 6.4.1 row failed.
    entry_on_widget: bool,
}

/// The six rows whose answer is an action of a forbidden type removed.
const TYPE_ROWS: [&str; 6] = [
    "actions/no-launch-multimedia-or-form-actions",
    "actions/no-deprecated-set-state-or-no-op-actions",
    "actions/no-javascript-action",
    "actions/no-optional-content-or-view-action",
    "actions/optional-content-or-view-action-only-in-engineering-files",
    "actions/named-action-is-page-navigation",
];

impl InForce {
    /// Which rules this document's failures put in force.
    fn of(failed: &BTreeSet<&'static str>) -> Self {
        Self {
            types: TYPE_ROWS.iter().any(|id| failed.contains(id)),
            additional_forbidden: failed.contains("actions/no-additional-actions-dictionary"),
            additional_restricted: failed.contains(
                "actions/additional-actions-outside-widgets-hold-only-annotation-triggers",
            ),
            entry_on_widget: failed.contains("forms/no-action-on-widget-or-field"),
        }
    }

    /// Whether anything at all is to be done.
    const fn any(&self) -> bool {
        self.types
            || self.additional_forbidden
            || self.additional_restricted
            || self.entry_on_widget
    }
}

/// The rows whose answer is an `/AA` entry or a widget's `/A` taken out.
const ENTRY_ROWS: [&str; 3] = [
    "actions/no-additional-actions-dictionary",
    "actions/additional-actions-outside-widgets-hold-only-annotation-triggers",
    "forms/no-action-on-widget-or-field",
];

/// The rows whose answer is the removal [`prepare`] reads first: ISO 19005-2 and ISO 19005-4
/// section 6.3.1's subtype rows, and ISO 19005-4's two flavour rows the same removal answers.
const SUBTYPE_ROWS: [&str; 4] = [
    "annotations/subtype-defined-in-iso-32000-1",
    "annotations/subtype-defined-in-iso-32000-2",
    "annotations/three-dimensional-only-in-engineering-files",
    "annotations/file-attachment-only-in-embedded-file-files",
];

/// **An action row met by the annotation removal** (ADR 2568).
///
/// Where every place an action row failed at goes with an annotation a subtype row removes, the
/// output holds no tree for a reader to perform, so the row is met by the removal and its
/// decision is the one the subtype rows took: the same rewrite, the same loss under the same
/// authorisation or `preserve`, and nothing lost twice — ADR 1105's reading of the appearance row
/// taken to the action rows. **Only a removal that happens meets anything**: where a subtype row
/// stops the conversion, or none was decided, the action row keeps its own decision, and the
/// refusal is reported once, at the row it belongs to.
pub(super) fn met_by_the_removal(
    decided: &mut [Decided],
    input: &pdf_archive::Report,
    removals: &Removals,
) {
    let met: Vec<&'static str> = input
        .failures()
        .filter(|judgement| {
            (TYPE_ROWS.contains(&judgement.id) || ENTRY_ROWS.contains(&judgement.id))
                && removals.answered_by_removal(&judgement.named)
        })
        .map(|judgement| judgement.id)
        .collect();
    if met.is_empty() {
        return;
    }
    let removal: Vec<Decision> = decided
        .iter()
        .filter(|row| SUBTYPE_ROWS.contains(&row.requirement))
        .map(|row| row.decision)
        .collect();
    let Some(&first) = removal.first() else {
        return;
    };
    if !removal.iter().all(|decision| decision.proceeds()) {
        return;
    }
    for row in decided
        .iter_mut()
        .filter(|row| met.contains(&row.requirement))
    {
        row.decision = first;
    }
}

/// What the action clauses ask of this document, read off the sites the validator walks.
///
/// `removed` is the annotations the section 6.3.1 subtype rows take off their pages. Their
/// holders are not walked: an action tree the output does not hold performs nothing, so nothing
/// is taken out of it and its loss is the annotation's (ADR 2568).
pub(super) fn prepare(
    document: &Document,
    target: Target,
    failed: &BTreeSet<&'static str>,
    removed: &BTreeSet<ObjectId>,
) -> Result<Removals, Because> {
    let force = InForce::of(failed);
    if !force.any() {
        return Ok(Removals::default());
    }
    let mut walk = Walk {
        document,
        target,
        force,
        out: Removals::default(),
        tails: BTreeMap::new(),
    };
    let mut kept = Reach::default();
    let mut gone = Reach::default();
    for site in action_sites(document, target) {
        if site.holder == ActionHolder::Annotation
            && site.at.is_some_and(|at| removed.contains(&at))
        {
            gone.site(document, &site);
            continue;
        }
        kept.site(document, &site);
        walk.site(&site)?;
    }
    let mut out = walk.out;
    // A chain cut at the bound may reach past it from one holder and not from the other, so
    // neither set is then the whole of what it names.
    if !kept.overlong && !gone.overlong {
        out.gone = gone.objects.difference(&kept.objects).copied().collect();
    }
    Ok(out)
}

/// Every object a set of holders performs actions from or through.
///
/// The roots are the validator's (`pdf_archive::action_sites`' holders, each with its `/A`, the
/// entries of its `/AA`, and the catalog's `/OpenAction` and §7.7.4's `/JavaScript` tree), and
/// each object is entered once, so the walk is bounded by the file's objects and by
/// [`MAX_DEPTH`] links of §12.6.2's `/Next` below a root.
#[derive(Default)]
struct Reach {
    /// The holders and the action objects reached.
    objects: BTreeSet<ObjectId>,
    /// Whether a chain was cut at [`MAX_DEPTH`].
    overlong: bool,
}

impl Reach {
    /// One holder's roots, walked.
    fn site(&mut self, document: &Document, site: &pdf_archive::ActionSite) {
        if let Some(at) = site.at {
            self.objects.insert(at);
        }
        let mut roots: Vec<Object> = site.dict.get("A").into_iter().cloned().collect();
        if let Some(additional) = document.get_key(&site.dict, "AA").as_dict() {
            roots.extend(additional.iter().map(|(_, entry)| entry.clone()));
        }
        if site.holder == ActionHolder::Catalog {
            roots.extend(site.dict.get("OpenAction").cloned());
            let names = document.get_key(&site.dict, "Names");
            if let Some(scripts) = names
                .as_dict()
                .and_then(|names| document.get_key(names, "JavaScript").as_dict().cloned())
            {
                let resolve = |object: &Object| document.resolve(object);
                roots.extend(
                    pdf_syntax::tree::name_entries(&scripts, &resolve)
                        .into_iter()
                        .map(|(_, entry)| entry),
                );
            }
        }
        let mut stack: Vec<(Object, usize)> = roots.into_iter().map(|root| (root, 0)).collect();
        while let Some((entry, depth)) = stack.pop() {
            if depth >= MAX_DEPTH {
                self.overlong = true;
                continue;
            }
            if let Some(id) = entry.as_reference()
                && !self.objects.insert(id)
            {
                continue;
            }
            let resolved = document.resolve(&entry);
            let Some(action) = resolved.as_dict() else {
                continue;
            };
            stack.extend(
                flatten(document, action.get("Next"))
                    .into_iter()
                    .map(|next| (next, depth.saturating_add(1))),
            );
        }
    }
}

/// The removal, in progress over one document.
struct Walk<'a> {
    /// The document being converted.
    document: &'a Document,
    /// The target, which every predicate turns on.
    target: Target,
    /// Which of the rules this document's failures put in force.
    force: InForce,
    /// What has been decided so far.
    out: Removals,
    /// The `/Next` each action object is to state, so that two chains cannot demand two.
    tails: BTreeMap<ObjectId, Option<Object>>,
}

impl Walk<'_> {
    /// One holder: its `/A`, its `/AA`, and the two entries only the catalog has.
    fn site(&mut self, site: &pdf_archive::ActionSite) -> Result<(), Because> {
        self.action_entry(site)?;
        self.additional_actions(site)?;
        if site.holder == ActionHolder::Catalog {
            self.open_action(site)?;
            self.document_scripts(site)?;
        }
        Ok(())
    }

    /// A holder's `/A`: removed where the part forbids the entry, filtered where it does not.
    fn action_entry(&mut self, site: &pdf_archive::ActionSite) -> Result<(), Because> {
        let Some(entry) = site.dict.get("A") else {
            return Ok(());
        };
        if action_entry_admitted(site.holder) {
            if !self.force.types {
                return Ok(());
            }
            let replacement = self.position(entry, site, "A")?;
            return self.restate(site, "A", replacement, Rewrite::ForbiddenActionRemoved);
        }
        if !self.force.entry_on_widget {
            return Ok(());
        }
        // Both parts' section 6.4.1 forbid the entry itself, so what goes is the whole chain
        // behind it rather than the actions of a type the type rows name: nothing of it may stay.
        let at = site
            .at
            .ok_or(Because::NotBuiltYet(REMOVAL_FROM_A_DIRECT_HOLDER))?;
        self.out.removed.push(RemovedAction {
            page: site.page,
            holder: RemovedAction::name_of(site.holder),
            entry: "A".to_owned(),
            what: self
                .kind_of(entry)
                .unwrap_or_else(|| "an action stating no type".to_owned()),
            by: Rewrite::WidgetActionEntryRemoved,
        });
        self.edit(at, "A", None, Rewrite::WidgetActionEntryRemoved);
        Ok(())
    }

    /// A holder's `/AA`, under whichever of the three answers the part gives here.
    fn additional_actions(&mut self, site: &pdf_archive::ActionSite) -> Result<(), Because> {
        let Some(stated) = site.dict.get("AA") else {
            return Ok(());
        };
        let admits = additional_actions_admitted(site.holder, self.target);
        if admits == AdditionalActions::Forbidden {
            if !self.force.additional_forbidden {
                return Ok(());
            }
            let at = site
                .at
                .ok_or(Because::NotBuiltYet(REMOVAL_FROM_A_DIRECT_HOLDER))?;
            self.out.removed.push(RemovedAction {
                page: site.page,
                holder: RemovedAction::name_of(site.holder),
                entry: "AA".to_owned(),
                what: "the whole additional-actions dictionary".to_owned(),
                by: Rewrite::AdditionalActionsRemoved,
            });
            self.edit(at, "AA", None, Rewrite::AdditionalActionsRemoved);
            return Ok(());
        }
        let resolved = self.document.resolve(stated);
        let Some(dict) = resolved.as_dict() else {
            return Ok(());
        };
        let restrict =
            admits == AdditionalActions::OnlyAnnotationTriggers && self.force.additional_restricted;
        let mut out = Dictionary::new();
        let mut changed = false;
        let mut trimmed = false;
        for (key, value) in dict.iter() {
            let name = String::from_utf8_lossy(key.as_bytes()).into_owned();
            if restrict && !annotation_trigger(&name) {
                self.out.removed.push(RemovedAction {
                    page: site.page,
                    holder: RemovedAction::name_of(site.holder),
                    entry: format!("AA /{name}"),
                    what: "a trigger the part does not admit outside a widget".to_owned(),
                    by: Rewrite::AdditionalActionsRemoved,
                });
                changed = true;
                trimmed = true;
                continue;
            }
            if self.force.types {
                let entry = format!("AA /{name}");
                let replacement = self.position(value, site, &entry)?;
                match replacement {
                    Replacement::Unchanged => {
                        out.insert(key.clone(), value.clone());
                    }
                    Replacement::Gone => changed = true,
                    Replacement::Value(new) => {
                        out.insert(key.clone(), new);
                        changed = true;
                    }
                }
                continue;
            }
            out.insert(key.clone(), value.clone());
        }
        if !changed {
            return Ok(());
        }
        // The edit is the rewrite's whose rule changed the dictionary, because the rewriter
        // applies only an edit whose rewrite a decision wanted: an `/AA` the part admits whole —
        // a widget's, at part 4 — changes only by the type rows, and tagging that edit with the
        // `/AA` rule nobody failed would leave the forbidden action in the output. Where both
        // rules took something out, the dictionary is the restriction's: without it the output
        // fails that rule's row whatever else is applied.
        let by = if trimmed {
            Rewrite::AdditionalActionsRemoved
        } else {
            Rewrite::ForbiddenActionRemoved
        };
        // An `/AA` with nothing left in it states no trigger event at all, so the key goes with
        // its last entry rather than staying as an empty dictionary the part would still see.
        let value = if out.is_empty() {
            None
        } else {
            Some(Object::Dictionary(out))
        };
        // Where the file wrote the dictionary as an object of its own, the edit belongs to that
        // object; where it wrote it directly, the edit is the holder's own entry.
        match (stated.as_reference(), value) {
            (Some(id), Some(Object::Dictionary(kept))) => {
                self.out
                    .edits
                    .entry(id)
                    .or_default()
                    .push(Edit::Whole { dict: kept, by });
            }
            // Nothing left in it, or a shape no dictionary came out of: the holder's own key goes.
            (_, value) => {
                let at = site
                    .at
                    .ok_or(Because::NotBuiltYet(REMOVAL_FROM_A_DIRECT_HOLDER))?;
                let value = value.filter(|_| stated.as_reference().is_none());
                self.edit(at, "AA", value, by);
            }
        }
        Ok(())
    }

    /// The catalog's `/OpenAction`, where it names an action rather than a destination.
    ///
    /// §7.7.2's Table 29 types the entry "array or dictionary" and says why: an array is a
    /// destination, which carries no behaviour and is none of these rules' business.
    fn open_action(&mut self, site: &pdf_archive::ActionSite) -> Result<(), Because> {
        if !self.force.types {
            return Ok(());
        }
        let Some(entry) = site.dict.get("OpenAction") else {
            return Ok(());
        };
        if self.document.resolve(entry).as_dict().is_none() {
            return Ok(());
        }
        let replacement = self.position(entry, site, "OpenAction")?;
        self.restate(
            site,
            "OpenAction",
            replacement,
            Rewrite::ForbiddenActionRemoved,
        )
    }

    /// The name dictionary's `/JavaScript`, where the target's part admits no such action.
    ///
    /// §7.7.4's Table 32 says what the tree is — a name tree mapping name strings to
    /// document-level ECMAScript actions — so removing the key removes exactly the actions ISO
    /// 19005-2 section 6.5.1 forbids and nothing else. The tree's own shape is never edited:
    /// there is no entry of it a conforming part 2 file may keep.
    fn document_scripts(&mut self, site: &pdf_archive::ActionSite) -> Result<(), Because> {
        if !self.force.types || action_admitted(Some("JavaScript"), None, self.target) {
            return Ok(());
        }
        let Some(names) = site.dict.get("Names") else {
            return Ok(());
        };
        let resolved = self.document.resolve(names);
        let Some(dict) = resolved.as_dict() else {
            return Ok(());
        };
        if dict.get("JavaScript").is_none() {
            return Ok(());
        }
        self.out.removed.push(RemovedAction {
            page: None,
            holder: RemovedAction::name_of(ActionHolder::Catalog),
            entry: "Names /JavaScript".to_owned(),
            what: "the document-level ECMAScript name tree".to_owned(),
            by: Rewrite::ForbiddenActionRemoved,
        });
        if let Some(id) = names.as_reference() {
            self.edit(id, "JavaScript", None, Rewrite::ForbiddenActionRemoved);
        } else {
            let at = site
                .at
                .ok_or(Because::NotBuiltYet(REMOVAL_FROM_A_DIRECT_HOLDER))?;
            let mut kept = dict.clone();
            kept.remove("JavaScript");
            self.edit(
                at,
                "Names",
                Some(Object::Dictionary(kept)),
                Rewrite::ForbiddenActionRemoved,
            );
        }
        Ok(())
    }

    /// What one entry that holds a single action dictionary becomes.
    fn position(
        &mut self,
        entry: &Object,
        site: &pdf_archive::ActionSite,
        named: &str,
    ) -> Result<Replacement, Because> {
        let mut path = BTreeSet::new();
        let survived = self.survivors(entry, 0, &mut path, site, named)?;
        if !survived.changed {
            return Ok(Replacement::Unchanged);
        }
        let mut kept = survived.kept.into_iter();
        let Some(first) = kept.next() else {
            return Ok(Replacement::Gone);
        };
        let rest: Vec<Object> = kept.collect();
        if rest.is_empty() {
            return Ok(Replacement::Value(first));
        }
        // Table 196 types this entry as one action dictionary, and the removal left several to
        // perform. The first takes the position and the rest go behind it, which is the order
        // NOTE 1 already states for them.
        Ok(Replacement::Value(self.append_tail(first, rest)?))
    }

    /// Writes what one position became, where it became anything.
    fn restate(
        &mut self,
        site: &pdf_archive::ActionSite,
        key: &'static str,
        replacement: Replacement,
        by: Rewrite,
    ) -> Result<(), Because> {
        let value = match replacement {
            Replacement::Unchanged => return Ok(()),
            Replacement::Gone => None,
            Replacement::Value(value) => Some(value),
        };
        let at = site
            .at
            .ok_or(Because::NotBuiltYet(REMOVAL_FROM_A_DIRECT_HOLDER))?;
        self.edit(at, key, value, by);
        Ok(())
    }

    /// The actions that survive at one position, in the order §12.6.2 performs them.
    fn survivors(
        &mut self,
        value: &Object,
        depth: usize,
        path: &mut BTreeSet<ObjectId>,
        site: &pdf_archive::ActionSite,
        entry: &str,
    ) -> Result<Survived, Because> {
        if depth >= MAX_DEPTH {
            return Err(Because::NotBuiltYet(CHAIN_TOO_LONG));
        }
        let id = value.as_reference();
        if let Some(id) = id
            && !path.insert(id)
        {
            return Err(Because::NotBuiltYet(SELF_REFERENTIAL));
        }
        let resolved = self.document.resolve(value);
        let Some(dict) = resolved.as_dict() else {
            // Not an action dictionary at all, so no prohibition reaches it and it stays as the
            // producer wrote it.
            if let Some(id) = id {
                path.remove(&id);
            }
            return Ok(Survived::kept(value.clone()));
        };
        let dict = dict.clone();
        let mut tail: Vec<Object> = Vec::new();
        let mut changed = false;
        if let Some(next) = dict.get("Next") {
            // Table 196 types `/Next` as "either a single action dictionary or an array of
            // action dictionaries that shall be performed in order", so both shapes become the
            // same list.
            let each = match self.document.resolve(next) {
                Object::Array(items) => items,
                _ => vec![next.clone()],
            };
            for item in &each {
                let below = self.survivors(item, depth.saturating_add(1), path, site, entry)?;
                changed |= below.changed;
                tail.extend(below.kept);
            }
        }
        if let Some(id) = id {
            path.remove(&id);
        }
        let kind = self.name_at(&dict, "S");
        let named = self.name_at(&dict, "N");
        if action_admitted(kind.as_deref(), named.as_deref(), self.target) {
            if !changed {
                return Ok(Survived::kept(value.clone()));
            }
            let value = self.with_tail(value, &dict, shape(tail))?;
            return Ok(Survived {
                kept: vec![value],
                changed: true,
            });
        }
        self.out.removed.push(RemovedAction {
            page: site.page,
            holder: RemovedAction::name_of(site.holder),
            entry: entry.to_owned(),
            what: describe(kind.as_deref(), named.as_deref()),
            by: Rewrite::ForbiddenActionRemoved,
        });
        Ok(Survived {
            kept: tail,
            changed: true,
        })
    }

    /// One kept action, restated with the `/Next` its subtree's removals left.
    fn with_tail(
        &mut self,
        value: &Object,
        dict: &Dictionary,
        tail: Option<Object>,
    ) -> Result<Object, Because> {
        if let Some(id) = value.as_reference() {
            self.demand_tail(id, tail)?;
            return Ok(value.clone());
        }
        let mut out = dict.clone();
        state_next(&mut out, tail);
        Ok(Object::Dictionary(out))
    }

    /// Puts `rest` behind `first`, which is where §12.6.2 performs them.
    fn append_tail(&mut self, first: Object, rest: Vec<Object>) -> Result<Object, Because> {
        let resolved = self.document.resolve(&first);
        let Some(dict) = resolved.as_dict() else {
            return Ok(first);
        };
        let mut tail: Vec<Object> = Vec::new();
        if let Some(id) = first.as_reference() {
            if let Some(held) = self.tails.get(&id) {
                tail.extend(flatten(self.document, held.as_ref()));
            } else if let Some(next) = dict.get("Next") {
                tail.extend(flatten(self.document, Some(next)));
            }
            tail.extend(rest);
            self.demand_tail(id, shape(tail))?;
            return Ok(first);
        }
        let mut out = dict.clone();
        tail.extend(flatten(self.document, dict.get("Next")));
        tail.extend(rest);
        state_next(&mut out, shape(tail));
        Ok(Object::Dictionary(out))
    }

    /// Records the `/Next` one action object is to state, and refuses a second demand.
    fn demand_tail(&mut self, id: ObjectId, tail: Option<Object>) -> Result<(), Because> {
        if let Some(held) = self.tails.get(&id) {
            if *held == tail {
                return Ok(());
            }
            return Err(Because::NotBuiltYet(SHARED_ACTION_TWO_TAILS));
        }
        self.tails.insert(id, tail.clone());
        self.edit(id, "Next", tail, Rewrite::ForbiddenActionRemoved);
        Ok(())
    }

    /// Records one entry's new value.
    fn edit(&mut self, at: ObjectId, key: &'static str, value: Option<Object>, by: Rewrite) {
        self.out
            .edits
            .entry(at)
            .or_default()
            .push(Edit::Entry { key, value, by });
    }

    /// A name entry read as a string.
    fn name_at(&self, dict: &Dictionary, key: &str) -> Option<String> {
        self.document
            .get_key(dict, key)
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
    }

    /// The `/S` of the action one entry names, where it names one.
    fn kind_of(&self, entry: &Object) -> Option<String> {
        let resolved = self.document.resolve(entry);
        let dict = resolved.as_dict()?;
        self.name_at(dict, "S")
    }
}

/// What one position's entry becomes.
enum Replacement {
    /// Nothing about it changed, so nothing is written.
    Unchanged,
    /// Nothing survived, so the key goes.
    Gone,
    /// What stands in its place.
    Value(Object),
}

/// What survived at one node of an action tree.
struct Survived {
    /// The actions that take this node's place, in order.
    kept: Vec<Object>,
    /// Whether anything below or at this node was removed.
    changed: bool,
}

impl Survived {
    /// One node kept exactly as the producer wrote it.
    fn kept(value: Object) -> Self {
        Self {
            kept: vec![value],
            changed: false,
        }
    }
}

/// Writes one action dictionary's `/Next`, or removes the key where nothing follows it.
fn state_next(dict: &mut Dictionary, tail: Option<Object>) {
    match tail {
        None => {
            dict.remove("Next");
        }
        Some(next) => {
            dict.insert(Name::new(&b"Next"[..]), next);
        }
    }
}

/// A `/Next` value read as the list of actions it names.
fn flatten(document: &Document, next: Option<&Object>) -> Vec<Object> {
    let Some(next) = next else {
        return Vec::new();
    };
    match document.resolve(next) {
        Object::Array(items) => items,
        _ => vec![next.clone()],
    }
}

/// A list of actions written as Table 196 types a `/Next`: absent, one dictionary, or an array.
fn shape(mut actions: Vec<Object>) -> Option<Object> {
    match actions.len() {
        0 => None,
        1 => actions.pop(),
        _ => Some(Object::Array(actions)),
    }
}

/// What a removed action was, for the report.
fn describe(kind: Option<&str>, named: Option<&str>) -> String {
    match (kind, named) {
        (Some("Named"), Some(named)) => format!("a Named action performing {named}"),
        (Some("Named"), None) => "a Named action naming nothing to perform".to_owned(),
        (Some(kind), _) => format!("a {kind} action"),
        (None, _) => "an action stating no type".to_owned(),
    }
}
