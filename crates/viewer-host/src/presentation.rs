//! What full screen means for this program — ISO 32000-2 Table 29, §12.2 and §12.4.4.
//!
//! # The window §12.4.4 has never had
//!
//! `viewer-core` drives a slide show (ADR 0135), draws its transitions (ADR 0230) and keeps
//! §12.4.4.2's presentation *mode* because that clause conditions a state machine on it (ADR 0316).
//! What is a host's is the window: a presentation played inside a sidebar, a tool bar and a status
//! line is a slide show nobody would show anybody.
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
//!
//! # The window's size and place
//!
//! Table 147 has two more entries about the window rather than about its chrome: `/FitWindow`, "[a]
//! flag specifying whether to resize the document's window to fit the size of the first displayed
//! page", and `/CenterWindow`, "[a] flag specifying whether to position the document's window in
//! the centre of the screen". Neither can be obeyed when the document opens, because the size of
//! the first displayed page is not known until it has been displayed: [`Presenting::place`] answers
//! them at the frames of a document opened in front until its window fits the page, and [`fitted`] and [`centred`] are
//! the arithmetic every host shares. The toolkit call is each host's, and one of them has none:
//! GTK 4 gives a client no way to position its own window, so `quorra-gtk` says so rather than
//! centring (ADR 1429).

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
    /// Table 147's `/FitWindow` and `/CenterWindow`, while still owed.
    placing: Option<Placing>,
    /// The page the first frame asked about showed, which the placing is owed to.
    placing_page: Option<usize>,
    /// How many frames have been asked about the placing.
    placing_asked: u8,
}

/// How many frames a window is given to come to fit the first displayed page.
///
/// A window that hides a tool bar or a status line when the document opens draws its first frame
/// before that chrome has left, so the first fit is measured against a viewport about to grow and
/// a second frame measures it again; two frames is what the two native windows take under `Xvfb`.
/// The bound is for a platform that refuses a size, which would otherwise be asked again at every
/// frame; the last frame gives up the fit and still centres (ADR 1429).
const PLACING_FRAMES: u8 = 4;

