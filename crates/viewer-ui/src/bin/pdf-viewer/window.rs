//! winit's side of the program: the window, the event loop's three callbacks, and the keyboard.
//!
//! Everything the toolkit calls is here and nothing else is, which is what makes the rest of this
//! program readable without knowing winit: an event arrives, is named, and is handed to the
//! module that owns the answer.
//!
//! **The key *table* is no longer here, and that is the point of ADR 0526.** What this module
//! holds is the two halves a toolkit genuinely owns — [`press`], which turns a
//! `winit::keyboard::Key` into the key [`viewer_host::keys`] states a meaning for, and
//! [`App::pressed`], which decides in what order this window's chrome claims a press before the
//! page ever sees it. What a press *means* is the same value in all three hosts.

use std::sync::Arc;

use viewer_core::{Command, Edit, Find, PointerAction};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::app::App;
use crate::surface::State;
use crate::trace::{Topic, describe_window_event};
use crate::typing::Drawing;
use viewer_host::Clock;

impl App {
    /// One end of a click on the page, at a point of the page's own viewport.
    ///
    /// **What a click *is* on this host, in one place rather than in the mouse handler.** It is
    /// three things and the order matters: §12.5.1's activation aims the keyboard where a field
    /// takes one, §12.7.5.2's toggling gives a check box or a radio button its value, and
    /// `Command::Pointer` is what the core sees — §12.6.3's triggers, §12.5.5's appearance state,
    /// §12.5.6.5's link, and the anchor a selection is dragged from.
    ///
    /// It is a method of its own because a mouse is no longer the only thing that clicks: an
    /// assistive technology asking `org.a11y.atspi.Action` for a click on a node reaches
    /// [`App::act`], which sends a press and a release here. Two callers of one definition, so a
    /// screen reader's click cannot become a *different* click from a person's by drifting from
    /// it (ADR 0425).
    pub(crate) fn click_page(&mut self, at: (f32, f32), element: ElementState) {
        if element == ElementState::Pressed {
            // **§12.7.5.4's open list claims the press first**, which is the same ordering the
            // key handler applies to a modal card: a control drawn over the page is between the
            // pointer and the page, so a press on it is not a press on what is underneath.
            if self.press_on_choices(at) {
                return;
            }
            self.aim_at_field(at);
            // And the other half of §12.7.5.4, which this host had no way to do at all until the
            // seven-hundred-and-seventeenth session: a press on a choice field lists its options,
            // because picking one is what the clause's two controls are *for* and typing a value
            // is what Table 233 bit 19 permits for one of them.
            if self.open_choices(at) {
                return;
            }
            // And §12.7.5.2's other kind of press, which takes no keyboard: a click on a check
            // box or a radio button is what *gives* it a value. Nothing happens where the press
            // was not on one.
            self.toggle_button(at);
        } else if !self.dragging {
            // The press this release pairs with went to §12.7.5.4's list, which is *over* the
            // page, so the core never saw one. Sending the release alone would be a §12.6.3
            // mouse-up with no mouse-down under it, ending a selection drag nobody started.
            return;
        }
        self.dragging = element == ElementState::Pressed;
        self.dispatch(Command::Pointer {
            at,
            action: match element {
                ElementState::Pressed => PointerAction::Pressed,
                ElementState::Released => PointerAction::Released,
            },
        });
    }
}

