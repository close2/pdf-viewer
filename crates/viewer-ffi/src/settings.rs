//! `quorra_settings`: the reader's levels and other words a `quorra_window` opens under, stated by
//! a host before the window exists (RFC 0010 section 5, ADRs 2120, 2156).
//!
//! **A handle built one call at a time rather than a struct passed by value**, for ADR 0247's
//! reason: a struct's size is part of the ABI, and a policy added later would change the size of a
//! type every caller has compiled. A host sets what it has a word for and leaves the rest at
//! `viewer_host::policy::Words`' defaults, which are what a window with no command line holds.
//!
//! Two readings arrive at the same value, and both are `viewer-host`'s: a host with a command line
//! hands each word to [`Stated::take`], which is the reading the four windows' own command lines
//! go through (`viewer_host::policy::Words::take`), and a host with a settings page sets a policy
//! at a level with [`Stated::set`] and states each of the reader's other words with
//! [`Stated::state`], [`Stated::accept_unknown_revocation`] and [`Stated::archive`].

use crate::kinds::{LevelKind, PolicyKind, ReaderKind, RestrictKind, RestrictedKind};
use crate::status::Status;

/// What a host stated, and the sentence its last word was refused with.
#[derive(Debug, Default)]
pub struct Stated {
    /// The levels and the reader's other words, as a window opened now would hold them.
    words: viewer_host::policy::Words,
    /// Why the last word or value was refused; `None` once one is taken.
    refused: Option<String>,
}

impl Stated {
    /// The levels as they now stand.
    #[must_use]
    pub const fn settings(&self) -> viewer_host::Settings {
        self.words.settings()
    }

    /// Everything stated, as a window opened now takes it.
    #[must_use]
    pub fn words(&self) -> viewer_host::policy::Words {
        self.words.clone()
    }

    /// One policy at one level: `QUORRA_POLICY_*` and `QUORRA_LEVEL_*`.
    ///
    /// # Errors
    ///
    /// [`Status::WrongKind`] for a number neither names, and for §10.8.3's simulation at a level
    /// between off and on; nothing changes.
    pub fn set(&mut self, policy: u32, level: u32) -> Result<(), Status> {
        let (Some(policy), Some(level)) =
            (PolicyKind::from_code(policy), LevelKind::from_code(level))
        else {
            return Err(Status::WrongKind);
        };
        self.words
            .set(policy.policy(), level.level())
            .map_err(|_| Status::WrongKind)
    }

    /// One restriction's level: `QUORRA_RESTRICTED_*` and `QUORRA_RESTRICT_*`, as
    /// `quorra_restrict_operation` takes them.
    ///
    /// # Errors
    ///
    /// [`Status::WrongKind`] for a number neither names; nothing changes.
    pub fn restrict(&mut self, operation: u32, level: u32) -> Result<(), Status> {
        let (Some(operation), Some(level)) = (
            RestrictedKind::from_code(operation),
            RestrictKind::from_code(level),
        ) else {
            return Err(Status::WrongKind);
        };
        self.words.restrict(operation.operation(), level.level());
        Ok(())
    }

    /// A word of a command line and the word after it, read as the four windows read theirs: how
    /// many of the two were taken — `0` where `word` is none of the words, `1` where it is one
    /// that takes no value, `2` where its value was `next`. With `next` `None`, a word that wants
    /// a value is refused as a command line ending at it is.
    ///
    /// # Errors
    ///
    /// [`Status::WrongKind`] where it is one of the words and is refused — a value missing, a
    /// directory that is not one, a level its word does not take; the sentence is
    /// [`Self::refusal`], and nothing changes.
    pub fn take(&mut self, word: &str, next: Option<&str>) -> Result<usize, Status> {
        let mut rest = next.into_iter();
        match self.words.take(std::ffi::OsStr::new(word), &mut rest) {
            Ok(taken) => {
                self.refused = None;
                Ok(match taken {
                    None => 0,
                    Some(_) if next.is_some() && rest.next().is_none() => 2,
                    Some(_) => 1,
                })
            }
            Err(sentence) => Err(self.refuse(sentence)),
        }
    }

    /// One of the reader's words that names a value, `QUORRA_READER_*`, with the value: a
    /// directory for the anchors and the target documents, a name, title, organisation or
    /// language tag for the reader.
    ///
    /// # Errors
    ///
    /// [`Status::WrongKind`] for a number that names none, or where the value is refused as the
    /// word's would be — a directory that is not one; the sentence is [`Self::refusal`], and
    /// nothing changes.
    pub fn state(&mut self, word: u32, value: &str) -> Result<(), Status> {
        let word = ReaderKind::from_code(word).ok_or(Status::WrongKind)?;
        match self.words.state(word.word(), std::ffi::OsStr::new(value)) {
            Ok(()) => {
                self.refused = None;
                Ok(())
            }
            Err(sentence) => Err(self.refuse(sentence)),
        }
    }

    /// Whether a revocation status nobody could establish is accepted, as
    /// `--accept-unknown-revocation` says it is.
    pub const fn accept_unknown_revocation(&mut self, accept: bool) {
        self.words.accept_unknown_revocation(accept);
    }

