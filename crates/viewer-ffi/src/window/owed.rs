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

use super::repainted::Repainted;

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
    /// The page area is to be drawn again from `quorra_window_frame_count` and its frames, as far
    /// as `quorra_owed_repaint` says it changed (ADR 2168).
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
    /// Read effective chrome and apply the optional one-shot `quorra_owed_chrome_panel` request.
    Chrome = 8,
    /// Table 147's `/FitWindow` or `/CenterWindow` is owed at this frame.
    Place = 9,
    /// The set of controls placed over the page changed: rebuilt from `quorra_window_fields_read`
    /// rather than moved, so a control a person is typing into keeps the keyboard while it stays.
    Controls = 10,
    /// §12.5.6.14's open popup windows changed, or one moved: `quorra_window_popups_read`.
    Popups = 11,
    /// The strip of open documents changed — one added, one closed, or the one in front moved:
    /// `quorra_window_document_count` and the calls beside it.
    Strip = 12,
    /// ISO 19005's chooser: `quorra_owed_text` is its title, a line feed, and what it says under
    /// the title; `quorra_owed_archive` the target and the profile it starts on, as indices into
    /// `quorra_archive_target`'s list and `quorra_archive_profile`'s; answered with
    /// `quorra_window_archive` (ADRs 1966, 2121).
    Archive = 13,
    /// A script's `app.alert` or `app.response` for `quorra_owed_document`: `quorra_owed_text` is
    /// the dialogue's title, `quorra_owed_script_question` and `quorra_owed_script_text` the rest;
    /// answered with `quorra_window_answer_script` (ADRs 1628, 2121).
    Script = 14,
    /// The script question for `quorra_owed_document` was withdrawn unanswered: its dialogue is to
    /// close, and an answer reaches nothing (ADR 1643).
    ScriptWithdrawn = 15,
    /// §7.6.4.2's bit 3 let a print through: `quorra_owed_print` gives the pages and whether bit 12
    /// limits the quality, `quorra_owed_text` the sentence; ended with `quorra_window_print_finish`
    /// whether the job ran or the person cancelled (ADRs 1180, 2121).
    Print = 16,
}

impl OwedKind {
    /// How many kinds this build defines: `QUORRA_OWED_KIND_COUNT`.
    pub const COUNT: u32 = 17;

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
            10 => Self::Controls,
            11 => Self::Popups,
            12 => Self::Strip,
            13 => Self::Archive,
            14 => Self::Script,
            15 => Self::ScriptWithdrawn,
            16 => Self::Print,
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
            Self::Controls => "controls\0",
            Self::Popups => "popups\0",
            Self::Strip => "strip\0",
            Self::Archive => "archive\0",
            Self::Script => "script\0",
            Self::ScriptWithdrawn => "script withdrawn\0",
            Self::Print => "print\0",
        }
    }
}

/// One thing the window owes its host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Owed {
    /// A sentence for the status line.
    Said(String),
    /// The page area drawn again, and what of it changed.
    Repaint(Repainted),
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
    Chrome(Option<i32>),
    /// Table 147's placement.
    Place,
    /// The controls over the page rebuilt.
    Controls,
    /// The popups rebuilt.
    Popups,
    /// The strip of open documents read again.
    Strip,
    /// ISO 19005's chooser, on the target and the profile it starts on.
    Archive {
        /// Which of `pdf_archive::Target::ALL` it starts on.
        target: usize,
        /// Which of the shipped profiles it starts on.
        profile: usize,
        /// Its title, a line feed, and what it says under the title.
        words: String,
    },
    /// A script's question.
    Script {
        /// The document whose script asks.
        document: DocumentId,
        /// The dialogue's title, naming that document.
        title: String,
        /// What the script asked.
        question: viewer_core::ScriptQuestion,
    },
    /// A script's question withdrawn.
    ScriptWithdrawn {
        /// The document whose question it was.
        document: DocumentId,
    },
    /// A print let through.
    Print {
        /// How many pages the document has.
        pages: usize,
        /// Whether §7.6.4.2's bit 12 limits the quality.
        degraded: bool,
        /// What the window says about it.
        words: String,
    },
}

impl Owed {
    /// Which kind of record this is.
    #[must_use]
    pub(crate) const fn kind(&self) -> OwedKind {
        match self {
            Self::Said(_) => OwedKind::Said,
            Self::Repaint(_) => OwedKind::Repaint,
            Self::Title(_) => OwedKind::Title,
            Self::Password { .. } => OwedKind::Password,
            Self::Question(_) => OwedKind::Question,
            Self::Clipboard(_) => OwedKind::Clipboard,
            Self::Beep => OwedKind::Beep,
            Self::Panels => OwedKind::Panels,
            Self::Chrome(_) => OwedKind::Chrome,
            Self::Place => OwedKind::Place,
            Self::Controls => OwedKind::Controls,
            Self::Popups => OwedKind::Popups,
            Self::Strip => OwedKind::Strip,
            Self::Archive { .. } => OwedKind::Archive,
            Self::Script { .. } => OwedKind::Script,
            Self::ScriptWithdrawn { .. } => OwedKind::ScriptWithdrawn,
            Self::Print { .. } => OwedKind::Print,
        }
    }

    /// The record's text, for the kinds that carry one.
    #[must_use]
    fn text(&self) -> Option<&str> {
        match self {
            Self::Said(text)
            | Self::Title(text)
            | Self::Question(text)
            | Self::Clipboard(text)
            | Self::Password { words: text, .. }
            | Self::Script { title: text, .. }
            | Self::Print { words: text, .. }
            | Self::Archive { words: text, .. } => Some(text),
            Self::ScriptWithdrawn { .. }
            | Self::Repaint(_)
            | Self::Beep
            | Self::Panels
            | Self::Chrome(_)
            | Self::Place
            | Self::Controls
            | Self::Popups
            | Self::Strip => None,
        }
    }
}

