//! A document edited and written: a field typed into, an annotation changed, the result saved or
//! extracted, and the keyboard focus that decides where a character lands.

use pdf_model::action::Trigger;
use pdf_syntax::ObjectId;

use super::geometry::page_object;
use super::layout::damage;
use super::media::hand_over;
use super::pointer::is_widget;
use super::{DocumentId, Standing, Viewer};
#[cfg(doc)]
use crate::Command;
#[cfg(doc)]
use crate::Query;
use crate::event::{Event, Extraction};
use crate::open::Open;

impl Viewer {
    /// Writes §7.5.6's incremental update, or says why it could not be written.
    pub(super) fn save(&mut self, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused_mut() else {
            return;
        };
        // Table 200's `/WS`, "before saving a document", so that what the script writes into a
        // field is in the update; `/DS` follows a save that was written (ADR 1614).
        let page = open.page_index;
        open.view.run_document_scripts(
            &open.document,
            pdf_model::view::DocumentTrigger::WillSave,
            page,
        );
        let saved = open.view.save(&open.document);
        if saved.is_ok() {
            open.view.run_document_scripts(
                &open.document,
                pdf_model::view::DocumentTrigger::DidSave,
                page,
            );
        }
        match saved {
            Ok(written) => {
                // Table 231 bit 14's NOTE, said out loud. `pdf_model::view::ViewState::save`
                // declines to store a password field's value and this is the only channel that
                // can tell a person their typing did not reach the file — a save that quietly
                // dropped it would be exactly the silence trap 5 exists against. Not `Refused`:
                // that one is for what a *document* asserts over its reader, and this is a rule
                // this program keeps on the person's own behalf.
                for field in written.withheld {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![format!(
                            "{field} is a password field, so what was typed into it was not \
                             written to the file (ISO 32000-2 Table 231, bit 14)"
                        )],
                    });
                }
                // The same rule one clause over, and the same channel. §12.5.6.6's annotation is
                // its text, so an annotation written without an appearance is one whose note this
                // program could not draw — Table 177 makes `/DA` Required, so the next reader has
                // what it needs and this one says which annotations it left to them.
                for annotation in written.unappeared {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![format!(
                            "the free text annotation {} {} was written with what was typed into \
                             it and no appearance stream, because this program could not lay that \
                             text out (ISO 32000-2 §12.5.6.6)",
                            annotation.number, annotation.generation
                        )],
                    });
                }
                // Table 224's `/NeedAppearances` is in the written file, and a flag names
                // nobody. The entry's own row binds a writer "if it has not provided appearance
                // streams for all visible widget annotations present in the document", so a file
                // written with it set has something outstanding in it and this says which field
                // — trap 5's rule on the one path where a *save* falls short of the clause.
                for field in written.unconstructed {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![format!(
                            "{field} was written with its value and without the whole of the \
                             appearance stream this program lays out for it, so the file asks \
                             the next reader to construct it (ISO 32000-2 §12.7.4.3, Table 224's \
                             /NeedAppearances)"
                        )],
                    });
                }
                // §7.11.4.1 gives an embedded file more than one home, so a detach that left
                // the tree is not a deletion where another home still names the stream; the
                // objects stay in use, and the person who asked is told which home keeps them.
                // §12.7.8.3.3's template pages are shown and not written: the update carries no
                // page into the page tree, so the person who saves is told what the file lacks
                // that the window shows (ADR 2154).
                if written.unwritten_pages > 0 {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![format!(
                            "{} page(s) the imported form data added are shown here and were not \
                             written to the file, because this program's save writes no page into \
                             the page tree (ISO 32000-2 §12.7.8.3.3)",
                            written.unwritten_pages
                        )],
                    });
                }
                for (name, homes) in written.still_reached {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![format!(
                            "{name:?} was taken out of the /EmbeddedFiles tree, but its file \
                             specification and stream are left in use because {homes} still \
                             reaches the stream (ISO 32000-2 §7.11.4.1)"
                        )],
                    });
                }
                events.push(Event::Saved {
                    document: id,
                    bytes: written.bytes,
                });
                // What the update wrote is now in a file, so the log up to the cursor owes it
                // nothing. Said out loud for the same reason an edit says it: `Event::Dirty` is
                // the only thing a host has to go on, and a mark that never comes off tells a
                // person their work is unsaved after they saved it.
                let was = open.dirty();
                open.saved();
                if was {
                    events.push(Event::Dirty {
                        document: id,
                        dirty: false,
                    });
                }
            }
            // Trap 5 on the one path where a *file* can refuse to be written: a save that
            // quietly did nothing is a person's work lost without a word.
            Err(error) => events.push(Event::Reported {
                document: id,
                page: None,
                notes: vec![format!("this document cannot be saved: {error}")],
            }),
        }
    }

    /// §7.11.4: hands an embedded file's decoded bytes to the host.
    ///
    /// The list is re-read rather than cached, for the reason [`Query::Attachments`] is answered
    /// the same way: it is a walk of one name tree over a document that cannot change, and
    /// holding a copy of every attachment's stream would hold a copy of every attachment.
    pub(super) fn extract(
        &mut self,
        name: &str,
        asked: Extraction,
        fragment: Option<String>,
        events: &mut Vec<Event>,
    ) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused() else { return };
        // The log's list, so that a file attached this sitting comes out again before it is
        // saved — its stream is the one the save would write, unfiltered.
        let Some(file) = open
            .view
            .attachments(&open.document)
            .into_iter()
            .find(|file| file.name == name)
        else {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes: vec![format!("this document embeds no file called {name:?}")],
            });
            return;
        };
        hand_over(id, asked, &open.document, &file, fragment, events);
    }

    /// Resolves one edit, asks the policy once, and does what the verdict says.
    ///
    /// Four verdicts, four arms, and none of them quiet: *proceed* commits the edit; *refuse*
    /// says why and commits nothing; *warn* commits and then says why; *ask* holds the resolved
    /// edit and says why, for [`Command::Answer`] to settle. A new edit after an undo discards
    /// what was undone: the log is one sequence with a cursor, which is what makes a replay of
    /// its prefix the whole of the state.
    pub(super) fn edit(&mut self, edit: crate::command::Edit, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let operation = operation_of(&edit);
        let standing = self.standing(id, operation, field_of(&edit), annotation_of(&edit));
        if let Standing::Refuse(refused) = standing {
            events.push(refused);
            return;
        }
        // §12.5.6.6's rectangle and §12.5.6.15's point are measured in the viewport, and the map
        // out of it needs the viewport's size and the display's scale — this type's rather than
        // an open document's. Taken before the mutable borrow, because the two cannot be held at
        // once.
        let (drag, dropped) = match &edit {
            crate::command::Edit::FreeText { from, to, .. } => {
                let Some(open) = self.focused() else { return };
                let (Some((_, from)), Some((_, to))) =
                    (self.user_space(open, *from), self.user_space(open, *to))
                else {
                    return;
                };
                (Some([from, to]), None)
            }
            crate::command::Edit::Attach {
                home: crate::command::AttachHome::Page { at },
                ..
            } => {
                let Some(open) = self.focused() else { return };
                let Some(dropped) = self.user_space(open, *at) else {
                    return;
                };
                (None, Some(dropped))
            }
            _ => (None, None),
        };
        let Some(open) = self.focused_mut() else {
            return;
        };
        // §7.9.6's keys "shall not overlap", a detach needs something to detach, and a retyping
        // needs a note: each is answered before the log is touched, and said.
        if let Some(note) = changes_nothing(open, &edit) {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes: vec![note],
            });
            return;
        }
        // What was *done*, rather than what was asked for: `Edit::Markup` names its target as
        // "what is selected", and a replay after the selection moved would mark up something
        // else. See `open::Done`.
        let Some(done) = open.resolve(edit, drag, dropped) else {
            return;
        };
        // Characters the field's script refuses change nothing, so they are said rather than
        // logged, for the reason a duplicate attachment is (ADR 1592).
        if !matches!(standing, Standing::Refuse(_)) && said_instead(id, open, &done, events) {
            return;
        }
        match standing {
            Standing::Refuse(_) => {}
            Standing::Proceed => commit(id, open, done, events),
            Standing::Warn(notes) => {
                commit(id, open, done, events);
                events.push(Event::Warned {
                    document: id,
                    operation,
                    notes,
                });
            }
            Standing::Ask(notes) => {
                open.asking = Some(crate::open::Held::Edit(done));
                events.push(Event::Asking {
                    document: id,
                    operation,
                    notes,
                });
            }
        }
    }

    /// [`Command::Answer`]: settles the edit [`Event::Asking`] held.
    ///
    /// Addressed by document rather than to the focused one, because the question named a
    /// document and a person may have turned to another window before answering.
    pub(super) fn answer(&mut self, document: DocumentId, proceed: bool, events: &mut Vec<Event>) {
        let Some(open) = self.documents.get_mut(&document) else {
            return;
        };
        let Some(held) = open.asking.take() else {
            return;
        };
        // §7.6.4.2's bit 12 is the one question whose `no` is an answer and not a cancellation:
        // Table 22 makes bit 3 the permission to print and bit 12 only the permission to print
        // faithfully, so declining there asks for the degraded job the cell describes rather than
        // for no job at all (ADR 1203). Every other `no` forgets what was held.
        let answered_either_way = matches!(held, crate::open::Held::PrintFaithfully(_));
        if !proceed && !answered_either_way {
            // §12.11.6's question is the one whose `no` has something to undo: a document held
            // before it was processed was never opened, no `Event::Opened` named it, and leaving
            // it in the map would be this crate holding a document nobody can see or close.
            if matches!(held, crate::open::Held::Process { .. }) {
                self.documents.remove(&document);
            }
            return;
        }
        match held {
            crate::open::Held::Process { fragment } => {
                self.process(document, fragment.as_deref(), events);
            }
            crate::open::Held::Edit(done) => commit(document, open, done, events),
            // Nothing to commit: a copy changes no document. What the `yes` releases is the text
            // itself, taken at the moment the question was asked.
            crate::open::Held::Copy {
                logical,
                page_order,
            } => events.push(Event::Copied {
                document,
                logical,
                page_order,
            }),
            // Nothing to commit here either: a print operation changes no document. What the
            // `yes` releases is the intent, applied to the sheet the question named.
            // A `yes` to bit 3 releases the printing; bit 12 is a second question and is asked
            // here for the first time, because a person answering one had not been asked the
            // other (ADR 1203).
            crate::open::Held::PrintFaithfully(sheet) => {
                let fidelity = if proceed {
                    crate::Fidelity::Faithful
                } else {
                    crate::Fidelity::Degraded
                };
                Self::begin_printing(document, open, sheet, fidelity, events);
            }
            // A conversion changes no document either: the `yes` releases the file to the host
            // for the target and profile the question was asked about (ADR 1954).
            crate::open::Held::Archive(asked) => {
                events.push(Self::archiving(document, open, asked));
            }
            // Nothing to commit: handing a clip over changes no document (ADR 1961).
            crate::open::Held::Media {
                content_type,
                media,
            } => events.push(Event::Media {
                document,
                content_type,
                media,
            }),
            crate::open::Held::Print(sheet) => {
                let granted = Self::faithfulness(
                    document,
                    open,
                    self.restrictions.under(open.restrictions),
                    sheet,
                    events,
                );
                if let Some(fidelity) = granted {
                    Self::begin_printing(document, open, sheet, fidelity, events);
                }
            }
        }
    }

    /// [`Command::Copy`]: §7.6.4.2's bit 5 asked as an operation, and the text if it is granted.
    ///
    /// The selection is taken **before** the policy is asked and in both of §14.8.2.5's orders,
    /// for `crate::open::Done`'s reason: under [`crate::RestrictionLevel::Ask`] the answer may
    /// come after the person has dragged somewhere else, and what goes ahead on a `yes` has to be
    /// what they were asked about. Nothing selected is nothing copied and nothing said — a
    /// question about an empty copy would be a question about nothing.
    pub(super) fn copy(&mut self, events: &mut Vec<Event>) {
        use pdf_model::restriction::Operation;

        let Some(id) = self.focused else { return };
        let Some(open) = self.focused() else { return };
        let Some(selected) = self.selected(open) else {
            return;
        };
        // §14.8.2.3's divisions undone as the logical copy undoes them (ADR 1971).
        let page_order = crate::select::rejoined(&selected.text).into_owned();
        if page_order.is_empty() {
            return;
        }
        let logical = Self::logical_selection(open);
        match self.standing(id, Operation::Extract, None, None) {
            Standing::Refuse(refused) => events.push(refused),
            Standing::Proceed => events.push(Event::Copied {
                document: id,
                logical,
                page_order,
            }),
            Standing::Warn(notes) => {
                events.push(Event::Copied {
                    document: id,
                    logical,
                    page_order,
                });
                events.push(Event::Warned {
                    document: id,
                    operation: Operation::Extract,
                    notes,
                });
            }
            Standing::Ask(notes) => {
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                open.asking = Some(crate::open::Held::Copy {
                    logical,
                    page_order,
                });
                events.push(Event::Asking {
                    document: id,
                    operation: Operation::Extract,
                    notes,
                });
            }
        }
    }

    /// The one place a document's restrictions are consulted, and it is asked per **operation**.
    ///
    /// Once per thing a person did, rather than once per widget the value lands on: §12.7.4.1
    /// makes one field's value shared by all of its widgets, and a question asked per widget
    /// would ask a person the same question three times about one keystroke. That is the shape
    /// *ask* and *warn* need, and it is why the check moved out of `ViewState::set_field` — where
    /// it could only ever say "nothing happened" — into the crate that holds the policy.
    ///
    /// [`Standing::Proceed`] means go ahead: either the document asserts nothing against the
    /// operation, or this reader has turned its restrictions off, which `CLAUDE.md` says shall
    /// always be possible. The two are one answer here and two answers to a person, which is
    /// what [`crate::Event::Refused`] carries the operation for.
    ///
    /// **The policy is `pdf_model::restriction::decide`'s, asked here once and matched
    /// exhaustively**, and all four of its verdicts have an arm of their own — two of them are
    /// [`Event::Asking`] and [`Event::Warned`] rather than a refusal, and the notes are worded
    /// for each (ADR 0803, ADR 0814).
    pub(super) fn standing(
        &self,
        id: DocumentId,
        operation: pdf_model::restriction::Operation,
        field: Option<&str>,
        annotation: Option<ObjectId>,
    ) -> Standing {
        let Some(open) = self.focused() else {
            return Standing::Proceed;
        };
        Self::verdict_of(
            id,
            // The window's levels as this document sees them: `under` is the whole of the
            // layering and is asked here rather than composed by a caller (ADR 1145).
            self.restrictions
                .under(open.restrictions)
                .level(operation)
                .level(),
            &open.document,
            operation,
            field,
            annotation,
        )
    }

    /// `pdf_model::restriction::decide` at a stated level, worded.
    ///
    /// Separate from [`Self::standing`] because §12.11.6 is asked of a document that is not in
    /// this viewer yet: the level is the window's, there is nothing focused to read it from, and
    /// the wording of all four verdicts must still be the one wording (ADR 1167).
    pub(super) fn verdict_of(
        id: DocumentId,
        level: pdf_model::restriction::Level,
        document: &pdf_syntax::Document,
        operation: pdf_model::restriction::Operation,
        field: Option<&str>,
        annotation: Option<ObjectId>,
    ) -> Standing {
        use crate::notes::{Standing as Worded, restricted};
        use pdf_model::restriction::Verdict;
        match pdf_model::restriction::decide(level, document, operation, field, annotation) {
            Verdict::Proceed => Standing::Proceed,
            Verdict::Refuse(restrictions) => Standing::Refuse(Event::Refused {
                document: id,
                operation,
                notes: restricted(operation, &restrictions, Worded::Refused),
            }),
            Verdict::Warn(restrictions) => {
                Standing::Warn(restricted(operation, &restrictions, Worded::Warned))
            }
            Verdict::Ask(restrictions) => {
                Standing::Ask(restricted(operation, &restrictions, Worded::Asked))
            }
        }
    }

    /// The shapes covering a range of the value of the field at a viewport point.
    ///
    /// The model answers in default user space, one quadrilateral per line, and this is the same
    /// mapping onto the screen every other shape here goes through — [`Self::device_point`] per
    /// corner rather than [`Self::device_quad`], because the corners have already been turned by
    /// whatever Table 192's `/R` and §12.5.5's placement do and a rectangle could not carry that.
    pub(super) fn field_selection(
        &self,
        open: &Open,
        at: (f32, f32),
        range: (usize, usize),
    ) -> Option<Vec<[f32; 8]>> {
        let (page, (x, y)) = self.user_space(open, at)?;
        let object = open.placed_page(page)?;
        let quads = open
            .view
            .field_selection(&open.document, object, (x, y), range)?;
        quads
            .into_iter()
            .map(|quad| {
                let mut out = [0.0_f32; 8];
                for (corner, place) in quad.chunks_exact(2).zip(out.chunks_exact_mut(2)) {
                    let (x, y) = self.device_point(open, page, (corner[0], corner[1]))?;
                    place[0] = x;
                    place[1] = y;
                }
                Some(out)
            })
            .collect()
    }

    /// §12.5.1's tab order, applied: the focus moves to the next or previous annotation.
    ///
    /// The order is `pdf_model::tab_order::order`, which is Table 31's `/Tabs` and all five of
    /// its values; what is decided *here* is the two things the clause leaves to an interface —
    /// that the move wraps, and that §12.6.3's `/Bl` and `/Fo` are raised in Table 197's own
    /// order, one thing losing the focus before the next receives it, exactly as a press does.
    ///
    /// **`/Fo` and `/Bl` still fire for widgets only**, because Table 197 says so of both, while
    /// the *focus itself* moves through every annotation the page has, because §12.5.1 says "the
    /// annotations on a page" without qualification and Table 31's `W` exists to name the
    /// narrower order.
    pub(super) fn move_focus(
        &mut self,
        move_to: crate::command::FocusMove,
        events: &mut Vec<Event>,
    ) {
        let (Some(id), Some(open)) = (self.focused, self.focused_mut()) else {
            return;
        };
        let wants = match move_to {
            crate::command::FocusMove::None => None,
            direction => {
                let Some(page) = open.shown_page() else {
                    return;
                };
                let Some(page_id) =
                    page_object(&pdf_model::Pages::new(&open.document), open.page_index)
                else {
                    return;
                };
                let order = pdf_model::tab_order::order(&open.document, page, page_id);
                match direction {
                    crate::command::FocusMove::Previous => {
                        pdf_model::tab_order::previous(&order, open.focus)
                    }
                    _ => pdf_model::tab_order::next(&order, open.focus),
                }
            }
        };
        self.focus_on(id, wants, events);
    }

    /// Moves the keyboard focus of document `id` to `wants`, raising Table 197's `/Bl` on the
    /// widget it leaves and `/Fo` on the one it reaches — a tab's half of [`Self::move_focus`], and
    /// what a script's `setFocus` comes to.
    pub(super) fn focus_on(
        &mut self,
        id: DocumentId,
        wants: Option<ObjectId>,
        events: &mut Vec<Event>,
    ) {
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        if open.focus == wants {
            return;
        }
        let mut raised = Vec::new();
        raised.extend(
            open.focus
                .filter(|left| is_widget(&open.document, *left))
                .map(|left| (left, Trigger::Blur)),
        );
        raised.extend(
            wants
                .filter(|got| is_widget(&open.document, *got))
                .map(|got| (got, Trigger::Focus)),
        );
        open.focus = wants;
        let viewport = self.viewport;
        self.raise(id, raised, None, events);
        // A focus ring is chrome the host draws, so what changed is the viewport rather than the
        // page — the same statement a selection makes, and for the same reason.
        events.push(damage(viewport));
    }

    /// The selection in §14.8.2.5's logical content order, where the page states one: the
    /// structure's pieces of each page's range, joined by [`crate::select::logical_copy`] with the
    /// paragraph and cell breaks the structure states and the artifacts it declares left out
    /// (ADR 1948).
    ///
    /// Walks the structure tree, so it is deliberately not on the path a drag takes — see
    /// [`Query::LogicalSelection`]. The page's object is needed because §14.7's tree is the
    /// document's and its content items name the page they are on.
    pub(super) fn logical_selection(open: &Open) -> Option<String> {
        let spans = open.spans();
        if spans.is_empty() {
            return None;
        }
        let tree = pdf_model::structure::Tree::of(&open.document)?;
        let pages = pdf_model::Pages::new(&open.document);
        let mut ordered: Vec<String> = Vec::with_capacity(spans.len());
        for (index, range) in spans {
            let interpreted = open.interpretation(index)?;
            // As `accessibility` does, and for the same reason: Table 355's `/Pg` names a page
            // *object* and what this crate holds is an index.
            let page = page_object(&pages, index)?;
            // **A page whose tree does not reach the range takes the whole answer with it**, the
            // same way a range the tree half-reaches always has: a copy that silently dropped one
            // page of a selection crossing a boundary would be worse than one handing back
            // content order for all of it.
            let pieces = tree.logical_range(
                &open.document,
                page,
                pdf_model::structure::ContentSpans {
                    marked: &interpreted.marked,
                    artifacts: &interpreted.artifacts,
                    objects: &interpreted.objects,
                },
                range.0..range.1,
            )?;
            ordered.push(crate::select::logical_copy(&interpreted.text, &pieces));
        }
        Some(ordered.join("\n"))
    }
}

