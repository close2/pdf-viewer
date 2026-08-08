//! [`PanelRow`]s in a real platform tree.
//!
//! `doc/todo/30`'s first item: "a platform tree view over `Query::Outline`, `Query::Layers` and
//! `Query::Attachments`, rather than `viewer_ui::chrome`'s own drawn sidebar. The chrome exists to
//! prove the queries answer enough; a native host is the proof they answer enough for somebody
//! else's widgets."
//!
//! The widgets are GTK4's modern list stack — [`gtk4::ListView`] over a
//! [`gtk4::TreeListModel`], with a [`gtk4::TreeExpander`] per row — rather than `GtkTreeView`,
//! which GTK deprecated in 4.10. That is not taste: this workspace turns warnings into errors in
//! CI, so a deprecated widget would not build.
//!
//! **Rows build their own widgets in `bind` rather than recycling ones built in `setup`.** The
//! recycling shape is what a list of a million rows needs, and it costs a signal handler whose
//! lifetime has to be managed by hand — the handler on a layer's switch must forget the row it
//! was bound to. An outline is tens to thousands of rows and only the *visible* ones are ever
//! bound, so the trade goes the other way here: fresh widgets, and a handler that cannot outlive
//! the row it names.

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gio, glib, pango};

use crate::panel::{PanelRow, RowAction};

/// How many rows the initial expansion will open before it stops.
///
/// §12.3.3 lets a document ask for an outline to open — the sign of `/Count` — and ISO 32000-2's
/// own outline is nine hundred items, so "do what the file asked" has to have an end. A document
/// that asks for more than this gets the first of them opened and the rest closed, which is a
/// tree a person can still open by hand.
const EXPANSION_LIMIT: u32 = 4096;

/// A scrollable tree over these rows, with `act` called for every row a person acts on.
pub(crate) fn tree(rows: &[PanelRow], act: &Rc<dyn Fn(&RowAction)>) -> gtk4::Widget {
    let model = gtk4::TreeListModel::new(store(rows), false, false, |parent| {
        let row = parent.downcast_ref::<glib::BoxedAnyObject>()?;
        let children = row.borrow::<PanelRow>().children.clone();
        if children.is_empty() {
            return None;
        }
        Some(store(&children).upcast())
    });
    open_what_the_document_asked_for(&model);

    let factory = gtk4::SignalListItemFactory::new();
    let act = Rc::clone(act);
    factory.connect_bind(move |_, item| bind(item, &act));

    let selection = gtk4::SingleSelection::new(Some(model));
    selection.set_autoselect(false);
    selection.set_can_unselect(true);
    let list = gtk4::ListView::new(Some(selection), Some(factory));

    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_child(Some(&list));
    scroller.set_vexpand(true);
    scroller.set_hexpand(true);
    scroller.upcast()
}

/// The rows of one level, as a list model.
fn store(rows: &[PanelRow]) -> gio::ListStore {
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    for row in rows {
        store.append(&glib::BoxedAnyObject::new(row.clone()));
    }
    store
}

/// Opens the rows whose document asked for them to be open, bounded by [`EXPANSION_LIMIT`].
///
/// Expanding a row inserts its children after it, so the scan walks forward over a model that
/// grows underneath it — which is what makes one pass enough for a whole subtree.
fn open_what_the_document_asked_for(model: &gtk4::TreeListModel) {
    let mut index = 0_u32;
    while index < model.n_items().min(EXPANSION_LIMIT) {
        if let Some(row) = model.row(index) {
            let wanted = row
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .is_some_and(|held| held.borrow::<PanelRow>().expanded);
            if wanted && !row.is_expanded() {
                row.set_expanded(true);
            }
        }
        index = index.saturating_add(1);
    }
}

/// Builds the widgets for one row and wires what acting on it does.
fn bind(item: &glib::Object, act: &Rc<dyn Fn(&RowAction)>) {
    let Some(item) = item.downcast_ref::<gtk4::ListItem>() else {
        return;
    };
    let Some(tree_row) = item.item().and_downcast::<gtk4::TreeListRow>() else {
        return;
    };
    let Some(held) = tree_row.item().and_downcast::<glib::BoxedAnyObject>() else {
        return;
    };
    let row = held.borrow::<PanelRow>().clone();

    let line = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    if let RowAction::Toggle { group, on, locked } = row.action {
        let switch = gtk4::CheckButton::new();
        switch.set_active(on);
        // Table 99's `/Locked`: "[t]he state of a locked group cannot be changed through the user
        // interface of an interactive PDF processor". An insensitive switch is the platform's own
        // way of saying so, and it is the clause obeyed rather than merely carried.
        switch.set_sensitive(!locked);
        // Connected *after* the state is set, so that showing the document's own answer is not
        // mistaken for a person changing it.
        let act = Rc::clone(act);
        switch.connect_toggled(move |switch| {
            act(&RowAction::Toggle {
                group,
                on: switch.is_active(),
                locked,
            });
        });
        line.append(&switch);
    }

    let text = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    // The text fills the rest of the row, so that §12.3.3's "[c]licking the text" is the whole
    // row rather than the width of the string — a title of four characters would otherwise be a
    // four-character target.
    text.set_hexpand(true);
    let label = gtk4::Label::new(Some(&row.label));
    label.set_xalign(0.0);
    label.set_ellipsize(pango::EllipsizeMode::End);
    text.append(&label);
    if let Some(detail) = row.detail.as_deref() {
        let second = gtk4::Label::new(Some(detail));
        second.set_xalign(0.0);
        second.set_ellipsize(pango::EllipsizeMode::End);
        second.add_css_class("dim-label");
        text.append(&second);
    }
    line.append(&text);

    let expander = gtk4::TreeExpander::new();
    expander.set_list_row(Some(&tree_row));
    expander.set_child(Some(&line));

    // §12.3.3: "[c]licking the text of any visible item activates the item". The gesture is on the
    // *text* rather than on the whole row, because a click on a layer's switch must change the
    // layer and not also activate the row the switch sits in.
    if matches!(
        row.action,
        RowAction::Activate(_) | RowAction::Extract { .. }
    ) {
        let click = gtk4::GestureClick::new();
        let act = Rc::clone(act);
        let action = row.action.clone();
        click.connect_released(move |_, _, _, _| act(&action));
        text.add_controller(click);
    }

    item.set_child(Some(&expander));
}
