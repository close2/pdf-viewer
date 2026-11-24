//! A clip's pictures in a window of their own: ISO 32000-2 Table 294's `/W` 0, "\[a] floating
//! window", and 1, "\[a] full-screen window that obscures all other windows", with Table 295's
//! floating window parameters and §13.2.7.5's monitor specifier as far as GTK 4 can honour them
//! (RFC 0015 section 6, rounds 10 and 11; ADRs 3586, 3587).
//!
//! **What GTK 4 cannot do, and what that costs.** GTK 4 places no toplevel: the window manager
//! puts a floating window where it chooses, so Table 295's `/RT` and `/P`, and `/M` for a floating
//! window, cannot be honoured — the core was told so ([`SCREENS`]), so a rendition that must have
//! them is non-viable and one that would like them has them set aside and said, and the window
//! says once that the window manager placed it. A full-screen window goes on the monitor `/M`
//! names through `gtk_window_fullscreen_on_monitor`. GDK 4 reports no colour depth and no primary
//! monitor, so Table 304's 3 is not offered, and 2 is "treat\[ed] as case 0" as the table says;
//! nor where the document window stands, only the monitor holding most of it, so 1 is not offered
//! either.
//!
//! **What is drawn.** [`viewer_host::video::Pictures::window_picture`]: Table 294's `/B` over the
//! window and the picture fitted by Table 291's `/F`, drawn by the processor at the window's own
//! size and held up as a texture.

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use pdf_model::media::{MonitorSpecifier, Resize, Screens, WindowKind};
use viewer_host::native_window::NativeWindow;
use viewer_host::video::screens::{self, Area, DocumentWindow, Monitor};

use super::Host;

/// What this window's toolkit can do for a clip's window, as the core is told at the launch:
/// windows open; none is placed; Table 304's 0, 2, 4, 5 and 6 are found, and 1 and 3 are not.
pub(super) const SCREENS: Screens = Screens {
    windows: true,
    placed: false,
    monitors: [true, false, true, false, true, true, true],
};

/// The window a clip's pictures are shown in, and the widget holding them.
#[derive(Debug)]
pub(super) struct MediaWindow {
    /// The toplevel.
    window: gtk4::Window,
    /// The picture, filling it.
    picture: gtk4::Picture,
    /// The size a floating window was asked for, in pixels, for a picture drawn before GTK has
    /// given the widget one.
    asked: Option<(u32, u32)>,
}

impl MediaWindow {
    /// The toplevel, for a test.
    #[cfg(test)]
    pub(super) fn window(&self) -> &gtk4::Window {
        &self.window
    }

    /// The widget holding the picture, for a test.
    #[cfg(test)]
    pub(super) fn picture(&self) -> &gtk4::Picture {
        &self.picture
    }
}

/// GDK's monitors, as the shared arithmetic reads them: each one's geometry in device pixels,
/// no colour depth (GDK 4 reports none) and no primary (GDK 4 has no such notion).
fn monitors(display: &gdk::Display) -> Vec<(gdk::Monitor, Monitor)> {
    let list = display.monitors();
    (0..list.n_items())
        .filter_map(|index| list.item(index)?.downcast::<gdk::Monitor>().ok())
        .map(|monitor| {
            let geometry = monitor.geometry();
            let scale = monitor.scale_factor().max(1);
            let pixels = |logical: i32| u32::try_from(logical.saturating_mul(scale)).unwrap_or(0);
            let described = Monitor {
                name: monitor
                    .connector()
                    .map_or_else(|| "a monitor".to_owned(), |name| name.to_string()),
                area: Area {
                    x: geometry.x().saturating_mul(scale),
                    y: geometry.y().saturating_mul(scale),
                    width: pixels(geometry.width()),
                    height: pixels(geometry.height()),
                },
                colour_depth: None,
                primary: false,
            };
            (monitor, described)
        })
        .collect()
}