/// Which of §7.6.4.1's operations an [`crate::Edit`] is.
///
/// The one place a host's vocabulary is mapped onto the standard's, so that the two enums stay
/// separate: `Edit` says what a person did in a viewer's terms and
/// [`pdf_model::restriction::Operation`] says what a clause restricts, and folding them into one
/// type would make every future edit a claim about which permission covers it.
fn operation_of(edit: &crate::command::Edit) -> pdf_model::restriction::Operation {
    match edit {
        // §12.7.5.3's file-select control is a form field being filled in, whatever the value
        // happens to be: Table 22 bit 9 permits "[f]ill in existing interactive form fields
        // (including signature fields)", and the pathname is the field's own text.
        crate::command::Edit::SetField { .. } | crate::command::Edit::ChooseFile { .. } => {
            pdf_model::restriction::Operation::FillInForm
        }
        // §12.5.6.6's annotation and the text inside it are both annotating, which Table 22's own
        // wording separates from filling in a form: bit 6 is "[a]dd or modify text annotations,
        // fill in interactive form fields", and bit 9 permits filling alone. So an edit to a free
        // text annotation's `/Contents` is Annotate and not FillInForm, whatever it resembles at a
        // keyboard.
        //
        // §12.5.6.15's annotation carries its file, so a file put on a page is an annotation
        // added — bit 6 too — while a file put in §7.7.4's tree is bit 4's residual, as is
        // taking one out. `AttachHome` carries the argument. ADR 0814.
        crate::command::Edit::Markup { .. }
        | crate::command::Edit::FreeText { .. }
        | crate::command::Edit::SetFreeText { .. }
        | crate::command::Edit::SetNoteText { .. }
        | crate::command::Edit::Attach {
            home: crate::command::AttachHome::Page { .. },
            ..
        } => pdf_model::restriction::Operation::Annotate,
        crate::command::Edit::Attach {
            home: crate::command::AttachHome::Document,
            ..
        }
        | crate::command::Edit::Detach { .. } => pdf_model::restriction::Operation::Modify,
    }
}

