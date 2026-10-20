//! The questions this window puts to a person: the ask level's dialogue, §7.6.4.1's password, and a script's questions.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Command, DocumentId};
use viewer_host::native_window::{Asked, NativeWindow};

use super::window::named;
use super::{Answered, Host, with};

impl Host {
    /// RFC 0008 section 4.2's `app.alert` and `app.response`, as a dialogue titled with the
    /// document that asks: the script's text, the buttons its alert asked for or an entry holding
    /// its response's default, and a close answered as [`viewer_core::ScriptAnswer::dismissed`]
    /// (ADR 1628).
    ///
    /// Answered from GTK's event loop like every other question here, so the window's thread
    /// never waits on a person; the script waits in its worker.
    pub(super) fn ask_for_a_script(
        &mut self,
        document: DocumentId,
        question: &viewer_core::ScriptQuestion,
    ) {
        use viewer_core::{ScriptAnswer, ScriptQuestion};
        use viewer_host::script_asks;
        let name = self.state.documents.label_of(document);
        self.say(&script_asks::put(&name, question));
        let dialog = gtk4::Window::new();
        dialog.set_title(Some(&script_asks::title(&name, question)));
        dialog.set_modal(true);
        dialog.set_transient_for(Some(&self.ui.window));
        dialog.set_default_size(420, -1);
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        column.set_margin_top(12);
        column.set_margin_bottom(12);
        column.set_margin_start(12);
        column.set_margin_end(12);
        let text = gtk4::Label::new(Some(script_asks::text(question)));
        text.set_xalign(0.0);
        text.set_wrap(true);
        column.append(&text);
        let password = matches!(question, ScriptQuestion::Response { password: true, .. });
        // Whether it was answered lives beside the dialogue, for `put_a_question`'s reason.
        let answered = Rc::new(Cell::new(false));
        let answered_flag = Rc::clone(&answered);
        let answer: ScriptAnswered = Rc::new(move |host: &mut Self, answer: ScriptAnswer| {
            host.script_dialogue = None;
            host.say(&script_asks::answered(&answer, password));
            host.dispatch(Command::AnswerScript { document, answer });
        });
        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        buttons.set_halign(gtk4::Align::End);
        let (choices, entered) = script_choices(&column, question, password);
        let last = choices.len().saturating_sub(1);
        for (index, (label, chosen)) in choices.into_iter().enumerate() {
            let button = gtk4::Button::with_label(&label);
            if index == last {
                button.add_css_class("suggested-action");
                dialog.set_default_widget(Some(&button));
            }
            let me = self.me.clone();
            let dialogue = dialog.clone();
            let done = Rc::clone(&answered);
            let said = Rc::clone(&answer);
            button.connect_clicked(move |_| {
                done.set(true);
                dialogue.close();
                let given = chosen();
                with(&me, |host| said(host, given));
            });
            buttons.append(&button);
        }
        column.append(&buttons);
        dialog.set_child(Some(&column));
        // A dialogue closed without a press is the closed dialogue's answer, never silence: the
        // script is waiting on it (trap 5).
        let me = self.me.clone();
        let closed = ScriptAnswer::dismissed(question);
        dialog.connect_close_request(move |_| {
            if !answered.get() {
                let closed = closed.clone();
                with(&me, |host| answer(host, closed));
            }
            glib::Propagation::Proceed
        });
        closes_on_escape(&dialog);
        dialog.present();
        if let Some(entry) = entered {
            entry.grab_focus();
        }
        // A tick of no time when the wait runs out is the command after which the viewer asks
        // whether the question was withdrawn, so a dialogue nobody answers comes down on time
        // rather than at the person's next action (ADR 1643).
        let me = self.me.clone();
        glib::timeout_add_local_once(script_asks::wake_after(), move || {
            with(&me, |host| {
                if host.script_dialogue.is_some() {
                    host.dispatch(Command::Tick { millis: 0 });
                }
            });
        });
        self.script_dialogue = Some((document, dialog, Rc::clone(&answered_flag)));
    }

