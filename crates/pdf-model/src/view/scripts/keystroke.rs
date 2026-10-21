//! Keystroke and choice-selection verdicts before a value is committed.

use super::{
    Document, Keyed, Keystroke, ObjectId, ScriptEvent, ScriptSite, Selected, Site, Trigger, Typed,
    Verdict, ViewState, site,
};

impl ViewState {
    /// What Table 199's `/K`, in its typing form, makes of the characters.
    ///
    /// The host hands a whole value, so the keystroke is that value replacing the whole of the
    /// field's text — what `AFMergeChange` then reassembles is the value itself, and a runner's
    /// rewritten `event.change` is the whole value the field takes.
    pub(in crate::view) fn keystroke_verdict(
        &mut self,
        document: &Document,
        taking: &[ObjectId],
        text: &str,
    ) -> Verdict {
        let Some(first) = taking.first().copied() else {
            return Verdict::Stands;
        };
        let Some(widget) = document.get(first).as_dict().cloned() else {
            return Verdict::Stands;
        };
        match site::of_widget(document, &widget, Trigger::Keystroke) {
            Site::Absent => Verdict::Stands,
            Site::NotRun(sentence) => {
                let table = crate::view::widgets_by_field_name(document);
                let name = table
                    .iter()
                    .find(|(_, widgets)| widgets.contains(&first))
                    .map_or_else(
                        || format!("the field of object {}", first.number),
                        |(name, _)| name.clone(),
                    );
                let current = self.text_of(document, first).unwrap_or_default();
                let typed = Typed::of(document, taking, &widget, text);
                let event = ScriptEvent {
                    value: &current,
                    change: &typed.change,
                    change_ex: &typed.change_ex,
                    field_full: typed.full,
                    selection: (0, current.len()),
                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Keystroke), &name)
                };
                match self.run_field_script(document, &table, &widget, event) {
                    None => {
                        self.report(format!("{name}: {sentence}"));
                        Verdict::Stands
                    }
                    Some(result) => {
                        self.report_each(&name, result.report);
                        match (result.rc, result.change) {
                            (false, _) => Verdict::Rejected,
                            (true, Some(change)) => Verdict::Rewritten(change),
                            // A full field takes the change cropped to what fits, which is what
                            // the event handed the script as its change (ADR 1626).
                            (true, None) if typed.full => Verdict::Rewritten(typed.change),
                            (true, None) => Verdict::Stands,
                        }
                    }
                }
            }
            Site::Library(call) => {
                let current = self.text_of(document, first).unwrap_or_default();
                let event = Keystroke {
                    value: &current,
                    change: text,
                    selection: (0, current.len()),
                    will_commit: false,
                };
                match call.keystroke(&event) {
                    Ok(Keyed::Accepted { .. }) => Verdict::Stands,
                    Ok(Keyed::Rejected { .. }) => Verdict::Rejected,
                    Err(refusal) => {
                        let name = crate::view::fields::field_name_of(document, first);
                        self.report(format!(
                            "{name}: the field's keystroke script {} did not run: {refusal}",
                            call.function.name()
                        ));
                        Verdict::Stands
                    }
                }
            }
        }
    }

    /// What Table 199's `/K` makes of a choice field's new selection.
    ///
    /// The entry is performed when the user "modifies the selection in a scrollable list box", and
    /// for a combo box when the user "modifies a character", which a drop-down's pick is the whole
    /// of. The keystroke is the one [`Self::keystroke_verdict`] runs, its change the text a person
    /// sees for the option the selection names and its `changeEx` that option's export value: the
    /// lowest of the indices, which is the option a single selection keeps (Table 233 bit 22), and
    /// empty where nothing is selected. The reference states no change for a selection of several
    /// options, so the first of them is this tree's choice. A script that rewrites the change to
    /// the text of an option selects that option; one that rewrites it to text no option shows is
    /// reported, and the selection stands as made, since a list box holds no value outside `/Opt`
    /// (ADR 1786).
    pub(in crate::view) fn selection_verdict(
        &mut self,
        document: &Document,
        taking: &[ObjectId],
        indices: &[usize],
    ) -> Selected {
        let Some(first) = taking.first().copied() else {
            return Selected::Stands;
        };
        let Some(stored) = document.get(first).as_dict().cloned() else {
            return Selected::Stands;
        };
        let annotation =
            crate::appearance::with_rewritten_options(document, &stored, self.scripted(first));
        let field =
            crate::appearance::Field::read(document, &annotation, crate::view::FieldValue::Stored);
        if !matches!(
            field.kind,
            Some(crate::appearance::FieldKind::Choice { .. })
        ) {
            // Not §12.7.5.4's field: `set_field` refuses the choice itself.
            return Selected::Stands;
        }
        let options = crate::form::options(document, &field);
        let change = indices
            .iter()
            .copied()
            .filter(|index| *index < options.len())
            .min()
            .and_then(|index| options.get(index))
            .map(|option| option.label.clone())
            .unwrap_or_default();
        match self.keystroke_verdict(document, taking, &change) {
            Verdict::Stands => Selected::Stands,
            Verdict::Rejected => Selected::Rejected,
            Verdict::Rewritten(text) if text == change => Selected::Stands,
            Verdict::Rewritten(text) => {
                if let Some(option) = options.iter().position(|option| option.label == text) {
                    Selected::Chooses(option)
                } else {
                    let name = crate::view::fields::field_name_of(document, first);
                    self.report(format!(
                        "{name}: the field's keystroke script changed the selection to \"{text}\", \
                         which no option of /Opt shows, so the selection stands as made"
                    ));
                    Selected::Stands
                }
            }
        }
    }
}
