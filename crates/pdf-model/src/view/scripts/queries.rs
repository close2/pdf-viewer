//! The field, page, word and window values presented to a script.

use super::{
    BTreeMap, Displayed, Document, Instant, MAX_WORD_PAGES, MAX_WORD_TIME, Object, ObjectId,
    Property, ScriptEvent, ScriptSite, Site, Trigger, ViewState, WindowView, page_words, site,
    supplied_script,
};

impl ViewState {
    /// The properties scripts have set on one field as a whole, the latest of each member, in the
    /// order they were set; a member set through one widget's `Field` is that widget's alone and is
    /// drawn from [`crate::view::AnnotationView::scripted`] (ADR 1664).
    ///
    /// What a host drawing its own control over a field reads beside the value. A script's
    /// `textColor`, `fillColor`, `strokeColor`, `borderStyle`, `alignment` and `charLimit` are
    /// drawn into the page's appearance as well and `required` is saved (ADR 1617); `display` and
    /// `readonly` are applied as well as kept.
    #[must_use]
    pub fn script_properties(&self, name: &str) -> &[Property] {
        self.scripting
            .overrides
            .get(name)
            .map_or(&[], |overrides| overrides.set.as_slice())
    }

    /// What one field displays: its value through its format script, or as it stands.
    ///
    /// [`Self::field_value`] answers with the characters a host edits; this answers with what the
    /// page shows when nobody is editing them — Table 199's `/F` "performed before the field is
    /// formatted to display its value" — which is what an assistive technology reading an
    /// unfocused field, or a host drawing its own control over one, wants. A field being typed
    /// into displays as typed. `None` in [`Self::field_value`]'s cases, and a password field
    /// answers with its echo, unformatted (ADR 1579).
    ///
    /// A format a runner ran at the last commit or open is answered from what it displayed then,
    /// which is what the page draws; one it has not run is run now, told no field, and nothing it
    /// says is recorded — this answers a question (ADR 1603).
    #[must_use]
    pub fn displayed_value(&self, document: &Document, name: &str) -> Option<String> {
        self.displayed_values(document, [name]).pop().flatten()
    }