    /// [`Event::Beep`](viewer_core::Event::Beep): the display's one system sound, and the line that says which of the
    /// reference's five was asked for — or, with no display, the line that says none was played
    /// (ADR 1702).
    pub(super) fn beep(&self, document: DocumentId, sound: pdf_model::view::Sound) {
        let name = self.state.documents.label_of(document);
        if let Some(display) = gtk4::gdk::Display::default() {
            display.beep();
            self.say(&viewer_host::script_timers::played(&name, sound, "GDK"));
        } else {
            self.say(&viewer_host::script_timers::unplayed(
                &name,
                sound,
                "GDK has no display to sound",
            ));
        }
    }

    /// [`Event::ScriptQuestionWithdrawn`](viewer_core::Event::ScriptQuestionWithdrawn): the dialogue of that document's question comes down,
    /// marked answered first so that closing it does not answer the script again (ADR 1643).
    pub(super) fn withdraw_script_question(&mut self, document: DocumentId) {
        if let Some((asked, dialogue, answered)) = self.script_dialogue.take() {
            if asked == document {
                answered.set(true);
                dialogue.close();
            } else {
                self.script_dialogue = Some((asked, dialogue, answered));
            }
        }
        let name = self.state.documents.label_of(document);
        self.say(&viewer_host::script_asks::withdrawn(&name));
    }

    /// `CLAUDE.md`'s *ask* level, in a window of its own titled by what the question is about.
    ///
    /// Every subject is the same kind of question — this program holding something until a person
    /// says a word — so every one is the same dialogue, and what the answer does is
    /// `viewer_host::native_window`'s for every window (ADR 2105). The question is held beside the
    /// dialogue and taken by the first answer, so a window answered and then closed answers once.
    pub(super) fn put_the_question(&mut self, about: Asked, words: &viewer_host::Question) {
        let title = about.subject().title();
        let held = Cell::new(Some(about));
        self.put_a_question(
            title,
            words,
            Rc::new(move |host: &mut Self, proceed| {
                if let Some(about) = held.take() {
                    host.answer_the_question(about, proceed);
                }
            }),
        );
    }

    /// One question, two buttons, and whatever the person pressed handed to `answer`.
    ///
    /// **The modal shape §7.6.4.1's password already had**, and deliberately the same one: both
    /// are this program holding something until a person says a word, and a reader who has met one
    /// of them has met the other. What is different is the answer — two buttons rather than a
    /// text field — and what a dismissed window means, which is [`viewer_host::DO_NOT`] for the
    /// reason `viewer_host::restriction` records: going ahead on a question nobody answered would
    /// be the *off* level under another name.
    pub(super) fn put_a_question(
        &mut self,
        title: &str,
        words: &viewer_host::Question,
        answer: Answered,
    ) {
        let dialog = gtk4::Window::new();
        dialog.set_title(Some(title));
        dialog.set_modal(true);
        dialog.set_transient_for(Some(&self.ui.window));
        dialog.set_default_size(420, -1);
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        column.set_margin_top(12);
        column.set_margin_bottom(12);
        column.set_margin_start(12);
        column.set_margin_end(12);
        for (text, dim) in [(&words.reasons, false), (&words.choice, true)] {
            let label = gtk4::Label::new(Some(text));
            label.set_xalign(0.0);
            label.set_wrap(true);
            if dim {
                label.add_css_class("dim-label");
            }
            column.append(&label);
        }
        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        buttons.set_halign(gtk4::Align::End);
        // Whether it was answered lives beside the dialogue rather than on the host, for
        // `ask_for_a_password`'s reason: `GtkWindow::close` fires `close-request` synchronously,
        // so a flag the host set afterwards would still be false when the close handler read it.
        let answered = Rc::new(Cell::new(false));
        for (label, proceed) in [
            (viewer_host::restriction::DO_NOT, false),
            (viewer_host::restriction::GO_AHEAD, true),
        ] {
            let button = gtk4::Button::with_label(label);
            if proceed {
                button.add_css_class("suggested-action");
            }
            let me = self.me.clone();
            let dialogue = dialog.clone();
            let done = Rc::clone(&answered);
            let said = Rc::clone(&answer);
            button.connect_clicked(move |_| {
                done.set(true);
                dialogue.close();
                with(&me, |host| said(host, proceed));
            });
            buttons.append(&button);
        }
        column.append(&buttons);
        dialog.set_child(Some(&column));
        // Closing without pressing anything is a decline rather than silence — trap 5, in a window
        // a person walked away from — and it has to reach the core, which is holding the edit.
        let me = self.me.clone();
        dialog.connect_close_request(move |_| {
            if !answered.get() {
                with(&me, |host| answer(host, false));
            }
            glib::Propagation::Proceed
        });
        // A plain `GtkWindow` binds nothing, so Escape is bound by hand, exactly as the password
        // prompt binds it.
        let keys = gtk4::EventControllerKey::new();
        let dialogue = dialog.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                dialogue.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        dialog.add_controller(keys);
        dialog.present();
    }

