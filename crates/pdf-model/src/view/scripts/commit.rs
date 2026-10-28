//! Field commits, validation refusal and restoration of the value held before typing.

use super::{
    BTreeMap, Before, CommitKey, Committed, Document, Keyed, Keystroke, ObjectId, ScriptEvent,
    ScriptSite, Site, Trigger, ViewState, site,
};

impl ViewState {
    /// Commits what a person typed into a field: ISO 32000-2 §12.6.3's Table 199 `/K` in its commit
    /// form, then `/V`, then §12.7.3's Table 224 `/CO`, then `/F` wherever a runner formats. The
    /// order is that of §12.6.3's Table 199: `/V` is "performed when the field's value is changed",
    /// and `/C`'s row hands the recalculation that follows to `/CO`:
    ///
    /// > The order in which the document's fields are recalculated shall be defined by the CO
    /// > entry in the interactive form dictionary (see 12.7.3, "Interactive form dictionary").
    ///
    /// What a host calls when a person leaves the field or presses Enter — the moment Adobe's
    /// "Form event processing" sets `willCommit`. Where the keystroke's commit form or the validate
    /// script refuses — `event.rc` set false — the field goes back to what it showed before the
    /// typing began and the answer says so; where the keystroke's commit form rewrites the value —
    /// a comma decimal stored with a period, an arbitrary mask's literals put back — the rewrite is
    /// the value. `/CO` is walked after either outcome (ADRs 1579, 1603).
    ///
    /// A script that is not one call of the library, with no runner supplied, is not run and is
    /// reported in [`Self::script_reports`]; the value stands, because a validation nobody ran
    /// refused nothing.
    pub fn commit_field(&mut self, document: &Document, name: &str) -> Committed {
        self.commit_field_by(document, name, CommitKey::Enter)
    }

