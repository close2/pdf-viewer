//! Script-requested field edits, focus, navigation and display properties.

use super::{
    Applied, BTreeMap, ConsoleRequest, Document, FocusRequest, FontName, MAX_CONSOLE_REQUESTS,
    MAX_VIEW_CHANGES, Object, ObjectId, Property, ResetForm, ResetTarget, ScriptEdit, ViewChange,
    ViewState,
};

impl ViewState {
    /// Applies a script's edits as edits a person could have made, and says what they changed.
    ///
    /// A value goes into the edit log beside a typed one; `display` into the overrides beside
    /// §12.6.4.11's hide; a reset is §12.7.6.3's; and every property is kept by field name, read
    /// back by the realm and by [`Self::script_properties`]. A field a person is typing into is
    /// not written under them, which is the rule a calculation already keeps (ADR 1603).
    pub(in crate::view::scripts) fn apply_edits(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        edits: &[ScriptEdit],
    ) -> Applied {
        let mut applied = Applied::default();
        for edit in edits {
            match edit {
                ScriptEdit::Value { field, value } => {
                    let Some(widgets) = table.get(field) else {
                        continue;
                    };
                    if widgets.iter().any(|widget| self.is_editing(*widget)) {
                        self.report(format!(
                            "{field}: a script set the value while a person was typing into the \
                             field, and what they are typing stands"
                        ));
                        continue;
                    }
                    let differs = widgets
                        .iter()
                        .any(|widget| self.text_of(document, *widget).as_deref() != Some(value));
                    if differs {
                        self.write_text(widgets, value);
                        applied.values = true;
                    }
                }
                ScriptEdit::Property {
                    field,
                    widget,
                    property,
                } => self.apply_property(
                    document,
                    table,
                    (field, *widget),
                    property,
                    &mut applied.values,
                ),
                ScriptEdit::Reset { fields } => {
                    let action = ResetForm {
                        fields: fields.iter().cloned().map(ResetTarget::Name).collect(),
                        exclude: false,
                    };
                    self.reset_form(document, &action);
                    applied.values = true;
                }
                ScriptEdit::Calculate => applied.calculate = true,
                ScriptEdit::Focus { field, widget } => self.ask_focus(table, field, *widget),
                ScriptEdit::Layer {
                    number,
                    generation,
                    on,
                } => self.switch_layer(document, *number, *generation, *on),
                ScriptEdit::Annotation {
                    number,
                    generation,
                    change,
                } => {
                    let annotation = ObjectId {
                        number: *number,
                        generation: *generation,
                    };
                    self.change_annotation(document, annotation, change);
                    applied.values = true;
                }
                ScriptEdit::GoTo { page } => {
                    let pages = crate::page::Pages::new(document).len();
                    match usize::try_from(*page) {
                        Ok(page) if page < pages => self.scripting.page = Some(page),
                        _ => self.report(format!(
                            "a script turned to page {}, and this document has {pages}, so no \
                             page is turned (ADR 1640)",
                            u64::from(*page).saturating_add(1)
                        )),
                    }
                }
                ScriptEdit::Timer {
                    id,
                    script,
                    period,
                    repeat,
                } => {
                    if let Err(refused) = self.scripting.timers.set(*id, script, *period, *repeat) {
                        self.report(refused);
                    }
                }
                ScriptEdit::ClearTimer { id } => self.scripting.timers.clear(*id),
                ScriptEdit::Beep { sound } => self.scripting.timers.beep(*sound),
                ScriptEdit::Choose { field, indices } => {
                    applied.values |= self.choose(document, table, field, indices);
                }
                ScriptEdit::Destination { name } => self.go_to_named(document, name),
                ScriptEdit::Calculation { on } => self.scripting.calculations_off = !*on,
                ScriptEdit::View { change } => self.ask_view(document, *change),
                ScriptEdit::Console(command) => self.ask_console(*command),
                ScriptEdit::Bookmark { number, generation } => {
                    self.ask_bookmark(*number, *generation);
                }
            }
        }
        applied.drawn =
            applied.values || applied.calculate || edits.iter().any(ScriptEdit::redraws);
        applied
    }

