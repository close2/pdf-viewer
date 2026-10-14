//! What a person does with the keyboard and the pointer: the shared key table's meanings, the
//! wheel, the toolbar, a selection and its copy, and §12.9's measuring (filed here by ADR 1908).

use viewer_core::{Answer, Command, Edit, PageTarget, PointerAction, Printing, Query, Zoom};
use viewer_host::arrangement::next_layout;
use viewer_host::native_window::NativeWindow;
use viewer_host::trace::Topic;

use crate::bridge::ffi::QtQuad;
use crate::keys;

use super::{Host, quad};

impl Host {
    /// A key was pressed, as `Qt::Key`, with Qt's own modifier state beside it.
    ///
    /// **What it means is [`viewer_host::keys`]'s answer and not this host's** (ADR 0526). This
    /// method used to hold five special cases and a table, and the table disagreed with the other
    /// two hosts about the arrow keys, about `f` and about Escape; what is left is
    /// [`keys::stated`] turning a number into a [`viewer_host::Key`] and [`Host::window_act`]
    /// doing the half of the answer that is a widget's rather than a message.
    pub(crate) fn key(&mut self, code: u32, shift: bool, ctrl: bool) {
        let Some(stated) = keys::stated(code) else {
            return;
        };
        let mode = if self.presenting.full_screen() {
            viewer_host::Mode::Presenting
        } else {
            viewer_host::Mode::Reading
        };
        let waiting = self.waiting();
        let held = viewer_host::Modifiers {
            shift: shift || keys::shifted_by_name(code),
            ctrl,
        };
        // A push-button §12.5.1's focus is on takes Space and Enter before the page does, since
        // no toolkit button stands over one to take them (ADR 1357). A key a placed control had
        // the keyboard for never arrives: `MainWindow::keyPressEvent` keeps Enter from one.
        if let Some(pressed) = viewer_host::pressed(&self.state.viewer, stated, held) {
            self.press(&pressed);
            return;
        }
        let Some(meaning) = viewer_host::meaning(stated, held, mode, waiting) else {
            return;
        };
        match meaning {
            viewer_host::Meaning::Send(command) => {
                // §12.5.6.10's markups are defined over selected text, so a press with nothing
                // selected asks for an annotation over nothing. The core answers by doing nothing,
                // which is right and silent — and trap 5 is that a person who pressed a key and
                // saw no change has been told nothing at all.
                if matches!(command, Command::Edit(Edit::Markup { .. })) && !self.has_selection() {
                    self.say("select some text first — §12.5.6.10's markups mark up text");
                    return;
                }
                self.dispatch(command);
            }
            viewer_host::Meaning::Window(act) => self.window_act(act),
        }
    }

