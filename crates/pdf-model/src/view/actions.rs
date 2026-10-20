//! §12.6.4's actions performed against the view state: hiding an annotation, resetting a form,
//! setting an optional content group, and the request an action leaves for a host.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Document, ObjectId};

use crate::action::{Action, Hide, HideTarget, ResetForm, ResetTarget};

use super::field_tree::{widgets_by_field_name, widgets_under};
use super::{Request, ViewState};

impl ViewState {
    /// Performs one action, and answers what it asks of the viewer.
    ///
    /// `Some` for the three types that change something this state does not hold: which page
    /// is shown (§12.6.4.2's destination and §12.6.4.12's page commands) and what is outside
    /// the document altogether (§12.6.4.8's URI). Everything else is performed here and
    /// answers `None`, because a layer's state and an annotation's Hidden flag are what a
    /// `ViewState` *is*.
    ///
    /// A [`Action::Refused`] does nothing and is not an error: the action is named so that a
    /// caller may say what it declined to do, and doing nothing is what §12.6.1's list of
    /// twenty types leaves a renderer that implements five of them.
    pub fn perform(&mut self, document: &Document, action: &Action) -> Option<Request> {
        match action {
            Action::GoTo(destination) => return Some(Request::Display(*destination)),
            Action::Named(named) => return Some(Request::Page(*named)),
            Action::Uri(uri) => return Some(Request::Resolve(uri.clone())),
            Action::Thread(jump) => return Some(Request::Thread(jump.clone())),
            Action::GoToDp(jump) => return Some(Request::DocumentPart(*jump)),
            Action::ImportData(import) => return Some(Request::Import(import.clone())),
            Action::SubmitForm(submit) => return Some(Request::Submit(submit.clone())),
            Action::GoToE(target) => return Some(Request::Embedded(target.clone())),
            // §12.6.4.6's launch naming a file asks for it as §12.6.4.3's go-to does; which of
            // the two it was travels in `RemoteGoTo::act` (ADR 1358).
            Action::GoToR(remote) | Action::Launch(remote) => {
                return Some(Request::Remote(remote.clone()));
            }
            Action::Trans(transition) => {
                return Some(Request::Transition(transition.clone()));
            }
            Action::Rendition(rendition) => return Some(Request::Rendition(rendition.clone())),
            Action::SetOcgState(state) => {
                if let Some(content) = self.optional_content.as_mut() {
                    content.apply(&state.changes, state.preserve_radio_buttons);
                }
                // `Self::set_group`'s reason: an event's overlay stands over the states this
                // action just wrote.
                self.reapply_usage();
            }
            Action::Hide(hide) => self.hide(document, hide),
            Action::ResetForm(reset) => {
                self.reset_form(document, reset);
                // A reset changes values, and Table 224's `/CO` recalculates on any change; a
                // reset is a commit of the values it restores, so a script's `/C` runs too.
                let table = widgets_by_field_name(document);
                self.recalculate_scripts(document, &table, "");
                self.refresh_formatted(document, &table);
            }
            Action::Refused(_) => {}
        }
        None
    }

    /// Performs a whole `/Next` chain and answers everything it asks of the viewer, in order.
    ///
    /// A list rather than one answer, because §12.6.2 makes a chain a sequence of actions and
    /// two of them may each ask for something: a link may open a URI and then jump. Every
    /// action is performed even after one that moves the page — a `/SetOCGState` after a
    /// `/GoTo` is a layer change for the page being navigated *to*, and the clause states no
    /// ordering that would drop it — while §12.6.2's NOTE 1 leaves the caller to decide what
    /// to do with two requests, since it is the caller that knows whether the first made the
    /// second impossible.
    pub fn perform_all(&mut self, document: &Document, actions: &[Action]) -> Vec<Request> {
        let mut requests = Vec::new();
        for action in actions {
            if let Some(request) = self.perform(document, action) {
                requests.push(request);
            }
        }
        requests
    }

    /// §12.6.4.11 applied: every annotation the action names, in either of `/T`'s two forms.
    fn hide(&mut self, document: &Document, action: &Hide) {
        let mut fields: Option<BTreeMap<String, Vec<ObjectId>>> = None;
        for target in &action.targets {
            match target {
                HideTarget::Annotation(id) => self.set_hidden(*id, action.hide),
                HideTarget::Field(name) => {
                    // Built at most once per action, and only for an action that names a
                    // field: the walk is over `/AcroForm /Fields`, which most documents do
                    // not have and no document needs for the reference form.
                    let table = fields.get_or_insert_with(|| widgets_by_field_name(document));
                    for id in table.get(name).into_iter().flatten() {
                        self.set_hidden(*id, action.hide);
                    }
                }
            }
        }
    }

