//! Calculation-order walks and the values and formatting they update.

use super::{
    BTreeMap, Displayed, Document, Entry, Instant, MAX_SEQUENCE_TIME, Object, ObjectId,
    ScriptEvent, ScriptSite, Site, Trigger, ViewState, calculation_order, site, supplied_script,
};

impl ViewState {
    /// Walks Table 224's `/CO` whole, every script's `/C` through the runner, then every format the
    /// runner runs: what a host calls when a runner arrives for a document whose values were set
    /// without one, so that a calculated field shows what its script computes from the values
    /// already there (ADR 1616).
    pub fn recalculate_with_runner(&mut self, document: &Document) {
        if self.runner.0.is_none() {
            return;
        }
        // A runner arrives as the document opens, so the walk asks the calculation order's own
        // entries for the roots `/Fields` omits rather than every page (ADR 1653 section 4).
        let table = crate::view::field_table(document, crate::view::Omitted::Calculation);
        self.recalculate_scripts(document, &table, "");
        self.refresh_formatted(document, &table);
    }

    /// Walks Table 224's `/CO` after a keystroke, running each entry's one-call `/C`.
    ///
    /// In the array's order and once — "the calculation order in which their values will be
    /// recalculated" — so a calculation that changes a field earlier in the array does not restart
    /// the walk; Adobe's reference is silent on that re-entrancy and running the order once is what
    /// the array states (ADR 1579). A calculated value is the document changing its own value,
    /// like §12.7.6.3's reset, so Table 227's `ReadOnly` — a bar on the *user* — does not stop it.
    /// A field being typed into is left alone: what a person is typing is not overwritten under
    /// them. A script's `/C` is the commit's ([`Self::recalculate_scripts`]); with no runner it is
    /// reported here as Tier 0 reports it.
    pub(in crate::view) fn recalculate(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
    ) {
        self.walk_calculations(document, table, None);
    }

    /// Walks Table 224's `/CO` after a commit, a reset or a script that asked: each entry's
    /// one-call `/C` as [`Self::recalculate`] runs it, and each script's `/C` through the runner
    /// with `event.source` the field that changed (ADR 1603).
    ///
    /// The script's `event.value` is the value the field takes and `event.rc` set false leaves the
    /// field's value as it was. Every sentence a script's run owes is reported with the entry's
    /// position in the order, so a calculation stopped by its budget names where in the chain it
    /// was; the walk as a whole is held to [`MAX_SEQUENCE_TIME`].
    pub(in crate::view) fn recalculate_scripts(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        source: &str,
    ) {
        self.walk_calculations(document, table, Some(source));
    }

