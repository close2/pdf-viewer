//! What a person does to the window: keys, the pointer and the wheel, the selection and the clipboard, §12.9's measuring, and the window's acts.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Answer, Command, PointerAction, Query, Zoom};

use viewer_host::arrangement::next_layout;
use viewer_host::native_window::NativeWindow;
use viewer_host::trace::Topic;

use crate::controls;

use super::{Host, with};

impl Host {
    /// What a key press means, which is [`viewer_host::keys`]'s answer and not this host's.
    ///
    /// **Only the translation is GTK's** (ADR 0526). This host used to carry a table of its own
    /// and it disagreed with the other two about the arrow keys, about `f` and about Escape; what
    /// is left here is [`key_pressed`] turning a `gdk::Key` into a [`viewer_host::Key`] and
    /// [`Host::window_act`] doing the half of the table that is a widget's rather than a message.
    ///
    /// Answers whether the press was taken, so that the window's own bindings do not act on it as
    /// well: GTK moves the keyboard focus on an arrow key no widget used, and the Left that turned
    /// the page also gave the keyboard to the outline, whose list then took Home, `+` and `-`
    /// (ADR 1453). §12.5.1's Tab is the exception and answers `false`: GTK's focus move for it is
    /// what [`Self::follow_focus`] corrects afterwards.
    fn key(&mut self, key: gtk4::gdk::Key, held: viewer_host::Modifiers) -> bool {
        let Some(stated) = key_pressed(key) else {
            return false;
        };
        // When the key reached this window, against the `events` line its command prints once
        // handled: the length of the handler, which a page turn no longer spends reading (ADR
        // 2344).
        self.state
            .trace
            .say(Topic::Events, format_args!("key {stated:?} arrives"));
        // A push-button §12.5.1's focus is on takes Space and Enter before the page does, since no
        // toolkit button stands over one to take them (ADR 1357) — unless one of this host's own
        // controls has the keyboard, which has had the key first and whose Enter GTK may pass on.
        let keyboard = if self.a_control_has_the_keyboard() {
            viewer_host::Keyboard::Control
        } else {
            viewer_host::Keyboard::Page
        };
        let mode = viewer_host::Mode::of(&self.state.presenting);
        let waiting = self.waiting();
        match viewer_host::decide(&self.state.viewer, stated, held, mode, waiting, keyboard) {
            viewer_host::Decided::Activate { name, annotation } => {
                self.state.trace.say(
                    Topic::Pointer,
                    format_args!("pressing the push-button {}", name.shown()),
                );
                self.dispatch(Command::Activate(annotation));
                true
            }
            viewer_host::Decided::Say(said) => {
                self.say(&said);
                true
            }
            viewer_host::Decided::Send(command) => {
                let walked = matches!(command, Command::Focused(_));
                self.dispatch(command);
                if walked {
                    self.follow_focus();
                }
                !walked
            }
            viewer_host::Decided::Window(act) => {
                self.window_act(act);
                true
            }
            viewer_host::Decided::Nothing => false,
        }
    }

    /// Gives GTK's keyboard to whatever §12.5.1's tab walk just landed on.
    ///
    /// GTK moves its own focus on the same Tab — to a toolbar button or a row of the panel — and a
    /// focused GTK widget takes Space and Enter before this window's key table is asked. So where
    /// the walk lands on a control this host placed, that control gets the keyboard; anywhere else,
    /// including a push-button the page draws itself (ADR 1357), the window's focus is cleared so
    /// that the next Space or Enter reaches [`viewer_host::pressed`]. Done from an idle because
    /// GTK's own focus move for the same key runs after this controller has returned.
    fn follow_focus(&mut self) {
        let control = match self.state.viewer.query(Query::Focus) {
            Answer::Focus { object, .. } => self
                .placed
                .iter()
                .find(|placed| placed.key.1 == object)
                .map(|placed| placed.widget.clone()),
            _ => None,
        };
        let window = self.ui.window.clone();
        glib::idle_add_local_once(move || match control {
            Some(control) => controls::give_the_keyboard(&control),
            None => GtkWindowExt::set_focus(&window, None::<&gtk4::Widget>),
        });
    }

    /// §12.7.5.3's text field or §12.7.5.4's choice, clicked by an assistive technology: the
    /// keyboard goes to the control this host placed over the widget (ADR 1566).
    ///
    /// What a click on one asks for is a caret, or the options, and a person's click gets both
    /// from the `GtkEntry` or the list it lands on. A client's click is a point on the page, under
    /// the control, so the control is given the keyboard instead — the same `grab_focus` §12.5.1's
    /// tab walk ends in, from an idle for that walk's reason: GTK moves its own focus after the
    /// call that asked has returned. What the keys then do in the control is the toolkit's.
    pub(crate) fn aim(
        &mut self,
        name: &pdf_model::view::FieldName,
        annotation: pdf_syntax::ObjectId,
    ) {
        let Some(control) = self
            .placed
            .iter()
            .find(|placed| placed.key.1 == annotation)
            .map(|placed| placed.widget.clone())
        else {
            // A widget the frame placed no control over — off the arrangement, or a kind this
            // host draws — has nothing to aim at, and says so (trap 5).
            let said = format!("the field {} has no control placed over it", name.shown());
            self.state
                .trace
                .say(Topic::Access, format_args!("refused: {said}"));
            eprintln!("note: {said}");
            return;
        };
        self.state.trace.say(
            Topic::Access,
            format_args!("the keyboard goes to the field {}", name.shown()),
        );
        glib::idle_add_local_once(move || controls::give_the_keyboard(&control));
    }

