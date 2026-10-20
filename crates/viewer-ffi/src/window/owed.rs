//! What a window owes its host since the host last asked: `quorra_owed *`, an owned batch the
//! caller reads with indexed accessors and frees, in the shape every other batch in this ABI has
//! (ADR 0247 section 2, ADR 2088).
//!
//! **The batch is the window's half of [`viewer_host::native_window::NativeWindow`]'s required
//! methods**: each method a toolkit answers its own way — a sentence for the status line, the page
//! area drawn again, the title, §7.6.4.1's prompt, the one question at `CLAUDE.md`'s *ask* level,
//! the clipboard — leaves one record here, and the caller acts on the records after the call that
//! produced them returns. Nothing calls back into the caller, so nothing the caller does in answer
//! can reach the window while the window is still borrowed.

use viewer_core::DocumentId;

/// What one record asks of the host: `QUORRA_OWED_*`.
///
/// **Numbered by this ABI and counted**, as an event kind is: a caller switches on the number, a
/// kind it does not know is named by `quorra_owed_kind_name`, and `QUORRA_OWED_KIND_COUNT` against
/// `quorra_owed_kind_count` says at startup that one was added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum OwedKind {
    /// A sentence for where the person reads them: `quorra_owed_text`.
    Said = 0,
    /// The page area is to be drawn again from `quorra_window_frame_count` and its frames.
    Repaint = 1,
    /// The title bar is to say `quorra_owed_text`.
    Title = 2,
    /// §7.6.4.1's prompt for `quorra_owed_document`, worded as `quorra_owed_text` (the question,
    /// a line feed, then the attempt counted); answered with `quorra_window_password`.
    Password = 3,
    /// The one question at `CLAUDE.md`'s *ask* level, worded as `quorra_owed_text` (the reasons,
    /// an empty line, then the choice); answered with `quorra_window_answer`.
    Question = 4,
    /// `quorra_owed_text` is to be put on the platform's clipboard.
    Clipboard = 5,
    /// `app.beep`: the display's one system sound.
    Beep = 6,
    /// The panels are to be built again, because §7.11.4's list moved.
    Panels = 7,
    /// The window's chrome is to follow Table 29's `/PageMode` and §12.2's flags as they now stand.
    Chrome = 8,
    /// Table 147's `/FitWindow` or `/CenterWindow` is owed at this frame.
    Place = 9,
}

impl OwedKind {
    /// How many kinds this build defines: `QUORRA_OWED_KIND_COUNT`.
    pub const COUNT: u32 = 10;

    /// The number the header declares, as C sees it.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// The kind a number names, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Said,
            1 => Self::Repaint,
            2 => Self::Title,
            3 => Self::Password,
            4 => Self::Question,
            5 => Self::Clipboard,
            6 => Self::Beep,
            7 => Self::Panels,
            8 => Self::Chrome,
            9 => Self::Place,
            _ => return None,
        })
    }

    /// The kind's name, NUL-terminated for `quorra_owed_kind_name`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Said => "said\0",
            Self::Repaint => "repaint\0",
            Self::Title => "title\0",
            Self::Password => "password\0",
            Self::Question => "question\0",
            Self::Clipboard => "clipboard\0",
            Self::Beep => "beep\0",
            Self::Panels => "panels\0",
            Self::Chrome => "chrome\0",
            Self::Place => "place\0",
        }
    }
}

/// One thing the window owes its host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Owed {
    /// A sentence for the status line.
    Said(String),
    /// The page area drawn again.
    Repaint,
    /// The title bar's text.
    Title(String),
    /// §7.6.4.1's prompt, about one document.
    Password {
        /// The document the prompt is about.
        document: DocumentId,
        /// What the prompt says.
        words: String,
    },
    /// The one question at the *ask* level.
    Question(String),
    /// Text for the clipboard.
    Clipboard(String),
    /// The system sound.
    Beep,
    /// The panels built again.
    Panels,
    /// The chrome as Table 29 and §12.2 now permit it.
    Chrome,
    /// Table 147's placement.
    Place,
}

impl Owed {
    /// Which kind of record this is.
    #[must_use]
    pub(crate) const fn kind(&self) -> OwedKind {
        match self {
            Self::Said(_) => OwedKind::Said,
            Self::Repaint => OwedKind::Repaint,
            Self::Title(_) => OwedKind::Title,
            Self::Password { .. } => OwedKind::Password,
            Self::Question(_) => OwedKind::Question,
            Self::Clipboard(_) => OwedKind::Clipboard,
            Self::Beep => OwedKind::Beep,
            Self::Panels => OwedKind::Panels,
            Self::Chrome => OwedKind::Chrome,
            Self::Place => OwedKind::Place,
        }
    }

