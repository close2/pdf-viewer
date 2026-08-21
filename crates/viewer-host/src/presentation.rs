//! What full screen means for this program — ISO 32000-2 Table 29, §12.2 and §12.4.4.
//!
//! # The window §12.4.4 has never had
//!
//! `viewer-core` has driven a slide show since the hundred-and-fiftieth session and drawn its
//! transitions since the three-hundred-and-ninety-third, and since the four-hundred-and-eighty-first
//! it keeps §12.4.4.2's presentation *mode* because that clause conditions a state machine on it
//! (ADR 0316). What none of the three hosts had was the window: a presentation played inside a
//! sidebar, a tool bar and a status line, which is a slide show nobody would show anybody.
//!
//! **The standard states the window, and it states it outside §12.4.4.** §7.7.2's Table 29 gives
//! `/PageMode` as "[a] name object specifying how the document shall be displayed when opened",
//! and one of its six names is
//!
//! > Full-screen mode, with no menu bar, window controls, or any other window visible
//!
//! — which is a description of chrome and therefore, by `doc/ui-boundary.md`'s rule 5, a host's
//! to obey.
//!
//! §12.2's Table 147 states the same subject in the smaller. `/HideToolbar` is "[a] flag
//! specifying whether to hide the interactive PDF processor's tool bars when the document is
//! active", `/HideMenubar` says the same of its menu bar, and `/HideWindowUI` hides the window's
//! own scroll bars and navigation controls,
//!
//! > leaving only the document's contents displayed
//!
//! and `/NonFullScreenPageMode` states the way back out — the document's page mode,
//!
//! > specifying how to display the document on exiting full-screen mode
//!
//! under a condition the table attaches to the *catalog* rather than to itself: it is meaningful
//! only where `/PageMode` is `FullScreen`, and
//!
//! > it shall be ignored otherwise
//!
//! So there are four sentences about chrome and one about coming back, and between them they are
//! the whole of what the standard says a full-screen window is. Everything else a slide show
//! conventionally does — a cursor that fades, a click that advances, a black surround — is stated
//! nowhere at all, and the choices this program makes about those are written down below as
//! choices.
//!
//! # Why the decision is here and not in a host
//!
//! Three hosts, and the *toolkit call* differs in each: `GtkWindow::fullscreen`,
//! `QWidget::showFullScreen`, `winit::window::Window::set_fullscreen`. What does **not** differ is
//! which of Table 147's flags a window is obeying, whether Table 29 asked for full screen when the
//! document opened, and which page mode §12.2 says to come back to — and the third copy of that is
//! where two hosts stop agreeing, which is the test [`crate`] states for what belongs in it
//! (ADR 0246, and [`crate::arrangement`] is the precedent).
//!
//! **And it needed no new message on the boundary.** `viewer_core::Command::Present` already
//! carries the mode, `Query::Opening` already answers Table 29's `/PageMode`, and
//! `Query::Preferences` already answers Table 147 whole — which is `doc/ui-boundary.md`'s rule
//! working rather than a coincidence: those three exist because a *clause* needed a channel, and
//! the clause that needed this one is the same clause.
//!
//! # What is chosen rather than derived
//!
//! - **Entering full screen enters §12.4.4's presentation mode, and leaving leaves it.** The
//!   standard describes exactly one full-screen mode and exactly one presentation mode and never
//!   distinguishes a window in the first from a window in the second; a reader who asked for a
//!   slide show and got a window with a sidebar in it has been given neither. So this program has
//!   one act, and [`Presenting::mode`] is what a host sends.
//! - **Escape leaves.** No clause states how full screen ends — it states only what happens
//!   afterwards — and a reader who cannot get their menu bar back has had a restriction imposed on
//!   them by somebody else's file, which `CLAUDE.md` principle 3 forbids in the general case.
//! - **A click does not advance the page, and the pointer is not hidden.** Both are conventions of
//!   other slide shows and neither is in the standard. Taking the click would *remove* two things
//!   the standard does define for a page being displayed — §12.5.6.5's link activation and
//!   §12.4.2's text selection — in order to add one it does not, which is the wrong direction of
//!   trade.
//! - **Where §12.2's `/NonFullScreenPageMode` is not meaningful, a host puts back what the reader
//!   had.** The entry is ignored unless the catalog asked to open full screen, so leaving a full
//!   screen the *reader* asked for is not the clause's subject at all; [`Presenting::on_exit`]
//!   answers `None` there and says so rather than substituting `UseNone`, which is Table 147's
//!   default for a different question.

use pdf_model::viewer_preferences::{Opening, PageMode, ViewerPreferences};
use viewer_core::PresentationMode;

