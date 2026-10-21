//! What a person did to the open document: resolved, applied, logged and replayed.
//!
//! The view state is an edit log beside the immutable `Document` (ADR 1787), so everything here
//! changes [`Open::view`] and never the file.

use pdf_render::Point;
use pdf_syntax::ObjectId;

use super::{Committed, Done, Open};

impl Open {
    /// Turns what a host asked for into what was done, or `None` where nothing was.
    ///
    /// The one place [`crate::Edit`] and [`Done`] meet. A field's value passes through unchanged;
    /// a markup is resolved *here*, against the selection and the page as they are now, because
    /// the log has to record geometry rather than an intention — see [`Done`].
    ///
    /// `None` for a markup with nothing selected, with no page interpreted, or on a page reached
    /// without an identity ([`pdf_model::Pages::detached`]'s, which is §12.7.7's template): each is a
    /// request this crate cannot turn into an annotation, and adding an empty one would be worse
    /// than doing nothing.
    ///
    /// `drag` is [`crate::Edit::FreeText`]'s two corners, **already in default user space**, and
    /// it is a parameter rather than something read here because the map from the viewport is the
    /// *viewer's*: it needs the viewport's size and the display's scale, which are properties of
    /// the window and not of an open document. Everything else a resolution needs — the page, the
    /// selection, the page transform — is this type's.
    pub(crate) fn resolve(
        &self,
        edit: crate::command::Edit,
        drag: Option<[(f32, f32); 2]>,
        dropped: Option<(usize, (f32, f32))>,
    ) -> Option<Done> {
        match edit {
            crate::command::Edit::SetField { field, value } => {
                Some(Done::SetField { field, value })
            }
            crate::command::Edit::ChooseFile {
                field,
                pathname,
                bytes,
                mime,
            } => Some(Done::ChooseFile {
                field,
                pathname,
                bytes,
                mime,
            }),
            crate::command::Edit::Detach { name } => Some(Done::Detach { name }),
            crate::command::Edit::Attach {
                bytes,
                name,
                description,
                mime,
                home,
            } => {
                let home = match home {
                    crate::command::AttachHome::Document => pdf_model::view::FilingHome::Document,
                    crate::command::AttachHome::Page { .. } => {
                        // The page under the point rather than the current page, for the reason
                        // `Markup` takes the selection's: Table 29's continuous arrangements
                        // put several on the screen, and a person dropped the file on one.
                        let (index, point) = dropped?;
                        let page = self.page(index)?;
                        pdf_model::view::FilingHome::Page {
                            page: page.id?,
                            rect: pdf_model::attachment::filing::rect_around(point),
                        }
                    }
                };
                Some(Done::Attach {
                    filing: pdf_model::view::Filing {
                        bytes,
                        name,
                        description,
                        media_type: mime,
                        home,
                    },
                })
            }
            crate::command::Edit::SetFreeText { annotation, text } => {
                Some(Done::SetFreeText { annotation, text })
            }
            crate::command::Edit::SetNoteText { annotation, text } => {
                Some(Done::SetNoteText { annotation, text })
            }
            crate::command::Edit::FreeText { colour, .. } => {
                let [from, to] = drag?;
                let page = self.page(self.page_index)?;
                Some(Done::FreeText {
                    page: page.id?,
                    rect: [from.0, from.1, to.0, to.1],
                    colour,
                })
            }
            crate::command::Edit::Markup { kind, colour } => {
                // The *selection's* pages rather than the current one: a mark-up is over what is
                // selected, and Table 29's continuous arrangements let a person scroll on after
                // selecting without having changed what they chose — and select across a row
                // boundary in the first place, which is why this is a page at a time.
                let mut marked: Vec<(ObjectId, Vec<[f32; 8]>)> = Vec::new();
                for (index, range) in self.spans() {
                    let Some(interpreted) = self.interpretation(index) else {
                        continue;
                    };
                    let quads = crate::select::quads_for(&interpreted.placed, range);
                    let page = self.page(index)?;
                    // §12.5.6.10's `/QuadPoints` is in default user space and everything this
                    // crate answers with is in the display list's, so the shapes go back through
                    // the transform that put them there. A page transform is a similarity with a
                    // flip and is always invertible; the `?` is the type system's rather than a
                    // case.
                    let back = pdf_model::content::page_transform(&page).invert()?;
                    let quads = quads
                        .into_iter()
                        .map(|quad| {
                            let mut mapped = [0.0; 8];
                            for (corner, out) in
                                quad.chunks_exact(2).zip(mapped.chunks_exact_mut(2))
                            {
                                let point = back.apply(Point {
                                    x: corner[0],
                                    y: corner[1],
                                });
                                out[0] = point.x;
                                out[1] = point.y;
                            }
                            mapped
                        })
                        .collect::<Vec<[f32; 8]>>();
                    if !quads.is_empty() {
                        marked.push((page.id?, quads));
                    }
                }
                (!marked.is_empty()).then_some(Done::Markup {
                    pages: marked,
                    kind,
                    colour,
                })
            }
        }
    }