    /// Whether this window has the keyboard, which the accessibility bridge is told (ADR 1565).
    pub(crate) fn window_active(&self) -> bool {
        self.ui.window.is_active()
    }

    /// Gives the keyboard to the page a person has just pressed on, unless the press went into the
    /// find bar or a note's editor — a press into one of this host's controls never reaches here
    /// (ADR 1752).
    ///
    /// The page area is not a GTK control, and a press on something GTK cannot focus hands its
    /// keyboard to the outline's `GtkListView` — whose bindings take the arrow keys, Home and End
    /// before this window's key table hears them. Driven under `Xvfb`: one click on the page, and
    /// every page-turning key after it moved the outline's cursor instead (ADR 1453).
    ///
    /// **A press on the page takes the keyboard out of a field's control**, as a press beside a
    /// field does in `quorra`: the core has raised Table 197's `/Bl` for the press, and a control
    /// keeping the keyboard would go on taking keys for a field the document has been told lost it.
    ///
    /// A note's editor that the same press opened keeps the keyboard it was given (ADR 1726).
    fn page_takes_the_keyboard(&mut self) {
        let focused = GtkWindowExt::focus(&self.ui.window);
        let in_the_note = self.note_editor.as_ref().is_some_and(|editor| {
            focused.as_ref().is_some_and(|focused| {
                *focused == editor.frame || focused.is_ancestor(&editor.frame)
            })
        });
        if in_the_note || self.ui.find_entry.has_focus() {
            return;
        }
        self.ui.page_area.grab_focus();
    }

    /// Whether the point, in the page area's own coordinates, is over one of the §12.7 controls this
    /// host placed over the page — the control or a widget it is built of. Not "inside the
    /// `GtkFixed`": the page's own picture is a child of it too.
    fn in_a_control(&self, x: f64, y: f64) -> bool {
        self.ui
            .page_area
            .pick(x, y, gtk4::PickFlags::DEFAULT)
            .is_some_and(|picked| {
                self.placed
                    .iter()
                    .any(|placed| picked == placed.widget || picked.is_ancestor(&placed.widget))
            })
    }

    /// A press or a release of the primary button that one of §12.7's placed controls takes, told
    /// to the core as the pointer's own: the press that went down on a control, and its release
    /// wherever it lands (ADR 1752).
    ///
    /// GTK's own gesture inside an entry or a list claims the sequence before the page's drag can
    /// see it, so without this a field a person clicks into would raise none of Table 197's `/D`,
    /// `/U`, `/Fo` and later `/Bl` — the events whose scripts a form runs there.
    fn pointer_through_a_control(&mut self, x: f64, y: f64, action: PointerAction) {
        match action {
            PointerAction::Pressed if self.in_a_control(x, y) => self.pressed_a_control = true,
            PointerAction::Released if std::mem::take(&mut self.pressed_a_control) => {}
            _ => return,
        }
        self.pointer(x, y, action);
    }

    /// Whether the keyboard is in one of the §12.7 controls this host placed over the page.
    fn a_control_has_the_keyboard(&self) -> bool {
        let Some(focused) = GtkWindowExt::focus(&self.ui.window) else {
            return false;
        };
        focused.is_ancestor(&self.ui.fixed)
    }

