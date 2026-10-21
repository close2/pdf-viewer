//! Script-event dispatch, runner replies and resumed events.

use super::{
    Applied, Arc, BTreeMap, BTreeSet, Dictionary, Displayed, Document, DocumentState, FieldState,
    MAX_PENDING, ObjectId, Pending, Resumed, Runner, ScriptEvent, ScriptResult, ScriptRunner,
    ScriptSite, Told, Trigger, ViewState, changed_keys, spells_words, supplied_script,
};

impl ViewState {
    /// Hands every script Tier 0 does not run to `runner`, or, with `None`, to nothing — RFC 0008
    /// section 6.3's level `off`, which is where every view state starts.
    ///
    /// The one place a host's level for scripts reaches this crate (ADR 1591): what the runner may
    /// reach, and whether it asks first, is the host's and the runner's, never decided here. A new
    /// runner holds a new realm, so the next event tells it of every field again (ADR 1602).
    pub fn run_scripts_with(&mut self, runner: Option<Arc<dyn ScriptRunner>>) {
        self.runner = Runner(runner);
        self.scripting.told = None;
        self.scripting.formatted.clear();
    }

    /// Hands a field event to the runner with the field's own script for its trigger, where there
    /// is a runner and the trigger's action is one ECMAScript action whose text can be read.
    pub(in crate::view::scripts) fn run_field_script(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        widget: &Dictionary,
        event: ScriptEvent<'_>,
    ) -> Option<ScriptResult> {
        let ScriptSite::Field(trigger) = event.site else {
            return None;
        };
        self.runner.0.as_ref()?;
        let script = supplied_script(document, widget, trigger)?;
        self.run_event(
            document,
            table,
            ScriptEvent {
                script: &script,
                ..event
            },
        )
        .map(|(result, _)| result)
    }

    /// Hands one event to the runner, telling it of every field whose state changed since it last
    /// heard, and applies the edits its result carries.
    ///
    /// `None` where no runner is supplied. The result's `value`, `change` and `rc` are the
    /// caller's to read, because what they mean is the site's.
    pub(in crate::view) fn run_event(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        event: ScriptEvent<'_>,
    ) -> Option<(ScriptResult, Applied)> {
        let runner = self.runner.0.clone()?;
        if self.scripting.words.is_none() && spells_words(event.script) {
            self.read_words(document);
        }
        self.read_outline_for(
            document,
            event.script,
            matches!(event.site, ScriptSite::Outline { .. }),
        );
        let (fields, whole) = self.tell(document, table);
        let rich_value = match event.site {
            // The reference makes the pair a field event's, so a cursor crossing a widget reads no
            // rich text string.
            ScriptSite::Field(_) if !event.field.is_empty() => {
                table.get(event.field).and_then(|widgets| {
                    widgets
                        .first()
                        .and_then(|widget| self.rich_value_of(document, *widget))
                })
            }
            _ => None,
        };
        let page = match event.site {
            ScriptSite::Field(_) => self
                .field_page(document, table, event.field)
                .unwrap_or(event.page),
            _ => event.page,
        };
        let result = runner.run(&ScriptEvent {
            fields: &fields,
            page,
            pages: crate::page::Pages::new(document).len(),
            dirty: self.unsaved(),
            document: whole.as_ref(),
            view: self.window_view(document),
            keys: self.scripting.keys,
            rich_value: rich_value.as_deref().unwrap_or(event.rich_value),
            ..event
        });
        if runner.waiting() && self.scripting.pending.len() < MAX_PENDING {
            self.scripting.pending.push(Pending {
                site: event.site,
                field: event.field.to_owned(),
                before: None,
            });
        }
        let applied = self.apply_edits(document, table, &result.edits);
        Some((result, applied))
    }

    /// Every field whose state differs from what the realm was last told, read now, and the
    /// document as a whole where it differs too; the realm counted as told of both.
    fn tell(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
    ) -> (Vec<FieldState>, Option<DocumentState>) {
        let annotations = self.annotation_states(document);
        let words = self.scripting.words.is_some();
        let outline = self.outline_read();
        let whole = match self.scripting.told.as_deref() {
            Some(told)
                if told.optional_content == self.optional_content
                    && told.annotations == annotations
                    && told.words == words
                    && told.outline == outline =>
            {
                None
            }
            _ => Some(DocumentState {
                annotations: annotations.clone(),
                ..self.document_state(document)
            }),
        };
        let names: BTreeSet<&String> = match self.scripting.told.as_deref() {
            None => table.keys().collect(),
            Some(told) => {
                let mut widgets = BTreeSet::new();
                changed_keys(&self.edited, &told.edited, &mut widgets);
                changed_keys(&self.imported, &told.imported, &mut widgets);
                widgets.extend(self.reset.symmetric_difference(&told.reset).copied());
                widgets.extend(self.hidden.symmetric_difference(&told.hidden).copied());
                widgets.extend(self.shown.symmetric_difference(&told.shown).copied());
                let mut changed: BTreeSet<&String> = BTreeSet::new();
                let mut properties = BTreeSet::new();
                changed_keys(&self.scripting.overrides, &told.overrides, &mut properties);
                for (name, held) in table {
                    if properties.contains(name)
                        || held.iter().any(|w| widgets.contains(w))
                        || !told.names.contains(name)
                    {
                        changed.insert(name);
                    }
                }
                changed
            }
        };
        if names.is_empty() && whole.is_none() {
            return (Vec::new(), None);
        }
        let pages = self
            .scripting
            .pages
            .get_or_insert_with(|| crate::page::Pages::new(document).indices())
            .clone();
        let mut told_names = self
            .scripting
            .told
            .as_deref()
            .map(|told| told.names.clone())
            .unwrap_or_default();
        told_names.extend(names.iter().map(|name| (*name).clone()));
        let states = names
            .into_iter()
            .filter_map(|name| {
                let widgets = table.get(name)?;
                self.field_state(document, name, widgets, &pages)
            })
            .collect();
        self.scripting.told = Some(Box::new(Told {
            edited: self.edited.clone(),
            imported: self.imported.clone(),
            reset: self.reset.clone(),
            hidden: self.hidden.clone(),
            shown: self.shown.clone(),
            overrides: self.scripting.overrides.clone(),
            optional_content: self.optional_content.clone(),
            annotations,
            names: told_names,
            words,
            outline,
        }));
        (states, whole)
    }

