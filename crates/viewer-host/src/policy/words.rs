//! Everything a reader states before a window opens — the window's [`Settings`] and the reader's
//! other words — read by one function for the four binaries and set by number for a host with no
//! command line (ADR 2156).
//!
//! **One value rather than a pair every window keeps in step.** [`Settings::take`] reads the seven
//! settings words and [`ReaderWords::take`] the reader's nine, and both hold RFC 0008 section
//! 6.3's scripts level: each binary called the two parsers in turn and then copied the level from one
//! into the other, and the C ABI's window copied it back the other way. [`Words::take`] is the
//! two readings in the order every window asked them, and the level is written in both halves in
//! one place, so a window cannot open with its core at one level and its own questions at another.

use std::ffi::{OsStr, OsString};

use super::{
    INTERFACE_LANGUAGE, READER_NAME, READER_ORGANISATION, READER_TITLE, REFERENCE_FILES, Scripts,
    Settings, TRUST_ANCHORS,
};
use super::{Level, Policy};
use crate::{ReaderSupply, ReaderWords};

/// What a word [`Words::take`] took was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taken {
    /// One of the reader's words, with its value where it has one ([`ReaderWords::take`]).
    Reader,
    /// One of the settings words, named by its constant ([`Settings::take`]), so that a window
    /// with no level for one can refuse it by name.
    Setting(&'static str),
}

/// One of the reader's words that names a value, for a host that states it without a command line.
///
/// **The same reading as the word's**: [`Words::state`] hands the value to [`ReaderWords::take`]
/// after the word this names, so a directory that is not one is refused in the sentence a command
/// line gets, and a value is kept where the word would have kept it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReaderWord {
    /// [`TRUST_ANCHORS`]: a directory of PEM or DER certificates, RFC 5280 section 6.1.1's input
    /// (d) for §12.8.1's third question.
    TrustAnchors,
    /// [`REFERENCE_FILES`]: a directory of PDF files, §8.10.4's target documents.
    ReferenceFiles,
    /// [`READER_NAME`]: one more of Table 100's `Ind`.
    Name,
    /// [`READER_TITLE`]: one more of Table 100's `Ttl`.
    Title,
    /// [`READER_ORGANISATION`]: one more of Table 100's `Org`.
    Organisation,
    /// [`INTERFACE_LANGUAGE`]: §8.11.4.4's `Language` category, replacing one stated before.
    Language,
}

impl ReaderWord {
    /// The command-line word this states the value of.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::TrustAnchors => TRUST_ANCHORS,
            Self::ReferenceFiles => REFERENCE_FILES,
            Self::Name => READER_NAME,
            Self::Title => READER_TITLE,
            Self::Organisation => READER_ORGANISATION,
            Self::Language => INTERFACE_LANGUAGE,
        }
    }
}

/// What a reader said before a window opened: typed as words, or stated by a host one call at a
/// time.
///
/// The defaults are the words not having been typed: [`Settings`]' own and [`ReaderWords`]' own —
/// no anchor, no target document, nobody reading, no script run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Words {
    /// The levels and policies a window holds.
    settings: Settings,
    /// The reader's other words, which become the four commands sent before the first document.
    reader: ReaderWords,
}

impl Words {
    /// Takes `word` if it is one of the reader's words or one of the settings words, reading a
    /// value from `rest` where the word wants one, and answers which it was.
    ///
    /// `Ok(None)` where it is neither, which leaves it to the window's own parser.
    ///
    /// # Errors
    ///
    /// The sentence the word's own parser refuses it with: a value missing, a directory that is
    /// not one, a level its word does not take. Nothing changes.
    pub fn take<W: Into<OsString>>(
        &mut self,
        word: &OsStr,
        rest: &mut impl Iterator<Item = W>,
    ) -> Result<Option<Taken>, String> {
        if self.reader.take(word, rest)? {
            self.settings.scripts = self.reader.scripts;
            return Ok(Some(Taken::Reader));
        }
        Ok(self
            .settings
            .take(&word.to_string_lossy())?
            .map(Taken::Setting))
    }

    /// One of the reader's words with its value, as [`Self::take`] reads it after the word.
    ///
    /// # Errors
    ///
    /// The sentence [`ReaderWords::take`] refuses the word and value with; nothing changes.
    pub fn state(&mut self, word: ReaderWord, value: &OsStr) -> Result<(), String> {
        self.reader
            .take(OsStr::new(word.word()), &mut std::iter::once(value))
            .map(|_| ())
    }

    /// Whether a revocation status nobody could establish is accepted
    /// (`super::ACCEPT_UNKNOWN_REVOCATION`).
    pub const fn accept_unknown_revocation(&mut self, accept: bool) {
        self.reader.accept_unknown_revocation = accept;
    }

    /// The conversion the archive key asks for, or none for the chooser (ADR 1954).
    pub fn archive(&mut self, chosen: Option<viewer_core::Archiving>) {
        self.reader.archive = chosen;
    }

    /// One policy at one level, as [`Settings::set`] puts it.
    ///
    /// # Errors
    ///
    /// [`Settings::set`]'s sentence; nothing changes.
    pub fn set(&mut self, policy: Policy, level: Level) -> Result<(), String> {
        self.settings.set(policy, level)?;
        self.reader.scripts = self.settings.scripts;
        Ok(())
    }

