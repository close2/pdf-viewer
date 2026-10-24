//! Effective chrome, read after a chrome record without reinterpreting document preferences.

use super::Window;

/// The four independent pieces of effective chrome, as `quorra_window_chrome` answers them.
/// Each field is zero for hidden and one for shown. Adding this shape changes no existing ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct WindowChrome {
    /// Whether the menu bar may be shown.
    pub menu_bar: u32,
    /// Whether tool bars may be shown.
    pub tool_bar: u32,
    /// Whether window controls, including navigation and scroll bars, may be shown.
    pub window_ui: u32,
    /// Whether panels and other windows may be shown; independent of navigation controls.
    pub other_windows: u32,
}

impl From<viewer_host::Chrome> for WindowChrome {
    fn from(chrome: viewer_host::Chrome) -> Self {
        Self {
            menu_bar: u32::from(chrome.menu_bar),
            tool_bar: u32::from(chrome.tool_bar),
            window_ui: u32::from(chrome.window_ui),
            other_windows: u32::from(chrome.other_windows),
        }
    }
}

impl Window {
    /// The effective chrome after the most recent document or presentation change.
    /// Read after `OwedKind::Chrome`; hidden navigation controls do not imply hidden panels.
    #[must_use]
    pub fn chrome(&self) -> WindowChrome {
        self.state.presenting.chrome().into()
    }
}

impl Window {
    /// Applies the shared presentation state while leaving physical window placement to C.
    pub(super) fn presentation(&mut self, leaving: bool) {
        use viewer_host::native_window::NativeWindow as _;
        let presenting = if leaving {
            if !self.state.presenting.leave() {
                return;
            }
            false
        } else {
            self.state.presenting.toggle()
        };
        if presenting {
            self.start_the_clock();
            self.dispatch(viewer_core::Command::Present(
                viewer_core::PresentationMode::On,
            ));
            self.owing.owe(super::owed::Owed::Chrome(None));
        } else {
            viewer_host::PresentationPump::drop_the_clock(self);
            self.playing = None;
            self.dispatch(viewer_core::Command::Present(
                viewer_core::PresentationMode::Off,
            ));
            if let Some(mode) = self.state.presenting.on_exit() {
                self.show_the_page_mode(mode);
            } else {
                self.owing.owe(super::owed::Owed::Chrome(None));
            }
        }
        self.repaint();
    }
}

impl Window {
    /// A document swap restores the arriving document's presentation, and the pump owes its mode
    /// and chrome once the queue has settled (`viewer_host::presentation_pump`, ADR 2300); a real
    /// swap also forgets the chrome records the document left had not yet delivered.
    pub(super) fn restore_document_presentation(&mut self, temporary: bool) {
        if !temporary {
            self.owing.forget_chrome();
            self.playing = None;
        }
        viewer_host::PresentationPump::bring_the_presentation_forward(self, temporary);
    }
}