    /// The conversion the archive key asks for, as indices into `quorra_archive_target`'s and
    /// `quorra_archive_profile`'s lists — the chooser's own, as `--archive` names them; either
    /// negative for none, so that the key puts up the chooser.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] for an index past either list; nothing changes.
    pub fn archive(&mut self, target: i32, profile: i32) -> Result<(), Status> {
        let (Ok(target), Ok(profile)) = (usize::try_from(target), usize::try_from(profile)) else {
            self.words.archive(None);
            return Ok(());
        };
        let mut choosing = viewer_host::archiving::Choosing::starting_on(None);
        if target >= choosing.targets().len() || profile >= choosing.profiles().len() {
            return Err(Status::OutOfRange);
        }
        choosing.set_target(target);
        choosing.set_profile(profile);
        self.words.archive(Some(choosing.chosen()));
        Ok(())
    }

    /// Keeps the sentence for [`Self::refusal`] and answers the status every refusal here is.
    fn refuse(&mut self, sentence: String) -> Status {
        self.refused = Some(sentence);
        Status::WrongKind
    }

    /// Why the last word was refused, in the sentence each window prints for it.
    #[must_use]
    pub fn refusal(&self) -> Option<&str> {
        self.refused.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::Stated;
    use crate::Status;

    /// A level set by number, a restriction set by number and a word taken arrive where the
    /// window's own command line would have put them.
    #[test]
    fn a_number_and_a_word_arrive_at_the_same_settings() {
        let mut by_number = Stated::default();
        by_number.set(0, 3).expect("links at proceed");
        by_number.set(3, 0).expect("submissions at refuse");
        by_number.set(5, 3).expect("separations on");
        by_number.restrict(0, 2).expect("copy at ask");
        let mut by_word = Stated::default();
        for word in [
            "--links=open",
            "--submissions=refuse",
            "--separations=on",
            "--restrictions=copy:ask",
        ] {
            assert_eq!(by_word.take(word, None), Ok(1), "{word}");
        }
        assert_eq!(by_number.settings(), by_word.settings());
    }

    /// A562's launched-document policy has the same four levels by word and by number.
    #[test]
    fn launched_document_levels_cross_the_settings_boundary() {
        for (level, word) in ["refuse", "ask", "warn", "open"].into_iter().enumerate() {
            let mut by_number = Stated::default();
            let mut by_word = Stated::default();
            by_number
                .set(6, u32::try_from(level).expect("four levels"))
                .expect("a level");
            by_word
                .take(&format!("--launched-documents={word}"), None)
                .expect("a word");
            assert_eq!(by_number.settings(), by_word.settings(), "{word}");
        }
    }

    /// A word that is none of them is not taken, one that names no level is refused with its
    /// parser's sentence, and a number that names nothing changes nothing.
    #[test]
    fn what_names_nothing_changes_nothing_and_says_why() {
        let mut stated = Stated::default();
        assert_eq!(stated.take("--cpu", Some("doc.pdf")), Ok(0));
        assert_eq!(stated.refusal(), None);
        assert_eq!(
            stated.take("--links=sometimes", None),
            Err(Status::WrongKind)
        );
        assert!(
            stated
                .refusal()
                .is_some_and(|said| said.contains("--links=sometimes"))
        );
        assert_eq!(stated.set(7, 0), Err(Status::WrongKind));
        assert_eq!(stated.set(0, 4), Err(Status::WrongKind));
        assert_eq!(
            stated.set(5, 1),
            Err(Status::WrongKind),
            "separations has no ask"
        );
        assert_eq!(stated.restrict(99, 0), Err(Status::WrongKind));
        assert_eq!(stated.state(6, "Ada"), Err(Status::WrongKind));
        assert_eq!(stated.archive(99, 0), Err(Status::OutOfRange));
        assert_eq!(
            stated.state(0, "/nowhere/at/all"),
            Err(Status::WrongKind),
            "not a directory"
        );
        assert!(
            stated
                .refusal()
                .is_some_and(|said| said.contains("not a directory"))
        );
        assert_eq!(stated.words(), viewer_host::policy::Words::default());
    }

    /// The reader's words arrive by number where the command line puts them by word, and a word
    /// with a value says it took the word after it.
    #[test]
    fn the_reader_s_words_arrive_by_number_where_the_words_put_them() {
        let mut by_number = Stated::default();
        by_number.state(2, "Ada").expect("a name");
        by_number.state(4, "Acme").expect("an organisation");
        by_number.state(5, "es-MX").expect("a language");
        by_number.accept_unknown_revocation(true);
        by_number
            .archive(0, 0)
            .expect("the first target and profile");
        let mut by_word = Stated::default();
        assert_eq!(by_word.take("--reader-name", Some("Ada")), Ok(2));
        assert_eq!(by_word.take("--reader-organisation", Some("Acme")), Ok(2));
        assert_eq!(by_word.take("--interface-language", Some("es-MX")), Ok(2));
        assert_eq!(
            by_word.take("--accept-unknown-revocation", Some("doc.pdf")),
            Ok(1),
            "a flag leaves the word after it"
        );
        assert_ne!(by_number.words(), by_word.words(), "no conversion yet");
        let mut choosing = viewer_host::archiving::Choosing::starting_on(None);
        choosing.set_target(0);
        choosing.set_profile(0);
        by_word.words.archive(Some(choosing.chosen()));
        assert_eq!(by_number.words(), by_word.words());
        assert_eq!(by_word.take("--reader-title", None), Err(Status::WrongKind));
        assert!(
            by_word
                .refusal()
                .is_some_and(|said| said.contains("--reader-title wants a value"))
        );
        by_number.archive(-1, 0).expect("none");
        assert_eq!(by_number.words().reader().archive, None);
    }
}
