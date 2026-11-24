//! A clip's pictures in a window of their own: ISO 32000-2 Table 294's `/W` 0, "\[a] floating
//! window", and 1, "\[a] full-screen window that obscures all other windows", placed by Table
//! 295's `/D`, `/RT`, `/P` and `/O` on the monitor §13.2.7.5's specifier names among winit's (RFC
//! 0015 section 6, rounds 10 and 11; ADRs 3586, 3587).
//!
//! **What winit can do here.** On X11, Windows and macOS a window is placed where it is asked, the
//! document window's place is known, and each monitor states a colour depth (X11's is the screen's
//! root depth, the same for every monitor, which makes every monitor tie). On Wayland no client
//! places a window, the document window's place is not known — only the monitor it entered — and
//! winit states 32 bits for every monitor whatever it is, so `/RT`, `/P`, a floating window's `/M`
//! and Table 304's 1 and 3 are not offered there ([`screens()`]): the core judges a rendition that
//! must have them non-viable and sets them aside where it would only like them, and the window
//! says the compositor placed it.
//!
//! **What is drawn.** [`viewer_host::video::VideoPlayer::window_picture`]: Table 294's `/B` over the
//! window and the picture fitted by Table 291's `/F`, drawn by the processor at the window's size
//! and presented through a software surface of its own — a second graphics device surface for a
//! window of a few pictures a second is not built (ADR 3587).

use std::sync::Arc;

use pdf_model::media::{Relative, Resize, Screens, WindowKind};
use viewer_host::video::screens::{self, Area, DocumentWindow, Monitor};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::monitor::MonitorHandle;
use winit::window::{Fullscreen, Window, WindowId};

use crate::app::App;

/// What this window's toolkit can do for a clip's window on the display `event_loop` reached.
pub(crate) fn screens(event_loop: &ActiveEventLoop) -> Screens {
    let placed = !on_wayland(event_loop);
    Screens {
        windows: true,
        placed,
        monitors: [true, placed, true, placed, true, true, true],
    }
}

/// Whether the loop runs on a Wayland compositor, where no client places a window.
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn on_wayland(event_loop: &ActiveEventLoop) -> bool {
    use winit::platform::wayland::ActiveEventLoopExtWayland as _;
    event_loop.is_wayland()
}

/// Whether the loop runs on a Wayland compositor: never, off the Unix platforms.
#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn on_wayland(_event_loop: &ActiveEventLoop) -> bool {
    false
}

/// The window a clip's pictures are shown in, and the surface they are presented through.
pub(crate) struct MediaWindow {
    /// The window.
    window: Arc<Window>,
    /// Its software surface.
    surface: viewer_ui::software::SoftwareSurface,
}

impl std::fmt::Debug for MediaWindow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MediaWindow")
            .field("window", &self.window.id())
            .finish_non_exhaustive()
    }
}

/// winit's monitors as the shared arithmetic reads them, with their handles.
fn monitors(event_loop: &ActiveEventLoop, wayland: bool) -> Vec<(MonitorHandle, Monitor)> {
    let primary = event_loop.primary_monitor();
    event_loop
        .available_monitors()
        .map(|handle| {
            let position = handle.position();
            let size = handle.size();
            // winit states 32 bits for every Wayland output whatever it is, so that is no report.
            let colour_depth = if wayland {
                None
            } else {
                handle.video_modes().map(|mode| mode.bit_depth()).max()
            };
            let described = Monitor {
                name: handle.name().unwrap_or_else(|| "a monitor".to_owned()),
                area: Area {
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                },
                colour_depth,
                primary: primary.as_ref() == Some(&handle),
            };
            (handle, described)
        })
        .collect()
}

/// Where the document window `main` stands — its area where winit can say, and otherwise the
/// monitor it is on among `listed`.
fn document_window(
    main: Option<&Window>,
    listed: &[(MonitorHandle, Monitor)],
) -> (Option<Area>, DocumentWindow) {
    let area = main.and_then(|main| {
        let at = main.outer_position().ok()?;
        let size = main.outer_size();
        Some(Area {
            x: at.x,
            y: at.y,
            width: size.width,
            height: size.height,
        })
    });
    let document = match (area, main) {
        (Some(area), _) => DocumentWindow::At(area),
        (None, Some(main)) => main
            .current_monitor()
            .and_then(|current| listed.iter().position(|(handle, _)| *handle == current))
            .map_or(DocumentWindow::Unknown, DocumentWindow::On),
        (None, None) => DocumentWindow::Unknown,
    };
    (area, document)
}

impl App {
    /// Opens the window the clip in hand asks for where none is open, and closes the one open where
    /// no such clip is in hand — before the loop rests, so a clip held at its start has its window.
    pub(crate) fn keep_the_media_window(&mut self, event_loop: &ActiveEventLoop) {
        let wanted = self
            .video
            .as_ref()
            .and_then(viewer_host::video::VideoPlayer::window)
            .map(|(_, label, window)| (label.to_owned(), window.clone()));
        match (wanted, self.media_window.is_some()) {
            (Some((label, window)), false) => {
                match self.open_the_media_window(event_loop, &label, &window) {
                    Ok(open) => {
                        self.media_window = Some(open);
                        self.present_the_media_window();
                    }
                    Err(refused) => {
                        println!("note: {label}: its pictures are not shown, because {refused}");
                        self.stop_the_pictures();
                    }
                }
            }
            (None, true) => self.media_window = None,
            _ => {}
        }
    }