    /// The half of the key table that is a widget's rather than a message.
    ///
    /// Matched exhaustively and with no catch-all arm, which is `doc/ui-boundary.md`'s rule applied
    /// one layer out: a binding added to [`viewer_host::keys`] fails to compile in all three hosts.
    ///
    /// **Four of these arms are a flag rather than a call**, and the reason is this crate's shape
    /// rather than Qt's: [`crate::bridge`] states that C++ owns the `Host` for the life of
    /// `QApplication::exec` and that Rust never calls a Qt object, which is what keeps the crate to
    /// one hand-written `unsafe` token. So showing a find bar, a panel or a card of notices is a
    /// [`QtUpdate`](crate::bridge::ffi::QtUpdate) field the window reads back — the shape `window`
    /// has had since ADR 0470 and `clipboard` since ADR 0519.
    fn window_act(&mut self, act: viewer_host::WindowAct) {
        match act {
            // Qt places widgets in logical pixels and `Command::Scroll` speaks device ones, which
            // is why the table states a distance rather than building the message itself.
            viewer_host::WindowAct::ScrollBy(by) => self.dispatch(Command::Scroll {
                dx: 0.0,
                dy: by * self.scale,
            }),
            // The only binding whose answer leaves the program: §14.8.2.5's text on the session's
            // clipboard. Not a command, because what a copy *is* belongs to the platform (ADR
            // 0519).
            viewer_host::WindowAct::Copy => self.copy_selection(),
            // The strip of open documents, whose *order* is this window's and whose focus is a
            // message. One document is its own next, so the key is harmless before anything has
            // opened a second (ADR 1264).
            viewer_host::WindowAct::NextDocument => {
                let next = self.state.documents.next_id();
                self.show_document(next);
            }
            viewer_host::WindowAct::CloseDocument => self.close_document(),
            // A `QFileDialog` is a Qt object and Rust never calls one, so the key goes out as a
            // flag and the path comes back through `Host::open_chosen` (ADR 1275).
            viewer_host::WindowAct::OpenDocument => self.update.choose_document = true,
            viewer_host::WindowAct::Find => self.update.find_bar = true,
            // Table 29's "any other window visible" is a *permission* and this is the reader's
            // wish beside it; `Host::chrome` composes the two, so leaving full screen puts back
            // what the reader had rather than what the document last permitted.
            viewer_host::WindowAct::Panel => {
                self.panel_shown = !self.panel_shown;
                self.update.window = true;
            }
            // §12.9's measuring. No pixel of the page is different: what the mode changes is
            // that a press is a point rather than the start of the text selection.
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
            viewer_host::WindowAct::Notices => self.update.notices = true,
            viewer_host::WindowAct::Restrictions => self.update.menu = true,
            viewer_host::WindowAct::Present | viewer_host::WindowAct::LeaveFullScreen => {
                self.present_or_stop();
            }
            // Table 29's six arrangements are cycled, so what to send depends on which one is in
            // force — a fact about this host's state rather than about the key.
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
            // What `w` sends depends on what this page's controls measured, so the command cannot
            // be built from the key alone. `viewer-gtk` answers the same way.
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
            // **Refused by name rather than ignored**, which is trap 5 and the honest answer here:
            // §12.5.6.6's annotation is authored by dragging a rectangle and then typing into it,
            // and this host has neither the drag mode nor the editor. `doc/todo/30` carries it as
            // the remaining asymmetry rather than leaving it silent.
            viewer_host::WindowAct::FreeText => self.say(
                "this host cannot draw a §12.5.6.6 free text annotation yet — the drag mode and \
                 its editor are viewer-ui's alone (doc/todo/30)",
            ),
            // §7.6.4.2's bit 3, asked as an operation, and then RFC 0004 §6's preview — which
            // is what this host has and says so. The same key ends it, because a window that
            // could enter print intent and not leave it would have taken §8.11.4.5's revert away
            // from the person who asked for the operation (ADR 1180).
            viewer_host::WindowAct::Print => {
                if self.printing {
                    self.print_finish();
                    self.say("the printed page is no longer what this window shows");
                } else {
                    self.dispatch(Command::Print(Printing::Start(
                        viewer_host::printing::unknown_sheet(None),
                    )));
                }
            }
            viewer_host::WindowAct::AbortDrawing => self.stop_the_long_draw(),
        }
    }

    /// The wheel turned, already in the device pixels the boundary speaks.
    ///
    /// The conversion from Qt's notches is C++'s, beside the event that states them; what is here
    /// is the message, so that the two hosts differ in the toolkit and in nothing else.
    pub(crate) fn scrolled(&mut self, dx: f32, dy: f32) {
        self.dispatch(Command::Scroll { dx, dy });
    }

    /// Control and the wheel: whole zoom steps about the point under the pointer.
    ///
    /// `amount` is Qt's travel turned away from the person, in notches, or in pixels where
    /// `pixels` says the device reported those; `viewer_host::ZoomWheel` spends it, so the three
    /// windows count a gesture alike (ADR 1118). `(x, y)` is the pointer in device pixels of the
    /// viewport, the point `quorra` and `quorra-gtk` keep still too.
    pub(crate) fn wheel_zoom(&mut self, amount: f32, pixels: bool, x: f32, y: f32) {
        let steps = if pixels {
            self.zoom_wheel.pixels(amount)
        } else {
            self.zoom_wheel.lines(amount)
        };
        let zoom = if steps > 0 { Zoom::In } else { Zoom::Out };
        for _ in 0..steps.unsigned_abs() {
            self.dispatch(Command::Zoom {
                zoom,
                at: Some((x, y)),
            });
        }
    }

    /// The keyboard's Shift and Control, and whether an arrow key is making a choice control's
    /// selection, told to the core where they changed since it was last told: the field's Table
    /// 199 `/K` reads the arrows as Adobe's `event.keyDown` (ADRs 1762, 1771, 1786).
    pub(crate) fn keys(&mut self, shift: bool, control: bool, arrows: bool) {
        let keys = pdf_model::view::Keys {
            shift,
            modifier: control,
            arrows,
        };
        if keys != self.keys_told {
            self.keys_told = keys;
            self.dispatch(Command::Keys(keys));
        }
    }