    /// The half of the key table that is a widget's rather than a message.
    ///
    /// Matched exhaustively and with no catch-all arm, which is `doc/ui-boundary.md`'s rule
    /// applied one layer out: a binding added to [`viewer_host::keys`] fails to compile in all
    /// three hosts, so "the hosts stay level" is checked rather than agreed to.
    fn window_act(&mut self, act: viewer_host::WindowAct) {
        match act {
            // GTK draws in logical pixels and `Command::Scroll` speaks device ones, which is why
            // the table states a distance and not the message: the same conversion `scrolled`
            // does for a wheel notch.
            viewer_host::WindowAct::ScrollBy(by) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a compositor's scale is a small ratio stated in 120ths, which f32 \
                              holds to far below a pixel"
                )]
                let scale = self.scale as f32;
                self.dispatch(viewer_host::scroll_by(by, scale));
            }
            // §14.8.2.5 leaving the program, which needed no message at all (ADR 0519). A `c`
            // reaching here has already passed the find bar, and a `c` typed into a §12.7 control
            // never reaches a window-level controller because the widget has the focus.
            viewer_host::WindowAct::Copy => self.copy_selection(),
            // The strip of open documents, whose *order* is this window's and whose focus is a
            // message. One document is its own next, so the key is harmless before anything has
            // opened a second (ADR 1264).
            viewer_host::WindowAct::NextDocument => {
                let next = self.state.documents.next_id();
                let mut queue = VecDeque::new();
                self.show_document(next, &mut queue, false);
                if !queue.is_empty() {
                    self.pump(queue);
                }
            }
            viewer_host::WindowAct::CloseDocument => self.close_document(),
            viewer_host::WindowAct::OpenDocument => self.choose_a_document(),
            // The find bar is revealed by a key this host binds rather than by
            // `gtk_search_bar_set_key_capture_widget`, which forwards *every* letter to the entry
            // and would take `a`, `s`, `z` and `y` away from the rest of the table.
            viewer_host::WindowAct::Find => self.ui.find.set_search_mode(true),
            // Table 29's "any other window visible" is a *permission*, and this is the reader's
            // wish beside it: the panel is on the screen when the clause allows one and the
            // person asked for one. `apply_chrome` composes the two.
            viewer_host::WindowAct::Panel => {
                self.panel_wanted = !self.panel_wanted;
                self.apply_chrome();
            }
            // §12.9's measuring. Nothing is drawn differently: what the mode changes is that a
            // press on the page is a point rather than the start of the text selection.
            viewer_host::WindowAct::Measure => {
                let on = self.state.showing.measuring.toggle();
                self.say(&viewer_host::measuring::switched(on));
            }
            // §10.8.3: the state is this window's, because it is a preference a person toggles
            // and `viewer-core` holds none; what crosses is the answer (ADR 1228).
            viewer_host::WindowAct::Separations => {
                self.state.separations = !self.state.separations;
                self.dispatch(Command::Separations(self.state.separations));
                self.say(&viewer_host::separations_note(self.state.separations));
            }
            viewer_host::WindowAct::Notices => self.show_notices(),
            // **From the idle queue**: `popup` runs the button's create-popup hook synchronously,
            // and that hook fills the menu through the host this call is holding — so asked from
            // here it found the host busy, the menu stayed empty and `r` opened nothing (ADR 1453).
            viewer_host::WindowAct::Restrictions => {
                let menu = self.ui.menu.clone();
                glib::idle_add_local_once(move || menu.popup());
            }
            viewer_host::WindowAct::Present | viewer_host::WindowAct::LeaveFullScreen => {
                self.present_or_stop();
            }
            // Table 29's six arrangements, in the order that table states them. A key rather
            // than a menu because this host has no menu bar; what matters for `doc/todo/30` is
            // that the *message* is exercised by a person driving a real window.
            viewer_host::WindowAct::NextLayout => {
                self.state.showing.layout = next_layout(self.state.showing.layout);
                self.dispatch(Command::Layout(self.state.showing.layout));
                self.say(&format!(
                    "page layout: {:?} (§7.7.2)",
                    self.state.showing.layout
                ));
                // A new arrangement is a new set of pages on the screen, and therefore a new set
                // of things they could not draw.
                self.restate();
            }
            // ADR 0245's third decision, made with the messages that already exist: magnify until
            // every platform control fits the `/Rect` the document states for it.
            viewer_host::WindowAct::FitControls => match self.fit_magnification {
                Some(wanted) => {
                    self.say(&format!("fitting §12.7's controls at {wanted:.3}"));
                    self.dispatch(Command::Zoom {
                        zoom: Zoom::Scale(wanted),
                        at: None,
                    });
                }
                None => self.say("every control on this page already fits its /Rect"),
            },
            // **Refused by name rather than ignored**, which is trap 5 and is the honest answer
            // here: §12.5.6.6's annotation is authored by dragging a rectangle and then typing
            // into it, and this host has neither the drag mode nor the editor. `doc/todo/30`
            // carries it as the remaining asymmetry rather than leaving it silent.
            viewer_host::WindowAct::FreeText => self.say(
                "this host cannot draw a §12.5.6.6 free text annotation yet — the drag mode and \
                 its editor are viewer-ui's alone (doc/todo/30)",
            ),
            viewer_host::WindowAct::Print => self.ask_to_print(),
            viewer_host::WindowAct::Archive => self.ask_to_archive(),
            viewer_host::WindowAct::ChooseArchive => self.choose_the_archive_again(),
            viewer_host::WindowAct::AbortDrawing => self.stop_the_long_draw(),
        }
    }

    /// The wheel's whole notches, or a finger's surface pixels, in GTK's signs — towards the
    /// person, which is the boundary's positive.
    ///
    /// The distance is `viewer_host::WheelTravel::distance`'s, the one `quorra-qt` spends: a notch
    /// the `DISCRETE` controller counted moves `viewer_host::SCROLL_NOTCH`, and a finger's pixel a
    /// pixel (ADRs 2274, 2291). Device pixels out, because that is what `Command::Scroll` speaks.
    fn scrolled(&mut self, travel: viewer_host::WheelTravel) {
        let (dx, dy) = travel.distance();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the window's scale, which is a small number"
        )]
        let scale = self.scale as f32;
        self.dispatch(Command::Scroll {
            dx: dx * scale,
            dy: dy * scale,
        });
    }

    /// Control and the wheel: a step a click and a proportion of one between, about the point
    /// under the pointer.
    ///
    /// `travel` is turned away from the person, in notches or a touchpad's pixels as the smooth
    /// controller reported them; `viewer_host::ZoomTravel` turns it into the commands `quorra-qt`
    /// sends for the same event (ADR 2274). The point kept still is the pointer's, in device pixels
    /// of the page area, which is the one rule `quorra` keeps; before the pointer has entered there
    /// is no such point and the core holds the centre.
    fn wheel_zoom(&mut self, travel: viewer_host::WheelTravel) {
        let zoom = viewer_host::ZoomTravel::of(travel);
        let scale = self.scale;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a pointer position inside a window is far inside f32's exact integer range"
        )]
        let at = self
            .pointer_at
            .map(|(x, y)| ((x * scale) as f32, (y * scale) as f32));
        let steps = match zoom {
            viewer_host::ZoomTravel::Steps(steps) => steps,
            viewer_host::ZoomTravel::Factor(_) => 0,
        };
        super::window::zoom_probe(format_args!(
            "wheel steps={steps} zoom={zoom:?} at={at:?} scale={scale}"
        ));
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a window's scale factor, a small number exact in f32 to the digits it has"
        )]
        let commands = zoom.commands(&self.state.viewer, scale as f32, at);
        let zoomed = !commands.is_empty();
        for command in commands {
            self.dispatch(command);
        }
        if zoomed
            && let Some((x, y)) = at
            && let Some(placed) = viewer_host::placement_receipt(&self.state.viewer)
        {
            super::window::zoom_probe(format_args!(
                "zoom-geometry {{\"anchor_x\":{x},\"anchor_y\":{y},{placed}}}"
            ));
        }
    }

    /// The pointer, in logical pixels of the overlay.
    fn pointer(&mut self, x: f64, y: f64, action: PointerAction) {
        self.pointer_at = Some((x, y));
        let scale = self.scale;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a pointer position inside a window is far inside f32's exact integer range"
        )]
        let at = ((x * scale) as f32, (y * scale) as f32);
        // §12.9's mode takes the press before the text selection does, which is the one thing it
        // takes away: the two gestures are the same gesture and a window cannot tell them apart
        // from the pointer alone (ADR 1191).
        if self.state.showing.measuring.is_on() {
            if matches!(action, PointerAction::Pressed) {
                self.measure(at);
            }
            return;
        }
        // §12.5.6.14's popup "shall be used for editing the parent's text": a press on a window
        // whose text is a note a person may retype gives it an editor, and the press and its
        // release go no further — the window is over the page, and the page saw no press. The
        // windows' layer takes no pointer (`page_area`), so the press is placed here (ADR 1726).
        match action {
            PointerAction::Pressed => {
                self.pressed_a_note = self
                    .popups_shown
                    .iter()
                    .rev()
                    .find(|window| viewer_host::covers(window.quad, at))
                    .and_then(|window| window.note);
                if let Some(note) = self.pressed_a_note {
                    let place = self.note_place(note, (x, y));
                    self.edit_note(note, place);
                    return;
                }
            }
            PointerAction::Dragged | PointerAction::Released if self.pressed_a_note.is_some() => {
                if matches!(action, PointerAction::Released) {
                    self.pressed_a_note = None;
                }
                return;
            }
            _ => {}
        }
        self.tell_keys();
        self.dispatch(Command::Pointer { at, action });
        self.show_whether_it_is_a_link(at);
    }

    /// §12.5.6.5's activation region under the pointer, as this platform's cursor.
    ///
    /// The clause makes a link annotation "either a hypertext link to a destination elsewhere in
    /// the document or an action to be performed", and states nothing at all about a pointer — so
    /// what a reader sees over one is a **convention**, and this is GTK's name for it. It is
    /// nevertheless the difference between a page whose links can be found and one where they can
    /// only be discovered by clicking, which is why `viewer-ui` has had it since ADR 0166 and why
    /// two windows of three having it was a debt rather than a platform difference.
    ///
    /// Asked at pointer speed, which is what makes `Query::LinkAt` a query rather than a command —
    /// and set only when the answer *changes*, because `gdk_surface_set_cursor` reaches the
    /// display server and a pointer sweeping a page of links would reach it on every motion event.
    fn show_whether_it_is_a_link(&mut self, at: (f32, f32)) {
        let Answer::Link(over) = self.state.viewer.query(Query::LinkAt(at)) else {
            return;
        };
        if over == self.over_link {
            return;
        }
        self.over_link = over;
        self.state.trace.say(
            Topic::Pointer,
            format_args!(
                "the pointer is {} §12.5.6.5's activation region",
                if over { "over" } else { "off" }
            ),
        );
        // The layer the pages are in rather than the toplevel: GTK takes the cursor from the
        // widget the pointer is *picked* on, so setting it on the window would override the text
        // cursor a `GtkEntry` over a §12.7 widget sets for itself, and setting it on the chrome
        // layer would set it on a widget `set_can_target(false)` means is never picked.
        self.ui
            .fixed
            .set_cursor_from_name(over.then_some("pointer"));
    }
}