/// Table 147's two entries about the window's extent and position, as one first frame owes them.
///
/// `struct_excessive_bools` would have two flags be an enumeration of four states; the table
/// states two independent entries with two independent defaults, as [`Chrome`] states four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placing {
    /// `/FitWindow`: resize the window to the first displayed page.
    pub fit: bool,
    /// `/CenterWindow`: put the window in the centre of the screen.
    pub centre: bool,
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
            placing: None,
            placing_page: None,
            placing_asked: 0,
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
        let placing = Placing {
            fit: preferences.fit_window,
            centre: preferences.center_window,
        };
        Self {
            stated: Chrome::stated(preferences),
            on_exit: full_screen.then_some(preferences.non_full_screen_page_mode),
            full_screen,
            placing: (placing.fit || placing.centre).then_some(placing),
            placing_page: None,
            placing_asked: 0,
        }
    }

    /// Table 147's `/FitWindow` and `/CenterWindow`, asked at a frame of page `page`: `Some` while
    /// they are owed, and `None` once [`Self::placed`] has been said, once a frame of another page
    /// has been drawn, or once [`PLACING_FRAMES`] frames have been asked.
    ///
    /// At a frame rather than on opening, because "the size of the first displayed page" is the
    /// size a page is drawn at, which the window knows only once it has drawn it. A host fits the
    /// window where it does not fit yet and otherwise centres it and says [`Self::placed`], so a
    /// person who resizes or moves the window afterwards keeps what they chose; the last frame
    /// asked answers with `fit` false, so that a window the platform will not resize is still
    /// centred. A window that opens full screen owes neither: Table 29's full screen is the whole
    /// screen, which no size or position the table states can improve on.
    pub fn place(&mut self, page: usize) -> Option<Placing> {
        if self.full_screen {
            self.placing = None;
        }
        let mut placing = self.placing?;
        if self.placing_page.is_some_and(|first| first != page) {
            self.placing = None;
            return None;
        }
        self.placing_page = Some(page);
        self.placing_asked = self.placing_asked.saturating_add(1);
        if self.placing_asked >= PLACING_FRAMES {
            self.placing = None;
            placing.fit = false;
        }
        Some(placing)
    }

    /// Whether Table 147's `/FitWindow` or `/CenterWindow` is still owed, for a host that must
    /// ask its window's extents before it can answer [`Self::place`].
    #[must_use]
    pub const fn owes_placing(&self) -> bool {
        self.placing.is_some() && !self.full_screen
    }

    /// The window fits the first displayed page and has been centred where that was asked:
    /// nothing more is owed.
    pub const fn placed(&mut self) {
        self.placing = None;
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

/// The inner size a window takes for Table 147's `/FitWindow`, in the units it is given.
///
/// The chrome is kept as it is and the part that shows pages is made the size of the page drawn in
/// it: `window` less `viewport` is what surrounds the pages, and `page` is the first displayed
/// page's extent at the magnification it was displayed at. Held to `screen` where a host knows it,
/// because a page drawn larger than the screen is a window no one could see the edges of; and
/// never below one unit either way.
#[must_use]
pub fn fitted(
    window: (f64, f64),
    viewport: (f64, f64),
    page: (f64, f64),
    screen: Option<(f64, f64)>,
) -> (f64, f64) {
    let width = window.0 - viewport.0 + page.0;
    let height = window.1 - viewport.1 + page.1;
    let (width, height) = screen.map_or((width, height), |(most_wide, most_tall)| {
        (width.min(most_wide), height.min(most_tall))
    });
    (width.max(1.0), height.max(1.0))
}

/// Where a window's top-left corner goes for Table 147's `/CenterWindow`: its `outer` extent,
/// frame included, centred on `screen` given as `(left, top, width, height)`.
///
/// A window larger than the screen keeps its top-left corner on it, because the half that would
/// otherwise be off the top holds the title bar that moves it.
#[must_use]
pub fn centred(outer: (f64, f64), screen: (f64, f64, f64, f64)) -> (f64, f64) {
    let (left, top, width, height) = screen;
    (
        left + ((width - outer.0) / 2.0).max(0.0),
        top + ((height - outer.1) / 2.0).max(0.0),
    )
}

#[cfg(test)]
mod tests {
    use pdf_model::viewer_preferences::{Opening, PageLayout, PageMode, ViewerPreferences};
    use viewer_core::PresentationMode;

    use super::{Chrome, Placing, Presenting, centred, fitted};

    /// Table 147's `/FitWindow` and `/CenterWindow` are owed once, to the first frame, and not by a
    /// window that opened full screen.
    #[test]
    fn the_windows_size_and_place_are_owed_once() {
        let preferences = ViewerPreferences {
            fit_window: true,
            center_window: true,
            ..ViewerPreferences::default()
        };
        let single = Opening {
            mode: PageMode::UseNone,
            layout: PageLayout::SinglePage,
        };
        let asked = Some(Placing {
            fit: true,
            centre: true,
        });
        let mut presenting = Presenting::opening(single, &preferences);
        assert_eq!(presenting.place(0), asked);
        assert_eq!(presenting.place(0), asked, "owed until the window fits");
        presenting.placed();
        assert_eq!(presenting.place(0), None, "paid once");

        let mut another_page = Presenting::opening(single, &preferences);
        assert_eq!(another_page.place(0), asked);
        assert_eq!(
            another_page.place(1),
            None,
            "the first displayed page's, not the next one's"
        );

        let mut refused = Presenting::opening(single, &preferences);
        for _ in 1..super::PLACING_FRAMES {
            assert_eq!(refused.place(0), asked);
        }
        assert_eq!(
            refused.place(0),
            Some(Placing {
                fit: false,
                centre: true
            }),
            "the last frame gives up the fit and still centres"
        );
        assert_eq!(refused.place(0), None);

        let mut silent = Presenting::opening(single, &ViewerPreferences::default());
        assert_eq!(silent.place(0), None, "Table 147's defaults are false");
        let mut full = Presenting::opening(
            Opening {
                mode: PageMode::FullScreen,
                layout: PageLayout::SinglePage,
            },
            &preferences,
        );
        assert_eq!(full.place(0), None, "full screen is the whole screen");
    }

    /// The chrome is kept and the viewport becomes the page: a 1000 by 1100 window whose pages are
    /// shown in 610 by 1020 of it, showing a page drawn 610 by 812, becomes 1000 by 892.
    #[test]
    fn a_fitted_window_keeps_its_chrome_and_takes_the_pages_size() {
        assert_eq!(
            fitted((1000.0, 1100.0), (610.0, 1020.0), (610.0, 812.0), None),
            (1000.0, 892.0)
        );
        assert_eq!(
            fitted(
                (800.0, 1000.0),
                (800.0, 1000.0),
                (750.0, 3000.0),
                Some((1400.0, 1200.0))
            ),
            (750.0, 1200.0),
            "held to the screen"
        );
        assert_eq!(
            centred((750.0, 1000.0), (0.0, 0.0, 1400.0, 1200.0)),
            (325.0, 100.0)
        );
        assert_eq!(
            centred((2000.0, 1000.0), (10.0, 0.0, 1400.0, 1200.0)),
            (10.0, 100.0),
            "a window wider than the screen keeps its corner on it"
        );
    }

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
