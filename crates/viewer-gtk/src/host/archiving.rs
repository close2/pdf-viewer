//! ISO 19005's conversion chosen after launch: the six targets and the shipped profiles, in a
//! dialogue (ADR 1966).
//!
//! What it offers is [`viewer_host::archiving::Choosing`] unchanged — the list `quorra`'s card and
//! the Qt dialogue offer — and what comes back is the trait's
//! [`NativeWindow::archive_chosen`], so the window keeps the choice for its life and converts.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_host::archiving::{CHOOSING, CHOOSING_TITLE, Choosing, NOT_CHOSEN};
use viewer_host::native_window::NativeWindow;

use super::{Host, with};

impl Host {
    /// Puts the choice up: a drop-down of the targets over a list of the profiles, each with its
    /// sentence. The list has the keyboard, so the arrows move through the profiles and Enter
    /// converts, as on `quorra`'s card; closing it chooses nothing and says so.
    pub(super) fn offer_the_archive_choice(&mut self, choosing: Choosing) {
        let dialog = gtk4::Window::new();
        dialog.set_title(Some(CHOOSING_TITLE));
        dialog.set_modal(true);
        dialog.set_transient_for(Some(&self.ui.window));
        dialog.set_default_size(560, -1);
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        column.set_margin_top(12);
        column.set_margin_bottom(12);
        column.set_margin_start(12);
        column.set_margin_end(12);
        let about = gtk4::Label::new(Some(CHOOSING));
        about.set_xalign(0.0);
        about.set_wrap(true);
        about.add_css_class("dim-label");
        column.append(&about);

        let target = targets_of(&choosing);
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        row.append(&gtk4::Label::new(Some("Target")));
        row.append(&target);
        column.append(&row);
        let list = profiles_of(&choosing);
        column.append(&list);

        // Whether it was answered lives beside the dialogue, for `put_a_question`'s reason:
        // `GtkWindow::close` fires `close-request` synchronously.
        let answered = Rc::new(Cell::new(false));
        let choosing = Rc::new(RefCell::new(choosing));
        let convert: Rc<dyn Fn()> = {
            let (me, dialogue, done) = (self.me.clone(), dialog.clone(), Rc::clone(&answered));
            let (target, list) = (target.clone(), list.clone());
            Rc::new(move || {
                let mut chosen = choosing.borrow_mut();
                if let Ok(index) = usize::try_from(target.selected()) {
                    chosen.set_target(index);
                }
                if let Some(index) = list
                    .selected_row()
                    .and_then(|row| usize::try_from(row.index()).ok())
                {
                    chosen.set_profile(index);
                }
                let asked = chosen.chosen();
                done.set(true);
                dialogue.close();
                with(&me, |host| host.archive_chosen(asked));
            })
        };
        let on_row = Rc::clone(&convert);
        list.connect_row_activated(move |_, _| on_row());

        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        buttons.set_halign(gtk4::Align::End);
        let cancel = gtk4::Button::with_label("Cancel");
        let dialogue = dialog.clone();
        cancel.connect_clicked(move |_| dialogue.close());
        let go = gtk4::Button::with_label("Convert");
        go.add_css_class("suggested-action");
        go.connect_clicked(move |_| convert());
        buttons.append(&cancel);
        buttons.append(&go);
        column.append(&buttons);
        dialog.set_child(Some(&column));

        let me = self.me.clone();
        dialog.connect_close_request(move |_| {
            if !answered.get() {
                with(&me, |host| host.say(NOT_CHOSEN));
            }
            glib::Propagation::Proceed
        });
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
        if let Some(row) = list.selected_row() {
            row.grab_focus();
        }
        self.say(&format!("{CHOOSING_TITLE}: {CHOOSING}"));
    }
}

/// The six targets, as a drop-down on the one chosen.
fn targets_of(choosing: &Choosing) -> gtk4::DropDown {
    let targets: Vec<String> = choosing.targets().iter().map(ToString::to_string).collect();
    let named: Vec<&str> = targets.iter().map(String::as_str).collect();
    let target = gtk4::DropDown::from_strings(&named);
    target.set_selected(u32::try_from(choosing.target_index()).unwrap_or(0));
    target
}

/// The shipped profiles, each name over its sentence, with the chosen one selected.
fn profiles_of(choosing: &Choosing) -> gtk4::ListBox {
    let list = gtk4::ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::Single);
    for offered in choosing.profiles() {
        let entry = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        let name = gtk4::Label::new(None);
        name.set_markup(&format!(
            "<b>{}</b>",
            glib::markup_escape_text(offered.name)
        ));
        name.set_xalign(0.0);
        let sentence = gtk4::Label::new(Some(&offered.sentence));
        sentence.set_xalign(0.0);
        sentence.set_wrap(true);
        sentence.add_css_class("dim-label");
        entry.append(&name);
        entry.append(&sentence);
        list.append(&entry);
    }
    let first = i32::try_from(choosing.profile_index())
        .ok()
        .and_then(|index| list.row_at_index(index));
    list.select_row(first.as_ref());
    list
}