/// Everything a person does to the window, in the vocabulary the viewer takes.
///
/// Separated from building the widgets because the two are different kinds of decision: which
/// widgets exist is a layout, and what a key or a drag *means* is this project's reading of
/// §12.5.5's pointer and §12.5.1's focus.
pub(super) fn listen(
    window: &gtk4::ApplicationWindow,
    overlay: &gtk4::Overlay,
    chrome: &gtk4::DrawingArea,
    me: &Weak<RefCell<Host>>,
    probe: &Rc<super::window::ZoomProbe>,
) {
    let listener = me.clone();
    let resize_probe = Rc::clone(probe);
    chrome.connect_resize(move |_, width, height| {
        with(&listener, |host| {
            host.resized(width, height);
            if width > 0 && height > 0 {
                resize_probe.record(
                    "core-viewport",
                    format_args!(
                        "\"width_device\":{},\"height_device\":{},\"scale\":{}",
                        super::window::device(width, host.scale)
                            .map_or_else(|| "null".to_owned(), |d| d.to_string()),
                        super::window::device(height, host.scale)
                            .map_or_else(|| "null".to_owned(), |d| d.to_string()),
                        host.scale,
                    ),
                );
            }
        });
    });

    // A compositor states a surface's fractional scale after its first configure, and again when
    // the window moves to another output; neither needs to change the logical size the resize above
    // reads, so the viewport is measured again on the scale itself (ADR 2250).
    let listener = me.clone();
    chrome.connect_realize(move |area| {
        let Some(surface) = area.native().and_then(|native| native.surface()) else {
            return;
        };
        let property = if surface.find_property("scale").is_some() {
            "scale"
        } else {
            "scale-factor"
        };
        let listener = listener.clone();
        let area = area.downgrade();
        surface.connect_notify_local(Some(property), move |_, _| {
            let Some(area) = area.upgrade() else {
                return;
            };
            with(&listener, |host| {
                if (super::window::surface_scale(&area) - host.scale).abs() > f64::EPSILON {
                    host.resized(area.width(), area.height());
                }
            });
        });
    });

    let keys = gtk4::EventControllerKey::new();
    let listener = me.clone();
    // Both modifiers the table reads: Shift for §12.5.1's tab key, and Control for the four
    // conventional bindings — a Control this program does not bind means nothing rather than the
    // unmodified row, which is why it has to cross rather than being dropped here (ADR 1192).
    keys.connect_key_pressed(move |_, key, _, held| {
        let mut taken = false;
        with(&listener, |host| taken = host.key(key, modifiers(held)));
        if taken {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(keys);

    // **Ctrl + Tab is taken before the focused widget sees it**, and only that press. GTK binds it
    // to moving the keyboard focus out of whatever holds it, so on its way up to the controller
    // above it was spent on a button and the strip of documents never moved — found by driving
    // this window with three documents open. The capture phase is kept to the one binding the
    // toolkit would otherwise take; every other key still reaches the focused widget first, which
    // is what a §12.7 field being typed into needs (ADR 1275).
    let tabs = gtk4::EventControllerKey::new();
    tabs.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let listener = me.clone();
    tabs.connect_key_pressed(move |_, key, _, held| {
        let tab = matches!(key, gtk4::gdk::Key::Tab | gtk4::gdk::Key::ISO_Left_Tab);
        if !tab || !held.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
            return glib::Propagation::Proceed;
        }
        with(&listener, |host| {
            host.key(key, modifiers(held));
        });
        glib::Propagation::Stop
    });
    window.add_controller(tabs);

    listen_wheel(overlay, me, probe);

    listen_motion(overlay, me, probe);

    presses(overlay, me);
}

/// The wheel: a scroll on one controller and Control's zoom on another (ADR 2274).
fn listen_wheel(
    overlay: &gtk4::Overlay,
    me: &Weak<RefCell<Host>>,
    probe: &Rc<super::window::ZoomProbe>,
) {
    // **The wheel, which this host had no binding for until Table 29's `/PageLayout` was
    // obeyed.** Under `SinglePage` at `Zoom::FitPage` there is nothing to scroll, so the gap was
    // invisible; a continuous arrangement is a thing a person moves through and a viewer that
    // could not would be offering an arrangement it cannot use. GTK reports a discrete wheel in
    // notches, so a notch is a distance — `viewer_host::SCROLL_NOTCH`, the same in every window,
    // and `viewer_host::DiscreteWheel` is this controller's count, kept for a window whose
    // toolkit hands over the fractions (ADR 2274).
    let scrolling = gtk4::EventControllerScroll::new(
        gtk4::EventControllerScrollFlags::BOTH_AXES | gtk4::EventControllerScrollFlags::DISCRETE,
    );
    let listener = me.clone();
    let wheel_probe = Rc::clone(probe);
    scrolling.connect_scroll(move |controller, dx, dy| {
        // Control is a magnification rather than a movement, as it is in the other two windows,
        // and the controller below makes it; this one only scrolls.
        if controlled(controller) {
            return glib::Propagation::Stop;
        }
        // A finger's surface pixels are the smooth controller's below: counted here, ten of them
        // would be a notch of forty-eight (ADR 2291).
        if in_surface_pixels(controller) {
            return glib::Propagation::Proceed;
        }
        wheel_probe.record(
            "wheel-branch",
            format_args!("\"branch\":\"scroll\",\"notches_dx\":{dx},\"notches_dy\":{dy}"),
        );
        #[expect(
            clippy::cast_possible_truncation,
            reason = "whole notches the controller counted, a handful"
        )]
        let travel = viewer_host::WheelTravel::Notches {
            x: dx as f32,
            y: dy as f32,
        };
        with(&listener, |host| host.scrolled(travel));
        glib::Propagation::Stop
    });
    overlay.add_controller(scrolling);
    // **Control and the wheel read the travel as the device reported it**, on a controller of its
    // own without `DISCRETE`: a high-resolution wheel's eighth of a notch is an eighth of a zoom
    // step at once rather than a share of the click `DISCRETE` would wait for, a whole notch is
    // one step, and a touchpad's surface pixels are a proportion (ADR 2274). GTK runs every
    // controller of a widget on each event, so the two each take their half.
    let zooming = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::BOTH_AXES);
    let listener = me.clone();
    let zoom_probe = Rc::clone(probe);
    let wheel_probe_pixels = Rc::clone(probe);
    zooming.connect_scroll(move |controller, dx, dy| {
        let pixels = controller.unit() == gtk4::gdk::ScrollUnit::Surface;
        // **A finger scrolls by what it travelled**, through the distance `quorra-qt` spends: the
        // `DISCRETE` controller above passes surface pixels over, and they are taken here as they
        // come (ADR 2291).
        if !controlled(controller) {
            if !pixels {
                return glib::Propagation::Proceed;
            }
            wheel_probe_pixels.record(
                "wheel-branch",
                format_args!("\"branch\":\"scroll\",\"pixels_dx\":{dx},\"pixels_dy\":{dy}"),
            );
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a finger's surface pixels of one event"
            )]
            let travel = viewer_host::WheelTravel::Pixels {
                x: dx as f32,
                y: dy as f32,
            };
            with(&listener, |host| host.scrolled(travel));
            return glib::Propagation::Stop;
        }
        zoom_probe.record(
            "wheel-branch",
            format_args!(
                "\"branch\":\"zoom\",\"amount_dx\":{dx},\"amount_dy\":{dy},\"pixels\":{pixels}"
            ),
        );
        // GTK's positive is towards the person, and a zoom in is the wheel turned away.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a wheel's notches or a touchpad's pixels of one event"
        )]
        let away = -dy as f32;
        let travel = if pixels {
            viewer_host::WheelTravel::Pixels { x: 0.0, y: away }
        } else {
            viewer_host::WheelTravel::Notches { x: 0.0, y: away }
        };
        with(&listener, |host| host.wheel_zoom(travel));
        glib::Propagation::Stop
    });
    overlay.add_controller(zooming);
    if super::window::zoom_probe_enabled() {
        overlay.add_controller(wheel_receipts(probe));
    }
}

