//! `quorra_settings`: the reader's levels a `quorra_window` opens under, stated by a host before
//! the window exists (RFC 0010 section 5, ADR 2120).
//!
//! **A handle built one call at a time rather than a struct passed by value**, for ADR 0247's
//! reason: a struct's size is part of the ABI, and a policy added later would change the size of a
//! type every caller has compiled. A host sets what it has a word for and leaves the rest at
//! `viewer_host::Settings`' defaults, which are what a window with no command line holds.
//!
//! Two readings arrive at the same value, and both are `viewer-host`'s: a host with a command line
//! hands each word to [`Stated::take`], which is the reading the four windows' own command lines
//! go through (`viewer_host::Settings::take`), and a host with a settings page sets a policy at a
//! level with [`Stated::set`].

use crate::kinds::{LevelKind, PolicyKind, RestrictKind, RestrictedKind};
use crate::status::Status;

/// What a host stated, and the sentence its last word was refused with.
#[derive(Debug, Default)]
pub struct Stated {
    /// The levels, as a window opened now would hold them.
    settings: viewer_host::Settings,
    /// Why the last word [`Self::take`] was handed was refused; `None` once one is taken.
    refused: Option<String>,
}

impl Stated {
    /// The levels as they now stand.
    #[must_use]
    pub const fn settings(&self) -> viewer_host::Settings {
        self.settings
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
        self.settings
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
        self.settings.restrictions = self
            .settings
            .restrictions
            .with(operation.operation(), level.level());
        Ok(())
    }

    /// A word of a command line, read as the four windows read theirs: `true` where it was one of
    /// the settings words and is now in force, `false` where it is none of them.
    ///
    /// # Errors
    ///
    /// [`Status::WrongKind`] where it is one of them and names no level they take; the sentence
    /// is [`Self::refusal`], and nothing changes.
    pub fn take(&mut self, word: &str) -> Result<bool, Status> {
        match self.settings.take(word) {
            Ok(taken) => {
                self.refused = None;
                Ok(taken.is_some())
            }
            Err(sentence) => {
                self.refused = Some(sentence);
                Err(Status::WrongKind)
            }
        }
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
            assert_eq!(by_word.take(word), Ok(true), "{word}");
        }
        assert_eq!(by_number.settings(), by_word.settings());
    }

    /// A word that is none of them is not taken, one that names no level is refused with its
    /// parser's sentence, and a number that names nothing changes nothing.
    #[test]
    fn what_names_nothing_changes_nothing_and_says_why() {
        let mut stated = Stated::default();
        assert_eq!(stated.take("--cpu"), Ok(false));
        assert_eq!(stated.refusal(), None);
        assert_eq!(stated.take("--links=sometimes"), Err(Status::WrongKind));
        assert!(
            stated
                .refusal()
                .is_some_and(|said| said.contains("--links=sometimes"))
        );
        assert_eq!(stated.set(6, 0), Err(Status::WrongKind));
        assert_eq!(stated.set(0, 4), Err(Status::WrongKind));
        assert_eq!(
            stated.set(5, 1),
            Err(Status::WrongKind),
            "separations has no ask"
        );
        assert_eq!(stated.restrict(99, 0), Err(Status::WrongKind));
        assert_eq!(stated.settings(), viewer_host::Settings::default());
    }
}
