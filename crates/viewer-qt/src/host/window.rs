//! The window itself: what the title bar and the status line say, where Table 147 puts the window
//! and how large, which chrome Table 29 and §12.2 let it show, and the viewport's size (filed here
//! by ADR 1908).

use viewer_core::{Answer, Command, Query};
use viewer_host::native_window::NativeWindow;
use viewer_host::panel::Tab;
use viewer_host::trace::Topic;

use crate::bridge::ffi::{QtChrome, QtWindowExtents};

use super::{Host, WINDOW, level};

impl Host {
    /// The viewport changed size, in device pixels.
    pub(crate) fn resized(&mut self, width: u32, height: u32, scale: f32) {
        if width == 0 || height == 0 || !scale.is_finite() || scale <= 0.0 {
            return;
        }
        self.scale = scale;
        // §12.4.4.1's transition is drawn in the viewport rather than in a page's own rectangle,
        // so the size the core is told is also the size a frame is shaped for.
        self.state.viewport = (width, height);
        let resize = Command::Resize {
            width,
            height,
            scale,
        };
        // Page one's drawing ahead draws into the target the first frame will be asked at, which is
        // this viewport's; a size that changes while the page is read is told again, and a frame
        // asked at another target than the one drawn is drawn whole (ADR 1855).
        let window = viewer_host::loading::Window {
            width,
            height,
            inset: 0,
            scale,
        };
        self.state.extent = Some(window);
        if let Some(ahead) = self.state.ahead.as_ref() {
            ahead.window_is(window);
        }
        if !self.opened {
            self.opened = true;
            self.state.trace.say(
                Topic::Launch,
                format_args!("first resize {width}x{height} device px, scale {scale}"),
            );
            self.join_the_opening(resize);
            return;
        }
        // A document still opening past `WARN` is told the window's size when it is taken in.
        if self.state.awaiting.is_some() {
            self.state.awaiting = Some(resize);
            return;
        }
        self.dispatch(resize);
        // A window asked to fit the first displayed page is measured again once it has taken
        // the size it was asked for, because a frame is not always drawn for a viewport the page
        // already fits the width of (ADR 1429).
        if self.state.presenting.owes_placing() {
            self.update.placement = true;
        }
    }

    /// Which pieces of chrome this window may show, and whether it is full screen.
    pub(crate) fn chrome(&self) -> QtChrome {
        let chrome = self.state.presenting.chrome();
        QtChrome {
            full_screen: self.state.presenting.full_screen(),
            menu_bar: chrome.menu_bar,
            tool_bar: chrome.tool_bar,
            window_ui: chrome.window_ui,
            // **Two conditions rather than one since ADR 0526**: Table 29's permission, and the
            // reader's own wish, which `o` moves. Composed here so that leaving full screen puts
            // back what the reader had rather than what the document last permitted.
            other_windows: chrome.other_windows && self.panel_shown,
        }
    }

    /// Which of Table 29's six panels the document asks to be showing, as a tab index.
    ///
    /// The page mode in force: `/NonFullScreenPageMode` while full screen is ending and
    /// nothing while it is not, because §12.2 states no page mode for a window that never left.
    /// `-1` where this host has no panel for the name.
    pub(crate) fn panel_wanted(&self) -> i32 {
        let mode = if self.state.presenting.full_screen() {
            return -1;
        } else if let Some(mode) = self.state.presenting.on_exit() {
            mode
        } else {
            let Answer::Opening(opening) = self.state.viewer.query(Query::Opening) else {
                return -1;
            };
            opening.mode
        };
        // Which panel a page mode opens is `viewer_host::Tab`'s, shared with the other two hosts,
        // and **`UseThumbs` is no longer among the names that reach none**: this host reported it
        // as a mode it had no panel for until §12.3.4's list was built. `UseNone` asks for nothing
        // and `FullScreen` is the window rather than a panel, so those two still answer -1.
        Tab::of_page_mode(mode)
            .and_then(|tab| i32::try_from(tab.index()).ok())
            .unwrap_or(-1)
    }

