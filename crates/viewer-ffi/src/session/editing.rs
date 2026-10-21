//! Selection, form edits, annotations, attachments and saving.

use super::Session;
use crate::events::Events;
use crate::shapes::Quads;
use crate::status::Status;
use viewer_core::{Answer, Command, Edit, Query};

impl Session {
    /// The selected text, as the pages read back.
    ///
    /// **Pages**, since a drag across Table 29's continuous arrangements may cross a page
    /// boundary: what comes back is then each page's part in page order, joined by a newline
    /// (ADR 0444). Owned here whichever it is, because nothing of the viewer may cross this
    /// boundary borrowed — and the copy is the one the caller was going to make anyway.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn selection_text(&self) -> Result<String, Status> {
        match self.viewer.query(Query::Selection) {
            Answer::Selected(selected) => Ok(selected.text.into_owned()),
            _ => Err(Status::NoAnswer),
        }
    }

    /// The text the last [`Self::copy`] released, and which of §14.8.2.5's two orders it is in.
    ///
    /// **The copy's own text, and nothing a copy was not asked for.** Table 22's bit 5 restricts
    /// taking text out of the document, so the text a caller puts on its clipboard arrives only
    /// through [`viewer_core::Command::Copy`], where the reader's level is asked as it is for
    /// every other operation; a readback answered from the selection would hand it over at every
    /// level, `on` included. It is also the one string [`viewer_core::Event::Copied`] carries —
    /// §14.8.2.3's division after a soft hyphen undone (ADR 1971), §14.8.2.5.1's logical order
    /// where the structure tree gave one — and the order is [`viewer_host::copied`]'s choice, the
    /// one the three windowed hosts put on their clipboards (ADR 0519), so a caller reading this
    /// and a caller reading the event get the same answer (ADR 2087).
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where the last copy released nothing: none was asked yet, it was
    /// refused, its question is unanswered or was declined, or nothing was selected — the last
    /// because a copy with nothing to copy must not empty a caller's clipboard.
    pub fn copy_text(&self) -> Result<(String, viewer_host::ContentOrder), Status> {
        self.copied
            .as_ref()
            .map(|copied| (copied.text.clone(), copied.order))
            .ok_or(Status::NoAnswer)
    }

    /// What the field at a point is called: §12.7.4.2's qualified name and §14.9.3's shown one.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no field is there.
    pub fn field_at(&self, at: (f32, f32)) -> Result<(String, String), Status> {
        match self.viewer.query(Query::FieldAt(at)) {
            Answer::Field { name, .. } => Ok((name.qualified.clone(), name.shown().to_owned())),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Where the caret sits in the field at a point, given how far into the value it is.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] in the cases [`viewer_core::Query::Caret`] answers `None` in.
    /// Answered as `[from_x, from_y, to_x, to_y]`: a caret has no width, so what crosses is two
    /// points, and a flat array is what the entry point writes out anyway.
    pub fn caret(&self, at: (f32, f32), offset: usize) -> Result<[f32; 4], Status> {
        match self.viewer.query(Query::Caret { at, offset }) {
            Answer::Caret { from, to } => Ok([from.0, from.1, to.0, to.1]),
            _ => Err(Status::NoAnswer),
        }
    }

    /// How far into a field's value a point inside it is, in bytes — the caret's inverse.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] in the cases [`Self::caret`] answers it in.
    pub fn offset(&self, at: (f32, f32), point: (f32, f32)) -> Result<usize, Status> {
        match self.viewer.query(Query::Offset { at, point }) {
            Answer::Offset(offset) => Ok(offset),
            _ => Err(Status::NoAnswer),
        }
    }

    /// The shapes covering a byte range of a field's value, one per line it touches.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] in the cases [`Self::caret`] answers it in.
    pub fn field_selection(&self, at: (f32, f32), from: usize, to: usize) -> Result<Quads, Status> {
        match self.viewer.query(Query::FieldSelection { at, from, to }) {
            Answer::FieldSelection(quads) => Ok(Quads::new(quads)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.5.6.6: the free text annotation **this session added** at a point, and what it says.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no such annotation covers the point — including for every free
    /// text annotation the file itself states, which nothing in this vocabulary can change.
    pub fn free_text_at(&self, at: (f32, f32)) -> Result<((u32, u16), String), Status> {
        match self.viewer.query(Query::FreeTextAt { at }) {
            Answer::FreeText { annotation, text } => {
                Ok(((annotation.number, annotation.generation), text))
            }
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.7.4: put a value into a field, by the name §12.7.4.2 gives it.
    #[must_use]
    pub fn set_field(&mut self, field: String, value: pdf_model::view::Entered) -> Events {
        self.handle(Command::Edit(Edit::SetField { field, value }))
    }

    /// §12.5.6.10: mark up what is selected, in one of the clause's four ways.
    #[must_use]
    pub fn markup(&mut self, kind: pdf_model::view::Markup, colour: [f32; 3]) -> Events {
        self.handle(Command::Edit(Edit::Markup { kind, colour }))
    }

    /// §12.5.6.6: put an empty free text annotation over a rectangle a person drew.
    #[must_use]
    pub fn free_text(&mut self, from: (f32, f32), to: (f32, f32), colour: [f32; 3]) -> Events {
        self.handle(Command::Edit(Edit::FreeText { from, to, colour }))
    }

    /// §12.5.6.6: say what a free text annotation this session added says.
    #[must_use]
    pub fn set_free_text(&mut self, number: u32, generation: u16, text: String) -> Events {
        self.handle(Command::Edit(Edit::SetFreeText {
            annotation: pdf_syntax::ObjectId::new(number, generation),
            text,
        }))
    }

    /// §12.5.6.4: say what a text note says, as a person typing into its window does (ADR 1726).
    #[must_use]
    pub fn set_note_text(&mut self, number: u32, generation: u16, text: String) -> Events {
        self.handle(Command::Edit(Edit::SetNoteText {
            annotation: pdf_syntax::ObjectId::new(number, generation),
            text,
        }))
    }

    /// §7.11.4: put a file into the document, in one of §7.11.4.1's two homes.
    #[must_use]
    pub fn attach(
        &mut self,
        bytes: Vec<u8>,
        name: String,
        description: Option<String>,
        mime: Option<String>,
        home: viewer_core::AttachHome,
    ) -> Events {
        self.handle(Command::Edit(Edit::Attach {
            bytes: bytes.into(),
            name,
            description,
            mime,
            home,
        }))
    }

    /// §7.11.4: take an embedded file out of the document, by the name it is filed under.
    #[must_use]
    pub fn detach(&mut self, name: String) -> Events {
        self.handle(Command::Edit(Edit::Detach { name }))
    }

    /// Undo the last edit.
    #[must_use]
    pub fn undo(&mut self) -> Events {
        self.handle(Command::Undo)
    }

    /// Redo the last undone edit.
    #[must_use]
    pub fn redo(&mut self) -> Events {
        self.handle(Command::Redo)
    }

    /// Whether anything has been edited since the document opened.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn dirty(&self) -> Result<bool, Status> {
        match self.viewer.query(Query::Dirty) {
            Answer::Dirty(dirty) => Ok(dirty),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Write §7.5.6's incremental update for everything the log holds.
    ///
    /// The bytes come back on [`viewer_core::Event::Saved`], because rule 2 gives this crate no
    /// filesystem and where a file lands is the caller's decision.
    #[must_use]
    pub fn save(&mut self) -> Events {
        self.handle(Command::Save)
    }

    /// §7.11.4: take an embedded file's bytes out of the document.
    #[must_use]
    pub fn extract(&mut self, name: String) -> Events {
        self.handle(Command::Extract { name })
    }

    /// The bytes the viewer asked for with [`viewer_core::Event::NeedsFile`], or a refusal.
    #[must_use]
    pub fn supply(&mut self, purpose: viewer_core::Purpose, bytes: Option<Vec<u8>>) -> Events {
        self.handle(Command::Supply { purpose, bytes })
    }

    /// §7.6.4.2's bit 5 asked as an operation: a person pressed copy.
    ///
    /// What a grant releases is the event's text, and [`Self::copy_text`] reads it back.
    #[must_use]
    pub fn copy(&mut self) -> Events {
        self.handle(Command::Copy)
    }

    /// §12.7.4.3: the caller's own control lost the keyboard, or Enter in a single-line field.
    #[must_use]
    pub fn commit_field(&mut self, field: String) -> Events {
        self.handle(Command::CommitField { field })
    }
}