    /// [`Self::commit_field`], with the way the person committed stated: Adobe's
    /// `event.commitKey`, which the commit's keystroke, its validation and its field's format are
    /// handed (ADR 1626).
    ///
    /// [`Self::commit_field`] is this with [`CommitKey::Enter`], a documented choice for a host
    /// that does not say how the commit came (ADR 1626); a host that knows it was a click or a Tab
    /// says so here.
    #[expect(
        clippy::too_many_lines,
        reason = "one commit's triggers in the reference's order — the keystroke's commit form,                   then validate, each a one-call arm and a runner's arm with the same refusal —                   read top to bottom as the event model they implement"
    )]
    pub fn commit_field_by(
        &mut self,
        document: &Document,
        name: &str,
        key: CommitKey,
    ) -> Committed {
        let Some(before) = self.uncommitted.remove(name) else {
            return Committed::Nothing;
        };
        let widgets: Vec<ObjectId> = before.iter().map(|(widget, _)| *widget).collect();
        let Some(first) = widgets.first().copied() else {
            return Committed::Nothing;
        };
        let Some(widget) = document.get(first).as_dict().cloned() else {
            return Committed::Nothing;
        };
        let table = crate::view::widgets_by_field_name(document);
        let mut text = self.text_of(document, first).unwrap_or_default();

        match site::of_widget(document, &widget, Trigger::Keystroke) {
            Site::Library(call) => {
                let at = text.len();
                let event = Keystroke {
                    value: &text,
                    change: "",
                    selection: (at, at),
                    will_commit: true,
                };
                match call.keystroke(&event) {
                    Ok(Keyed::Accepted {
                        value: Some(rewritten),
                    }) => {
                        self.write_text(&widgets, &rewritten);
                        text = rewritten;
                    }
                    Ok(Keyed::Accepted { value: None }) => {}
                    Ok(Keyed::Rejected { message }) => {
                        return self.refuse_commit(
                            document,
                            &table,
                            before,
                            format!(
                                "{name}: {}",
                                message.unwrap_or_else(|| {
                                    "the field's keystroke script refused the value".to_owned()
                                })
                            ),
                        );
                    }
                    Err(refusal) => self.report(format!(
                        "{name}: the field's keystroke script {} did not run: {refusal}",
                        call.function.name()
                    )),
                }
            }
            Site::NotRun(sentence) => {
                let at = text.len();
                let event = ScriptEvent {
                    value: &text,
                    selection: (at, at),
                    will_commit: true,
                    commit_key: Some(key),
                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Keystroke), name)
                };
                match self.run_field_script(document, &table, &widget, event) {
                    None => self.report(format!("{name}: {sentence}")),
                    Some(result) => {
                        self.hold_before(name, &before);
                        self.report_each(name, result.report);
                        if !result.rc {
                            return self.refuse_commit(
                                document,
                                &table,
                                before,
                                format!("{name}: the field's keystroke script refused the value"),
                            );
                        }
                        if let Some(rewritten) = result.value {
                            self.write_text(&widgets, &rewritten);
                            text = rewritten;
                        }
                    }
                }
            }
            Site::Absent => {}
        }

        match site::of_widget(document, &widget, Trigger::Validate) {
            Site::Library(call) => match call.validate(&text) {
                Ok(Some(message)) => {
                    return self.refuse_commit(
                        document,
                        &table,
                        before,
                        format!("{name}: {message}"),
                    );
                }
                Ok(None) => {}
                Err(refusal) => self.report(format!(
                    "{name}: the field's validate script {} did not run: {refusal}",
                    call.function.name()
                )),
            },
            Site::NotRun(sentence) => {
                // Adobe's "event properties" says both that a validate event does not listen to
                // `rc` and that setting it false invalidates the value; the second is the sentence
                // with an effect, and RFC 0008 section 6.5 takes it (ADR 1603). A value the script
                // writes into `event.value` is not taken: the event validates a value, it does not
                // produce one.
                let event = ScriptEvent {
                    value: &text,
                    selection: (text.len(), text.len()),
                    will_commit: true,
                    commit_key: Some(key),
                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Validate), name)
                };
                match self.run_field_script(document, &table, &widget, event) {
                    None => self.report(format!("{name}: {sentence}")),
                    Some(result) => {
                        self.hold_before(name, &before);
                        self.report_each(name, result.report);
                        if !result.rc {
                            return self.refuse_commit(
                                document,
                                &table,
                                before,
                                format!(
                                    "{name}: the field's validate script refused the value, so \
                                     the field shows what it showed before"
                                ),
                            );
                        }
                    }
                }
            }
            Site::Absent => {}
        }

        // XFA 3.3 page 379: a person's entry activates the calculations that depend on it.
        let _ = self.activate_form_on_entry(document, name);
        self.recalculate_scripts(document, &table, name);
        self.scripting.committing = Some((name.to_owned(), key));
        self.refresh_formatted(document, &table);
        self.scripting.committing = None;
        Committed::Accepted
    }

    /// Puts back what a refused commit replaced, recalculates, and answers the refusal.
    fn refuse_commit(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        before: Vec<(ObjectId, Before)>,
        sentence: String,
    ) -> Committed {
        self.revert(before);
        self.recalculate(document, table);
        self.refresh_formatted(document, table);
        Committed::Refused(sentence)
    }

    /// Records what each widget showed before typing began, the first time a field is typed into.
    pub(in crate::view) fn begin_typing(&mut self, name: &str, widgets: &[ObjectId]) {
        if self.uncommitted.contains_key(name) {
            return;
        }
        let before = widgets
            .iter()
            .map(|widget| {
                (
                    *widget,
                    Before {
                        edited: self.edited.get(widget).cloned(),
                        imported: self.imported.get(widget).cloned(),
                        reset: self.reset.contains(widget),
                    },
                )
            })
            .collect();
        self.uncommitted.insert(name.to_owned(), before);
    }

    /// Puts back what each widget showed before typing began.
    pub(in crate::view::scripts) fn revert(&mut self, before: Vec<(ObjectId, Before)>) {
        for (widget, was) in before {
            match was.edited {
                Some(entry) => self.edited.insert(widget, entry),
                None => self.edited.remove(&widget),
            };
            match was.imported {
                Some(import) => self.imported.insert(widget, import),
                None => self.imported.remove(&widget),
            };
            if was.reset {
                self.reset.insert(widget);
            } else {
                self.reset.remove(&widget);
            }
        }
    }

    /// Keeps what a commit's widgets showed before the typing began beside the run the runner just
    /// held, so that a late refusal can put it back.
    fn hold_before(&mut self, name: &str, before: &[(ObjectId, Before)]) {
        if let Some(pending) = self.scripting.pending.last_mut()
            && pending.field == name
            && pending.before.is_none()
            && matches!(
                pending.site,
                ScriptSite::Field(Trigger::Keystroke | Trigger::Validate)
            )
        {
            pending.before = Some(before.to_vec());
        }
    }
}