/// The sentence for an edit that would change nothing, said before the log is touched: an entry
/// that did nothing would be an undo step over nothing.
fn changes_nothing(open: &Open, edit: &crate::command::Edit) -> Option<String> {
    let note = match edit {
        crate::command::Edit::Attach { name, .. }
            if open.view.attachment_named(&open.document, name) =>
        {
            format!(
                "a file is already embedded under the name {name:?}, and a name tree's \
                     keys shall not overlap (ISO 32000-2 §7.9.6) — nothing was attached"
            )
        }
        // Table 231 bit 21 is what makes a pathname a *value*; without it a file-select
        // control is an ordinary text field and the bytes behind the path belong to nothing.
        // Said rather than applied as text, which would be the wrong value under the right
        // name (trap 5).
        crate::command::Edit::ChooseFile { field, .. }
            if !pdf_model::view::is_file_select(&open.document, field) =>
        {
            format!(
                "field {field:?} does not set Table 231 bit 21 FileSelect, so it is not a \
                     file-select control (ISO 32000-2 §12.7.5.3) — no file was chosen for it"
            )
        }
        // §12.5.6.4's note is the one annotation whose text a window retypes; any other is
        // said rather than logged as an entry that changed nothing (ADR 1726).
        crate::command::Edit::SetNoteText { annotation, .. }
            if !pdf_model::popup::retypable(&open.document, *annotation) =>
        {
            format!(
                "annotation {} {} is not a text note whose own text a popup window shows \
                     (ISO 32000-2 §12.5.6.4) — nothing was retyped",
                annotation.number, annotation.generation
            )
        }
        crate::command::Edit::Detach { name }
            if !open.view.attachment_named(&open.document, name) =>
        {
            format!(
                "this document's /EmbeddedFiles tree names no file called {name:?} — \
                     nothing was detached"
            )
        }
        _ => return None,
    };
    Some(note)
}