    /// [`Self::displayed_value`] for each of several fields, with §12.7.4.1's field tree walked
    /// once for all of them rather than once for each.
    ///
    /// What a host placing a control over every field of a page asks, on every repaint: one walk
    /// per field made that question grow with the square of the form — 0.54 ms became 5.2 ms on
    /// `160F-2019.pdf`'s first page, the corpus's most formatted, in `viewer-core`'s
    /// `examples/fields_cost.rs` (ADR 1604).
    #[must_use]
    pub fn displayed_values<'n>(
        &self,
        document: &Document,
        names: impl IntoIterator<Item = &'n str>,
    ) -> Vec<Option<String>> {
        // Asked on every repaint, so the pages are walked for a root `/Fields` omits only where a
        // name asked is not a field it lists — a document that has one on screen (ADR 1653).
        let names: Vec<&str> = names.into_iter().collect();
        let mut table = crate::view::field_table(document, crate::view::Omitted::Listed);
        if names.iter().any(|name| !table.contains_key(*name)) {
            table = crate::view::widgets_by_field_name(document);
        }
        names
            .into_iter()
            .map(|name| self.displayed_in(document, &table, name))
            .collect()
    }

    /// One field's displayed value, read through a table of the field tree the caller walked.
    fn displayed_in(
        &self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        name: &str,
    ) -> Option<String> {
        let widget = table.get(name)?.first().copied()?;
        let object = document.get(widget);
        let dictionary = object.as_dict()?;
        // `Self::field_value`'s reading, from the same table.
        let shown = crate::appearance::field_text_value(
            document,
            dictionary,
            self.annotation(widget).value,
        )?;
        if shown.obscured || self.is_editing(widget) {
            return Some(shown.text);
        }
        Some(
            match site::of_widget(document, dictionary, Trigger::Format) {
                Site::Library(call) => call
                    .format(&shown.text)
                    .map_or(shown.text, |formatted| formatted.text),
                Site::NotRun(_) => {
                    if let Some(displayed) = self.scripting.formatted.get(&widget)
                        && displayed.value == shown.text
                    {
                        return Some(displayed.shown.clone());
                    }
                    let at = shown.text.len();
                    let event = ScriptEvent {
                        value: &shown.text,
                        selection: (at, at),
                        ..ScriptEvent::at(ScriptSite::Field(Trigger::Format), name)
                    };
                    self.runner
                        .0
                        .as_ref()
                        .zip(supplied_script(document, dictionary, Trigger::Format))
                        .and_then(|(runner, script)| {
                            runner
                                .run(&ScriptEvent {
                                    script: &script,
                                    pages: crate::page::Pages::new(document).len(),
                                    view: self.window_view(document),
                                    ..event
                                })
                                .value
                        })
                        .unwrap_or(shown.text)
                }
                Site::Absent => shown.text,
            },
        )
    }

    /// Every page's zero-based index by its object, walked once (ADR 1640).
    pub(in crate::view) fn page_indices(
        &mut self,
        document: &Document,
    ) -> &BTreeMap<ObjectId, usize> {
        self.scripting
            .pages
            .get_or_insert_with(|| crate::page::Pages::new(document).indices())
    }

    /// The zero-based page Table 166's `/P` names for a field's first widget: the page a person is
    /// on when they type into the field, and so what `this.pageNum` reads at its own events, which
    /// a script's page turn counts from (ADR 1640). `None` where the widget names no page.
    pub(in crate::view::scripts) fn field_page(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        name: &str,
    ) -> Option<usize> {
        let widget = *table.get(name)?.first()?;
        let page = document
            .get(widget)
            .as_dict()?
            .get("P")
            .and_then(Object::as_reference)?;
        self.scripting
            .pages
            .get_or_insert_with(|| crate::page::Pages::new(document).indices())
            .get(&page)
            .copied()
    }

    /// What a script reads of the window's view: what a host last told
    /// ([`Self::set_window_view`]), or, where none has, the arrangement the document asks its
    /// window to open with — Table 29's `/PageLayout`, which "shall be used when the document is
    /// opened" — and no magnification, since nothing is drawing the page (ADR 1736).
    pub(in crate::view::scripts) fn window_view(&self, document: &Document) -> WindowView {
        self.scripting.window.unwrap_or_else(|| WindowView {
            layout: crate::viewer_preferences::Opening::read(document).layout,
            ..WindowView::default()
        })
    }

    /// Reads every page's words for the document's scripts, once, bounded by [`MAX_WORD_PAGES`]
    /// and [`MAX_WORD_TIME`], and says where the reading stopped short (ADR 1762).
    ///
    /// A page's words are what its interpretation reads back, so the page is interpreted as it is
    /// drawn, against this view state. Table 22's bit 5, which withholds "\[c]opy or otherwise
    /// extract text and graphics from the document", is not consulted: the words go to the
    /// document's own scripts, in a realm with no path out of the process, and leave the
    /// document for nobody (ADR 1762).
    pub(in crate::view::scripts) fn read_words(&mut self, document: &Document) {
        let pages = crate::page::Pages::new(document);
        let count = pages.len();
        let started = Instant::now();
        let mut words = Vec::with_capacity(count.min(MAX_WORD_PAGES));
        let mut stopped = None;
        for index in 0..count {
            if index >= MAX_WORD_PAGES || started.elapsed() > MAX_WORD_TIME {
                stopped = Some(index);
                break;
            }
            words.push(pages.get(index).map(|page| {
                page_words(&crate::content::interpret_with(document, &page, self).text)
            }));
        }
        if let Some(index) = stopped {
            self.report(format!(
                "the document's scripts read the pages' words, and the reading stopped at page {} \
                 of {count}, at its bound of {MAX_WORD_PAGES} pages or {} ms: a script asking for \
                 a later page's words is refused (ADR 1762)",
                index.saturating_add(1),
                MAX_WORD_TIME.as_millis()
            ));
        }
        self.scripting.words = Some(words);
    }

    /// The pages' words as a realm is told them, where they have been read.
    pub(in crate::view) fn words_of(&self, page: usize) -> Option<Vec<String>> {
        self.scripting
            .words
            .as_ref()
            .and_then(|words| words.get(page).cloned().flatten())
    }

    /// The rich text string `event.richValue` reads for a rich text field's widget (ADR 1762):
    /// Table 228's `/RV` where it holds the characters the field's value does, and otherwise — a
    /// value a person or a script set since — the value written as the save writes it, its own
    /// characters with no formatting of their own (ADR 1635). `None` for a field Table 231 bit 26
    /// does not make rich text.
    pub(in crate::view::scripts) fn rich_value_of(
        &self,
        document: &Document,
        widget: ObjectId,
    ) -> Option<String> {
        /// Table 231 bit 26, `RichText`.
        const RICH_TEXT: i64 = 1 << 25;
        let dictionary = document.get(widget);
        let dictionary = dictionary.as_dict()?;
        let field =
            crate::appearance::Field::read(document, dictionary, self.annotation(widget).value);
        if field.flags & RICH_TEXT == 0 {
            return None;
        }
        let value = self.text_of(document, widget).unwrap_or_default();
        let stated = field.chain().into_iter().find_map(|level| {
            crate::variable_text::value_text(document, &document.get_key(level, "RV"))
        });
        Some(
            stated
                .filter(|rich| {
                    crate::rich_text::characters(rich).is_some_and(|characters| {
                        crate::rich_text::same_characters(&characters, &value)
                    })
                })
                .unwrap_or_else(|| crate::rich_text::written(&value)),
        )
    }

    /// What one widget displays through a runner's format, where one ran for its current value.
    pub(in crate::view) fn displayed(&self, annotation: ObjectId) -> Option<&Displayed> {
        self.scripting.formatted.get(&annotation)
    }

    /// A widget's value as the text a script reads: a text field's characters, a button's state
    /// name, a choice field's first selected item.
    pub(in crate::view) fn text_of(&self, document: &Document, widget: ObjectId) -> Option<String> {
        let object = document.get(widget);
        let dictionary = object.as_dict()?;
        let field =
            crate::appearance::Field::read(document, dictionary, self.annotation(widget).value);
        match document.resolve(field.value.as_ref()?) {
            Object::Name(name) => Some(String::from_utf8_lossy(name.as_bytes()).into_owned()),
            Object::Integer(value) => Some(value.to_string()),
            Object::Real(value) => Some(crate::aform::number_text(value)),
            other => crate::variable_text::value_text(document, &other),
        }
    }
}
