//! §12.7.4.3's text fields typed into from the keyboard, and §12.9's points put down, across the
//! confinement.
//!
//! The smallest form a person can fill without a pointer: Tab walks §12.5.1's order, a text field
//! the walk lands on takes characters, Backspace takes the last one back, Enter in a single-line
//! field commits, and Control and S writes §7.5.6's update beside the file. Every value is the
//! worker's — the host reads it back through `Query::Fields` before each keystroke and sends the
//! whole of it, the way the flagship does (ADR 0197) — so a character Table 199's `/K` refuses is
//! one this window never shows, and a refusal at the commit arrives as the worker's sentence.
//!
//! The scope ADR 0713 drew for this window had no form controls in it; ADR 1592 adds this much,
//! because a commit the confined worker holds the scripts for is a commit this boundary has to be
//! able to carry, and a window that cannot type cannot show it.
//!
//! And §12.9's measuring, which `m` turns on as in the other three windows: a press is a point, and
//! what the worker makes of the path — §12.10's position among it (ADR 1593) — is printed.

use viewer_confined::Reply;
use viewer_core::{Command, Edit, Entered, FocusMove, FormField, Query};
use viewer_host::form::{ControlKind, control_kind};
use winit::keyboard::{Key, NamedKey};

use crate::Host;

/// What a key did while this window was asked about a field.
pub(crate) enum Typed {
    /// The key was a field's, and nothing else is to see it.
    Taken,
    /// The key is the page's.
    Passed,
}

impl Host {
    /// One key, offered to the field first. Answers whether the page may still have it.
    ///
    /// Tab is always the walk's, whatever has the keyboard; the rest are a field's only while one
    /// is being typed into, which is what keeps `q` a character there and a quit on the page.
    pub(crate) fn field_key(&mut self, key: &Key<&str>) -> Typed {
        if let Key::Named(NamedKey::Tab) = key {
            let direction = if self.modifiers.shift_key() {
                FocusMove::Previous
            } else {
                FocusMove::Next
            };
            self.dispatch(&Command::Focused(direction));
            self.typing = self.focused_text_field();
            if let Some(field) = &self.typing {
                eprintln!("note: typing into the field {field}");
            }
            return Typed::Taken;
        }
        // Control and S is the save, from the page or from a field.
        if self.modifiers.control_key() && matches!(key, Key::Character("s" | "S")) {
            self.dispatch(&Command::Save);
            return Typed::Taken;
        }
        let Some(field) = self.typing.clone() else {
            return Typed::Passed;
        };
        if self.modifiers.control_key() {
            // Every other chord is consumed while a field has the keyboard: a zoom under somebody
            // typing is the surprise this state exists to prevent.
            return Typed::Taken;
        }
        let current = self.value_of(&field).unwrap_or_default();
        let next = match key {
            // Escape gives the keyboard back to the page; the page's own Escape is the abort, and
            // a person leaving a field has not asked to end the worker.
            Key::Named(NamedKey::Escape) => {
                self.typing = None;
                eprintln!("note: the keyboard is back on the page");
                return Typed::Taken;
            }
            // Table 231 bit 13 clear restricts the text "to a single line", so Enter is no
            // character and commits what was typed (ADR 1592).
            Key::Named(NamedKey::Enter) => {
                self.typing = None;
                self.dispatch(&Command::CommitField { field });
                eprintln!("note: the keyboard is back on the page");
                return Typed::Taken;
            }
            Key::Named(NamedKey::Backspace) => {
                let mut shorter = current.clone();
                shorter.pop();
                shorter
            }
            Key::Named(NamedKey::Space) => format!("{current} "),
            Key::Character(text) if !text.is_empty() => format!("{current}{text}"),
            _ => return Typed::Taken,
        };
        if next != current {
            self.dispatch(&Command::Edit(Edit::SetField {
                field,
                value: Entered::Text(next),
            }));
        }
        Typed::Taken
    }

    /// The qualified name of the single-line text field §12.5.1's focus is on, where it is one a
    /// person may type into.
    ///
    /// A multiline field, a password field — whose value answers as bullets, so a keystroke read
    /// back would append to the bullets (ADR 0247) — a file-select control and a read-only field
    /// are not typed into here, and a note says which.
    fn focused_text_field(&mut self) -> Option<String> {
        let confined = self.confined.as_mut()?;
        let Ok(Reply::Focus { object, .. }) = confined.query(Query::Focus) else {
            return None;
        };
        let field = self.fields()?.into_iter().find(|field| {
            field
                .widgets
                .iter()
                .any(|widget| widget.annotation == object)
        })?;
        let single_line = matches!(
            control_kind(&field.control),
            ControlKind::Entry {
                multiline: false,
                password: false,
                file_select: false,
                ..
            }
        );
        if !single_line || field.read_only {
            eprintln!(
                "note: {} is not a single-line text field this window types into",
                field.name.shown()
            );
            return None;
        }
        Some(field.name.qualified)
    }

    /// What a field says now, read from the worker.
    fn value_of(&mut self, name: &str) -> Option<String> {
        self.fields()?
            .into_iter()
            .find(|field| field.name.qualified == name)?
            .value
            .map(|shown| shown.text)
    }

    /// The fields on the pages the worker is showing.
    fn fields(&mut self) -> Option<Vec<FormField>> {
        match self.confined.as_mut()?.query(Query::Fields) {
            Ok(Reply::Fields(fields)) => Some(fields),
            _ => None,
        }
    }

    /// A press on the page while measuring is on: one more point, and the sentence the other
    /// windows say about the path, printed.
    pub(crate) fn measure_at(&mut self) {
        if !self.measuring.point(self.cursor) {
            return;
        }
        let points = self.measuring.points().to_vec();
        let Some(confined) = self.confined.as_mut() else {
            return;
        };
        let (traced, located) = match confined.query(Query::Measure(&points)) {
            Ok(Reply::Measured { traced, located }) => (Some(*traced), located),
            _ => (None, None),
        };
        eprintln!(
            "note: {}",
            viewer_host::measuring::said(points.len(), traced.as_ref(), located.as_ref())
        );
    }

    /// Writes §7.5.6's update beside the file, as the flagship does.
    pub(crate) fn write_saved(&self, bytes: &[u8]) {
        let path = self.path.with_extension("edited.pdf");
        match std::fs::write(&path, bytes) {
            Ok(()) => eprintln!("saved {} bytes to {}", bytes.len(), path.display()),
            Err(error) => eprintln!("note: cannot write {}: {error}", path.display()),
        }
    }
}