/// Which field an [`crate::Edit`] names, where it names one.
///
/// §12.7.5.5's signature field lock is the one restriction that is about a *field* rather than
/// about the document, so the question `operation_of` answers — which permission covers this
/// verb — is not enough on its own to ask it. `None` for every edit whose subject is an
/// annotation, which §12.7.4.2 gives no name to.
fn field_of(edit: &crate::command::Edit) -> Option<&str> {
    match edit {
        crate::command::Edit::SetField { field, .. }
        | crate::command::Edit::ChooseFile { field, .. } => Some(field),
        crate::command::Edit::Markup { .. }
        | crate::command::Edit::FreeText { .. }
        | crate::command::Edit::SetFreeText { .. }
        | crate::command::Edit::SetNoteText { .. }
        | crate::command::Edit::Attach { .. }
        | crate::command::Edit::Detach { .. } => None,
    }
}

/// Which annotation an [`crate::Edit`] names, where it names one.
///
/// [`field_of`]'s counterpart, and it exists for the same shape one clause over: §12.5.3's Table
/// 167 bit 10 restricts **this annotation** rather than the document, so the verb cannot answer it
/// either. `None` for every edit whose subject is a field or an annotation that does not exist yet
/// — a restriction on changing an annotation's contents has nothing to say about creating one, and
/// the flag is read off an object the file already holds.
fn annotation_of(edit: &crate::command::Edit) -> Option<ObjectId> {
    match edit {
        crate::command::Edit::SetFreeText { annotation, .. }
        | crate::command::Edit::SetNoteText { annotation, .. } => Some(*annotation),
        crate::command::Edit::SetField { .. }
        | crate::command::Edit::ChooseFile { .. }
        | crate::command::Edit::Markup { .. }
        | crate::command::Edit::FreeText { .. }
        | crate::command::Edit::Attach { .. }
        | crate::command::Edit::Detach { .. } => None,
    }
}