    /// `textFont` as the `/DA` writes it: the script named a font by what it reads back, and the
    /// `/DA`'s `Tf` names it by a resource of the form's `/DR`, which §12.7.4.3 requires (ADR 1762).
    /// `None`, reported, where the form holds no such font.
    fn form_font(&mut self, document: &Document, field: &str, asked: &str) -> Option<Property> {
        let Some((base, resource)) = crate::appearance::form_font(document, asked) else {
            self.report(format!(
                "{field}: a script set Field.textFont to {asked:?}, which no font of the form's \
                 /DR is named, by its resource name or its /BaseFont, so the field keeps its font \
                 (ADR 1762)"
            ));
            return None;
        };
        Some(Property::TextFont(FontName { base, resource }))
    }

    /// Holds a script's request of the host's console, at the report it applies after; past
    /// [`MAX_CONSOLE_REQUESTS`] the latest replaces the last held (ADR 1762).
    fn ask_console(&mut self, command: crate::view::ConsoleCommand) {
        let request = ConsoleRequest {
            command,
            at: self.script_reports.len(),
        };
        let console = &mut self.scripting.console;
        if console.len() < MAX_CONSOLE_REQUESTS {
            console.push(request);
        } else if let Some(last) = console.last_mut() {
            *last = request;
        }
    }

    /// Holds a script's change to the window's view for a host, or reports why it names nothing
    /// a host can carry out (ADR 1736). A change past [`MAX_VIEW_CHANGES`] replaces the latest of
    /// its own kind, so the last zoom a script asked for is the one that stands.
    fn ask_view(&mut self, document: &Document, change: ViewChange) {
        if let ViewChange::Scroll { page, .. } | ViewChange::Destination { page, .. } = change {
            let pages = crate::page::Pages::new(document).len();
            if usize::try_from(page).map_or(true, |page| page >= pages) {
                self.report(format!(
                    "a script asked for a view of page {}, and this document has {pages}, so the \
                     view does not move (ADR 1736)",
                    u64::from(page).saturating_add(1)
                ));
                return;
            }
        }
        let views = &mut self.scripting.views;
        if views.len() < MAX_VIEW_CHANGES {
            views.push(change);
        } else if let Some(same) = views
            .iter_mut()
            .rev()
            .find(|held| std::mem::discriminant(*held) == std::mem::discriminant(&change))
        {
            *same = change;
        }
    }

    /// Applies a script's `currentValueIndices`: §12.7.5.4's items chosen by index, as a person's
    /// choice is written — `/V` naming them and `/I` listing them — on every widget of the field
    /// (ADR 1725). A field a person is typing into is not written under them.
    fn choose(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        field: &str,
        indices: &[u32],
    ) -> bool {
        let Some(widgets) = table.get(field) else {
            return false;
        };
        if widgets.iter().any(|widget| self.is_editing(*widget)) {
            self.report(format!(
                "{field}: a script chose items while a person was typing into the field, and what \
                 they are typing stands"
            ));
            return false;
        }
        let indices: Vec<usize> = indices
            .iter()
            .filter_map(|index| usize::try_from(*index).ok())
            .collect();
        let scripted = widgets
            .first()
            .map_or(&[][..], |widget| self.scripted(*widget));
        let Some(entry) =
            crate::view::fields::chosen(document, widgets.first().copied(), scripted, &indices)
        else {
            self.report(format!(
                "{field}: a script set currentValueIndices, and the field is not a choice field \
                 whose options it names, so nothing is chosen (ADR 1725)"
            ));
            return false;
        };
        for widget in widgets {
            self.reset.remove(widget);
            self.imported.remove(widget);
            self.edited.insert(*widget, entry.clone());
        }
        true
    }