    /// The third-party notices this binary is obliged to carry, for the window `?` opens.
    ///
    /// **A licence obligation with a surface, and this host had neither half of it until ADR
    /// 0526**: `pdf-font` compiles the standard 14 font programs (§9.6.2.2) into every binary in
    /// this tree, both of their licences require a binary distribution to reproduce their notices,
    /// and `quorra-qt` reproduced them nowhere at all. The text is `viewer_host::NOTICE`,
    /// shared with the other two hosts.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so a constant answer is \
                  still a method — an associated function would need a second declaration"
    )]
    pub(crate) fn notices(&self) -> String {
        viewer_host::NOTICE.to_owned()
    }

    /// What the title bar should say.
    pub(crate) fn title(&self) -> String {
        let mark = if self.state.showing.dirty { "• " } else { "" };
        // ADR 1831's sentence, where a page was drawn on the processor rather than the device.
        let lane = self
            .state
            .lane
            .as_ref()
            .and_then(|lane| lane.title())
            .map_or_else(String::new, |lane| format!(" — {lane}"));
        if self.state.showing.caption.is_empty() {
            return format!("{mark}{}{lane}", self.named());
        }
        format!(
            "{mark}{} — {}{lane}",
            self.named(),
            self.state.showing.caption
        )
    }

    /// Where the window goes and how large it is, for Table 147's `/FitWindow` and
    /// `/CenterWindow` — asked by the window when `placement` is set, with its own extents in
    /// logical pixels ([`QtWindowExtents`]).
    ///
    /// Answers `[resize, width, height, move, x, y]`, a flag before each pair, or nothing where no
    /// placement is owed. A window that does not fit the first displayed page yet is asked to
    /// resize and nothing else, and the next frame measures again — which it needs, because the
    /// first frame is drawn before the tool bar and status line this document hid have left the
    /// viewport; a window that fits is moved to the centre where that was asked, and nothing more
    /// is owed ([`viewer_host::Presenting::place`]). The arithmetic is [`viewer_host::fitted`]'s
    /// and [`viewer_host::centred`]'s, shared with the other two windows; what is Qt's is the
    /// `resize` and the `move`, both requests the platform may refuse (ADR 1429).
    pub(crate) fn place_window(&mut self, extents: QtWindowExtents) -> Vec<i32> {
        let QtWindowExtents {
            window_width,
            window_height,
            frame_width,
            frame_height,
            screen_left,
            screen_top,
            screen_width,
            screen_height,
        } = extents;
        let page = match self.state.viewer.query(Query::View) {
            Answer::View(viewing) => viewing.page,
            _ => 0,
        };
        let Some(placing) = self.state.presenting.place(page) else {
            return Vec::new();
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a window's extent and position in logical pixels, held to its screen"
        )]
        let whole = |value: f64| value.round() as i32;
        // A window Qt could not put on a screen has no screen to fit to or to centre on.
        let screen = (screen_width > 0 && screen_height > 0).then_some((
            f64::from(screen_left),
            f64::from(screen_top),
            f64::from(screen_width),
            f64::from(screen_height),
        ));
        let inner = (f64::from(window_width), f64::from(window_height));
        if placing.fit
            && let Answer::Geometry(geometry) = self.state.viewer.query(Query::PageGeometry(page))
        {
            let scale = f64::from(self.scale);
            let wanted = viewer_host::fitted(
                inner,
                (
                    f64::from(self.state.viewport.0) / scale,
                    f64::from(self.state.viewport.1) / scale,
                ),
                (
                    f64::from(geometry.width) / scale,
                    f64::from(geometry.height) / scale,
                ),
                screen.map(|(_, _, width, height)| {
                    (
                        width - f64::from(frame_width),
                        height - f64::from(frame_height),
                    )
                }),
            );
            if (wanted.0 - inner.0).abs() > 1.0 || (wanted.1 - inner.1).abs() > 1.0 {
                return vec![1, whole(wanted.0), whole(wanted.1), 0, 0, 0];
            }
        }
        self.state.presenting.placed();
        match screen {
            Some(screen) if placing.centre => {
                let (x, y) = viewer_host::centred(
                    (
                        inner.0 + f64::from(frame_width),
                        inner.1 + f64::from(frame_height),
                    ),
                    screen,
                );
                vec![0, 0, 0, 1, whole(x), whole(y)]
            }
            _ => Vec::new(),
        }
    }

    /// The most recent sentence for a person.
    pub(crate) fn status(&self) -> String {
        self.message.clone()
    }

    /// The window's initial size in logical pixels.
    ///
    /// A method on the host rather than a constant in the C++, because the two hosts asking for
    /// the same window is the thing that makes their launch numbers comparable, and a number
    /// written down twice is a number that stops agreeing.
    #[expect(
        clippy::unused_self,
        reason = "`cxx` carries methods on the opaque type; a free function would need a second \
                  entry in the bridge for a value that belongs to this host"
    )]
    pub(crate) fn window_size(&self) -> Vec<i32> {
        vec![WINDOW.0, WINDOW.1]
    }

    /// What the page area shows where no page lies: `pdf_render::SURROUND`, as three bytes.
    ///
    /// **Not §11.4.7's 𝑊.** The page's own colour is the standard's, is white, and is composited
    /// by the rasteriser inside §14.11.2.1's crop box; this is the ground a window lays the pages
    /// on, which no clause of ISO 32000-2 discusses — `pdf_render::medium` has the search that
    /// establishes the silence and the argument for the value. Read from there rather than
    /// restated here, for the same reason `window_size` is a method: a number written down twice
    /// is a number that stops agreeing, and this one has to agree with `viewer-gtk`, with
    /// `viewer-ui` and with all three rasterisers.
    #[expect(
        clippy::unused_self,
        reason = "`cxx` carries methods on the opaque type; a free function would need a second \
                  entry in the bridge for a value that belongs to this host"
    )]
    pub(crate) fn surround(&self) -> Vec<u8> {
        let colour = pdf_render::SURROUND;
        vec![level(colour.r), level(colour.g), level(colour.b)]
    }

    /// One sentence from the C++ side.
    pub(crate) fn note(&self, what: &str) {
        self.state.trace.say(Topic::Panel, format_args!("{what}"));
    }

    /// One sentence for a person, in the status bar and on standard output.
    ///
    /// Both, because the status bar is what a person sees and the terminal is what a test reads —
    /// and the `Xvfb` runs this project checks a window with read the terminal.
    pub(super) fn say(&mut self, what: &str) {
        if what.is_empty() {
            return;
        }
        println!("note: {what}");
        what.clone_into(&mut self.message);
        self.update.status = true;
    }
}