/// Commits what was typed into one field of a document, and says what changed.
///
/// The unsaved mark first, for [`commit`]'s reason, then the refusal's sentence, which is the
/// alert Adobe's library raises at this moment: a value a script refused goes back to what the
/// field showed, and a person who saw it go back is told why.
pub(super) fn commit_typed(id: DocumentId, open: &mut Open, field: &str, events: &mut Vec<Event>) {
    let before = open.dirty();
    let Some(committed) = open.commit_field(field) else {
        return;
    };
    if open.dirty() != before {
        events.push(Event::Dirty {
            document: id,
            dirty: open.dirty(),
        });
    }
    if let pdf_model::view::Committed::Refused(sentence) = committed {
        events.push(Event::Reported {
            document: id,
            page: None,
            notes: vec![sentence],
        });
    }
}

/// The fully qualified name of the field a widget belongs to.
///
/// §12.7.4.1 lets one field own several widgets, and the commit is the field's.
pub(super) fn field_of_widget(document: &pdf_syntax::Document, widget: ObjectId) -> Option<String> {
    pdf_model::view::widgets_by_field_name(document)
        .into_iter()
        .find(|(_, widgets)| widgets.contains(&widget))
        .map(|(name, _)| name)
}

/// Says a refused keystroke in place of logging it, and answers whether it did.
fn said_instead(
    id: DocumentId,
    open: &Open,
    done: &crate::open::Done,
    events: &mut Vec<Event>,
) -> bool {
    let Some(sentence) = refused_keystroke(open, done) else {
        return false;
    };
    events.push(Event::Reported {
        document: id,
        page: None,
        notes: vec![sentence],
    });
    true
}