    /// §7.6.4.1's prompt, in a window of the platform's own.
    ///
    /// The words are [`viewer_host::password`]'s so that three hosts ask the same question; the
    /// `gtk4::PasswordEntry` is this toolkit's answer to *what a password entry is* and is the
    /// whole of what this method adds.
    pub(super) fn ask_for_a_password(&mut self, document: DocumentId, attempt: u32, of: u32) {
        let dialog = gtk4::Window::new();
        dialog.set_title(Some("Password"));
        dialog.set_modal(true);
        dialog.set_transient_for(Some(&self.ui.window));
        dialog.set_default_size(360, -1);
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        column.set_margin_top(12);
        column.set_margin_bottom(12);
        column.set_margin_start(12);
        column.set_margin_end(12);
        let asked_about = match self.state.arrivals.named_mut(document) {
            Some(arriving) => viewer_host::documents::label(&arriving.named.path),
            None => named(&self.state.showing.path),
        };
        let words = viewer_host::password::prompt(&asked_about, attempt, of);
        let label = gtk4::Label::new(Some(&words.question));
        label.set_xalign(0.0);
        label.set_wrap(true);
        column.append(&label);
        let entry = gtk4::PasswordEntry::new();
        entry.set_show_peek_icon(true);
        entry.set_activates_default(true);
        column.append(&entry);
        let count = gtk4::Label::new(Some(&words.counted));
        count.set_xalign(0.0);
        count.add_css_class("dim-label");
        column.append(&count);
        let open = gtk4::Button::with_label("Open");
        open.add_css_class("suggested-action");
        column.append(&open);
        dialog.set_child(Some(&column));

        // **Whether the prompt was answered lives beside the dialogue rather than on the host**,
        // and that is not a style choice: `GtkWindow::close` fires `close-request`
        // *synchronously*, so a flag set inside the handler that runs after it is still false when
        // the close handler reads it — which is what happened here first time, and every password
        // supplied was reported as a decline.
        let answered = Rc::new(Cell::new(false));

        let me = self.me.clone();
        let dialogue = dialog.clone();
        let typed = entry.clone();
        let done = Rc::clone(&answered);
        open.connect_clicked(move |_| {
            let password = taken_from(&typed);
            done.set(true);
            dialogue.close();
            with(&me, |host| host.password_supplied(document, password));
        });
        let me = self.me.clone();
        let dialogue = dialog.clone();
        let done = Rc::clone(&answered);
        entry.connect_activate(move |typed| {
            let password = taken_from(typed);
            done.set(true);
            dialogue.close();
            with(&me, |host| host.password_supplied(document, password));
        });
        // Escape is the platform's own way out of a modal window, and closing it without typing has
        // to be a *decline* rather than silence — trap 5, in a window a person walked away from.
        // `close-request` fires for the button and for the key alike, so it answers only where
        // nothing was supplied through the two handlers above.
        let me = self.me.clone();
        dialog.connect_close_request(move |_| {
            if !answered.get() {
                with(&me, |host| {
                    host.say(viewer_host::password::CANCELLED);
                    host.cancelled(document);
                });
            }
            glib::Propagation::Proceed
        });
        // **Escape closes it, and this is the toolkit's own gap rather than an extra.** A
        // `GtkDialog` binds that key and is deprecated in the release this crate targets; a plain
        // `GtkWindow` binds nothing, so without this controller the only way out of the prompt is
        // the window manager's close button — and a reader with a keyboard could not decline at
        // all. Driven under `Xvfb`, where there is no window manager to have hidden it: the other
        // two hosts declined on Escape and this one did nothing.
        let dialogue = dialog.clone();
        let keys = gtk4::EventControllerKey::new();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                dialogue.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        dialog.add_controller(keys);
        dialog.present();
    }
}