/// Whether the event a scroll controller is handling states its travel in surface pixels — a
/// finger's, or a continuous device's — which a `DISCRETE` controller's own `unit` never says,
/// since it answers `Wheel` for everything it counts (ADR 2291).
fn in_surface_pixels(controller: &gtk4::EventControllerScroll) -> bool {
    controller
        .current_event()
        .and_then(|event| event.downcast::<gtk4::gdk::ScrollEvent>().ok())
        .is_some_and(|scroll| scroll.unit() == gtk4::gdk::ScrollUnit::Surface)
}

/// Whether Control is held for the event a scroll controller is handling.
fn controlled(controller: &gtk4::EventControllerScroll) -> bool {
    controller
        .current_event_state()
        .contains(gtk4::gdk::ModifierType::CONTROL_MASK)
}

/// Every scroll event as the compositor delivered it, under the probe alone.
///
/// The `DISCRETE` controller above calls back only when the travel completes a notch, so what a
/// high-resolution wheel or a touchpad sent between two callbacks is invisible to it; this observer
/// sees each event before the controller spends it and lets it through. The receipt is the GTK
/// half of the comparison `cpp/window.cpp`'s `wheel-entry` makes for Qt (ADR 2274).
fn wheel_receipts(probe: &Rc<super::window::ZoomProbe>) -> gtk4::EventControllerLegacy {
    let observer = gtk4::EventControllerLegacy::new();
    observer.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let probe = Rc::clone(probe);
    observer.connect_event(move |_, event| {
        if let Some(scroll) = event.downcast_ref::<gtk4::gdk::ScrollEvent>() {
            let (dx, dy) = scroll.deltas();
            let source = event
                .device()
                .map_or_else(|| "none".to_owned(), |device| format!("{:?}", device.source()));
            probe.record(
                "wheel-entry",
                format_args!(
                    "\"sequence\":{},\"direction\":\"{:?}\",\"unit\":\"{:?}\",\"delta_x\":{dx},\"delta_y\":{dy},\"stop\":{},\"source\":\"{source}\",\"ctrl\":{}",
                    probe.next(),
                    scroll.direction(),
                    scroll.unit(),
                    scroll.is_stop(),
                    event
                        .modifier_state()
                        .contains(gtk4::gdk::ModifierType::CONTROL_MASK),
                ),
            );
        }
        glib::Propagation::Proceed
    });
    observer
}