    /// Holds a script's `gotoNamedDest` for a host as the page and the view its destination names:
    /// §12.3.2.4's name looked up where the clause keeps it, and the destination's page found as a
    /// link's is (ADR 1724); Table 149's position and magnification go with it as one
    /// [`ViewChange::Destination`], so the host shows the place as it shows a link's (ADR 1751).
    fn go_to_named(&mut self, document: &Document, name: &str) {
        let key = Object::String(name.as_bytes().to_vec().into());
        let pages = crate::page::Pages::new(document);
        let Some(destination) = crate::destination::Destination::read(document, &key) else {
            self.report(format!(
                "a script asked for the named destination {name:?}, which this document does not \
                 define (ISO 32000-2 §12.3.2.4), so no page is turned (ADR 1724)"
            ));
            return;
        };
        let Some(page) = destination.page_index(document, &pages) else {
            self.report(format!(
                "a script asked for the named destination {name:?}, whose page is not one of this \
                 document's, so no page is turned (ADR 1724)"
            ));
            return;
        };
        let Ok(page) = u32::try_from(page) else {
            self.report(format!(
                "a script asked for the named destination {name:?}, whose page lies past the \
                 pages a script can name, so no page is turned (ADR 1751)"
            ));
            return;
        };
        self.ask_view(
            document,
            ViewChange::Destination {
                page,
                view: destination.view,
            },
        );
    }

    /// Holds a script's `setFocus` for a host, on the widget it names or on its field's first, or
    /// reports why no widget can take it (ADRs 1615, 1688).
    fn ask_focus(
        &mut self,
        table: &BTreeMap<String, Vec<ObjectId>>,
        field: &str,
        widget: Option<u32>,
    ) {
        let Some(widgets) = table.get(field) else {
            self.report(format!(
                "a script asked for the focus on {field}, which is not a field of this document"
            ));
            return;
        };
        let index = widget.map_or(0, |index| usize::try_from(index).unwrap_or(usize::MAX));
        if index >= widgets.len() {
            self.report(format!(
                "{field}: a script asked for the focus on its widget {index}, and the field has \
                 {} widget(s), so the focus stays where it is (ADR 1688)",
                widgets.len()
            ));
            return;
        }
        self.scripting.focus = Some(FocusRequest {
            field: field.to_owned(),
            widget: index,
        });
    }