    /// Replays the log up to the cursor onto a state that has none of it applied.
    ///
    /// Undo and redo are *replays* rather than inverses, and that is the decision: an inverse
    /// would have to remember what each edit replaced — which for a field with no `/V` is "no
    /// value at all", a state distinct from every value — and would drift from the log the moment
    /// two edits touched one field. Replaying costs one pass over the log per undo, and the log
    /// is what a person did in one sitting.
    pub(crate) fn replay(&mut self) {
        let log = std::mem::take(&mut self.log);
        self.view.clear_all_fields();
        self.view.clear_all_additions();
        self.view.clear_all_free_text();
        self.view.clear_all_attachments();
        for edit in log.iter().take(self.cursor) {
            self.apply(edit);
        }
        self.log = log;
        // The page's ink depends on the values, so the display list and every readback are stale.
        self.stale();
    }

    /// Applies one entry of the log to the state the entries before it made.
    ///
    /// [`Self::replay`] applies each surviving entry in turn, and [`Self::commit`] applies the one
    /// it adds and no other, so a document's scripts run once for what a person did when they did
    /// it — Table 199's `/K` "shall be performed when the user modifies a character in a text
    /// field or combo box or modifies the selection in a scrollable list box", not again at every
    /// later edit — and run again only where an undo or a redo rebuilds the state (ADR 1787).
    fn apply(&mut self, edit: &Done) {
        match edit {
            // An attach the log holds was accepted when it was made, and the name it was
            // refused for cannot have arrived since — the log is the only writer — so a
            // refusal here is not a case; `let _ =` says so rather than unwrapping.
            Done::Attach { filing } => {
                let _ = self.view.attach(&self.document, filing.clone());
            }
            Done::Detach { name } => {
                self.view.detach(&self.document, name);
            }
            Done::SetField { field, value } => {
                self.view.set_field(&self.document, field, value);
            }
            // The sentence a refusal carries was said when the commit was made; a replay
            // reaches the same outcome and has nobody new to tell.
            Done::CommitField { field } => {
                let _ = self.view.commit_field(&self.document, field);
            }
            Done::ChooseFile {
                field,
                pathname,
                bytes,
                mime,
            } => {
                self.view.choose_file(
                    &self.document,
                    field,
                    pdf_model::view::ChosenFile {
                        pathname: pathname.clone(),
                        media_type: mime.clone(),
                        bytes: bytes.as_ref().to_vec(),
                    },
                );
            }
            Done::Markup {
                pages,
                kind,
                colour,
            } => {
                for (page, quads) in pages {
                    self.view
                        .add_markup(&self.document, *page, *kind, *colour, quads);
                }
            }
            Done::FreeText { page, rect, colour } => {
                self.view
                    .add_free_text(&self.document, *page, *rect, "", *colour);
            }
            Done::SetFreeText { annotation, text } => {
                self.view.set_free_text(&self.document, *annotation, text);
            }
            Done::SetNoteText { annotation, text } => {
                self.view.set_note_text(&self.document, *annotation, text);
            }
        }
    }