/// Which pieces of a window's chrome may be shown.
///
/// Each field is a **permission** rather than a state: `true` means the clause does not ask for
/// this to be hidden, and what is actually on the screen is still the reader's business. A host
/// hides what it is told to hide and never widens the answer.
///
/// Four fields for four sentences, and the fourth is Table 29's rather than Table 147's: full
/// screen shows no "other window", which is where a sidebar, an About card and a modal prompt go.
///
/// `struct_excessive_bools` is asking for a bitflags type or an enum; the standard states four
/// independent sentences with four independent defaults, and a host that had to unpack them at
/// every widget would be paying for a shape neither table has. The same argument
/// [`pdf_model::viewer_preferences::ViewerPreferences`] itself is written under.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per sentence; Table 147 and Table 29 state four independent ones"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chrome {
    /// Table 147's `/HideMenubar`, and Table 29's "no menu bar".
    pub menu_bar: bool,
    /// Table 147's `/HideToolbar`.
    pub tool_bar: bool,
    /// Table 147's `/HideWindowUI`: "scroll bars and navigation controls".
    pub window_ui: bool,
    /// Table 29's "or any other window visible" — the panels, the cards and the prompts.
    pub other_windows: bool,
}

impl Chrome {
    /// Nothing but the document's contents, which is Table 29's `FullScreen` in four booleans.
    pub const HIDDEN: Self = Self {
        menu_bar: false,
        tool_bar: false,
        window_ui: false,
        other_windows: false,
    };

    /// Every piece shown, which is Table 147's four defaults — all of them `false` for a *hide*.
    pub const SHOWN: Self = Self {
        menu_bar: true,
        tool_bar: true,
        window_ui: true,
        other_windows: true,
    };

    /// What §12.2 permits a window showing this document, full screen aside.
    ///
    /// Table 147 says what to *hide*, so each flag is inverted here once rather than at every
    /// widget. `other_windows` is not Table 147's at all — it is Table 29's `FullScreen` sentence
    /// — so an ordinary window keeps its panels whatever the preferences say.
    #[must_use]
    pub const fn stated(preferences: &ViewerPreferences) -> Self {
        Self {
            menu_bar: !preferences.hide_menubar,
            tool_bar: !preferences.hide_toolbar,
            window_ui: !preferences.hide_window_ui,
            other_windows: true,
        }
    }
}

/// Table 29's `FullScreen`, Table 147's four chrome entries, and the way back out.
///
/// One value per open document, held by a host beside its window. It is deliberately not in
/// `viewer-core`: every sentence it carries is about a *window*, and that crate has none by
/// construction (`doc/ui-boundary.md` rule 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presenting {
    /// What §12.2's three flags permit a window that is not full screen.
    stated: Chrome,
    /// §12.2's `/NonFullScreenPageMode`, where Table 147's own condition makes it meaningful.
    on_exit: Option<PageMode>,
    /// Whether the window is full screen now.
    full_screen: bool,
}

impl Default for Presenting {
    /// A window showing all its chrome, for a document that has said nothing.
    ///
    /// Table 147's defaults are all `false` and Table 29's is `UseNone`, so a document with no
    /// `/ViewerPreferences` and no `/PageMode` gets exactly this — which is the same rule
    /// [`ViewerPreferences`] itself is read under.
    fn default() -> Self {
        Self {
            stated: Chrome::SHOWN,
            on_exit: None,
            full_screen: false,
        }
    }
}

impl Presenting {
    /// What the document asks of the window it is about to be opened in.
    ///
    /// Table 29's `/PageMode` is "how the document shall be displayed **when opened**", so a
    /// catalog naming `FullScreen` opens full screen — the same obedience the other five names
    /// already get, each of which opens a panel. §12.2's `/NonFullScreenPageMode` is kept only
    /// where that is so, because Table 147 states the condition itself.
    #[must_use]
    pub fn opening(opening: Opening, preferences: &ViewerPreferences) -> Self {
        let full_screen = opening.mode == PageMode::FullScreen;
        Self {
            stated: Chrome::stated(preferences),
            on_exit: full_screen.then_some(preferences.non_full_screen_page_mode),
            full_screen,
        }
    }

    /// Whether the window should be full screen.
    #[must_use]
    pub const fn full_screen(&self) -> bool {
        self.full_screen
    }

    /// Which pieces of chrome may be shown.
    ///
    /// Full screen answers [`Chrome::HIDDEN`] outright, because Table 29 names the menu bar, the
    /// window controls and every other window in one sentence and leaves nothing for a flag to
    /// add. Otherwise it is what §12.2 stated.
    #[must_use]
    pub const fn chrome(&self) -> Chrome {
        if self.full_screen {
            Chrome::HIDDEN
        } else {
            self.stated
        }
    }