/// The sentence for characters Table 199's `/K` refused, or `None` where it took them.
///
/// The table's entry "may check the added text for validity and reject" it, and a rejected
/// character leaves the field as it was — which a person whose key did nothing visible has been
/// told nothing about. Adobe answers with a beep; this program says it (ADR 1592). Asked of a
/// copy of the view, so that the judgement is the model's own: a field no widget would take the
/// characters into, whose one-call keystroke script refuses them in its typing form, is the case.
/// A read-only field takes nothing either, and its script is asked so that only the script's
/// refusal is said.
fn refused_keystroke(open: &Open, done: &crate::open::Done) -> Option<String> {
    let crate::open::Done::SetField {
        field,
        value: value @ pdf_model::view::Entered::Text(typed),
    } = done
    else {
        return None;
    };
    let widget = *pdf_model::view::widgets_by_field_name(&open.document)
        .get(field)?
        .first()?;
    let object = open.document.get(widget);
    let dictionary = object.as_dict()?;
    let pdf_model::aform::site::Site::Library(call) = pdf_model::aform::site::of_widget(
        &open.document,
        dictionary,
        pdf_model::aform::Trigger::Keystroke,
    ) else {
        return None;
    };
    // Tried only for a one-call script, which runs in this tree without the runner: a script the
    // runner runs would run twice for one keystroke, once here on the copy (ADR 1787).
    let mut trial = open.view.clone();
    if trial.set_field(&open.document, field, value) > 0 {
        return None;
    }
    let kept = open
        .view
        .field_value(&open.document, field)
        .map(|shown| shown.text)
        .unwrap_or_default();
    let event = pdf_model::aform::Keystroke {
        value: &kept,
        change: typed,
        selection: (0, kept.len()),
        will_commit: false,
    };
    matches!(
        call.keystroke(&event),
        Ok(pdf_model::aform::Keyed::Rejected { .. })
    )
    .then(|| {
        format!(
            "{field}: the field's keystroke script {} refused {typed:?}, and the field still \
             reads {kept:?}",
            call.function.name()
        )
    })
}

/// Adds one resolved edit to a document's log and says what changed.
///
/// The two events an edit can cause, in the order a host wants them: the unsaved mark first,
/// because it is the state, and the attachment list second, because it is a consequence of it.
pub(super) fn commit(
    id: DocumentId,
    open: &mut Open,
    done: crate::open::Done,
    events: &mut Vec<Event>,
) {
    let committed = open.commit(done);
    if committed.dirty_changed {
        events.push(Event::Dirty {
            document: id,
            dirty: open.dirty(),
        });
    }
    if committed.attachments {
        events.push(Event::AttachmentsChanged { document: id });
    }
}
