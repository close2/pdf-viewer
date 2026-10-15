//! What a person does to the window: keys, the pointer and the wheel, the selection and the clipboard, §12.9's measuring, and the window's acts.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Weak;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Answer, Command, Edit, PointerAction, Query, Zoom};

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
        let mode = if self.presenting.full_screen() {
            viewer_host::Mode::Presenting
        } else {
            viewer_host::Mode::Reading
        };
        // A push-button §12.5.1's focus is on takes Space and Enter before the page does, since no
        // toolkit button stands over one to take them (ADR 1357) — unless one of this host's own
        // controls has the keyboard, which has had the key first and whose Enter GTK may pass on.
        if !self.a_control_has_the_keyboard()
            && let Some(pressed) = viewer_host::pressed(&self.state.viewer, stated, held)
        {
            self.press(pressed);
            return true;
        }
        let waiting = self.waiting();
        let Some(meaning) = viewer_host::meaning(stated, held, mode, waiting) else {
            return false;
        };
        match meaning {
            viewer_host::Meaning::Send(command) => {
                // §12.5.6.10's markups are defined over selected text, so a press with nothing
                // selected asks for an annotation over nothing. The core answers by doing
                // nothing, which is right and silent — and trap 5 is that a person who pressed a
                // key and saw no change has been told nothing at all.
                if matches!(command, Command::Edit(Edit::Markup { .. })) && !self.has_selection() {
                    self.say("select some text first — §12.5.6.10's markups mark up text");
                    return true;
                }
                let walked = matches!(command, Command::Focused(_));
                self.dispatch(command);
                if walked {
                    self.follow_focus();
                }
                !walked
            }
            viewer_host::Meaning::Window(act) => {
                self.window_act(act);
                true
            }
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

    /// Carries out a key's press on a push-button: its activation, or the refusal said by name.
    fn press(&mut self, pressed: viewer_host::Pressed) {
        if let Some(said) = pressed.note() {
            self.say(&said);
            return;
        }
        match pressed {
            viewer_host::Pressed::Activates { name, annotation } => {
                self.state.trace.say(
                    Topic::Pointer,
                    format_args!("pressing the push-button {}", name.shown()),
                );
                self.dispatch(Command::Activate(annotation));
            }
            viewer_host::Pressed::ReadOnly { .. } => {}
        }
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
                    reason = "a step in logical pixels times a display scale, which is tens"
                )]
                let dy = (f64::from(by) * f64::from(self.scale)) as f32;
                self.dispatch(Command::Scroll { dx: 0.0, dy });
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
                self.show_document(next, &mut queue);
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
                self.separations = !self.separations;
                self.dispatch(Command::Separations(self.separations));
                self.say(&viewer_host::separations_note(self.separations));
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
            viewer_host::WindowAct::AbortDrawing => self.stop_the_long_draw(),
        }
    }

    /// One notch of the wheel, in whichever direction it turned.
    ///
    /// The deltas GTK reports for a discrete device are notches rather than pixels, so the
    /// distance is this host's choice and `SCROLL_STEP` is where it is written down. Device
    /// pixels out, because that is what `Command::Scroll` speaks.
    fn scrolled(&mut self, dx: f64, dy: f64) {
        let step = f64::from(self.scale) * SCROLL_STEP;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a notch count times a step in pixels, which is tens"
        )]
        let (dx, dy) = ((dx * step) as f32, (dy * step) as f32);
        self.dispatch(Command::Scroll { dx, dy });
    }

    /// Control and the wheel: whole zoom steps about the point under the pointer.
    ///
    /// GTK reports a discrete wheel in notches, positive turned towards the person, and a notch is
    /// one step out — `viewer_host::ZoomWheel` spends the travel, so the three windows count a
    /// gesture alike (ADR 1118). The point kept still is the pointer's, in device pixels of the
    /// page area, which is the one rule `quorra` keeps; before the pointer has entered there is
    /// no such point and the core holds the centre.
    fn wheel_zoom(&mut self, dy: f64) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a notch count, which is a handful"
        )]
        let steps = self.zoom_wheel.lines(-dy as f32);
        let zoom = if steps > 0 { Zoom::In } else { Zoom::Out };
        let scale = f64::from(self.scale);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a pointer position inside a window is far inside f32's exact integer range"
        )]
        let at = self
            .pointer_at
            .map(|(x, y)| ((x * scale) as f32, (y * scale) as f32));
        for _ in 0..steps.unsigned_abs() {
            self.dispatch(Command::Zoom { zoom, at });
        }
    }

    /// The pointer, in logical pixels of the overlay.
    fn pointer(&mut self, x: f64, y: f64, action: PointerAction) {
        self.pointer_at = Some((x, y));
        let scale = f64::from(self.scale);
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
) {
    let listener = me.clone();
    chrome.connect_resize(move |_, width, height| {
        with(&listener, |host| host.resized(width, height));
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

    // **The wheel, which this host had no binding for until Table 29's `/PageLayout` was
    // obeyed.** Under `SinglePage` at `Zoom::FitPage` there is nothing to scroll, so the gap was
    // invisible; a continuous arrangement is a thing a person moves through and a viewer that
    // could not would be offering an arrangement it cannot use. GTK reports a discrete wheel in
    // notches, so a notch is a distance this host chooses — see `SCROLL_STEP`.
    let scrolling = gtk4::EventControllerScroll::new(
        gtk4::EventControllerScrollFlags::BOTH_AXES | gtk4::EventControllerScrollFlags::DISCRETE,
    );
    let listener = me.clone();
    scrolling.connect_scroll(move |controller, dx, dy| {
        // Control is a magnification rather than a movement, as it is in the other two windows.
        let control = controller
            .current_event_state()
            .contains(gtk4::gdk::ModifierType::CONTROL_MASK);
        with(&listener, |host| {
            if control {
                host.wheel_zoom(dy);
            } else {
                host.scrolled(dx, dy);
            }
        });
        glib::Propagation::Stop
    });
    overlay.add_controller(scrolling);

    // §12.5.5's three appearances follow the pointer, and §12.5.6.19's `/H` with them — which is
    // why a move with no button down is a command and not only a cursor question.
    let motion = gtk4::EventControllerMotion::new();
    let listener = me.clone();
    motion.connect_motion(move |_, x, y| {
        with(&listener, |host| host.pointer(x, y, PointerAction::Moved));
    });
    overlay.add_controller(motion);

    presses(overlay, me);
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

/// How far one notch of the wheel moves the page, in logical pixels.
///
/// A choice, and written down as one: the standard says nothing about a wheel. Three lines of a
/// document's body text is what every reader the project owner uses has converged on, and this is
/// about that.
const SCROLL_STEP: f64 = 48.0;