    /// The walk itself: `source` is `None` after a keystroke and the changed field after a commit.
    fn walk_calculations(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        source: Option<&str>,
    ) {
        // Adobe's "Doc properties" page: while `calculate` is false, no calculation is performed
        // for the document — `calculateNow` among them (ADR 1724).
        if self.scripting.calculations_off {
            return;
        }
        let order = calculation_order(document);
        let count = order.len();
        let started = Instant::now();
        for (index, field) in order.into_iter().enumerate() {
            let position = index.saturating_add(1);
            if source.is_some() && started.elapsed() > MAX_SEQUENCE_TIME {
                self.report(format!(
                    "the calculation order was stopped at entry {position} of {count}: the walk \
                     ran longer than its budget of {} ms, and the entries from there on were not \
                     recalculated (ADR 1603)",
                    MAX_SEQUENCE_TIME.as_millis()
                ));
                return;
            }
            let Some(dictionary) = document.get(field).as_dict().cloned() else {
                continue;
            };
            let widgets = crate::view::widgets_under(document, field);
            let name = table
                .iter()
                .find(|(_, held)| widgets.iter().any(|widget| held.contains(widget)))
                .map_or_else(
                    || format!("the field of object {}", field.number),
                    |(name, _)| name.clone(),
                );
            match site::of_field(document, &dictionary, Trigger::Calculate) {
                Site::Absent => {}
                Site::NotRun(sentence) => match source {
                    // A script's calculation runs at the commit; typing leaves it to that.
                    None if self.runner.0.is_some() => {}
                    None => self.report(format!("{name}: {sentence}")),
                    Some(source) => {
                        if widgets.iter().any(|widget| self.is_editing(*widget)) {
                            continue;
                        }
                        let value = widgets
                            .first()
                            .and_then(|widget| self.text_of(document, *widget))
                            .unwrap_or_default();
                        let script = supplied_script(document, &dictionary, Trigger::Calculate);
                        let ran = script.as_deref().and_then(|script| {
                            self.run_event(
                                document,
                                table,
                                ScriptEvent {
                                    script,
                                    value: &value,
                                    source,
                                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Calculate), &name)
                                },
                            )
                        });
                        match ran {
                            None => self.report(format!("{name}: {sentence}")),
                            Some((result, _)) => {
                                self.report_each(
                                    &format!(
                                        "{name} (entry {position} of {count} in the calculation \
                                         order)"
                                    ),
                                    result.report,
                                );
                                if result.rc
                                    && let Some(calculated) = result.value
                                {
                                    self.set_calculated(document, &widgets, &calculated);
                                }
                            }
                        }
                    }
                },
                Site::Library(call) => {
                    let result = {
                        let mut values =
                            |listed: &str| self.terminal_values(document, table, listed);
                        call.calculate(&mut values)
                    };
                    match result {
                        Ok(text) => self.set_calculated(document, &widgets, &text),
                        Err(refusal) => self.report(format!(
                            "{name}: the field's calculate script {} did not run: {refusal}",
                            call.function.name()
                        )),
                    }
                }
            }
        }
    }

    /// Runs every format script the runner runs whose field's value has changed since it last ran,
    /// and keeps what each displayed.
    ///
    /// Adobe's "Form event processing" formats after the calculations, and RFC 0008 section 6.5
    /// formats every field whose value changed: a field is re-formatted where its value differs
    /// from the one its kept display was made from. A field being typed into is left as typed.
    /// What a format's own edits change is applied after it, and the walk does not restart for
    /// them (RFC 0008 section 6.5's re-entrancy rule).
    pub(in crate::view) fn refresh_formatted(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
    ) {
        if self.runner.0.is_none() {
            return;
        }
        for (name, widgets) in table {
            let Some(first) = widgets.first().copied() else {
                continue;
            };
            if self.is_editing(first) {
                continue;
            }
            let Some(widget) = document.get(first).as_dict().cloned() else {
                continue;
            };
            if !matches!(
                site::of_widget(document, &widget, Trigger::Format),
                Site::NotRun(_)
            ) {
                continue;
            }
            let Some(shown) = self.field_value(document, name) else {
                continue;
            };
            if shown.obscured
                || self
                    .scripting
                    .formatted
                    .get(&first)
                    .is_some_and(|displayed| displayed.value == shown.text)
            {
                continue;
            }
            let at = shown.text.len();
            let commit_key = self
                .scripting
                .committing
                .as_ref()
                .filter(|(field, _)| field == name)
                .map(|(_, key)| *key);
            let event = ScriptEvent {
                value: &shown.text,
                selection: (at, at),
                commit_key,
                ..ScriptEvent::at(ScriptSite::Field(Trigger::Format), name)
            };
            let Some(result) = self.run_field_script(document, table, &widget, event) else {
                continue;
            };
            self.report_each(name, result.report);
            let displayed = Displayed {
                shown: result.value.unwrap_or_else(|| shown.text.clone()),
                value: shown.text,
            };
            for widget in widgets {
                self.scripting.formatted.insert(*widget, displayed.clone());
            }
        }
    }

    /// The values of every terminal field a name in `AFSimple_Calculate`'s list names.
    ///
    /// The field itself where it is terminal, and every field below it where it is not — a name
    /// followed by `.` in §12.7.4.2's qualified names — one value per field, read from its first
    /// widget, since §12.7.4.1 makes a field's value shared by all of them.
    fn terminal_values(
        &self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        listed: &str,
    ) -> Vec<String> {
        // The table is ordered by name, so the field itself is one lookup and its descendants one
        // contiguous run from `listed.` — a calculation is walked per keystroke, and a scan of the
        // whole table per listed name is quadratic in the form's size.
        let below = format!("{listed}.");
        table
            .get(listed)
            .into_iter()
            .chain(
                table
                    .range(below.clone()..)
                    .take_while(|(name, _)| name.starts_with(&below))
                    .map(|(_, widgets)| widgets),
            )
            .filter_map(|widgets| widgets.first())
            .map(|widget| self.text_of(document, *widget).unwrap_or_default())
            .collect()
    }

    /// Puts a calculated value into a field's widgets, where it differs from what they show.
    pub(in crate::view::scripts) fn set_calculated(
        &mut self,
        document: &Document,
        widgets: &[ObjectId],
        text: &str,
    ) {
        if widgets.iter().any(|widget| self.is_editing(*widget)) {
            return;
        }
        let differs = widgets
            .iter()
            .any(|widget| self.text_of(document, *widget).as_deref() != Some(text));
        if differs {
            self.write_text(widgets, text);
        }
    }

    /// Writes a text value into widgets as an edit, the latest of the four statements about it.
    pub(in crate::view::scripts) fn write_text(&mut self, widgets: &[ObjectId], text: &str) {
        let entry = Entry {
            value: Some(Object::String(
                pdf_syntax::text_string::encode_text_string(text).into(),
            )),
            indices: None,
        };
        for widget in widgets {
            self.reset.remove(widget);
            self.imported.remove(widget);
            self.edited.insert(*widget, entry.clone());
        }
    }
}
