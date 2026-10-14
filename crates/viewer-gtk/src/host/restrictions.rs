//! The levels menu: a document's restrictions and the acts it asks this machine to do, each at the four levels `CLAUDE.md` names.

use std::cell::RefCell;
use std::rc::Weak;

use gtk4::prelude::*;
use gtk4::{gio, glib};

use super::{Host, with};

impl Host {
    /// Fills the restrictions menu in, immediately before it is shown.
    ///
    /// **Built here rather than in `build_window`**, which is `CLAUDE.md` section 2's rule and
    /// also the only way the ticks can be right: the levels change while the window is up, and a
    /// model built once would show the policy the program launched with. What it costs is one
    /// walk over 62 entries when a person opens a menu, which is on no path a page takes.
    fn fill_the_restrictions_menu(&mut self) {
        let (model, actions) = restrictions_menu(&self.me, self.restrictions);
        self.ui
            .window
            .insert_action_group(MENU_ACTIONS, Some(&actions));
        self.ui.menu.set_menu_model(Some(&model));
    }

    /// A person picked the level of an act a document asks this machine to do: kept here, and
    /// the viewer told only where the act is one it does — a script runs in its view state (ADRs
    /// 1291, 1331, 1616).
    fn chose_act(&mut self, level: viewer_host::ActLevel) {
        self.restrictions.set(level);
        if let Some(command) = viewer_host::told(level) {
            self.dispatch(command);
        }
        self.say(&viewer_host::act_chosen(level));
    }

    /// A level a person picked out of the menu, sent to the viewer and kept for the next opening.
    fn chose_restriction(&mut self, chose: viewer_host::Chose) {
        let command = self.restrictions.chose(chose);
        self.dispatch(command);
        self.say(&viewer_host::restriction::chosen(chose));
    }
}

/// The action group the restrictions menu's entries live in.
///
/// One prefix, inserted on the window when the menu is filled in, so that the model's detailed
/// action names resolve: a `GtkMenuButton`'s popover looks its actions up through the widget
/// hierarchy, and the window is what every popover of it is inside.
const MENU_ACTIONS: &str = "restrict";