    /// The record's text, for the four kinds that carry one.
    #[must_use]
    fn text(&self) -> Option<&str> {
        match self {
            Self::Said(text)
            | Self::Title(text)
            | Self::Question(text)
            | Self::Clipboard(text)
            | Self::Password { words: text, .. } => Some(text),
            Self::Repaint | Self::Beep | Self::Panels | Self::Chrome | Self::Place => None,
        }
    }
}

/// What the window owed, in the order it came to owe it, as `quorra_window_take_owed` hands it
/// over.
///
/// **A record that only says "again" is kept once**: the page area drawn again twice in one call
/// is drawn once, and a second title replaces the first where it stands, so a caller acting on
/// every record in order never draws a page or sets a title it is about to replace.
#[derive(Debug, Default)]
pub struct Owing {
    /// The records.
    records: Vec<Owed>,
}

impl Owing {
    /// One more record, kept once where its kind says only "again".
    pub(crate) fn owe(&mut self, owed: Owed) {
        match owed {
            Owed::Repaint | Owed::Panels | Owed::Chrome | Owed::Place
                if self.records.contains(&owed) => {}
            Owed::Title(title) => {
                match self
                    .records
                    .iter_mut()
                    .find(|record| matches!(record, Owed::Title(_)))
                {
                    Some(record) => *record = Owed::Title(title),
                    None => self.records.push(Owed::Title(title)),
                }
            }
            owed => self.records.push(owed),
        }
    }

    /// Everything owed so far, leaving nothing owed.
    #[must_use]
    pub(crate) fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    /// How many records there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether nothing is owed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The kind of the record at `index`, or `None` past the end.
    #[must_use]
    pub fn kind(&self, index: usize) -> Option<OwedKind> {
        self.records.get(index).map(Owed::kind)
    }

    /// The text of the record at `index`.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`](crate::Status::OutOfRange) past the end, and
    /// [`Status::WrongKind`](crate::Status::WrongKind) for a kind that carries no text.
    pub fn text(&self, index: usize) -> Result<&str, crate::Status> {
        let record = self.records.get(index).ok_or(crate::Status::OutOfRange)?;
        record.text().ok_or(crate::Status::WrongKind)
    }

    /// The document the record at `index` is about: §7.6.4.1's prompt's.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`](crate::Status::OutOfRange) past the end, and
    /// [`Status::WrongKind`](crate::Status::WrongKind) for a kind that names no document.
    pub fn document(&self, index: usize) -> Result<u64, crate::Status> {
        match self.records.get(index) {
            None => Err(crate::Status::OutOfRange),
            Some(Owed::Password { document, .. }) => Ok(document.0),
            Some(_) => Err(crate::Status::WrongKind),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Owed, OwedKind, Owing};

    /// Every kind's number names it back, and the count is one past the last.
    #[test]
    fn every_kind_is_its_own_number_and_the_count_is_one_past_the_last() {
        let mut seen = 0;
        for code in 0..OwedKind::COUNT {
            let kind = OwedKind::from_code(code).expect("every number below the count is a kind");
            assert_eq!(kind.code(), code);
            assert_eq!(kind.name().matches('\0').count(), 1);
            seen += 1;
        }
        assert_eq!(seen, OwedKind::COUNT);
        assert_eq!(OwedKind::from_code(OwedKind::COUNT), None);
    }

    /// A repaint asked twice is one record, and a second title replaces the first where it stood.
    #[test]
    fn a_record_that_only_says_again_is_kept_once() {
        let mut owing = Owing::default();
        owing.owe(Owed::Title("one".to_owned()));
        owing.owe(Owed::Repaint);
        owing.owe(Owed::Said("a sentence".to_owned()));
        owing.owe(Owed::Repaint);
        owing.owe(Owed::Title("two".to_owned()));
        let taken = owing.take();
        assert!(owing.is_empty());
        assert_eq!(taken.len(), 3);
        assert_eq!(taken.kind(0), Some(OwedKind::Title));
        assert_eq!(taken.text(0), Ok("two"));
        assert_eq!(taken.kind(1), Some(OwedKind::Repaint));
        assert_eq!(taken.text(1), Err(crate::Status::WrongKind));
        assert_eq!(taken.text(2), Ok("a sentence"));
        assert_eq!(taken.document(2), Err(crate::Status::WrongKind));
        assert_eq!(taken.kind(3), None);
    }
}