    /// RFC 0008 section 6.3's level, in both halves.
    pub const fn scripts(&mut self, level: Scripts) {
        self.settings.scripts = level;
        self.reader.scripts = level;
    }

    /// One restriction's level, every other operation's where it stood.
    pub const fn restrict(
        &mut self,
        operation: pdf_model::restriction::Operation,
        level: viewer_core::RestrictionLevel,
    ) {
        self.settings.restrictions = self.settings.restrictions.with(operation, level);
    }

    /// The levels and policies a window opened now would hold.
    #[must_use]
    pub const fn settings(&self) -> Settings {
        self.settings
    }

    /// The reader's other words, as a window hands them to its document thread.
    #[must_use]
    pub const fn reader(&self) -> &ReaderWords {
        &self.reader
    }

    /// The two halves, for a window that hands its settings to the window and the reader's words
    /// to its document thread.
    #[must_use]
    pub fn into_parts(self) -> (Settings, ReaderWords) {
        (self.settings, self.reader)
    }

    /// What the reader's words become ([`ReaderWords::commands`]): read on the document's thread.
    #[must_use]
    pub fn commands(&self) -> ReaderSupply {
        self.reader.commands()
    }
}

#[cfg(test)]
mod tests {
    use super::{ReaderWord, Taken, Words};
    use crate::policy::{LINKS, Level, Links, Policy, SCRIPTS, Scripts};
    use std::ffi::OsStr;

    /// Reads a whole command line through [`Words::take`], keeping what no word took.
    fn read(line: &[&str]) -> Result<(Words, Vec<String>), String> {
        let mut words = Words::default();
        let mut left = Vec::new();
        let mut line = line.iter().map(|word| (*word).to_owned());
        while let Some(word) = line.next() {
            if words.take(OsStr::new(&word), &mut line)?.is_none() {
                left.push(word);
            }
        }
        Ok((words, left))
    }

    /// Both readings in one pass, each word answering which it was, and the window's own left.
    #[test]
    fn a_reader_word_and_a_settings_word_are_both_taken_and_named() {
        let mut words = Words::default();
        let mut rest = ["Ada".to_owned()].into_iter();
        assert_eq!(
            words.take(OsStr::new("--reader-name"), &mut rest),
            Ok(Some(Taken::Reader))
        );
        assert_eq!(
            words.take(
                OsStr::new("--links=open"),
                &mut std::iter::empty::<String>()
            ),
            Ok(Some(Taken::Setting(LINKS)))
        );
        assert_eq!(
            words.take(OsStr::new("--cpu"), &mut std::iter::empty::<String>()),
            Ok(None)
        );
        assert_eq!(words.reader().names, ["Ada"]);
        assert_eq!(words.settings().links, Links::Open);
    }

    /// The level `--scripts` names is the core's and the window's at once, and so is the level a
    /// host sets by number.
    #[test]
    fn the_scripts_level_is_one_value_however_it_is_stated() {
        let (words, left) = read(&[SCRIPTS, "warn", "doc.pdf"]).expect("a level");
        assert_eq!(left, ["doc.pdf"]);
        assert_eq!(words.settings().scripts, Scripts::Warn);
        assert_eq!(words.reader().scripts, Scripts::Warn);
        let mut set = Words::default();
        set.set(Policy::Scripts, Level::Ask).expect("four levels");
        assert_eq!(set.reader().scripts, Scripts::Ask);
        assert_eq!(set.settings().scripts, Scripts::Ask);
        set.scripts(Scripts::Off);
        assert_eq!(set.reader().scripts, Scripts::Off);
        assert_eq!(set.settings().scripts, Scripts::Off);
    }

    /// A value stated by a host lands where the word with that value would have put it, and a
    /// directory that is not one is the command line's sentence.
    #[test]
    fn a_stated_value_is_the_word_s_value() {
        let mut stated = Words::default();
        for (word, value) in [
            (ReaderWord::Name, "Ada"),
            (ReaderWord::Title, "Editor"),
            (ReaderWord::Organisation, "Acme"),
            (ReaderWord::Name, "Grace"),
            (ReaderWord::Language, "es-MX"),
        ] {
            stated.state(word, OsStr::new(value)).expect("a value");
        }
        let (typed, _) = read(&[
            "--reader-name",
            "Ada",
            "--reader-title",
            "Editor",
            "--reader-organisation",
            "Acme",
            "--reader-name",
            "Grace",
            "--interface-language",
            "es-MX",
        ])
        .expect("every word has its value");
        assert_eq!(stated, typed);
        let refused = stated
            .state(ReaderWord::TrustAnchors, OsStr::new("/nowhere/at/all"))
            .expect_err("not a directory");
        assert!(refused.contains("--trust-anchors"), "{refused}");
        assert_eq!(stated, typed, "a refused value changes nothing");
        let directory = std::env::temp_dir();
        stated
            .state(ReaderWord::ReferenceFiles, directory.as_os_str())
            .expect("a directory");
        assert_eq!(
            stated.reader().reference_files.as_deref(),
            Some(directory.as_path())
        );
    }
}