    /// The pointer moved or a button changed.
    pub(crate) fn pointer(&mut self, x: f32, y: f32, action: u8) {
        let action = match action {
            0 => PointerAction::Moved,
            1 => PointerAction::Pressed,
            2 => PointerAction::Dragged,
            3 => PointerAction::Released,
            // Trap 5: a number this host did not send is a mistake in the C++ half, and a silent
            // `Moved` would hide it behind a working-looking window.
            other => {
                self.say(&format!(
                    "the window sent pointer action {other}, which is none"
                ));
                return;
            }
        };
        // §12.9's mode takes the press before the text selection does, which is the one thing
        // it takes away: the two gestures are the same gesture and a window cannot tell them
        // apart from the pointer alone (ADR 1191).
        if self.state.showing.measuring.is_on() {
            if matches!(action, PointerAction::Pressed) {
                self.measure((x, y));
            }
            return;
        }
        self.dispatch(Command::Pointer { at: (x, y), action });
        // §12.5.6.5's activation region, asked at pointer speed — which is what makes
        // `Query::LinkAt` a query rather than a command. The clause states no cursor at all, so
        // what a reader sees over a link is a convention; this is the one all three hosts now
        // keep, and the flag is how it reaches a Qt object without Rust calling one.
        if let Answer::Link(over) = self.state.viewer.query(Query::LinkAt((x, y)))
            && over != self.over_link
        {
            self.over_link = over;
            self.update.cursor = true;
            self.state.trace.say(
                Topic::Pointer,
                format_args!(
                    "the pointer is {} §12.5.6.5's activation region",
                    if over { "over" } else { "off" }
                ),
            );
        }
    }

    /// A toolbar button.
    pub(crate) fn command(&mut self, what: u8) {
        let command = match what {
            0 => Command::GoTo(PageTarget::Previous),
            1 => Command::GoTo(PageTarget::Next),
            2 => Command::Zoom {
                zoom: Zoom::Out,
                at: None,
            },
            3 => Command::Zoom {
                zoom: Zoom::In,
                at: None,
            },
            4 => Command::Zoom {
                zoom: Zoom::FitPage,
                at: None,
            },
            other => {
                self.say(&format!("the window sent command {other}, which is none"));
                return;
            }
        };
        self.dispatch(command);
    }

    /// The magnification the page is drawn at now, in the units [`Zoom::Scale`] takes.
    ///
    /// `PageGeometry::scale` is device pixels per user space unit — "the zoom and the display's
    /// scale together" — and `Zoom::Scale` is *logical* pixels per user space unit, so the
    /// display's own factor comes back out. The same arithmetic `viewer-gtk` does, and it has to
    /// be the same: a doubled display would otherwise put one host's answer out by two.
    pub(super) fn showing_at(&self) -> Option<f32> {
        let Answer::Page { index, .. } = self.state.viewer.query(Query::CurrentPage) else {
            return None;
        };
        let Answer::Geometry(geometry) = self.state.viewer.query(Query::PageGeometry(index)) else {
            return None;
        };
        let display = if self.scale.is_finite() && self.scale >= 1.0 {
            self.scale
        } else {
            1.0
        };
        let logical = geometry.scale / display;
        (logical.is_finite() && logical > 0.0).then_some(logical)
    }

    /// The text a copy is putting on the clipboard, which this also clears.
    ///
    /// Taken rather than borrowed, for the reason every other `take_` on this bridge is: a
    /// `cxx::String` is a copy whichever way it crosses, and leaving the characters here
    /// afterwards would be this host holding a second clipboard nobody reads.
    pub(crate) fn take_clipboard(&mut self) -> String {
        std::mem::take(&mut self.clipboard)
    }

    /// What is selected on the page.
    pub(crate) fn selection(&self) -> Vec<QtQuad> {
        match self.state.viewer.query(Query::Selection) {
            Answer::Selected(selected) => selected.quads.iter().copied().map(quad).collect(),
            _ => Vec::new(),
        }
    }

    /// §12.9's traced path, in device pixels of the viewport, as flat `x, y` pairs.
    ///
    /// **The rubber band, which is chrome and not a message.** §12.9 states no state for a viewer
    /// to be in, so the points are this host's (`viewer_host::Measuring`) and what they look like
    /// is this platform's; `viewer_core::Query::Measure` answers what they *mean* and never sees a
    /// pixel of this (ADR 1264).
    pub(crate) fn measuring(&self) -> Vec<f32> {
        self.state
            .showing
            .measuring
            .points()
            .iter()
            .flat_map(|point| [point[0], point[1]])
            .collect()
    }

    /// Whether the pointer is over §12.5.6.5's activation region.
    pub(crate) fn over_link(&self) -> bool {
        self.over_link
    }
}