    /// Performs §12.7.6.3's reset over the widgets the action names.
    ///
    /// The clause resets *fields*; what this program draws is *widgets*, and one field may have
    /// several — §12.7.4.1's field tree ends in the annotations that show it. So the set kept
    /// here is of widget annotations, resolved once through the same table §12.6.4.11's hide
    /// action uses.
    ///
    /// Three shapes, all Table 241's and Table 242's:
    ///
    /// - no `/Fields` at all: "all fields in the document's interactive form are reset";
    /// - `/Fields` with the flag clear: those fields, "\[a]ll descendants of the specified fields
    ///   in the field hierarchy" included — which §12.7.4.2's naming makes a prefix test for the
    ///   table's name form, since a descendant's fully qualified name is its ancestor's with `.`
    ///   and more appended, and which [`widgets_under`] makes a walk down `/Kids` for its
    ///   reference form;
    /// - `/Fields` with the flag set: everything *except* those, descendants included. **The
    ///   published table says only "those listed in the Fields array"**, and Errata Collection 3's
    ///   Issue #174 adds the parenthesis the clear branch already had — *(All descendants of the
    ///   specified fields in the field hierarchy are also exempt from being reset.)*, in italics
    ///   because an erratum's added text is in no clause of `doc/md/` — so the two
    ///   branches are one subtree question asked twice, which is what the set built below is.
    ///
    /// **The reset is the latest statement about each field it reaches**, so what a person typed
    /// and what an import replaced go with it: the clause sets `/V` to `/DV` whatever `/V` was,
    /// and [`ViewState::annotation`] reads a person's edit first (ADR 1796).
    pub(super) fn reset_form(&mut self, document: &Document, action: &ResetForm) {
        let table = widgets_by_field_name(document);
        let reached = Self::reached_by_reset(document, &table, action);
        self.forget_statements(&table, &reached);
        self.reset.extend(reached);
    }

    /// The widgets one reset-form action reaches, by Table 241's and Table 242's three shapes.
    fn reached_by_reset(
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        action: &ResetForm,
    ) -> BTreeSet<ObjectId> {
        if action.fields.is_empty() {
            return table.values().flatten().copied().collect();
        }
        let named: BTreeSet<ObjectId> = action
            .fields
            .iter()
            .flat_map(|target| match target {
                ResetTarget::Field(id) => widgets_under(document, *id),
                ResetTarget::Name(name) => table
                    .iter()
                    .filter(|(candidate, _)| {
                        *candidate == name
                            || candidate
                                .strip_prefix(name.as_str())
                                .is_some_and(|rest| rest.starts_with('.'))
                    })
                    .flat_map(|(_, widgets)| widgets.iter().copied())
                    .collect(),
            })
            .collect();
        if action.exclude {
            table
                .values()
                .flatten()
                .copied()
                .filter(|widget| !named.contains(widget))
                .collect()
        } else {
            named
        }
    }

    /// Forgets every earlier statement about these widgets' values but the file's own.
    ///
    /// A person's edit, an import's value, a reset and Table 226's `/V` answer one question about
    /// a widget, and the one performed last stands alone. [`ViewState::annotation`] asks them in a
    /// fixed order, so a later statement has to take the earlier ones away rather than rely on
    /// being read first. A field whose widgets are reached also loses its typing in progress and
    /// §12.7.5.3's chosen file, which belong to the value being replaced.
    pub(super) fn forget_statements(
        &mut self,
        table: &BTreeMap<String, Vec<ObjectId>>,
        widgets: &BTreeSet<ObjectId>,
    ) {
        for widget in widgets {
            self.edited.remove(widget);
            self.imported.remove(widget);
            self.reset.remove(widget);
        }
        for (name, held) in table {
            if held.iter().any(|widget| widgets.contains(widget)) {
                self.uncommitted.remove(name);
                self.chosen.remove(name);
            }
        }
    }

    /// Whether this widget's value has been reset to its default (§12.7.6.3).
    ///
    /// A widget rather than a field, for `Self::reset_form`'s reason. `false` for every
    /// annotation in every document that has performed no reset, which is all of them until a
    /// person clicks a button.
    #[must_use]
    pub fn is_reset(&self, annotation: ObjectId) -> bool {
        self.reset.contains(&annotation)
    }

    /// Records one annotation's new state, keeping the two sets disjoint.
    pub(super) fn set_hidden(&mut self, annotation: ObjectId, hide: bool) {
        if hide {
            self.shown.remove(&annotation);
            self.hidden.insert(annotation);
        } else {
            self.hidden.remove(&annotation);
            self.shown.insert(annotation);
        }
    }
}