impl ApplicationHandler for App {
    /// An assistive technology asked for something, on a thread this loop is not in.
    ///
    /// The only user event this program has, and it carries nothing: `viewer_accessibility`'s
    /// queue holds the requests and this is only what makes the loop come round to read them. A
    /// window resting in [`ControlFlow::Wait`] would otherwise carry a screen reader's request out
    /// at the next unrelated event, which for a person using only a screen reader is never.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, (): ()) {
        self.act();
        // A wake from another thread means somebody has news for the next tick — the
        // accessibility bridge's request, or the render thread's sharp picture (ADR 0699)
        // — and a loop at rest would otherwise sit on it until the next input.
        self.redraw();
    }

    /// The percentiles, printed on the way out.
    ///
    /// **The shape of the question "why did it feel slow" is a distribution**, and the trace
    /// that raised ADR 0227 had 63 frame lines and no way to say that their median was
    /// 60 ms and their worst 514 without a spreadsheet. Here because it costs nothing per frame
    /// and everything it needs is already recorded.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.frames.summary(
            self.trace,
            self.stale.count(),
            self.stale.refusals(),
            &self.cadence,
        );
    }

    /// Keeps the presenter's clock and §12.4.4's, and only while there is one to keep.
    ///
    /// **Four speeds, and the idle one is still the default.** A window reading a document waits
    /// for an event, which is what `main` sets and what a viewer is doing almost all of the time.
    /// A window that owes a frame — a reprojection to replace, a redraw the clock deferred, a
    /// §12.4.4 transition in flight — wakes on the *surface's* cadence ([`crate::cadence`]). A
    /// presentation between transitions wakes ten times a second, which is enough to notice a
    /// `/Dur` stated in seconds. A search polls, because a page read is not a frame and the
    /// cadence has no business slowing one down.
    ///
    /// **`doc/todo/36`'s fourth rule is this method's `Wait`**, and it is worth saying which line
    /// enforces it: every branch below that does not owe a frame leaves the loop waiting for an
    /// event, so a still window spends no tick, wakes for nothing and presents nothing. The rate
    /// is a ceiling on latency and never a duty to draw.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // **A reprojection may not be what this loop comes to rest on** (`doc/todo/37` rule 1,
        // ADR 0378), and since `doc/todo/36` the frame that replaces one arrives *on the
        // cadence* rather than at once. The difference is the whole feature: an immediate
        // redraw handed the event thread to a render that takes fifty refreshes, so a view
        // that kept moving could not be answered a second time, while a tick lets the next
        // reprojection stand in for it and the real frame follow when nothing new is asked.
        //
        // It cannot spin — every frame that is not a reprojection clears the flag, including a
        // frame that drew nothing at all, and `Cadence::presented` moves the tick on.
        let owes_frame = self.stale.showing_approximation() || self.cadence.owes();
        if owes_frame {
            let now = std::time::Instant::now();
            if self.cadence.due(now) {
                self.redraw();
            }
        }
        // One page of the search, once per turn round the loop. This is where the choice in
        // `viewer_core::search` is paid for on this host: the core reads one page per command and
        // has no thread to read a thousand on, so the loop that would otherwise be idle drives it
        // and the window keeps drawing while it does.
        //
        // **After the redraw above and before the wait below**, deliberately: a page read is not
        // a frame, so pacing it to the display would take a thousand-page search from the loop's
        // own speed down to sixty pages a second — while returning before the redraw was asked
        // for would leave a reprojection on the screen for the length of the search, which is
        // rule 1's failure.
        if self.pages_left > 0 {
            self.dispatch(Command::Find(Find::Continue));
            event_loop.set_control_flow(ControlFlow::Poll);
            return;
        }
        if owes_frame {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.cadence.next()));
            return;
        }
        let Some(presentation) = self.presentation.as_mut() else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };
        if presentation.clock.animating() {
            // §12.4.4's transition is an animation, and since `doc/todo/36` it is an animation on
            // the *surface's* clock rather than on `ControlFlow::Poll`. It drew as fast as the
            // loop could go, which on a fast device is frames nobody sees and a core at 100% for
            // the length of the transition; the cadence gives it one frame per refresh, which is
            // every frame a person can see and no more.
            let now = std::time::Instant::now();
            if self.cadence.due(now) {
                self.redraw();
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.cadence.next()));
            return;
        }
        let now = std::time::Instant::now();
        let due = now >= presentation.wake;
        if due {
            presentation.wake = now.checked_add(Clock::RESTING).unwrap_or(now);
        }
        let wake = presentation.wake;
        if due {
            self.redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(wake));
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 1000.0));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("window creation"),
        );

        self.launch.mark("window");

        // The cadence is the surface's, read here because this is the first moment there is a
        // surface to ask (`doc/todo/36`'s third unsettled question). Nothing on the launch path
        // waits for it and nothing fails without it: a display that states no refresh rate takes
        // the floor, and the trace says which of the two this run got.
        self.cadence = crate::cadence::Cadence::of(&window);
        self.trace.say(
            Topic::Launch,
            format_args!("presenting on a cadence of {}", self.cadence.described()),
        );

        let size = window.inner_size();
        let Some(surface) = self.bring_up(&window) else {
            // Nothing can put pixels on this window: said above, in a sentence, and not survived.
            // An event loop that runs with nothing to present to shows a blank window for ever,
            // which is the failure this program spends its rounds removing.
            std::process::exit(1);
        };

        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        self.state = Some(State {
            window,
            surface,
            size: (size.width.max(1), size.height.max(1)),
        });

        // **The document's thread is joined here and not a line earlier.** Everything above this
        // — the event loop, the window, the instance, the device — is what it was running beside,
        // and joining after the presenter exists is what makes the two costs the *longer* of the
        // pair rather than the sum. If the mark below reads a few hundred microseconds after
        // `graphics device`, the document was ready and waiting; if it reads milliseconds later,
        // this thread waited, which is a document large enough for the overlap to have been worth
        // more than it took.
        if let Some(opening) = self.opening.take() {
            let (viewer, events) = opening.join().expect("the thread opening the document");
            self.viewer = viewer;
            self.launch.mark("document joined");
            self.receive(events);
        }
        self.retitle();
        // The window's size is the first thing the core has been told about the viewport, and
        // it is what makes page one render. **Less the sidebar**, which Table 29's `/PageMode`
        // may already have opened: the document was opened before this window existed, so the
        // first `Resize` is the first chance to say how much of it the page has.
        self.dispatch(Command::Resize {
            width: size.width.saturating_sub(self.panel.inset(scale)).max(1),
            height: size.height.max(1),
            scale,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.state.is_none() {
            return;
        }
        // Every window event but the pointer's, which arrives faster than a person can read and
        // which `pump` prints under `pointer` a moment later as the command it becomes — one
        // line for the movement rather than two. What this answers is the question a stuck
        // window raises first: *is the program being told anything at all?*
        if self.trace.on(Topic::Window) && !matches!(event, WindowEvent::CursorMoved { .. }) {
            self.trace.say(
                Topic::Window,
                format_args!("window event {}", describe_window_event(&event)),
            );
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        ..
                    },
                ..
            } => self.pressed(&logical_key.as_ref()),

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                self.pointer_moved();
            }

            WindowEvent::MouseInput {
                state: element,
                button: MouseButton::Left,
                ..
            } => {
                // Both modal cards take the pointer as well as the keyboard: a click that
                // followed a link under §7.6.4.1's prompt would be acting on a document nobody has
                // authenticated.
                if self.about.shown || self.password.shown {
                    return;
                }
                if self.over_panel() {
                    // Answered once, on the press: a panel that acted on both ends of a click
                    // would follow a destination twice.
                    if element == ElementState::Pressed {
                        self.click_panel();
                    }
                    return;
                }
                // §12.5.1's activation, for the one subtype that takes a keyboard: a press
                // inside a text field's rectangle is how a person says "type here". The core
                // already raises §12.6.3's focus events from the same press; what this adds is
                // the host's own state, because *where the keys go* is chrome and `viewer-core`
                // has no opinion about chrome by construction (rule 5).
                // §12.5.6.6's geometry is a *drag*, which is the whole of what this mode adds:
                // the press puts one corner down and the release sends both. It runs before the
                // two below because while it is armed the press means nothing else — a person who
                // has said "draw a box here" has not said "type in the field underneath it".
                if let Some(drawing) = self.drawing {
                    self.draw_free_text(drawing, element);
                    self.dragging = element == ElementState::Pressed;
                    return;
                }
                self.click_page(self.on_page(self.cursor), element);
            }

            WindowEvent::Resized(size) => {
                let scale = self.state.as_ref().map_or(1.0, |state| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a display's scale factor is a small ratio"
                    )]
                    let scale = state.window.scale_factor() as f32;
                    scale
                });
                if let Some(state) = self.state.as_mut() {
                    let extent = (size.width.max(1), size.height.max(1));
                    state.size = extent;
                    // **The presenter is told every time the window system speaks**, which is
                    // what quorra's `Presenter::resize` asks of a host: it configures nothing and
                    // the swapchain follows at the next present, so calling it with a size it
                    // already had costs a field write. A presenter that was never told refuses by
                    // name, and a minimised window is a state rather than an error.
                    if let crate::surface::Surface::Device(window) = &mut state.surface {
                        window.resize(extent.0, extent.1);
                    }
                }
                self.dispatch(Command::Resize {
                    width: size.width.saturating_sub(self.panel.inset(scale)).max(1),
                    height: size.height.max(1),
                    scale,
                });
                // A resize moves the *inner* rectangle inside the outer one, so AT-SPI's screen
                // coordinates change with it. This and `Moved` are the two events that can move
                // them; a page turn is not one, which is what took it off that path (ADR 0228).
                self.place_window();
            }

            // See `Resized`: the window's place on the screen is asked for where it can change.
            WindowEvent::Moved(_) => self.place_window(),

            WindowEvent::MouseWheel { delta, .. } => self.wheel(delta),

            // Remembered rather than read at the wheel, because winit puts no modifier state in
            // the wheel's own event.
            WindowEvent::ModifiersChanged(modifiers) => {
                self.control = modifiers.state().control_key();
                self.shift = modifiers.state().shift_key();
            }

            WindowEvent::RedrawRequested => self.redraw_requested(),

            _ => {}
        }
    }
}

