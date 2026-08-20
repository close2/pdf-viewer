//! winit's side of the program: the window, the event loop's three callbacks, and the key table.
//!
//! Everything the toolkit calls is here and nothing else is, which is what makes the rest of this
//! program readable without knowing winit: an event arrives, is named, and is handed to the
//! module that owns the answer. The key bindings sit beside it because a binding is only half a
//! decision until you can see which keys the handler answers before it ever reaches the table.

use std::sync::Arc;

use viewer_core::{Command, Edit, Find, FocusMove, PageTarget, PointerAction, Selection, Zoom};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::app::App;
use crate::presentation::PRESENTATION_TICK;
use crate::surface::State;
use crate::trace::{Topic, describe_window_event};
use crate::typing::Drawing;

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
            self.aim_at_field(at);
            // And §12.7.5.2's other kind of press, which takes no keyboard: a click on a check
            // box or a radio button is what *gives* it a value. Nothing happens where the press
            // was not on one.
            self.toggle_button(at);
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
        if presentation.playing.is_some() {
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
            presentation.wake = now.checked_add(PRESENTATION_TICK).unwrap_or(now);
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

    #[expect(
        clippy::too_many_lines,
        reason = "every window event this host answers, in one match — which is where a reader \
                  looking for \"what does this program do with a click\" should find them all"
    )]
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
            } => {
                if matches!(logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    // **A field with the keyboard takes this key first**, which is what ADR 0201
                    // decided and what this branch quietly defeated: the press exited the program
                    // before `typed` was ever asked, so the one binding typing changes the meaning
                    // of was dead code from the round that wrote it. Found by reading the two
                    // against each other in the three-hundred-and-seventy-first session, because
                    // no gate in this tree presses a key twice in one window.
                    if self.typing.is_some() && self.typed(&logical_key.as_ref()) {
                        return;
                    }
                    event_loop.exit();
                    return;
                }
                // **A field being typed into takes every character key**, which is what makes `+`
                // a plus sign there and a magnification everywhere else — and what this branch
                // defeated for two of them in exactly the shape Escape's did: an `o` typed into a
                // field toggled the sidebar and a `?` opened the About card, because both were
                // answered before `typed` was ever asked. Escape's copy of this defect was found
                // in the three-hundred-and-seventy-first session and these two survived it to the
                // three-hundred-and-eighty-eighth, which is the reason `keys_reach_the_field`
                // now presses one of them at a field rather than trusting the order.
                if self.typing.is_some() && self.typed(&logical_key.as_ref()) {
                    return;
                }
                // **The find bar takes every key while it is open**, for the reason the field
                // above does and in the same place: a `/` typed into a search string is a
                // slash, and answering `o`, `p` or `c` first would be the defect ADR 0201 found
                // twice already. Opening it is a key this host answers itself — whether a bar is
                // on the screen is chrome, and `viewer-core` has no opinion about chrome (rule 5).
                if self.find.shown && self.find_key(&logical_key.as_ref()) {
                    return;
                }
                if matches!(logical_key.as_ref(), Key::Character("/")) {
                    let shown = self.find.toggle();
                    if !shown {
                        self.pages_left = 0;
                        self.dispatch(Command::Find(Find::Stop));
                    }
                    self.redraw();
                    return;
                }
                // The two keys this program answers itself rather than by sending a command:
                // whether a panel is shown is chrome, and `viewer-core` has no opinion about
                // chrome by construction (rule 5).
                if matches!(logical_key.as_ref(), Key::Character("o")) {
                    self.panel.toggle();
                    self.resize_page();
                    return;
                }
                if matches!(logical_key.as_ref(), Key::Character("?")) {
                    self.about.toggle();
                    self.redraw();
                    return;
                }
                // §12.4.4's presentation, and the third key this program answers itself: whether
                // a clock is running is a fact about this host and not about the document
                // (ADR 0135), so there is no command for it.
                if matches!(logical_key.as_ref(), Key::Character("p")) {
                    self.present_or_stop();
                    return;
                }
                // §14.8.2.5, and the fifth key this program answers itself: what a copy *is* —
                // a clipboard — belongs to the platform, so `viewer-core` has no command for it
                // and only the two orders to answer with. Ctrl and no Ctrl reach the same arm,
                // because a field being typed into took this key above (`clipped`) and what is
                // left is a page, where the modifier changes nothing.
                if matches!(logical_key.as_ref(), Key::Character("c")) {
                    self.copy_selection();
                    return;
                }
                // §12.5.6.6, and the sixth: whether the next drag draws a text box is a mode this
                // host is in, and `viewer-core` has no opinion about chrome by construction
                // (rule 5). The command goes out on the *release*, with both corners.
                if matches!(logical_key.as_ref(), Key::Character("f")) {
                    self.drawing = if self.drawing.is_some() {
                        println!("note: not drawing a free text annotation after all");
                        None
                    } else {
                        println!(
                            "note: drag out a rectangle for a free text annotation (§12.5.6.6)"
                        );
                        Some(Drawing::Armed)
                    };
                    return;
                }
                // Table 29's six arrangements, and the seventh key this program answers itself:
                // §7.7.2 states the layout a document *opens* in, so which of the six a reader
                // has moved on to is a fact about this window and the command to send depends on
                // which one is in force. The other two hosts bind the same letter (ADR 0442).
                if matches!(logical_key.as_ref(), Key::Character("l")) {
                    self.cycle_layout();
                    return;
                }
                // Everything else goes to the page, and the About card is over it: a key press
                // that turned a page nobody can see would be answering the wrong question.
                if self.about.shown {
                    return;
                }
                let Some(command) = key_command(&logical_key.as_ref(), self.shift) else {
                    return;
                };
                // §12.5.6.10's markups are defined over selected text, so a press with nothing
                // selected asks for an annotation over nothing. `viewer-core` answers by doing
                // nothing, which is right and silent — and a person who pressed a key and saw no
                // change has been told nothing at all (trap 5). The host has the selection
                // already, because it draws it.
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

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                self.pointer_moved();
            }

            WindowEvent::MouseInput {
                state: element,
                button: MouseButton::Left,
                ..
            } => {
                if self.about.shown {
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

/// What a key press asks for, where it asks for anything.
///
/// One place rather than an arm apiece inside the event handler, because this is the whole of
/// this program's key bindings and a reader looking for them should find them together.
fn key_command(key: &Key<&str>, shift: bool) -> Option<Command> {
    Some(match *key {
        // §12.5.1 names this key: "[i]nteractive PDF processors may permit the user to navigate
        // through the annotations on a page by using the keyboard (in particular, the tab key)".
        // The *order* is the document's, in `pdf_model::tab_order`; shift is the only thing that
        // separates the two directions, because winit reports one key for both.
        Key::Named(NamedKey::Tab) => Command::Focused(if shift {
            FocusMove::Previous
        } else {
            FocusMove::Next
        }),
        Key::Named(NamedKey::ArrowRight | NamedKey::PageDown | NamedKey::Space) => {
            Command::GoTo(PageTarget::Next)
        }
        Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => Command::GoTo(PageTarget::Previous),
        Key::Named(NamedKey::Home) => Command::GoTo(PageTarget::First),
        Key::Named(NamedKey::End) => Command::GoTo(PageTarget::Last),
        // No anchor: a keyboard names no point, so the core holds the viewport's centre.
        Key::Character("+" | "=") => Command::Zoom {
            zoom: Zoom::In,
            at: None,
        },
        Key::Character("-") => Command::Zoom {
            zoom: Zoom::Out,
            at: None,
        },
        Key::Character("0") => Command::Zoom {
            zoom: Zoom::FitPage,
            at: None,
        },
        Key::Character("a") => Command::Select(Selection::All),
        Key::Character("s") => Command::Save,
        // §12.5.6.10 over what is selected. Four subtypes and one key apiece would be four
        // bindings a person has to learn; this host offers the one a person means by "mark
        // this" and leaves the other three to a host with a menu. The colour is this host's
        // choice — the standard states none, Table 166's `/C` simply carries what a processor
        // was told — and a soft yellow is what a highlighter is.
        Key::Character("h") => Command::Edit(Edit::Markup {
            kind: pdf_model::view::Markup::Highlight,
            colour: [1.0, 0.9, 0.2],
        }),
        // The same mark struck through rather than washed over, because a person marking up a
        // draft means both and the two are one construction in `pdf-model`.
        Key::Character("k") => Command::Edit(Edit::Markup {
            kind: pdf_model::view::Markup::StrikeOut,
            colour: [0.85, 0.15, 0.15],
        }),
        // A page taller than the window: the scroll is in device pixels, so this is about a
        // fifteenth of a fitted A4 page and the same on any display.
        Key::Named(NamedKey::ArrowDown) => Command::Scroll { dx: 0.0, dy: 60.0 },
        Key::Named(NamedKey::ArrowUp) => Command::Scroll { dx: 0.0, dy: -60.0 },
        _ => return None,
    })
}