    /// Makes the window `window` describes for the clip called `label`, or why it cannot be.
    fn open_the_media_window(
        &self,
        event_loop: &ActiveEventLoop,
        label: &str,
        window: &pdf_model::media::Window,
    ) -> Result<MediaWindow, String> {
        let wayland = on_wayland(event_loop);
        let listed = monitors(event_loop, wayland);
        let described: Vec<Monitor> = listed.iter().map(|(_, monitor)| monitor.clone()).collect();
        let main = self.state.as_ref().map(|state| Arc::clone(&state.window));
        let (document_area, document) = document_window(main.as_deref(), &listed);
        let chosen = screens::monitor(window.monitor, &described, document);
        // §13.2.6.1's "windows containing arbitrary media content" are to say they hold one: the
        // title names the clip and its document, after the document's own `/TT` (ADR 3587).
        let mut attributes = Window::default_attributes().with_title(label);
        match &window.kind {
            WindowKind::FullScreen => {
                let index = chosen.map_err(|why| {
                    format!(
                        "Table 304's {} cannot be found: {why}",
                        window.monitor.value()
                    )
                })?;
                println!(
                    "note: {label}: shown full screen on {}, Table 304's {}",
                    described[index].name,
                    window.monitor.value()
                );
                attributes = attributes
                    .with_fullscreen(Some(Fullscreen::Borderless(Some(listed[index].0.clone()))));
            }
            WindowKind::Floating(floating) => {
                if let Some(title) = &floating.title {
                    attributes = attributes.with_title(format!("{title} — {label}"));
                }
                // Table 295's `/R`: 2 lets the reader resize it; 0 does not, and 1 — "only if
                // aspect ratio is preserved" — is honoured by not resizing, since winit keeps no
                // aspect (ADR 3587). `/T` and `/UC` false ask for nothing the reader is denied, so
                // the window keeps its title bar and its close.
                attributes = attributes.with_resizable(floating.resize == Resize::Free);
                let mut inner = floating.size;
                if wayland {
                    println!(
                        "note: {label}: shown in a floating window of {} by {} pixels, which the \
                         compositor places: no Wayland client places a window",
                        inner.0, inner.1
                    );
                } else {
                    // The document window's own frame is the measure of the title bar and
                    // handles Table 295's `/P` places the window with.
                    let frame = main.as_ref().map_or((0, 0), |main| {
                        let outer = main.outer_size();
                        let inside = main.inner_size();
                        (
                            outer.width.saturating_sub(inside.width),
                            outer.height.saturating_sub(inside.height),
                        )
                    });
                    let monitor_index = if floating.relative == Relative::Monitor {
                        chosen.map_err(|why| {
                            format!(
                                "Table 304's {} cannot be found: {why}",
                                window.monitor.value()
                            )
                        })?
                    } else {
                        0
                    };
                    let reference = screens::relative_to(
                        floating.relative,
                        &described,
                        document_area,
                        monitor_index,
                    )?;
                    let placed = screens::place(floating, reference, frame, &described)?;
                    inner = placed.inner;
                    attributes = attributes.with_position(winit::dpi::PhysicalPosition::new(
                        placed.outer.x,
                        placed.outer.y,
                    ));
                    println!(
                        "note: {label}: shown in a floating window of {} by {} pixels at {}, {}",
                        inner.0, inner.1, placed.outer.x, placed.outer.y
                    );
                }
                attributes =
                    attributes.with_inner_size(winit::dpi::PhysicalSize::new(inner.0, inner.1));
            }
        }
        let opened = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|refused| format!("no window was made: {refused}"))?,
        );
        let surface = viewer_ui::software::SoftwareSurface::new(Arc::clone(&opened))
            .map_err(|refused| format!("no surface was made: {refused}"))?;
        Ok(MediaWindow {
            window: opened,
            surface,
        })
    }

    /// Presents what the window of its own shows now, at its size.
    pub(crate) fn present_the_media_window(&mut self) {
        let Some(open) = self.media_window.as_mut() else {
            return;
        };
        let size = open.window.inner_size();
        let Some(video) = self.video.as_ref() else {
            return;
        };
        let drawn = video.window_picture(self.picture.as_ref(), (size.width, size.height));
        let presented = match drawn {
            Some(Ok(raster)) => open
                .surface
                .present(&raster, &[])
                .map_err(|refused| refused.to_string()),
            Some(Err(refused)) => Err(refused),
            None => Ok(()),
        };
        if let Err(refused) = presented {
            println!("note: a clip's picture was not shown in its window, because {refused}");
        }
    }

    /// Takes `event` where `id` is the window of its own a clip is shown in, whose few events are
    /// its own ([`Self::media_window_event`]), and answers whether it was.
    pub(crate) fn media_window_took(&mut self, id: WindowId, event: &WindowEvent) -> bool {
        let took = self
            .media_window
            .as_ref()
            .is_some_and(|open| open.window.id() == id);
        if took {
            self.media_window_event(event);
        }
        took
    }

    /// An event of the window of its own: closing it or Escape stops the clip, Space resumes it,
    /// and a size or a redraw presents it again.
    fn media_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.media_window = None;
                self.stop_the_pictures();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        ..
                    },
                ..
            } => match logical_key.as_ref() {
                Key::Named(NamedKey::Escape) => {
                    self.stop_the_pictures();
                }
                Key::Named(NamedKey::Space) => {
                    self.resume_the_pictures();
                }
                _ => {}
            },
            WindowEvent::Resized(_) | WindowEvent::RedrawRequested => {
                self.present_the_media_window();
            }
            _ => {}
        }
    }
}
