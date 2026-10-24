//! The window itself: what the title bar and the status line say, where Table 147 puts the window
//! and how large, which chrome Table 29 and §12.2 let it show, and the viewport's size (filed here
//! by ADR 1908).

use viewer_core::{Answer, Query};
use viewer_host::native_window::{Extents, FirstSize, NativeWindow, Placement, Sizing};
use viewer_host::panel::Tab;
use viewer_host::trace::Topic;

use crate::bridge::ffi::{QtChrome, QtWindowExtents};

use super::{Host, WINDOW, level};

/// Qt's part of the sizes a window takes: its first is a resize, and it fits itself to Table 147's
/// `/FitWindow` at the frame [`Host::place_window`] is asked at (ADRs 1429, 2172).
const SIZING: Sizing = Sizing {
    first: FirstSize::Resize,
};

impl Host {
    /// The viewport changed size, in device pixels.
    pub(crate) fn resized(&mut self, width: u32, height: u32, scale: f32) {
        if width == 0 || height == 0 || !scale.is_finite() || scale <= 0.0 {
            return;
        }
        self.scale = scale;
        // Qt's drawing pump looks at a document still opening, so nothing is armed for it here.
        let _ = self.sized(width, height, scale, SIZING);
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
        // ADR 1831's sentence, where a page was drawn on the processor rather than the device, and
        // the pages whose marks a draw left to the library's converter (ADR 2290).
        let lane = self.state.title_notes();
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
    /// placement is owed or none is left to make. The decision is
    /// [`NativeWindow::place_the_window`]'s, shared with the C window; what is Qt's is the `resize`
    /// and the `move`, both requests the platform may refuse, and on Wayland, where `move` places
    /// nothing, the window says so instead of asking (ADRs 1429, 2308).
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
            positions,
        } = extents;
        let extents = Extents {
            window: (window_width, window_height),
            frame: (frame_width, frame_height),
            screen: (screen_left, screen_top, screen_width, screen_height),
            positions,
        };
        match self.place_the_window(self.scale, extents) {
            Some(Placement::Resize { width, height }) => vec![1, width, height, 0, 0, 0],
            Some(Placement::Move { x, y }) => vec![0, 0, 0, 1, x, y],
            Some(Placement::Placed) | None => Vec::new(),
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
