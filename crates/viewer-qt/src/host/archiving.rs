//! ISO 19005's conversion chosen after launch: the six targets and the shipped profiles, in a
//! dialogue the C++ side opens (ADR 1966).
//!
//! What it offers is [`viewer_host::archiving::Choosing`] unchanged — the list `quorra`'s card and
//! the GTK dialogue offer — handed across as words with the two indices it starts on, and what
//! comes back is two indices, which [`NativeWindow::archive_chosen`] turns into the window's choice
//! for its life and a conversion.

use viewer_host::archiving::{CHOOSING, CHOOSING_TITLE, Choosing, NOT_CHOSEN};
use viewer_host::native_window::NativeWindow;

use super::Host;

impl Host {
    /// Holds the choice and asks the C++ side for its dialogue: a `QDialog` is a Qt object and
    /// Rust never calls one.
    pub(super) fn offer_the_archive_choice(&mut self, choosing: Choosing) {
        self.archive_choice = Some(choosing);
        self.update.archive_dialogue = true;
        self.say(&format!("{CHOOSING_TITLE}: {CHOOSING}"));
    }

    /// The dialogue's title.
    #[expect(
        clippy::unused_self,
        reason = "the bridge declares it as a method on `Host` and cxx calls it as one; a free \
                  function could not cross"
    )]
    pub(crate) fn archive_title(&self) -> String {
        CHOOSING_TITLE.to_owned()
    }

    /// What the dialogue says under its title.
    #[expect(
        clippy::unused_self,
        reason = "the bridge declares it as a method on `Host` and cxx calls it as one; a free \
                  function could not cross"
    )]
    pub(crate) fn archive_about(&self) -> String {
        CHOOSING.to_owned()
    }

    /// The targets, in the order offered; empty where no choice is being made.
    pub(crate) fn archive_targets(&self) -> Vec<String> {
        self.archive_choice
            .as_ref()
            .map(|choosing| choosing.targets().iter().map(ToString::to_string).collect())
            .unwrap_or_default()
    }

    /// The profiles' names, in the order offered.
    pub(crate) fn archive_profiles(&self) -> Vec<String> {
        self.archive_choice
            .as_ref()
            .map(|choosing| {
                choosing
                    .profiles()
                    .iter()
                    .map(|offered| offered.name.to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Each profile's sentence, beside [`Self::archive_profiles`]'s names.
    pub(crate) fn archive_sentences(&self) -> Vec<String> {
        self.archive_choice
            .as_ref()
            .map(|choosing| {
                choosing
                    .profiles()
                    .iter()
                    .map(|offered| offered.sentence.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The target the dialogue starts on, as an index into [`Self::archive_targets`]: the held
    /// choice's where the window holds one (ADR 2001), and the first where no choice is being made.
    pub(crate) fn archive_target_start(&self) -> i32 {
        self.archive_choice
            .as_ref()
            .and_then(|choosing| i32::try_from(choosing.target_index()).ok())
            .unwrap_or(0)
    }

    /// The profile the dialogue starts on, as an index into [`Self::archive_profiles`].
    pub(crate) fn archive_profile_start(&self) -> i32 {
        self.archive_choice
            .as_ref()
            .and_then(|choosing| i32::try_from(choosing.profile_index()).ok())
            .unwrap_or(0)
    }

    /// The person chose the target and the profile at these indices, or — either negative —
    /// closed the dialogue having chosen nothing.
    pub(crate) fn archive_answer(&mut self, target: i32, profile: i32) {
        let Some(mut choosing) = self.archive_choice.take() else {
            return;
        };
        let (Ok(target), Ok(profile)) = (usize::try_from(target), usize::try_from(profile)) else {
            self.say(NOT_CHOSEN);
            return;
        };
        choosing.set_target(target);
        choosing.set_profile(profile);
        self.archive_chosen(choosing.chosen());
    }
}