    /// Applies one property a script set, on the widget it names or on every widget of its field,
    /// and sets `values` where the page has to be drawn again (ADRs 1603, 1617, 1664).
    fn apply_property(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        (field, widget): (&str, Option<u32>),
        property: &Property,
        values: &mut bool,
    ) {
        let Some(all) = table.get(field) else {
            return;
        };
        // A widget-level member set through one widget's `Field` reaches that widget
        // alone; every other member is the field's (ADR 1664).
        let widget = widget.filter(|_| property.is_widget_level());
        let widgets: Vec<ObjectId> = match widget {
            None => all.clone(),
            Some(index) => {
                let Some(one) = usize::try_from(index).ok().and_then(|index| all.get(index)) else {
                    self.report(format!(
                        "{field}: a script set Field.{} on its widget {index}, and \
                         the field has {} widget(s), so nothing is set (ADR 1664)",
                        property.member(),
                        all.len()
                    ));
                    return;
                };
                vec![*one]
            }
        };
        // A style is a ZapfDingbats code, and in any other font the same code is a
        // letter: drawn there, it would be a mark the script did not ask for (ADR
        // 1665).
        if let Property::Style(glyph) = property
            && !widgets.iter().all(|widget| {
                document.get(*widget).as_dict().is_some_and(|dictionary| {
                    crate::appearance::draws_dingbats(document, dictionary)
                })
            })
        {
            self.report(format!(
                "{field}: a script set Field.style to {}, and the field's /DA font is \
                 not ZapfDingbats, in which alone the style's code is its glyph; the \
                 widget is drawn as before (ADR 1665)",
                glyph.adobe()
            ));
            return;
        }
        let resolved;
        let property = if let Property::TextFont(asked) = property {
            let Some(found) = self.form_font(document, field, &asked.base) else {
                return;
            };
            resolved = found;
            &resolved
        } else {
            property
        };
        let member = property.member();
        if widget.is_some() {
            self.scripting
                .one_widget
                .extend(widgets.iter().map(|widget| (*widget, member)));
        } else {
            for widget in &widgets {
                self.scripting.one_widget.remove(&(*widget, member));
            }
        }
        self.scripting
            .overrides
            .entry(field.to_owned())
            .or_default()
            .record(widget, property.clone());
        match property {
            Property::Display(display) => {
                for widget in &widgets {
                    self.set_hidden(*widget, !display.on_screen());
                }
            }
            // Table 227 bit 1 bars a *user*, and `set_field` is where a user's value
            // arrives, so that is where the override is read.
            Property::ReadOnly(_) => {}
            // What the appearance draws, and `required`, which a save writes: kept
            // per widget for `AnnotationView::scripted`, and the page drawn again
            // (ADR 1617).
            // A caption is Table 192's `/CA`, `/AC` or `/RC`, drawn and saved the same
            // way (ADR 1626).
            Property::TextColor(_)
            | Property::FillColor(_)
            | Property::StrokeColor(_)
            | Property::BorderStyle(_)
            | Property::Alignment(_)
            | Property::LineWidth(_)
            | Property::TextSize(_)
            | Property::TextFont(_)
            | Property::CharLimit(_)
            | Property::Required(_)
            | Property::Caption(..)
            | Property::Style(_)
            // Table 234's `/Opt` as a script rewrote it: what a list box draws, what a host's
            // control lists and what a save writes, each read as the entry it writes (ADR 1737).
            | Property::Options(_) => {
                for widget in &widgets {
                    let held = self.scripting.drawn.entry(*widget).or_default();
                    held.retain(|kept| !kept.replaces(property));
                    held.push(property.clone());
                }
                *values = true;
            }
            other @ Property::TextFlag(..) => self.report(format!(
                "{field}: a script set Field.{}; this view state keeps it and the \
                 drawn appearance does not carry it (ADR 1603)",
                other.member()
            )),
        }
    }

    /// A script's `OCG.state = …`, made as a person's layer switch makes it, or reported where no
    /// person's switch could make it (ADR 1626).
    ///
    /// Table 99's `/Locked` says a locked group's state "cannot be changed through the user
    /// interface", and its next sentence leaves the rest to the processor: one "may allow the
    /// states of optional content groups to be changed by means other than the user interface,
    /// such as ECMAScript". This program takes the narrower reading for a document's scripts — a
    /// script reaches what the person reading could do by hand (RFC 0008 section 4.2) — so the
    /// change goes through [`ViewState::set_group`], and a locked group stays as it is.
    fn switch_layer(&mut self, document: &Document, number: u32, generation: u16, on: bool) {
        let group = ObjectId { number, generation };
        if self.set_group(group, on) {
            return;
        }
        let name = self
            .optional_content
            .as_ref()
            .and_then(|content| content.name(document, group))
            .unwrap_or_else(|| format!("object {number}"));
        let already = self
            .optional_content
            .as_ref()
            .and_then(|content| content.state(group))
            == Some(on);
        if already {
            return;
        }
        let why = if self
            .optional_content
            .as_ref()
            .is_some_and(|content| content.is_locked(group))
        {
            "Table 99's /Locked names it, and a script switches only what the person reading \
             could switch by hand (ADR 1626)"
        } else {
            "it is not a group whose state this document's configuration lets a switch change"
        };
        self.report(format!(
            "a script set the layer {name:?} {}, and it stays as it was: {why}",
            if on { "on" } else { "off" }
        ));
    }
}