/// Observe motion before borrowing the host, without changing its reentrancy policy.
fn listen_motion(
    overlay: &gtk4::Overlay,
    me: &Weak<RefCell<Host>>,
    probe: &Rc<super::window::ZoomProbe>,
) {
    // §12.5.5's three appearances follow the pointer, and §12.5.6.19's `/H` with them — which is
    // why a move with no button down is a command and not only a cursor question.
    let motion = gtk4::EventControllerMotion::new();
    let listener = me.clone();
    let motion_probe = Rc::clone(probe);
    motion.connect_motion(move |_, x, y| {
        let sequence = motion_probe.next();
        if sequence == 0 {
            with(&listener, |host| host.pointer(x, y, PointerAction::Moved));
            return;
        }
        motion_probe.record("motion-entry", format_args!(
            "\"sequence\":{sequence},\"logical_x\":{x},\"logical_y\":{y},\"action\":\"moved\"",
        ));
        let Some(host) = listener.upgrade() else {
            motion_probe.record("motion-outcome", format_args!("\"sequence\":{sequence},\"outcome\":\"host-gone\""));
            return;
        };
        let Ok(mut host) = host.try_borrow_mut() else {
            eprintln!("note: the host was busy and an action was dropped");
            motion_probe.record("motion-outcome", format_args!("\"sequence\":{sequence},\"outcome\":\"host-busy\""));
            return;
        };
        let measuring = host.state.showing.measuring.is_on();
        let scale = host.scale;
        host.pointer(x, y, PointerAction::Moved);
        motion_probe.record("motion-outcome", format_args!(
            "\"sequence\":{sequence},\"outcome\":\"{}\",\"scale\":{scale},\"core_x\":{},\"core_y\":{}",
            if measuring { "measuring" } else { "accepted" },
            x * scale, y * scale,
        ));
    });
    // A pointer that arrives without moving — a jump, or a window opened under it — is told by
    // `enter` alone, and a Control+wheel step before the first motion would be anchored at the
    // centre rather than at the pointer (ADR 2275).
    let listener = me.clone();
    motion.connect_enter(move |_, x, y| {
        with(&listener, |host| host.pointer(x, y, PointerAction::Moved));
    });
    overlay.add_controller(motion);
}