impl App {
    /// A key press that reached the window, in the order the chrome over the page claims it.
    ///
    /// **The ordering is this host's and the meaning is not** (ADR 0526). Which widget has the
    /// keyboard is a fact about a window that no shared value can know, so the three claims below
    /// are decided here; what a press means once it has got past them is
    /// [`viewer_host::keys::meaning`], which the other two hosts ask the same question of.
    fn pressed(&mut self, key: &Key<&str>) {
        // **A field being typed into takes every key, Escape included**, which is what ADR 0201
        // decided and what an earlier version of this handler quietly defeated three times: Escape
        // exited the program, `o` toggled the sidebar and `?` opened the notices card before
        // `typed` was ever asked. `keys_reach_the_field` presses one of them at a field rather
        // than trusting the order.
        if self.typing.is_some() && self.typed(key) {
            return;
        }
        // **An open list takes the key that dismisses it and nothing else.** It is a control over
        // the page rather than a modal card, so a person who has opened one has not stopped
        // reading the document — but Escape has to reach it before the table turns Escape into
        // "clear the selection", or the only way to close a list would be to press somewhere else.
        if self.choosing.is_some()
            && matches!(key, Key::Named(NamedKey::Escape))
            && self.close_choices()
        {
            return;
        }
        // **§7.6.4.1's card takes every key while it is up**, and it takes them before the find
        // bar and before the page: the document behind it is not open, so there is nothing for any
        // other key to be about. It is checked after the field above only because the two are never
        // up together — a document nobody has authenticated has no §12.7 field to type into.
        if self.password.shown {
            self.password_key(key);
            return;
        }
        // **The find bar takes every key while it is open**, for the same reason and in the same
        // place: a `/` typed into a search string is a slash. Whether a bar is on the screen is
        // chrome, and `viewer-core` has no opinion about chrome by construction (rule 5).
        if self.find.shown && self.find_key(key) {
            return;
        }
        let stated = press(key);
        // **And the notices card is modal**, so the only keys that reach the page while it is up
        // are the two that take it down. A key press that turned a page nobody can see would be
        // answering the wrong question.
        if self.about.shown {
            if matches!(
                stated,
                Some(viewer_host::Key::Question | viewer_host::Key::Escape)
            ) {
                self.about.toggle();
                self.redraw();
            }
            return;
        }
        let Some(stated) = stated else { return };
        let mode = if self.presenting.full_screen() {
            viewer_host::Mode::Presenting
        } else {
            viewer_host::Mode::Reading
        };
        let waiting = self.waiting();
        let Some(meaning) = viewer_host::meaning(stated, self.shift, mode, waiting) else {
            return;
        };
        match meaning {
            viewer_host::Meaning::Send(command) => self.send(command),
            viewer_host::Meaning::Window(act) => self.window_act(act),
        }
    }