/// `CLAUDE.md`'s four levels as a GTK menu: a submenu per scope, a submenu per operation, and one
/// stateful action per operation so that the levels are radio entries rather than a list of words.
///
/// **Two loops rather than a walk over a flat list**, because `viewer_host::Restrictions`
/// enumerates both — `Scope::ALL` and `RestrictionPolicy::OPERATIONS` — and asking it for one
/// group's entries at a time is what keeps the nesting out of the shared crate and out of the
/// other two windows (ADR 1145). `viewer-ui`, which draws one flat card, takes
/// `Restrictions::rows` instead.
fn restrictions_menu(
    me: &Weak<RefCell<Host>>,
    restrictions: viewer_host::Restrictions,
) -> (gio::Menu, gio::SimpleActionGroup) {
    let actions = gio::SimpleActionGroup::new();
    let model = gio::Menu::new();
    for (index, scope) in viewer_host::Scope::ALL.into_iter().enumerate() {
        let per_scope = gio::Menu::new();
        for operation in viewer_core::RestrictionPolicy::OPERATIONS {
            let word = viewer_core::RestrictionPolicy::word(operation);
            let entries = restrictions.entries(scope, operation);
            // The action's name is positional rather than made of the words, because a name may
            // hold only letters, digits, `-` and `.` — and the operations' words are a person's
            // vocabulary rather than this toolkit's.
            let name = format!("s{index}-{word}");
            let chosen = entries
                .iter()
                .find(|entry| entry.chosen)
                .map_or("", |entry| entry.label);
            let action = gio::SimpleAction::new_stateful(
                &name,
                Some(glib::VariantTy::STRING),
                &chosen.to_variant(),
            );
            let picked: Vec<(&'static str, viewer_host::Chose)> = entries
                .iter()
                .map(|entry| (entry.label, entry.chose))
                .collect();
            let listener = me.clone();
            action.connect_activate(move |action, parameter| {
                let Some(target) = parameter.and_then(glib::Variant::str) else {
                    return;
                };
                let Some((_, chose)) = picked.iter().find(|(label, _)| *label == target) else {
                    return;
                };
                action.set_state(&target.to_variant());
                let chose = *chose;
                with(&listener, |host| host.chose_restriction(chose));
            });
            actions.add_action(&action);
            let per_operation = gio::Menu::new();
            for entry in &entries {
                let item = gio::MenuItem::new(Some(entry.label), None);
                item.set_action_and_target_value(
                    Some(&format!("{MENU_ACTIONS}.{name}")),
                    Some(&entry.label.to_variant()),
                );
                per_operation.append_item(&item);
            }
            let note = viewer_host::restriction::inert(operation);
            let label = if note.is_empty() {
                word.to_owned()
            } else {
                format!("{word} ({note})")
            };
            per_scope.append_submenu(Some(&label), &per_operation);
        }
        let note = scope.note();
        let label = if note.is_empty() {
            scope.label().to_owned()
        } else {
            format!("{} — {note}", scope.label())
        };
        model.append_submenu(Some(&label), &per_scope);
    }
    let per_machine = gio::Menu::new();
    for act in viewer_host::Act::ALL {
        per_machine.append_submenu(
            Some(act.label()),
            &act_menu(me, restrictions, act, &actions),
        );
    }
    model.append_submenu(
        Some(&format!(
            "{} — {}",
            viewer_host::restriction::MACHINE,
            viewer_host::restriction::MACHINE_NOTE
        )),
        &per_machine,
    );
    (model, actions)
}

/// What a document may ask this machine to do, one act at a time — §12.7.6.2's submission and
/// §O.2.1's `ef` — whose four levels are a value this host reads rather than a command the viewer
/// takes (ADRs 1291, 1331). One stateful action per act, so the levels are radio entries as the
/// restrictions' are.
fn act_menu(
    me: &Weak<RefCell<Host>>,
    restrictions: viewer_host::Restrictions,
    act: viewer_host::Act,
    actions: &gio::SimpleActionGroup,
) -> gio::Menu {
    let entries = restrictions.act_entries(act);
    let chosen = entries
        .iter()
        .find(|entry| entry.chosen)
        .map_or("", |entry| entry.label);
    // Positional for `restrictions_menu`'s reason: an action's name may not hold a space.
    let name = match act {
        viewer_host::Act::Submitting => "sending",
        viewer_host::Act::OpeningEmbedded => "embedded",
        viewer_host::Act::RunningScripts => "scripts",
    };
    let action =
        gio::SimpleAction::new_stateful(name, Some(glib::VariantTy::STRING), &chosen.to_variant());
    let picked: Vec<(&'static str, viewer_host::ActLevel)> = entries
        .iter()
        .map(|entry| (entry.label, entry.level))
        .collect();
    let listener = me.clone();
    action.connect_activate(move |action, parameter| {
        let Some(target) = parameter.and_then(glib::Variant::str) else {
            return;
        };
        let Some((_, level)) = picked.iter().find(|(label, _)| *label == target) else {
            return;
        };
        action.set_state(&target.to_variant());
        let level = *level;
        with(&listener, |host| host.chose_act(level));
    });
    actions.add_action(&action);
    let per_act = gio::Menu::new();
    for entry in &entries {
        let item = gio::MenuItem::new(Some(entry.label), None);
        item.set_action_and_target_value(
            Some(&format!("{MENU_ACTIONS}.{name}")),
            Some(&entry.label.to_variant()),
        );
        per_act.append_item(&item);
    }
    per_act
}

/// The menu button, empty until somebody opens it.
///
/// **No model and no actions here**, which is the launch path's rule and also the only way the
/// ticks can be right: what the menu holds is a function of a policy that changes while the window
/// is up. `set_create_popup_func` is GTK4's own hook for exactly this — it runs immediately before
/// the popover is shown — so what this costs at startup is one widget.
pub(super) fn menu_button(me: &Weak<RefCell<Host>>) -> gtk4::MenuButton {
    let button = gtk4::MenuButton::new();
    button.set_label("Restrictions");
    button.set_tooltip_text(Some(
        "what this reader does with what a document asserts over it (CLAUDE.md's four levels)",
    ));
    let listener = me.clone();
    button.set_create_popup_func(move |_| {
        with(&listener, Host::fill_the_restrictions_menu);
    });
    button
}