    /// Whether anything a person did is unsaved.
    ///
    /// **The cursor's distance from the last save, not its distance from zero**: asking
    /// `cursor > 0` would leave a document saying it had unsaved work for as long as it stayed
    /// open after being saved. The model
    /// holds the other half of the answer in `ViewState::additions` and `ViewState::edits`,
    /// which is what `doc/todo/01`'s fifth sweep found no caller for: the question a host asks
    /// is *what is unwritten*, and a log's length is what was **done**.
    pub(crate) fn dirty(&self) -> bool {
        self.cursor != self.saved_at
    }

    /// Records that everything up to the cursor has been written to a file.
    pub(crate) fn saved(&mut self) {
        self.saved_at = self.cursor;
    }

    /// Adds one resolved edit to the log and applies it.
    ///
    /// A new edit after an undo discards what was undone: the log is one sequence with a cursor,
    /// which is what makes a replay of its prefix the whole of the state. Answers whether the
    /// document's unsaved mark changed, and whether the attachment list moved — the two facts
    /// the caller has events for.
    pub(crate) fn commit(&mut self, done: Done) -> Committed {
        let before = self.dirty();
        let attachments = done.moves_attachments();
        self.log.truncate(self.cursor);
        // The state is the replay of the log up to the cursor already — an undo or a redo replayed
        // it and every commit since applied its own entry — so the new entry is applied to it
        // alone (ADR 1787).
        self.apply(&done);
        self.log.push(done);
        self.cursor = self.log.len();
        // The page's ink depends on the values, so the display list and every readback are stale.
        self.stale();
        Committed {
            dirty_changed: self.dirty() != before,
            attachments,
        }
    }

    /// Commits what was typed into one field, and logs the commit where there was one.
    ///
    /// `None` where nothing was being typed into the field, which leaves the log alone: an entry
    /// that did nothing would be an undo step over nothing. The view is committed in place rather
    /// than through [`Self::commit`]'s replay, because the entry's outcome is the one a replay
    /// would reach and the log up to the cursor is already applied.
    pub(crate) fn commit_field(&mut self, field: &str) -> Option<pdf_model::view::Committed> {
        let committed = self.view.commit_field(&self.document, field);
        if committed == pdf_model::view::Committed::Nothing {
            return None;
        }
        self.log.truncate(self.cursor);
        self.log.push(Done::CommitField {
            field: field.to_owned(),
        });
        self.cursor = self.log.len();
        // The formatted value, a value put back and a recalculated total are all ink.
        self.stale();
        Some(committed)
    }

    /// The activation §12.5.1 describes, for an annotation that has one of §12.5.6.14's windows.
    ///
    /// ISO 32000-2 §12.5.1:
    ///
    /// > When the user activates the annotation by clicking it, it exhibits its associated
    /// > object, such as by opening a popup window displaying a text note
    ///
    /// The clause says *exhibits*, not *toggles*: what a second click does is not in the standard,
    /// and closing the window is the choice every reader of a sticky note makes. Recorded as a
    /// choice. Returns whether anything changed, which is what tells the caller to repaint.
    ///
    /// **The click is written where a script's `popupOpen` is**, the view state's map of windows:
    /// Table 186's `/Open` is "[a] flag specifying whether the popup annotation shall *initially*
    /// be displayed open", so the file states the first frame and the map every change since, and
    /// whichever of a person and a script wrote last is the window's state (ADR 1720).
    pub(crate) fn toggle_popup(&mut self, annotation: ObjectId) -> bool {
        let Some(page) = self.shown_page() else {
            return false;
        };
        let object = self.document.get(annotation);
        let Some(dict) = object.as_dict() else {
            return false;
        };
        // Either end of Table 172's `/Popup`: a click lands on the *markup* annotation, and a
        // host with a close button on the window has the popup itself.
        let popup = pdf_model::popup::popup_of(&self.document, dict).unwrap_or(annotation);
        let Some(state) = pdf_model::popup::popups(&self.document, page, &self.view)
            .iter()
            .find(|window| window.annotation == popup)
            .map(|window| window.open)
        else {
            return false;
        };
        self.view.set_popup_open(popup, !state);
        true
    }
}