    /// A message the key table produced whole, on its way to the core.
    ///
    /// One thing happens between the table and [`App::dispatch`] and it is trap 5's: §12.5.6.10's
    /// markups are defined over selected text, so a press with nothing selected asks for an
    /// annotation over nothing. The core answers by doing nothing, which is right and silent — and
    /// a person who pressed a key and saw no change has been told nothing at all. The host has the
    /// selection already, because it draws it.
    fn send(&mut self, command: Command) {
        if matches!(command, Command::Edit(Edit::Markup { .. })) && !self.has_selection() {
            println!("note: select some text first — §12.5.6.10's markups mark up text");
            return;
        }
        let walked = matches!(command, Command::Focused(_));
        self.dispatch(command);
        if walked {
            self.aim_at_focus();
        }
    }

    /// The half of the key table that is this window's rather than the document's.
    ///
    /// Matched exhaustively and with no catch-all arm, which is `doc/ui-boundary.md`'s rule
    /// applied one layer out: a binding added to [`viewer_host::keys`] has to fail to compile in
    /// all three hosts, or the level-hosts decision is a sentence with no instrument again.
    fn window_act(&mut self, act: viewer_host::WindowAct) {
        match act {
            // Logical pixels out of the table and device pixels into the command, which is the
            // whole reason the table states a scroll rather than building the message itself.
            viewer_host::WindowAct::ScrollBy(by) => self.dispatch(Command::Scroll {
                dx: 0.0,
                dy: by * self.scale(),
            }),
            viewer_host::WindowAct::Copy => self.copy_selection(),
            // Only ever *opens*. While the bar is shown it has the keyboard, so neither `f` nor
            // `/` reaches the table at all (see [`App::pressed`]), and Escape inside the bar is
            // what closes it and sends `Find::Stop`.
            viewer_host::WindowAct::Find => {
                if !self.find.shown {
                    self.find.toggle();
                    self.redraw();
                }
            }
            viewer_host::WindowAct::Panel => {
                self.panel.toggle();
                self.resize_page();
            }
            viewer_host::WindowAct::Notices => {
                self.about.toggle();
                self.redraw();
            }
            viewer_host::WindowAct::Present => self.present_or_stop(),
            viewer_host::WindowAct::LeaveFullScreen => {
                self.leave_full_screen();
            }
            viewer_host::WindowAct::NextLayout => self.cycle_layout(),
            // **The one binding this host answers by saying why it has nothing to do**, and it is
            // a fact about the tier rather than a gap. `viewer_host::ControlFit` compares a
            // *toolkit's* minimum size against the `/Rect` the document states, and this host
            // sends no `Command::Delegate` and places no toolkit control: what it draws is the
            // widget's own appearance stream, which is inside that rectangle by construction. So
            // the answer is the same one the native hosts give for a page whose controls all fit.
            viewer_host::WindowAct::FitControls => println!(
                "note: every §12.7 control on this page already fits its /Rect — this host draws \
                 the widget's own appearance rather than placing a toolkit control, so there is \
                 no minimum size to magnify for"
            ),
            // §12.5.6.6: whether the next drag draws a text box is a mode this host is in, and
            // `viewer-core` has no opinion about chrome by construction (rule 5). The command
            // goes out on the *release*, with both corners.
            viewer_host::WindowAct::AbortDrawing => self.stop_the_long_draw(),
            viewer_host::WindowAct::FreeText => {
                self.drawing = if self.drawing.is_some() {
                    println!("note: not drawing a free text annotation after all");
                    None
                } else {
                    println!("note: drag out a rectangle for a free text annotation (§12.5.6.6)");
                    Some(Drawing::Armed)
                };
            }
        }
    }