/// §7.6.4.1's password out of the entry it was typed into, and out of the entry.
///
/// The widget is emptied on the way past. GTK's own buffer is not this program's to clear — a
/// `GtkPasswordEntry` holds a `GtkEntryBuffer` whose storage is glib's — so what this buys is that
/// the *live* widget stops holding it, which is the part a host can reach; the rest is
/// [`viewer_core::Secret`]'s and is documented there as best effort.
fn taken_from(entry: &gtk4::PasswordEntry) -> viewer_core::Secret {
    let password = viewer_core::Secret::from(entry.text().to_string());
    entry.set_text("");
    password
}

/// Binds Escape to closing a dialogue, which a plain `GtkWindow` does not, as the password prompt
/// binds it by hand.
fn closes_on_escape(dialog: &gtk4::Window) {
    let keys = gtk4::EventControllerKey::new();
    let dialogue = dialog.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            dialogue.close();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    dialog.add_controller(keys);
}

/// One button of a script's dialogue: its label, and the answer it gives when pressed.
type Choice = (String, Rc<dyn Fn() -> viewer_core::ScriptAnswer>);

/// What a script's dialogue offers under its text: an alert's icon as a word and its buttons, or a
/// response's label, its entry holding the default, and Cancel and OK — each in the order laid out,
/// the affirming one last (ADR 1628).
fn script_choices(
    column: &gtk4::Box,
    question: &viewer_core::ScriptQuestion,
    password: bool,
) -> (Vec<Choice>, Option<gtk4::Entry>) {
    use viewer_core::{AlertButton, ScriptAnswer, ScriptQuestion};
    let dim = |words: &str| {
        let label = gtk4::Label::new(Some(words));
        label.set_xalign(0.0);
        label.add_css_class("dim-label");
        label
    };
    match question {
        ScriptQuestion::Alert { icon, buttons, .. } => {
            column.prepend(&dim(icon.word()));
            let choices = buttons
                .buttons()
                .iter()
                .map(|button| {
                    let button = *button;
                    let pressed: Rc<dyn Fn() -> ScriptAnswer> =
                        Rc::new(move || ScriptAnswer::Pressed(button));
                    (button.label().to_owned(), pressed)
                })
                .collect();
            (choices, None)
        }
        ScriptQuestion::Response { default, label, .. } => {
            if let Some(label) = label {
                column.append(&dim(label));
            }
            let entry = gtk4::Entry::new();
            entry.set_text(default);
            entry.set_visibility(!password);
            entry.set_activates_default(true);
            column.append(&entry);
            let typed_from = entry.clone();
            let typed: Rc<dyn Fn() -> ScriptAnswer> =
                Rc::new(move || ScriptAnswer::Typed(Some(typed_from.text().to_string())));
            let cancelled: Rc<dyn Fn() -> ScriptAnswer> = Rc::new(|| ScriptAnswer::Typed(None));
            (
                vec![
                    (AlertButton::Cancel.label().to_owned(), cancelled),
                    (AlertButton::Ok.label().to_owned(), typed),
                ],
                Some(entry),
            )
        }
    }
}

/// What a script's dialogue does with the person's answer, once (ADR 1628).
type ScriptAnswered = Rc<dyn Fn(&mut Host, viewer_core::ScriptAnswer)>;