    /// What a host sends as [`viewer_core::Command::Present`].
    ///
    /// The one place this program's choice to make full screen and §12.4.4's mode a single act is
    /// spent, which is why it is a method rather than a second field: two fields could disagree
    /// and one cannot.
    #[must_use]
    pub const fn mode(&self) -> PresentationMode {
        if self.full_screen {
            PresentationMode::On
        } else {
            PresentationMode::Off
        }
    }

    /// Enters or leaves full screen, answering whether the window is full screen now.
    pub const fn toggle(&mut self) -> bool {
        self.full_screen = !self.full_screen;
        self.full_screen
    }

    /// Leaves full screen, answering whether there was anything to leave.
    ///
    /// Separate from [`Presenting::toggle`] because Escape is not a toggle: a host binds it to
    /// several things and has to know whether this one took it.
    pub const fn leave(&mut self) -> bool {
        let leaving = self.full_screen;
        self.full_screen = false;
        leaving
    }

    /// §12.2's `/NonFullScreenPageMode`: how to display the document on exiting full screen.
    ///
    /// `None` where Table 147's condition is not met — "it shall be ignored otherwise" — and a
    /// host that gets `None` puts back whatever the reader had, which is a choice this module's
    /// header records rather than a reading.
    #[must_use]
    pub const fn on_exit(&self) -> Option<PageMode> {
        self.on_exit
    }
}

#[cfg(test)]
mod tests {
    use pdf_model::viewer_preferences::{Opening, PageLayout, PageMode, ViewerPreferences};
    use viewer_core::PresentationMode;

    use super::{Chrome, Presenting};

    /// A document that opens on `/PageMode /FullScreen`, which Table 29 states as a `shall`.
    fn asks_for_full_screen(non_full_screen: PageMode) -> Presenting {
        Presenting::opening(
            Opening {
                mode: PageMode::FullScreen,
                layout: PageLayout::SinglePage,
            },
            &ViewerPreferences {
                non_full_screen_page_mode: non_full_screen,
                ..ViewerPreferences::default()
            },
        )
    }

    /// Table 29: "Full-screen mode, with no menu bar, window controls, or any other window
    /// visible" — one sentence, and it leaves nothing for a flag to add back.
    #[test]
    fn full_screen_shows_none_of_the_chrome() {
        let presenting = asks_for_full_screen(PageMode::UseNone);
        assert!(presenting.full_screen(), "Table 29 states it when opened");
        assert_eq!(presenting.chrome(), Chrome::HIDDEN);
        assert_eq!(presenting.mode(), PresentationMode::On);
    }

    /// Table 147's three hide flags, each inverted into a permission, on a window that is not
    /// full screen.
    #[test]
    fn table_147s_flags_decide_the_chrome_of_an_ordinary_window() {
        let presenting = Presenting::opening(
            Opening::default(),
            &ViewerPreferences {
                hide_menubar: true,
                hide_window_ui: true,
                ..ViewerPreferences::default()
            },
        );
        let chrome = presenting.chrome();
        assert!(!chrome.menu_bar, "/HideMenubar");
        assert!(chrome.tool_bar, "/HideToolbar defaults to false");
        assert!(!chrome.window_ui, "/HideWindowUI");
        assert!(
            chrome.other_windows,
            "Table 29 hides those, and this is not"
        );
        assert_eq!(presenting.mode(), PresentationMode::Off);
    }

    /// "This entry is meaningful only if the value of the `PageMode` entry in the catalog
    /// dictionary … is `FullScreen`; it shall be ignored otherwise."
    #[test]
    fn the_page_mode_to_come_back_to_is_read_only_where_the_catalog_asked_for_full_screen() {
        assert_eq!(
            asks_for_full_screen(PageMode::UseOutlines).on_exit(),
            Some(PageMode::UseOutlines)
        );
        let reader_asked = Presenting::opening(
            Opening::default(),
            &ViewerPreferences {
                non_full_screen_page_mode: PageMode::UseOutlines,
                ..ViewerPreferences::default()
            },
        );
        assert_eq!(
            reader_asked.on_exit(),
            None,
            "the catalog states no /PageMode /FullScreen, so Table 147 says to ignore the entry"
        );
    }

    /// Escape is not a toggle, and a host binding it to several things has to know which took it.
    #[test]
    fn leaving_says_whether_there_was_anything_to_leave() {
        let mut presenting = asks_for_full_screen(PageMode::UseNone);
        assert!(presenting.leave(), "it was full screen");
        assert!(!presenting.leave(), "and now there is nothing to leave");
        assert!(presenting.toggle(), "the reader may ask for it themselves");
        assert_eq!(
            presenting.on_exit(),
            Some(PageMode::UseNone),
            "the catalog asked to open full screen, so the entry stays meaningful"
        );
    }
}