    /// This window's display scale, or one where there is no window yet.
    fn scale(&self) -> f32 {
        self.state.as_ref().map_or(1.0, |state| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a display's scale factor is a small ratio"
            )]
            let scale = state.window.scale_factor() as f32;
            scale
        })
    }
}

/// winit's key as the one [`viewer_host::keys`] states a meaning for, or nothing.
///
/// **This is the whole of what this host contributes to its key bindings**, which is the point of
/// ADR 0526: `winit::keyboard::Key` against `gdk::Key` against `Qt::Key` is what a toolkit is, and
/// what a press *means* is not. A letter arrives folded to lower case because none of the letters
/// the table binds means a second thing when shifted, and winit reports the shifted character.
fn press(key: &Key<&str>) -> Option<viewer_host::Key> {
    use viewer_host::Key as Stated;
    Some(match *key {
        Key::Named(NamedKey::Escape) => Stated::Escape,
        Key::Named(NamedKey::Tab) => Stated::Tab,
        Key::Named(NamedKey::Space) => Stated::Space,
        Key::Named(NamedKey::Home) => Stated::Home,
        Key::Named(NamedKey::End) => Stated::End,
        Key::Named(NamedKey::ArrowLeft) => Stated::Left,
        Key::Named(NamedKey::ArrowRight) => Stated::Right,
        Key::Named(NamedKey::ArrowUp) => Stated::Up,
        Key::Named(NamedKey::ArrowDown) => Stated::Down,
        Key::Named(NamedKey::PageUp) => Stated::PageUp,
        Key::Named(NamedKey::PageDown) => Stated::PageDown,
        Key::Character(text) => return character(text),
        _ => return None,
    })
}