impl Host {
    /// Opens the window the clip in hand asks for where none is open, and closes the one open where
    /// no such clip is in hand — at each look, so a clip held at its start has its window too.
    pub(super) fn keep_the_media_window(&mut self) {
        let wanted = self
            .state
            .pictures
            .window()
            .map(|(_, label, window)| (label.to_owned(), window.clone()));
        match (wanted, self.shown.media_window.is_some()) {
            (Some((label, window)), false) => {
                self.open_the_media_window(&label, &window);
                self.hold_up_in_the_media_window();
            }
            (None, true) => {
                if let Some(open) = self.shown.media_window.take() {
                    open.window.destroy();
                }
            }
            _ => {}
        }
    }

    /// Makes and presents the window `window` describes for the clip called `label`.
    fn open_the_media_window(&mut self, label: &str, window: &pdf_model::media::Window) {
        let toplevel = gtk4::Window::new();
        // §13.2.6.1's "windows containing arbitrary media content" are to say they hold one: the
        // title names the clip and its document, after the document's own `/TT` (ADR 3587).
        let title = match &window.kind {
            WindowKind::Floating(floating) => floating
                .title
                .as_ref()
                .map_or_else(|| label.to_owned(), |text| format!("{text} — {label}")),
            WindowKind::FullScreen => label.to_owned(),
        };
        toplevel.set_title(Some(&title));
        let picture = gtk4::Picture::new();
        picture.set_content_fit(gtk4::ContentFit::Fill);
        picture.set_can_shrink(true);
        toplevel.set_child(Some(&picture));
        let mut asked = None;
        match &window.kind {
            WindowKind::Floating(floating) => {
                let scale = f64::from(self.ui.window.scale_factor().max(1));
                let logical = |pixels: u32| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a window side in pixels over a scale of one or more"
                    )]
                    let side = (f64::from(pixels) / scale).round() as i32;
                    side.max(1)
                };
                toplevel.set_default_size(logical(floating.size.0), logical(floating.size.1));
                // Table 295's `/R`: 2 lets the reader resize it; 0 does not, and 1 — "only if
                // aspect ratio is preserved" — is honoured by not resizing, since GTK 4 keeps no
                // aspect for a toplevel (ADR 3587).
                toplevel.set_resizable(floating.resize == Resize::Free);
                asked = Some(floating.size);
                self.say(&format!(
                    "{label}: shown in a floating window of {} by {} pixels, which the window \
                     manager places: GTK 4 places no window",
                    floating.size.0, floating.size.1
                ));
            }
            WindowKind::FullScreen => self.full_screen(&toplevel, label, window),
        }
        self.closes_with_the_clip(&toplevel);
        self.presses_reach_the_bar(&picture);
        toplevel.present();
        self.shown.media_window = Some(MediaWindow {
            window: toplevel,
            picture,
            asked,
        });
    }

    /// Puts a full-screen window on the monitor Table 294's `/M` names among GDK's, and says
    /// which; where it cannot be found the window manager chooses, said.
    fn full_screen(
        &mut self,
        toplevel: &gtk4::Window,
        label: &str,
        window: &pdf_model::media::Window,
    ) {
        let chosen = gdk::Display::default().map(|display| {
            let listed = monitors(&display);
            let described: Vec<Monitor> =
                listed.iter().map(|(_, monitor)| monitor.clone()).collect();
            let holding = self
                .ui
                .window
                .surface()
                .and_then(|surface| display.monitor_at_surface(&surface))
                .and_then(|holding| listed.iter().position(|(gdk, _)| *gdk == holding));
            let document = holding.map_or(DocumentWindow::Unknown, DocumentWindow::On);
            screens::monitor(window.monitor, &described, document)
                .map(|index| (listed[index].0.clone(), described[index].name.clone()))
        });
        match chosen {
            Some(Ok((monitor, name))) => {
                toplevel.fullscreen_on_monitor(&monitor);
                self.say(&format!(
                    "{label}: shown full screen on {name}, Table 304's {} ({})",
                    window.monitor.value(),
                    describe(window.monitor)
                ));
            }
            Some(Err(why)) => {
                toplevel.fullscreen();
                self.say(&format!(
                    "{label}: shown full screen where the window manager chooses, because \
                     Table 304's {} cannot be found: {why}",
                    window.monitor.value()
                ));
            }
            None => {
                toplevel.fullscreen();
                self.say(&format!(
                    "{label}: shown full screen where the window manager chooses: GDK \
                     names no display"
                ));
            }
        }
    }

    /// The reader closes the window, by its title bar or Escape, and the clip stops.
    fn closes_with_the_clip(&self, toplevel: &gtk4::Window) {
        // The reader closes it, by its title bar or Escape, and the clip stops: Table 295's `/T`
        // and `/UC` false ask for nothing the reader is denied (ADR 3587).
        let me = self.me();
        toplevel.connect_close_request(move |_| {
            let me = me.clone();
            glib::idle_add_local_once(move || {
                if let Some(host) = me.upgrade()
                    && let Ok(mut host) = host.try_borrow_mut()
                {
                    host.shown.media_window = None;
                    host.stop_the_pictures();
                }
            });
            glib::Propagation::Proceed
        });
        let keys = gtk4::EventControllerKey::new();
        let me = self.me();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key != gdk::Key::Escape {
                return glib::Propagation::Proceed;
            }
            if let Some(host) = me.upgrade()
                && let Ok(mut host) = host.try_borrow_mut()
            {
                host.stop_the_pictures();
            }
            glib::Propagation::Stop
        });
        toplevel.add_controller(keys);
    }

    /// A press on the controller bar over the window's bottom pauses or resumes the clip, as one
    /// on the bar over a rectangle of the page does (ADR 3610).
    fn presses_reach_the_bar(&self, picture: &gtk4::Picture) {
        let click = gtk4::GestureClick::new();
        let me = self.me();
        let widget = picture.clone();
        click.connect_pressed(move |_, _, x, y| {
            let Some(host) = me.upgrade() else {
                return;
            };
            let Ok(mut host) = host.try_borrow_mut() else {
                return;
            };
            let scale = widget.scale_factor().max(1);
            let pixels = |side: i32| u32::try_from(side.saturating_mul(scale)).unwrap_or(0);
            let inner = (pixels(widget.width()), pixels(widget.height()));
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a pointer position inside a window is far inside f32's exact integer range"
            )]
            let at = (
                (x * f64::from(scale)) as f32,
                (y * f64::from(scale)) as f32,
            );
            let quad = screens::whole(inner);
            if host.state.pictures.press_over(quad, at) {
                host.show_the_pictures();
                host.hold_up_in_the_media_window();
            }
        });
        picture.add_controller(click);
    }

    /// Holds up the picture in the window of its own, drawn at the widget's size in pixels.
    pub(super) fn hold_up_in_the_media_window(&mut self) {
        let Some(open) = &self.shown.media_window else {
            return;
        };
        let scale = open.picture.scale_factor().max(1);
        let allocated = (
            u32::try_from(open.picture.width().saturating_mul(scale)).unwrap_or(0),
            u32::try_from(open.picture.height().saturating_mul(scale)).unwrap_or(0),
        );
        let inner = if allocated.0 > 0 && allocated.1 > 0 {
            allocated
        } else {
            open.asked.unwrap_or((1, 1))
        };
        let drawn = self.state.pictures.window_picture(inner);
        let texture = match drawn {
            Some(Ok(raster)) => {
                crate::page::texture(&raster).map_err(|refused| refused.to_string())
            }
            Some(Err(refused)) => Err(refused),
            None => return,
        };
        match texture {
            Ok(texture) => open.picture.set_paintable(Some(&texture)),
            Err(refused) => self.say(&format!("a clip's picture was not shown: {refused}")),
        }
    }
}

/// Table 304's description of `specifier`, for the sentence that says where a window went.
fn describe(specifier: MonitorSpecifier) -> &'static str {
    match specifier {
        MonitorSpecifier::Largest => {
            "the monitor containing the largest section of the document window"
        }
        MonitorSpecifier::Smallest => {
            "the monitor containing the smallest section of the document window"
        }
        MonitorSpecifier::Primary => "the primary monitor, or with none the document window's",
        MonitorSpecifier::ColourDepth => "the monitor with the greatest colour depth",
        MonitorSpecifier::Area => "the monitor with the greatest area",
        MonitorSpecifier::Height => "the monitor with the greatest height",
        MonitorSpecifier::Width => "the monitor with the greatest width",
    }
}