    /// Applies every run the runner held and has since finished, as the outcome of the trigger
    /// that ran it arriving late, and answers whether anything a page draws changed (ADR 1627).
    ///
    /// **What a host calls once a person has answered a script's question** — after it hands the
    /// answer to the runner — and it may call it after any command, since with nothing finished it
    /// does nothing. A run held on a person changed nothing when it was handed over, so its whole
    /// outcome is applied here, in the order the runs finished: every edit as an edit, every
    /// sentence reported. The event's own answer is applied where its trigger can still take it: a
    /// commit's keystroke or validation that refuses puts back what the field showed before the
    /// typing began, unless a person has begun typing into it again; a commit's keystroke that
    /// rewrites the value writes the rewrite; a calculation's value is set; and a format's text is
    /// what the field displays. Every other site takes its edits alone, as it would have at once.
    /// Table 224's `/CO` is walked again where a value changed.
    pub fn apply_resumed(&mut self, document: &Document) -> bool {
        let Some(runner) = self.runner.0.clone() else {
            return false;
        };
        // Asked after every command, so nothing is read until a run has finished: the table walks
        // every page, and an open must not (ADR 1653 section 4).
        let Some(first) = runner.take_resumed() else {
            return false;
        };
        let table = crate::view::widgets_by_field_name(document);
        let (mut changed, mut calculate) = (false, false);
        let mut next = Some(first);
        while let Some(resumed) = next.take() {
            let pending = self
                .scripting
                .pending
                .iter()
                .position(|held| held.site == resumed.site && held.field == resumed.field)
                .map(|at| self.scripting.pending.remove(at));
            let subject = if resumed.field.is_empty() {
                "a script that waited on an answer".to_owned()
            } else {
                resumed.field.clone()
            };
            self.report_each(&subject, resumed.result.report.clone());
            let applied = self.apply_edits(document, &table, &resumed.result.edits);
            changed |= applied.values;
            calculate |= applied.calculate;
            changed |= self.apply_late_event(document, &table, &resumed, pending);
            next = runner.take_resumed();
        }
        if changed || calculate {
            self.recalculate_scripts(document, &table, "");
            self.refresh_formatted(document, &table);
        }
        changed || calculate
    }

    /// The part of a late outcome that is its event's answer: what [`Self::apply_resumed`] says
    /// each site takes. Answers whether a value or a displayed text changed.
    fn apply_late_event(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        resumed: &Resumed,
        pending: Option<Pending>,
    ) -> bool {
        let ScriptSite::Field(trigger) = resumed.site else {
            return false;
        };
        let name = resumed.field.as_str();
        let Some(widgets) = table.get(name).cloned() else {
            return false;
        };
        let typing = widgets.iter().any(|widget| self.is_editing(*widget));
        let before = pending.and_then(|pending| pending.before);
        let result = &resumed.result;
        match (trigger, before) {
            (Trigger::Keystroke | Trigger::Validate, Some(before)) if !result.rc => {
                if typing {
                    self.report(format!(
                        "{name}: its script refused the value once its question was answered, and \
                         the field is being typed into again, so what is typed stands"
                    ));
                    return false;
                }
                self.revert(before);
                self.report(format!(
                    "{name}: its script refused the value once its question was answered, so the \
                     field shows what it showed before the typing began"
                ));
                true
            }
            (Trigger::Keystroke, Some(_)) => match &result.value {
                Some(rewritten) if !typing => {
                    self.write_text(&widgets, rewritten);
                    true
                }
                _ => false,
            },
            (Trigger::Calculate, _) => match &result.value {
                Some(calculated) if result.rc => {
                    self.set_calculated(document, &widgets, calculated);
                    true
                }
                _ => false,
            },
            (Trigger::Format, _) => {
                let Some(shown) = self.field_value(document, name) else {
                    return false;
                };
                let displayed = Displayed {
                    shown: result.value.clone().unwrap_or_else(|| shown.text.clone()),
                    value: shown.text,
                };
                for widget in &widgets {
                    self.scripting.formatted.insert(*widget, displayed.clone());
                }
                true
            }
            _ => false,
        }
    }
}