/// The character keys, from whatever a layout produced.
///
/// Separate from [`press`] because winit reports these as text rather than as names, so the match
/// is on a string and the fold to lower case has to happen somewhere.
fn character(text: &str) -> Option<viewer_host::Key> {
    use viewer_host::Key as Stated;
    let mut characters = text.chars();
    let (first, rest) = (characters.next()?, characters.next());
    if rest.is_some() {
        // A dead key's composition or an input method's phrase. Nothing this table binds is more
        // than one character, and a page turn on the first letter of somebody's word would be a
        // key press this program invented.
        return None;
    }
    Some(match first.to_ascii_lowercase() {
        'a' => Stated::A,
        'c' => Stated::C,
        'f' => Stated::F,
        'h' => Stated::H,
        'k' => Stated::K,
        'l' => Stated::L,
        'o' => Stated::O,
        'p' => Stated::P,
        's' => Stated::S,
        't' => Stated::T,
        'w' => Stated::W,
        'y' => Stated::Y,
        'z' => Stated::Z,
        '0' => Stated::Zero,
        '+' => Stated::Plus,
        '-' => Stated::Minus,
        '=' => Stated::Equals,
        '/' => Stated::Slash,
        '?' => Stated::Question,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::press;
    use winit::keyboard::{Key, NamedKey};

    /// Every key the shared table states has a `winit` key in this host.
    ///
    /// **This is the instrument the level-hosts decision never had** (ADR 0526). The match below
    /// is exhaustive over [`viewer_host::Key`], so a binding added to `viewer-host` fails to
    /// compile here until this host says which key produces it; and the assertion then checks that
    /// the *runtime* translation agrees, so a key named here and forgotten in [`press`] fails
    /// rather than drifting. `viewer-gtk` and `viewer-qt` carry the same test against their own
    /// toolkits.
    #[test]
    fn every_key_the_table_states_has_one_in_this_toolkit() {
        use viewer_host::Key as Stated;
        for stated in Stated::ALL {
            let key: Key<&str> = match stated {
                Stated::A => Key::Character("a"),
                Stated::C => Key::Character("c"),
                Stated::F => Key::Character("f"),
                Stated::H => Key::Character("h"),
                Stated::K => Key::Character("k"),
                Stated::L => Key::Character("l"),
                Stated::O => Key::Character("o"),
                Stated::P => Key::Character("p"),
                Stated::S => Key::Character("s"),
                Stated::T => Key::Character("t"),
                Stated::W => Key::Character("w"),
                Stated::Y => Key::Character("y"),
                Stated::Z => Key::Character("z"),
                Stated::Zero => Key::Character("0"),
                Stated::Plus => Key::Character("+"),
                Stated::Minus => Key::Character("-"),
                Stated::Equals => Key::Character("="),
                Stated::Slash => Key::Character("/"),
                Stated::Question => Key::Character("?"),
                Stated::Escape => Key::Named(NamedKey::Escape),
                Stated::Tab => Key::Named(NamedKey::Tab),
                Stated::Space => Key::Named(NamedKey::Space),
                Stated::Home => Key::Named(NamedKey::Home),
                Stated::End => Key::Named(NamedKey::End),
                Stated::Left => Key::Named(NamedKey::ArrowLeft),
                Stated::Right => Key::Named(NamedKey::ArrowRight),
                Stated::Up => Key::Named(NamedKey::ArrowUp),
                Stated::Down => Key::Named(NamedKey::ArrowDown),
                Stated::PageUp => Key::Named(NamedKey::PageUp),
                Stated::PageDown => Key::Named(NamedKey::PageDown),
            };
            assert_eq!(
                press(&key),
                Some(*stated),
                "{stated:?} is stated by the table and this host does not produce it"
            );
        }
    }

    /// A shifted letter is the same key, because none of them means a second thing shifted.
    #[test]
    fn a_capital_letter_is_the_same_key_as_its_lower_case() {
        assert_eq!(
            press(&Key::Character("A")),
            press(&Key::Character("a")),
            "winit reports the shifted character and the table binds the letter"
        );
    }

    /// An input method's phrase is not a key press this program invents a meaning for.
    #[test]
    fn a_composed_phrase_turns_no_page() {
        assert_eq!(press(&Key::Character("ss")), None);
        assert_eq!(press(&Key::Character("")), None);
    }
}