/// What the window owed, in the order it came to owe it, as `quorra_window_take_owed` hands it
/// over.
///
/// **A record that only says "again" is kept once**: the page area drawn again twice in one call
/// is drawn once, where the first stood and saying what either changed, and a second title
/// replaces the first where it stands, so a caller acting on every record in order never draws a
/// page or sets a title it is about to replace.
#[derive(Debug, Default)]
pub struct Owing {
    /// The records.
    records: Vec<Owed>,
}

impl Owing {
    /// One more record, kept once where its kind says only "again".
    pub(crate) fn owe(&mut self, owed: Owed) {
        match owed {
            Owed::Repaint(later) => {
                match self.records.iter_mut().find_map(|record| match record {
                    Owed::Repaint(earlier) => Some(earlier),
                    _ => None,
                }) {
                    Some(earlier) => earlier.merge(later),
                    None => self.records.push(Owed::Repaint(later)),
                }
            }
            Owed::Panels | Owed::Place | Owed::Controls | Owed::Popups | Owed::Strip
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

    /// A real document activation supersedes the previous document's pending chrome commands.
    pub(crate) fn forget_chrome(&mut self) {
        self.records
            .retain(|record| !matches!(record, Owed::Chrome(_)));
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

    /// A chrome notification's one-shot panel request: -1 closes, a tab index opens it.
    ///
    /// # Errors
    ///
    /// `NoAnswer` preserves the reader's pane choice; `WrongKind` names another record,
    /// and `OutOfRange` is past the batch. Reading does not consume the immutable record.
    pub fn chrome_panel(&self, index: usize) -> Result<i32, crate::Status> {
        match self.records.get(index).ok_or(crate::Status::OutOfRange)? {
            Owed::Chrome(Some(panel)) => Ok(*panel),
            Owed::Chrome(None) => Err(crate::Status::NoAnswer),
            _ => Err(crate::Status::WrongKind),
        }
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

    /// The document the record at `index` is about: §7.6.4.1's prompt's, or a script's.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`](crate::Status::OutOfRange) past the end, and
    /// [`Status::WrongKind`](crate::Status::WrongKind) for a kind that names no document.
    pub fn document(&self, index: usize) -> Result<u64, crate::Status> {
        match self.records.get(index) {
            None => Err(crate::Status::OutOfRange),
            Some(
                Owed::Password { document, .. }
                | Owed::Script { document, .. }
                | Owed::ScriptWithdrawn { document },
            ) => Ok(document.0),
            Some(_) => Err(crate::Status::WrongKind),
        }
    }

    /// The target and the profile ISO 19005's chooser at `index` starts on.
    ///
    /// # Errors
    ///
    /// As [`Self::document`], for a kind that is not the chooser.
    pub fn archive(&self, index: usize) -> Result<(usize, usize), crate::Status> {
        match self.records.get(index) {
            None => Err(crate::Status::OutOfRange),
            Some(Owed::Archive {
                target, profile, ..
            }) => Ok((*target, *profile)),
            Some(_) => Err(crate::Status::WrongKind),
        }
    }

    /// The script question at `index`.
    ///
    /// # Errors
    ///
    /// As [`Self::document`], for a kind that is not a script's question.
    pub fn script(&self, index: usize) -> Result<&viewer_core::ScriptQuestion, crate::Status> {
        match self.records.get(index) {
            None => Err(crate::Status::OutOfRange),
            Some(Owed::Script { question, .. }) => Ok(question),
            Some(_) => Err(crate::Status::WrongKind),
        }
    }

    /// What the repaint at `index` changed.
    ///
    /// # Errors
    ///
    /// As [`Self::document`], for a kind that is not a repaint.
    pub fn repaint(&self, index: usize) -> Result<&Repainted, crate::Status> {
        match self.records.get(index) {
            None => Err(crate::Status::OutOfRange),
            Some(Owed::Repaint(repainted)) => Ok(repainted),
            Some(_) => Err(crate::Status::WrongKind),
        }
    }

    /// The pages and the quality limit of the print let through at `index`.
    ///
    /// # Errors
    ///
    /// As [`Self::document`], for a kind that is not a print.
    pub fn print(&self, index: usize) -> Result<(usize, bool), crate::Status> {
        match self.records.get(index) {
            None => Err(crate::Status::OutOfRange),
            Some(Owed::Print {
                pages, degraded, ..
            }) => Ok((*pages, *degraded)),
            Some(_) => Err(crate::Status::WrongKind),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Owed, OwedKind, Owing, Repainted};

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
        owing.owe(Owed::Repaint(Repainted::default()));
        owing.owe(Owed::Said("a sentence".to_owned()));
        owing.owe(Owed::Repaint(Repainted::default()));
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
    #[test]
    fn chrome_commands_survive_refresh_and_are_consumed_with_the_batch() {
        let mut owing = Owing::default();
        owing.owe(Owed::Chrome(Some(1)));
        owing.owe(Owed::Chrome(None));
        owing.owe(Owed::Chrome(Some(-1)));
        let taken = owing.take();
        assert_eq!(taken.chrome_panel(0), Ok(1));
        assert_eq!(taken.chrome_panel(1), Err(crate::Status::NoAnswer));
        assert_eq!(taken.chrome_panel(2), Ok(-1));
        assert_eq!(taken.chrome_panel(0), Ok(1));
        assert!(owing.is_empty());
        owing.owe(Owed::Chrome(None));
        assert_eq!(owing.take().chrome_panel(0), Err(crate::Status::NoAnswer));
    }
}