/// The primary button on the page: the page's own drag, and the press a placed control takes,
/// each told to the core once (ADR 1752).
fn presses(overlay: &gtk4::Overlay, me: &Weak<RefCell<Host>>) {
    // A press on one of §12.7's controls is read off the raw events before the control sees it,
    // told to the core, and passed on untouched (`Host::pointer_through_a_control`).
    let raw = gtk4::EventControllerLegacy::new();
    raw.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let listener = me.clone();
    raw.connect_event(move |controller, event| {
        let action = match event.event_type() {
            gtk4::gdk::EventType::ButtonPress => PointerAction::Pressed,
            gtk4::gdk::EventType::ButtonRelease => PointerAction::Released,
            _ => return glib::Propagation::Proceed,
        };
        let primary = event
            .downcast_ref::<gtk4::gdk::ButtonEvent>()
            .is_some_and(|button| button.button() == gtk4::gdk::BUTTON_PRIMARY);
        if let (true, Some(widget)) = (primary, controller.widget())
            && let Some((x, y)) = in_widget(&widget, event)
        {
            with(&listener, |host| {
                host.pointer_through_a_control(x, y, action);
            });
        }
        glib::Propagation::Proceed
    });
    overlay.add_controller(raw);

    let drag = gtk4::GestureDrag::new();
    let listener = me.clone();
    drag.connect_drag_begin(move |_, x, y| {
        // A control that does not claim its press lets this gesture see it too; the press is the
        // control's, told once above.
        let mut on_the_page = false;
        with(&listener, |host| {
            host.dragging_a_control = host.pressed_a_control;
            on_the_page = !host.dragging_a_control;
            if on_the_page {
                host.pointer(x, y, PointerAction::Pressed);
            }
        });
        if on_the_page {
            // After GTK's own handling of the same press, which is what moves its focus.
            let listener = listener.clone();
            glib::idle_add_local_once(move || with(&listener, Host::page_takes_the_keyboard));
        }
    });
    let listener = me.clone();
    drag.connect_drag_update(move |gesture, dx, dy| {
        let Some((x, y)) = gesture.start_point() else {
            return;
        };
        with(&listener, |host| {
            if !host.dragging_a_control {
                host.pointer(x + dx, y + dy, PointerAction::Dragged);
            }
        });
    });
    let listener = me.clone();
    drag.connect_drag_end(move |gesture, dx, dy| {
        let Some((x, y)) = gesture.start_point() else {
            return;
        };
        with(&listener, |host| {
            if !std::mem::take(&mut host.dragging_a_control) {
                host.pointer(x + dx, y + dy, PointerAction::Released);
            }
        });
    });
    overlay.add_controller(drag);
}

/// Where a raw event happened, in `widget`'s own coordinates: GDK states an event's position on its
/// surface, and the native widget that owns the surface may sit inside it by a transform.
fn in_widget(widget: &gtk4::Widget, event: &gtk4::gdk::Event) -> Option<(f64, f64)> {
    let native = widget.native()?;
    let (x, y) = event.position()?;
    let (left, top) = native.surface_transform();
    native.translate_coordinates(widget, x - left, y - top)
}

/// GDK's key as the one [`viewer_host::keys`] states a meaning for, or nothing.
///
/// **This is the whole of what this host contributes to its key bindings** (ADR 0526).
/// `gdk::Key` against `Qt::Key` against `winit::keyboard::Key` is what a toolkit is, and what a
/// press *means* is this project's reading of §12.5.1 and §12.4.4.2 plus a page of choices — which
/// is why the table is in `viewer-host` and this function is here.
///
/// A letter arrives in both cases because GDK reports the shifted one and none of the letters the
/// table binds means a second thing when shifted.
pub(super) fn key_pressed(key: gtk4::gdk::Key) -> Option<viewer_host::Key> {
    use gtk4::gdk::Key as Gdk;
    use viewer_host::Key as Stated;
    Some(match key {
        Gdk::a | Gdk::A => Stated::A,
        Gdk::c | Gdk::C => Stated::C,
        Gdk::f | Gdk::F => Stated::F,
        Gdk::h | Gdk::H => Stated::H,
        Gdk::k | Gdk::K => Stated::K,
        Gdk::l | Gdk::L => Stated::L,
        Gdk::m | Gdk::M => Stated::M,
        Gdk::o | Gdk::O => Stated::O,
        Gdk::p | Gdk::P => Stated::P,
        Gdk::r | Gdk::R => Stated::R,
        Gdk::s | Gdk::S => Stated::S,
        Gdk::t | Gdk::T => Stated::T,
        Gdk::w | Gdk::W => Stated::W,
        Gdk::y | Gdk::Y => Stated::Y,
        Gdk::z | Gdk::Z => Stated::Z,
        Gdk::_0 => Stated::Zero,
        Gdk::plus => Stated::Plus,
        Gdk::minus => Stated::Minus,
        Gdk::equal => Stated::Equals,
        Gdk::slash => Stated::Slash,
        Gdk::question => Stated::Question,
        Gdk::Escape => Stated::Escape,
        Gdk::Tab | Gdk::ISO_Left_Tab => Stated::Tab,
        Gdk::space => Stated::Space,
        Gdk::Return | Gdk::KP_Enter => Stated::Enter,
        Gdk::Home => Stated::Home,
        Gdk::End => Stated::End,
        Gdk::Left => Stated::Left,
        Gdk::Right => Stated::Right,
        Gdk::Up => Stated::Up,
        Gdk::Down => Stated::Down,
        Gdk::Page_Up => Stated::PageUp,
        Gdk::Page_Down => Stated::PageDown,
        _ => return None,
    })
}

/// GDK's modifier state as the two modifiers [`viewer_host::keys`] reads.
///
/// **A function rather than two lines in the controller's closure**, so that the one thing this
/// host decides about a modifier can be tested without a display: a `gdk::ModifierType` is a set
/// of bits and nothing here calls into GTK. The other two hosts have the same seam.
///
/// Alt, Meta and the platform key are deliberately not folded into either: they belong to a
/// window manager and to GTK's own accelerators, and a host that passed one of them as Control
/// would bind a key it was not given (ADR 1192).
pub(super) fn modifiers(held: gtk4::gdk::ModifierType) -> viewer_host::Modifiers {
    viewer_host::Modifiers {
        shift: held.contains(gtk4::gdk::ModifierType::SHIFT_MASK),
        ctrl: held.contains(gtk4::gdk::ModifierType::CONTROL_MASK),
    }
}
